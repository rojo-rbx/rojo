# Writing a custom Config File

Configuration Files for Prometheus are just lua modules, that return a single object, which contains the configuration. Let's say we have the following config file:

{% code title="config.lua" %}
```lua
return {
        -- The default LuaVersion is Lua51
        LuaVersion = "Lua51"; -- or "LuaU"
        -- All Variables will start with this prefix
        VarNamePrefix = "";
        -- Name Generator for Variables that look like this: b, a, c, D, t, G
        NameGenerator = "MangledShuffled";
        -- No pretty printing
        PrettyPrint = false;
        -- Seed is generated based on current time 
        -- When specifying a seed that is not 0, you will get the same output every time
        Seed = 0;
        -- Obfuscation steps
        Steps = {
            {
                -- This obfuscation step puts all constants into an array at the beginning of the code
                Name = "ConstantArray";
                Settings = {
                    -- Apply to Strings only
                    StringsOnly = true;
                    -- Apply to all Constants, 0.5 would only affect 50% of strings
                    Treshold    = 1;
                }
            },
        }
    }
```
{% endcode %}

One can now obfuscate a script using this configuration by running:

```batch
lua ./cli.lua --config config.lua hello_world.lua
```

You should get the following output:

{% code title="hello_world.obfuscated.lua" %}
```lua
local N={"Hello, World!"}local function k(k)return N[k+40058]end print(k(-40057))
```
{% endcode %}

As you can see, the only transformation that was applied to our Hello World example was putting all strings (in this case only `"Hello, World!"` ) into an array and creating a wrapper function for retrieving the value.

### How does the Config File work?

The config file is simply a lua file, that returns the configuration object. Please note that this lua file is sandboxed by Prometheus when loading the configuration, meaning that you can't use any predefined functions like `tostring` or libraries like `math`.

See [The Config Object](the-config-object.md) to learn what this configuration object consists of.
