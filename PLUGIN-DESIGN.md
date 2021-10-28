# User Plugins Design Proposal

## Background

This is a design proposal for the long-standing [#55](https://github.com/rojo-rbx/rojo/issues/55)
desire to add user plugins to Rojo for things like source file transformation and instance tree
transformation.

As discussed in [#55](https://github.com/rojo-rbx/rojo/issues/55) and as initially explored in
[#257](https://github.com/rojo-rbx/rojo/pull/257), plugins as Lua scripts seem to be a good starting
point. This is quite similar to the way Rollup.js plugins work, although they are implemented with
JS. Rollup.js is a bundler which actually performs a similar job to Rojo in the JS world by taking a
number of source files and converting them into a single output bundle.

This proposal takes strong influence from the [Rollup.js plugins
API](https://rollupjs.org/guide/en/#plugins-overview) which is a joy to use for both plugin
developers and end-users.

## Project file changes

Add a new top-level field to the project file format:

-   `plugins`: An array of [Plugin Descriptions](#plugin-description).
    -   **Optional**
    -   Default is `[]`

### Plugin description

Either a `String` value or an object with the fields:

-   `source`: A filepath to the Lua source file of the plugin or a URL to a GitHub repo, optionally
    followed by an `@` character and a Git tag.
    -   **Required**
-   `options`: Any JSON dictionary. The options that will be passed to the plugin.
    -   **Optional**
    -   Default is `{}`

In the case that the value is just a `String`, it is interpreted as an object with the `source`
field set to its value, and `options` set to its default.

### Example

```json
{
    "name": "ProjectWithPlugins",
    "plugins": [
        "local-plugin.lua",
        "github.com/owner/remote-plugin-from-tag@v1.0.0",
        "github.com/owner/remote-plugin-from-head",
        { "source": "plugin-with-options.lua", "options": { "some": "option" } }
    ],
    "tree": {
        "$className": "DataModel",
        "ServerScriptService": {
            "$className": "ServerScriptService",
            "$path": "src"
        }
    }
}
```

## Plugin scripts

Plugin scripts should return a `CreatePluginFunction`:

```luau
-- Types provided in luau format

type PluginInstance = {
  name: string,
  projectDescription?: (project: ProjectDescription) -> (),
  syncStart?: () -> (),
  syncEnd?: () -> (),
  resolve?: (id: string) -> string,
  middleware?: (id: string) -> string,
  load?: (id: string) -> string,
}

-- TODO: Define properly. For now, this is basically just the JSON converted to Lua
type ProjectDescription = { ... }

type CreatePluginFunction = (options: {[string]: any}) -> PluginInstance
```

In this way, plugins have the opportunity to customize their hooks based on the options provided by
the user in the project file.

## Plugin environment

The plugin environment is created in the following way:

1. Create a new Lua context.
1. Initialize an empty `_G.plugins` table.
1. For each plugin description in the project file:
    1. Convert the plugin options from the project file from JSON to a Lua table.
    1. If the `source` field is a GitHub URL, download the plugin directory from the repo with the
       specified version tag (if no tag, from the head of the default branch) into a local
       `.rojo-plugins` directory with the repo identifier as its name. It is recommended that users
       add `.rojo-plugins` to their `.gitignore` file. The root of the plugin will be called
       `main.lua`.
    1. Load and evaluate the file contents into the Lua context to get a handle to the
       `CreatePluginFunction`
    1. Call the `CreatePluginFunction` with the converted options to get a handle of the result.
    1. Push the result at the end of the `_G.plugins` table

If at any point there is an error in the above steps, Rojo should quit with an appropriate error
message.

## Plugin instance

-   `name`
    -   **Required**: The name of the plugin that will be used in error messages, etc.
-   `projectDescription(project: ProjectDescription) -> ()`
    -   **Optional**: Called with a Lua representation of the current project description whenever
        it has changed.
-   `syncStart() -> ()`
    -   **Optional**: A sync has started.
-   `syncEnd() -> ()`
    -   **Optional**: A sync has finished.
-   `resolve(id: string) -> string`
    -   **Optional**: Takes a file path and returns a new file path that the file should be loaded
        from instead. The first plugin to return a non-nil value per id wins.
-   `middleware(id: string) -> string`
    -   **Optional**: Takes a file path and returns a snapshot middleware enum to determine how Rojo
        should build the instance tree for the file. The first plugin to return a non-nil value per
        id wins.
-   `load(id: string) -> string`
    -   **Optional**: Takes a file path and returns the file contents that should be interpreted by
        Rojo. The first plugin to return a non-nil value per id wins.

## Use case analyses

To demonstrate the effectiveness of this API, pseudo-implementations for a variety of use-cases are
shown using the API.

### MoonScript transformation

Requested by:

-   @Airwarfare in [#170](https://github.com/rojo-rbx/rojo/issues/170)
-   @dimitriye98 in [#55](https://github.com/rojo-rbx/rojo/issues/55#issuecomment-402616429) (comment)

```lua
local parse = require 'moonscript.parse'
local compile = require 'moonscript.compile'

return function(options)
  return {
    name = "moonscript",
    load = function(id)
      if id:match('%.lua$') then
        local file = io.open(id, 'r')
        local source = file:read('a')
        file:close()

        local tree, err = parse.string(source)
        assert(tree, err)

        local lua, err, pos = compile.tree(tree)
        if not lua then error(compile.format_error(err, pos, source)) end

        return lua
      end
    end
  }
end
```

### Obfuscation/minifier transformation

Requested by:

-   @cmumme in [#55](https://github.com/rojo-rbx/rojo/issues/55#issuecomment-794801625) (comment)
-   @blake-mealey in [#382](https://github.com/rojo-rbx/rojo/issues/382)

```lua
local minifier = require 'minifier.lua'

return function(options)
  return {
    name = "minifier",
    load = function(id)
      if id:match('%.lua$') then
        local file = io.open(id, 'r')
        local source = file:read('a')
        file:close()
        return minifier(source)
      end
    end
  }
end
```

### Markdown to Roblox rich text

```lua
-- Convert markdown to Roblox rich text format implementation here

return function(options)
    return {
        name = 'markdown-to-richtext',
        middleware = function(id)
            if id:match('%.md$') then
              return 'json_model'
            end
        end,
        load = function(id, contents)
            if id:match('%.md$') then
              local frontmatter = parseFrontmatter(contents)
              local richText = markdownToRichText(contents)
              local className = frontmatter.className or 'StringValue'
              local property = frontmatter.property or 'Value'
              return ('{"ClassName": "%s", "Properties": { "%s": "%s" }}')
                :format(className, property, richText)

              --[[
                With rojo plugin library:

                return rojo.toJson({
                  ClassName = className,
                  Properties = {
                    [property] = richText
                  }
                })
              ]]
            end
        end
    }
end
```

### Load custom files as StringValue instances

Requested by:

-   @rfrey-rbx in [#406](https://github.com/rojo-rbx/rojo/issues/406)
-   @Quenty in [#148](https://github.com/rojo-rbx/rojo/issues/148)

```lua
return function(options)
    options.extensions = options.extensions or {}

    return {
        name = 'load-as-stringvalue',
        middleware = function(id)
            local idExt = id:match('%.(%w+)$')
            for _, ext in next, options.extensions do
                if ext == idExt then
                    return 'json_model'
                end
            end
        end,
        load = function(id, contents)
            local idExt = id:match('%.(%w+)$')
            for _, ext in next, options.extensions do
                if ext == idExt then
                    local encoded = contents:gsub('\n', '\\n')
                    return ('{"ClassName": "StringValue", "Properties": { "Value": "%s" }}'):format(encoded)

                    --[[
                      With rojo plugin library:

                      return rojo.toJson({
                        ClassName = 'StringValue',
                        Properties = {
                          Value = encoded
                        }
                      })
                    ]]
                end
            end
        end
    }
end
```

```json
// default.project.json
{
  "plugins": [
    { "source": "load-as-stringvalue.lua", "options": { "extensions": {"md", "data.json"} }}
  ]
}
```

### Remote file requires

```lua
-- download/caching implementation inline here
-- this one is not really working even from a pseudo-implementation perspective

return function(options)
  return {
    name = "remote-require",
    resolve = function(id)
      if id:match('^https?://.*%.lua$') then
        local cachedId = fromCache(id)
        return cachedId or nil
      end
    end,
    load = function(id)
      if id:match('^https?://.*%.lua$') then
        local cachedId = downloadAndCache(id)
        local file = io.open(cachedId, 'r')
        local source = file:read('a')
        file:close()
        return source
      end
    end
  }
end
```

### File system requires

Requested by:

-   @blake-mealey in [#382](https://github.com/rojo-rbx/rojo/issues/382)

```lua
-- lua parsing/writing implementation here

return function(options)
  local project = nil

  return {
    name = "require-files",
    projectDescription = function(newProject)
      project = newProject
    end,
    load = function(id)
      if id:match('%.lua$') then
        local file = io.open(id, 'r')
        local source = file:read('a')
        file:close()

        -- This function will look for require 'file/path' statements in the source and replace
        -- them with Roblox require(instance.path) statements based on the project's configuration
        -- (where certain file paths are mounted)
        return replaceRequires(source, project)
      end
    end
  }
end
```

## Implementation priority

1. Loading plugins from local paths
2. Calling hooks at the appropriate time
3. Loading plugins from remote repos
4. Rojo plugin library

## Concerns

TODO: Implement a proposal for a rojo plugin library

-   Some operations will be common in plugins and a set of standardized functions may be helpful,
    for example reading files and checking file extensions. This could be provided as a global
    library injected in the initialization stage of the Lua context (e.g.
    `rojo.fileExtensionMatches(id, ext)`, `rojo.loadFile(id)`, `rojo.toJson(value)`).
