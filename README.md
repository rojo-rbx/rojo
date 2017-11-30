<h1 align="center">Rojo</h1>
<div align="center">
	<a href="https://travis-ci.org/LPGhatguy/Rojo">
		<img src="https://api.travis-ci.org/LPGhatguy/Rojo.svg?branch=master" alt="Travis-CI Build Status" />
	</a>
	<a href="#">
		<img src="https://img.shields.io/badge/docs-soon-red.svg" alt="Documentation" />
	</a>
</div>

<div>&nbsp;</div>

**EARLY DEVELOPMENT, USE WITH CARE**

Rojo is a flexible multi-tool designed for creating robust Roblox projects.

It's designed for power users who want to use the best tools available for building games, libraries, and plugins.

It has a number of desirable features *right now*:

* Work from the filesystem, in your favorite editor
* Version your place, library, or plugin using Git or another VCS

Soon, Rojo will be able to:

* Create installation scripts for libraries to be used in standalone places
	* Similar to [rbxpacker](https://github.com/LPGhatguy/rbxpacker), another one of my projects
* Add strongly-versioned dependencies to your project

## Installation
Rojo has two components:
* The binary, written in Rust
* The [Roblox Studio plugin](https://www.roblox.com/library/1211549683/Rojo-v0-0-0), written in Lua

To install the binary, there are two options:
* Cargo, which requires you to have Rust installed
* Pre-built binaries from the [the GitHub releases page](https://github.com/LPGhatguy/rojo/releases)

### Cargo (Recommended)
Make sure you have [Rust 1.21 or newer](https://www.rust-lang.org/) installed.

Install Rojo using:

```sh
cargo install rojo

# Installed!
rojo help
```

### Pre-Built (Windows only)
Download the latest binary from [the GitHub releases page](https://github.com/LPGhatguy/rojo/releases). Put it somewhere you can access it from a terminal!

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

### Migrating an Existing Roblox Project
Coming soon!

## License
Rojo is available under the terms of the MIT license. See [LICENSE.md](LICENSE.md) for details.