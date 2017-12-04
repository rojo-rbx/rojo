<h1 align="center">Rojo</h1>
<div align="center">
	<a href="https://travis-ci.org/LPGhatguy/Rojo">
		<img src="https://api.travis-ci.org/LPGhatguy/Rojo.svg?branch=master" alt="Travis-CI Build Status" />
	</a>
</div>

<div>&nbsp;</div>

Rojo is a flexible multi-tool designed for creating robust Roblox projects. It's in early development, but is still useful for many projects.

It's designed for power users who want to use the **best tools available** for building games, libraries, and plugins.

## Features

Rojo has a number of desirable features *right now*:

* Work on scripts from the filesystem, in your favorite editor
* Version your place, library, or plugin using Git or another VCS

Soon, Rojo will be able to:

* Sync Roblox objects (including models) bi-directionally between the filesystem and Roblox Studio
* Create installation scripts for libraries to be used in standalone places
	* Similar to [rbxpacker](https://github.com/LPGhatguy/rbxpacker), another one of my projects
* Add strongly-versioned dependencies to your project

## Installation
Rojo has two components:
* The command line tool, written in Rust
* The [Roblox Studio plugin](https://www.roblox.com/library/1211549683/Rojo-v0-0-0), written in Lua

To install the command line tool, there are two options:
* Cargo, if you have Rust installed
	* Use `cargo install rojo` -- Rojo will be available with the `rojo` command
* Download a pre-built Windows binary from [the GitHub releases page](https://github.com/LPGhatguy/rojo/releases)

## Usage
For more help, use `rojo help`.

### New Project
Just create a new folder and tell Rojo to initialize it!

```sh
mkdir my-new-project
cd my-new-project

rojo init
```

Rojo will create an empty project in the directory.

The default project looks like this:

```json
{
  "name": "my-new-project",
  "servePort": 8000,
  "partitions": {}
}
```

### Start Dev Server
To create a server that allows the Rojo Dev Plugin to access your project, use:

```sh
rojo serve
```

The tool will tell you whether it found an existing project. You should then be able to connect and use the project from within Roblox Studio!

### Migrating an Existing Roblox Project
**Coming soon!**

### Syncing into Roblox
In order to sync code into Roblox, you'll need to add one or more "partitions" to your configuration. A partition tells Rojo how to map directories to Roblox objects.

Each entry in the partitions table has a unique name, a filesystem path, and the full name of the Roblox object to sync into.

For example, if you want to map your `src` directory to an object named `My Cool Game` in `ReplicatedStorage`, you could use this configuration:

```json
{
  "name": "rojo",
  "servePort": 8000,
  "partitions": {
    "game": {
      "path": "src",
      "target": "ReplicatedStorage.My Cool Game"
    }
  }
}
```

The `path` parameter is relative to the project file.

The `target` starts at `game` and crawls down the tree. If any objects don't exist along the way, they'll be created as `Folder` instances.

Run `rojo serve` in the directory containing this project, then press the "Sync In" or "Toggle Polling" buttons in the Roblox Studio plugin to move code into your game.

### Sync Details
The structure of files and folders on the filesystem are preserved when syncing into game.

Creation of Roblox instances follows a simple set of rules. The first rule that matches the file name is chosen:

| File Name      | Instance Type  | Notes                                     |
| -------------- | -------------- | ----------------------------------------- |
| `*.server.lua` | `Script`       | `Source` will contain the file's contents |
| `*.client.lua` | `LocalScript`  | `Source` will contain the file's contents |
| `*.lua`        | `ModuleScript` | `Source` will contain the file's contents |
| `*`            | `StringValue`  | `Value` will contain the file's contents  |

Any folders on the filesystem will turn into `Folder` objects unless they contain a file named `init.lua`, `init.server.lua`, or `init.client.lua`. Following the convention of Lua, those objects will instead be whatever the `init` file would turn into.

For example, this file tree:

* my-game
	* init.client.lua
	* foo.lua

Will turn into this tree in Roblox:

* `my-game` (`LocalScript` with source from `my-game/init.client.lua`)
	* `foo` (`ModuleScript` with source from `my-game/foo.lua`)

## Inspiration
There are lots of other tools that sync scripts into Roblox, or otherwise work to improve the development flow outside of Roblox Studio.

Here are a few, if you're looking for alternatives or supplements to Rojo:
* [Studio Bridge by Vocksel](https://github.com/vocksel/studio-bridge)
* [RbxRefresh by Osyris](https://github.com/osyrisrblx/RbxRefresh)
* [RbxSync by evaera](https://github.com/evaera/RbxSync)
* [CodeSync](https://github.com/MemoryPenguin/CodeSync) and [rbx-exteditor](https://github.com/MemoryPenguin/rbx-exteditor) by [MemoryPenguin](https://github.com/MemoryPenguin)
* [rbxmk by Anaminus](https://github.com/anaminus/rbxmk)

I also have a couple tools that Rojo intends to replace:
* [rbxfs](https://github.com/LPGhatguy/rbxfs), which has been deprecated by Rojo
* [rbxpacker](https://github.com/LPGhatguy/rbxpacker), which is still useful

## License
Rojo is available under the terms of the MIT license. See [LICENSE.md](LICENSE.md) for details.