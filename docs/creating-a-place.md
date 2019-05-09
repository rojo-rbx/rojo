[TOC]

## Creating the Rojo Project

To use Rojo to build a place, you'll need to create a new project file, which tells Rojo how your project is structured on-disk and in Roblox.

Create a new folder, then run `rojo init` inside that folder to initialize an empty project.

```sh
mkdir my-new-project
cd my-new-project

rojo init
```

Rojo will make a small project file in your directory, named `default.project.json`. It'll make sure that any code in the directory `src` will get put into `ReplicatedStorage.Source`.

Speaking of, let's make sure we create a directory named `src`, and maybe a Lua file inside of it:

```sh
mkdir src
echo 'print("Hello, world!")' > src/hello.lua
```

## Building Your Place
Now that we have a project, one thing we can do is build a Roblox place file for our project. This is a great way to get started with a project quickly with no fuss.

All we have to do is call `rojo build`:

```sh
rojo build -o MyNewProject.rbxlx
```

If you open `MyNewProject.rbxlx` in Roblox Studio now, you should see a `Folder` containing a `ModuleScript` under `ReplicatedStorage`!

!!! info
    To generate a binary place file instead, use `rbxl`. Note that support for binary model/place files (`rbxm` and `rbxl`) is very limited in Rojo presently.

## Live-Syncing into Studio
Building a place file is great for the initial build, but for actively working on your place, you'll want something quicker.

In Roblox Studio, make sure the Rojo plugin is installed. If you need it, check out [the installation guide](installation) to learn how to install it.

To expose your project to the plugin, you'll need to _serve_ it from the command line:

```sh
rojo serve
```

This will start up a web server that tells Roblox Studio what instances are in your project and sends notifications if any of them change.

Note the port number, then switch into Roblox Studio and press the Rojo **Connect** button in the plugins tab. Type in the port number, if necessary, and press **Start**.

If everything went well, you should now be able to change files in the `src` directory and watch them sync into Roblox Studio in real time!

## Uploading Your Place
Aimed at teams that want serious levels of automation, Rojo can upload places to Roblox.com automatically.

You'll need an existing place on Roblox.com as well as the `.ROBLOSECURITY` cookie of an account that has write access to that place.

!!! warning
    It's recommended that you set up a Roblox account dedicated to deploying your place instead of your personal account in case your security cookie is compromised.

Generating and uploading your place file is as simple as:

```sh
rojo upload --asset_id [PLACE ID] --cookie "[SECURITY COOKIE]"
```