[TOC]

## Project File
Rojo projects are JSON files that have the `.project.json` extension. They have the following fields:

* `name`: A string indicating the name of the project. This name is used when building the project into a model or place file.
    * **Required**
* `tree`: An [Instance Description](#instance-description) describing the root instance of the project.
    * **Required**
* `servePort`: The port that `rojo serve` should listen on. Passing `--port` will override this setting.
    * **Optional**
    * Default is `34872`
* `servePlaceIds`: A list of place IDs that this project may be live-synced to. This feature can help prevent overwriting the wrong game with source from Rojo.
    * **Optional**
    * Default is `null`

## Instance Description
Instance Descriptions correspond one-to-one with the actual Roblox Instances in the project.

* `$className`: The ClassName of the Instance being described.
    * **Optional if `$path` is specified.**
* `$path`: The path on the filesystem to pull files from into the project.
    * **Optional if `$className` is specified.**
    * Paths are relative to the folder containing the project file.
* `$properties`: Properties to apply to the instance. Values should be [Instance Property Values](#instance-property-value).
    * **Optional**
* `$ignoreUnknownInstances`: Whether instances that Rojo doesn't know about should be deleted.
    * **Optional**
    * Default is `false` if `$path` is specified, otherwise `true`.

All other fields in an Instance Description are turned into instances whose name is the key. These values should also be Instance Descriptions!

Instance Descriptions are fairly verbose and strict. In the future, it'll be possible for Rojo to [infer class names for known services like `Workspace`](https://github.com/LPGhatguy/rojo/issues/179).

## Instance Property Value
There are two kinds of property values on instances, **implicit** and **explicit**.

In the vast majority of cases, you should be able to use **implicit** property values. To use them, just use a value that's the same shape as the type that the property has:

```json
"MyPart": {
    "$className": "Part",
    "$properties": {
        "Size": [3, 5, 3],
        "Color": [0.5, 0, 0.5],
        "Anchored": true,
        "Material": "Granite"
    }
}
```

`Vector3` and `Color3` properties can just be arrays of numbers, as can types like `Vector2`, `CFrame`, and more!

Enums can be set to a string containing the enum variant. Rojo will raise an error if the string isn't a valid variant for the enum.

There are some cases where this syntax for assigning properties _doesn't_ work. In these cases, Rojo requires you to use the **explicit** property syntax.

Some reasons why you might need to use an **explicit** property:

* Using exotic property types like `BinaryString`
* Using properties added to Roblox recently that Rojo doesn't know about yet

The shape of explicit property values is defined by the [rbx-dom](https://github.com/LPGhatguy/rbx-dom) library, so it uses slightly different conventions than the rest of Rojo.

Each value should be an object with the following required fields:

* `Type`: The type of property to represent.
    * [Supported types can be found here](https://github.com/LPGhatguy/rbx-tree#property-type-coverage).
* `Value`: The value of the property.
    * The shape of this field depends on which property type is being used. `Vector3` and `Color3` values are both represented as a list of numbers, while `BinaryString` expects a base64-encoded string, for example.

Here's the same object, but with explicit properties:

```json
"MyPart": {
    "$className": "Part",
    "$properties": {
        "Size": {
            "Type": "Vector3",
            "Value": [3, 5, 3]
        },
        "Color": {
            "Type": "Color3",
            "Value": [0.5, 0, 0.5]
        },
        "Anchored": {
            "Type": "Bool",
            "Value": true
        },
        "Material": {
            "Type": "Enum",
            "Value": 832
        }
    }
}
```

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
                "HttpEnabled": true
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
                "Gravity": 67.3
            },

            "Terrain": {
                "$path": "Terrain.rbxm"
            }
        }
    }
}
```