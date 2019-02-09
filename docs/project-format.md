[TOC]

## Project File

Rojo projects are JSON files that have the `.project.json` extension. They have these fields:

* `name`: A string indicating the name of the project.
    * This is only used for diagnostics.
* `tree`: An [Instance Description](#instance-description) describing the root instance of the project.

## Instance Description
Instance Descriptions correspond one-to-one with the actual Roblox Instances in the project. They can be specified directly in the project file or be pulled from the filesystem.

* `$className`: The ClassName of the Instance being described.
    * Optional if `$path` is specified.
* `$path`: The path on the filesystem to pull files from into the project.
    * Optional if `$className` is specified.
    * Paths are relative to the folder containing the project file.
* `$properties`: Properties to apply to the instance. Values should be [Instance Property Values](#instance-property-value).
    * Optional
* `$ignoreUnknownInstances`: Whether instances that Rojo doesn't know about should be deleted.
    * Optional
    * Default is `false` if `$path` is specified, otherwise `true`.

All other fields in an Instance Description are turned into instances whose name is the key. These values should also be Instance Descriptions!

Instance Descriptions are fairly verbose and strict. In the future, it'll be possible for Rojo to infer class names for known services like `Workspace`.

## Instance Property Value
The shape of Instance Property Values is defined by the [rbx_tree](https://github.com/LPGhatguy/rbx-tree) library, so it uses slightly different conventions than the rest of Rojo.

Each value should be an object with the following required fields:

* `Type`: The type of property to represent.
    * [Supported types can be found here](https://github.com/LPGhatguy/rbx-tree#property-type-coverage).
* `Value`: The value of the property.
    * The shape of this field depends on which property type is being used. `Vector3` and `Color3` values are both represented as a list of numbers, for example.

Instance Property Values are intentionally very strict. Rojo will eventually be able to infer types for you!

## Example Projects
This project bundles up everything in the `src` directory. It'd be suitable for making a plugin or model:

```json
{
    "name": "AwesomeLibrary",
    "tree": {
        "$path": "src"
    }
}
```

This project describes the layout you might use if you were making the next hit simulator game, *Sisyphus Simulator*:

```json
{
    "name": "Sisyphus Simulator",
    "tree": {
        "$className": "DataModel",

        "HttpService": {
            "$className": "HttpService",
            "$properties": {
                "HttpEnabled": {
                    "Type": "Bool",
                    "Value": true
                }
            }
        },

        "ReplicatedStorage": {
            "$className": "ReplicatedStorage",
            "$path": "src/ReplicatedStorage"
        },

        "StarterPlayer": {
            "$className": "StarterPlayer",

            "StarterPlayerScripts": {
                "$className": "StarterPlayerScripts",
                "$path": "src/StarterPlayerScripts"
            }
        },

        "Workspace": {
            "$className": "Workspace",
            "$properties": {
                "Gravity": {
                    "Type": "Float32",
                    "Value": 67.3
                }
            },

            "Terrain": {
                "$path": "Terrain.rbxm"
            }
        }
    }
}
```