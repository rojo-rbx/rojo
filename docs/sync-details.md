# Sync Details
This page aims to describe how Rojo turns files on the filesystem into Roblox objects.

## Folders
Any directory on the filesystem will turn into a `Folder` instance in Roblox, unless that folder matches the name of a service or other existing instance. In those cases, that instance will be preserved.

## Scripts
Rojo can represent `ModuleScript`, `Script`, and `LocalScript` objects. The default script type is `ModuleScript`, since most scripts in well-structued Roblox projects will be modules.

| File Name      | Instance Type  |
| -------------- | -------------- |
| `*.server.lua` | `Script`       |
| `*.client.lua` | `LocalScript`  |
| `*.lua`        | `ModuleScript` |

If a directory contains a file named `init.server.lua`, `init.client.lua`, or `init.lua`, that folder will be transformed into a `*Script` instance with the conents of the `init` file. This can be used to create scripts inside of scripts.

For example, this file tree:

* my-game
    * init.client.lua
    * foo.lua

Will turn into these instances in Roblox:

![Example of Roblox instances](/images/sync-example.png)

## Models
Rojo supports a JSON model format for representing simple models. It's designed for instance types like `BindableEvent` or `Value` objects, and is not suitable for larger models.

Rojo JSON models are stored in `.model.json` files.

Starting in Rojo version **0.4.10**, model files named `init.model.json` that are located in folders will replace that folder, much like Rojo's `init.lua` support. This can be useful to version instances like `Tool` that tend to contain several instances as well as one or more scripts.

!!! info
    In the future, Rojo will support `.rbxmx` models. See [issue #7](https://github.com/LPGhatguy/rojo/issues/7) for more details and updates on this feature.

!!! warning
    Prior to Rojo version **0.4.9**, the `Properties` and `Children` properties are required on all instances in JSON models!

JSON model files are fairly strict; any syntax errors will cause the model to fail to sync! They look like this:

`hello.model.json`
```json
{
    "Name": "hello",
    "ClassName": "Model",
    "Children": [
        {
            "Name": "Some Part",
            "ClassName": "Part"
        },
        {
            "Name": "Some StringValue",
            "ClassName": "StringValue",
            "Properties": {
                "Value": {
                    "Type": "String",
                    "Value": "Hello, world!"
                }
            }
        }
    ]
}
```