# The Config Object

Prometheus takes a configuration objetct. In this object there can be many properties applied.   \
The following table provides an overview:

| Property      | type    | possible values                              | default           |
| ------------- | ------- | -------------------------------------------- | ----------------- |
| LuaVersion    | string  | "Lua51", "LuaU"                              | "Lua51"           |
| PrettyPrint   | boolean | true, false                                  | false             |
| VarNamePrefix | string  | any                                          | ""                |
| NameGenerator | string  | "Mangled", "MangledShuffled", "Il", "Number" | "MangledShuffled" |
| Seed          | number  | any                                          | 0                 |
| Steps         | table   | StepConfig\[]                                | {}                |

As this table shows, all properties in the config object are optional as they have a default value.

As an example, here is the code for the minify preset:

```lua
{
        -- The default LuaVersion is Lua51
        LuaVersion = "Lua51";
        -- For minifying no VarNamePrefix is applied
        VarNamePrefix = "";
        -- Name Generator for Variables
        NameGenerator = "MangledShuffled";
        -- No pretty printing
        PrettyPrint = false;
        -- Seed is generated based on current time
        Seed = 0;
        -- No obfuscation steps
        Steps = {
        
        }
    };
```

### Steps

The most important property is the Steps property. This property must be a table of so called Step Configs. A Step in Prometheus describes a single transformation applied to your script by the Prometheus obfuscation pipeline. A StepConfiguration consists of the Name of the Step as well as settings for the step. All Steps will later be applied in the order they are defined. A single Step can be defined twice and will then be applied twice.

```lua
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
```

Under [Steps](broken-reference), you can find all current Steps, their names as well as the possible options.
