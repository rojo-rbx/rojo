This page aims to describe how Rojo turns files on the filesystem into Roblox objects.

## Overview
| File Name      | Instance Type       |
| -------------- | ------------------- |
| any directory  | `Folder`            |
| `*.server.lua` | `Script`            |
| `*.client.lua` | `LocalScript`       |
| `*.lua`        | `ModuleScript`      |
| `*.csv`        | `LocalizationTable` |
| `*.txt`        | `StringValue`       |

## Folders
Any directory on the filesystem will turn into a `Folder` instance unless it contains an 'init' script, described below.

## Scripts
The default script type in Rojo projects is `ModuleScript`, since most scripts in well-structued Roblox projects will be modules.

If a directory contains a file named `init.server.lua`, `init.client.lua`, or `init.lua`, that folder will be transformed into a `*Script` instance with the contents of the 'init' file. This can be used to create scripts inside of scripts.

For example, these files:

* my-game
    * init.client.lua
    * foo.lua

Will turn into these instances in Roblox:

![Example of Roblox instances](/images/sync-example.png)

## Localization Tables
Any CSV files are transformed into `LocalizationTable` instances. Rojo expects these files to follow the same format that Roblox does when importing and exporting localization information.

## Plain Text Files
Plain text files (`.txt`) files are transformed into `StringValue` instances. This is useful for bringing in text data that can be read by scripts at runtime.