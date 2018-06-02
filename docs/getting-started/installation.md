# Installation
Rojo has two components:

* The server, a binary written in Rust
* The plugin, a Roblox Studio plugin written in Lua

It's important that the plugin and server are compatible. The plugin will show errors in the Roblox Studio Output window if there is a version mismatch.

## Installing the Server
To install the server, either:

* If you have Rust installed, use `cargo install rojo`
* Or, download a pre-built Windows binary from [the GitHub releases page](https://github.com/LPGhatguy/rojo/releases)

**The Rojo binary must be run from the command line, like Terminal on MacOS or `cmd.exe` on Windows. It's recommended that you put the Rojo binary on your `PATH` to make this easier.**

## Installing the Plugin
To install the plugin, either:

* Install the plugin from the [Roblox plugin page](https://www.roblox.com/library/1211549683/Rojo).
  * This gives you less control over what version you install -- you will always have the latest version.
* Or, download the latest release from [the GitHub releases section](https://github.com/LPGhatguy/rojo/releases) and install it into your Roblox plugins folder
  * You can open this folder by clicking the "Plugins Folder" button from the Plugins toolbar in Roblox Studio

## Visual Studio Code Extension
If you use Visual Studio Code on Windows, you can install [Evaera's unofficial Rojo extension](https://marketplace.visualstudio.com/items?itemName=evaera.vscode-rojo), which will install both halves of Rojo for you. It even has a nifty UI to add partitions and start/stop the Rojo server!