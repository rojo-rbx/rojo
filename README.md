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

Rojo is a flexible multi-tool designed for creating robust Roblox projects.

It's designed for power users who want to use the best tools available for building games, libraries, and plugins.

It has a number of desirable features:

* Work from the filesystem, in your favorite editor
* Version your place, library, or plugin using Git or another VCS
* Create installation scripts for libraries to be used in standalone places
* Add strongly-versioned dependencies to your project

## Installation

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

Rojo will ask you questions to get your project configured correctly.

### Migrating an Existing Roblox Project
Coming soon!

## License
Rojo is available under the terms of the MIT license. See [LICENSE.md](LICENSE.md) for details.