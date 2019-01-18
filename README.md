<div align="center">
    <img src="assets/rojo-logo.png" alt="Rojo" height="217" />
</div>

<div>&nbsp;</div>

<div align="center">
    <a href="https://travis-ci.org/LPGhatguy/rojo">
        <img src="https://api.travis-ci.org/LPGhatguy/rojo.svg?branch=master" alt="Travis-CI Build Status" />
    </a>
    <a href="https://crates.io/crates/rojo">
        <img src="https://img.shields.io/crates/v/rojo.svg?label=version" alt="Latest server version" />
    </a>
    <a href="https://lpghatguy.github.io/rojo/0.4.x">
        <img src="https://img.shields.io/badge/docs-0.4.x-brightgreen.svg" alt="Rojo Documentation" />
    </a>
    <a href="https://lpghatguy.github.io/rojo/0.5.x">
        <img src="https://img.shields.io/badge/docs-0.5.x-brightgreen.svg" alt="Rojo Documentation" />
    </a>
</div>

<hr />

**Rojo** is a flexible multi-tool designed for creating robust Roblox projects.

It lets Roblox developers use industry-leading tools like Git and VS Code, and crucial utilities like Luacheck.

Rojo is designed for **power users** who want to use the **best tools available** for building games, libraries, and plugins.

## Features
Rojo lets you:

* Work on scripts from the filesystem, in your favorite editor
* Version your place, model, or plugin using Git or another VCS
* Sync `rbxmx` and `rbxm` models into your game in real time
* Package and deploy your project to Roblox.com from the command line

Soon, Rojo will be able to:

* Sync instances from Roblox Studio to the filesystem
* Compile MoonScript and other custom things for your project

## [Documentation](https://lpghatguy.github.io/rojo/0.4.x)
You can also view the documentation by browsing the [docs](https://github.com/LPGhatguy/rojo/tree/master/docs) folder of the repository, but because it uses a number of Markdown extensions, it may not be very readable.

## Inspiration and Alternatives
There are lots of other tools that sync scripts into Roblox or provide other tools for working with Roblox places.

Here are a few, if you're looking for alternatives or supplements to Rojo:

* [rbxmk by Anaminus](https://github.com/anaminus/rbxmk)
* [Rofresh by Osyris](https://github.com/osyrisrblx/rofresh)
* [RbxRefresh by Osyris](https://github.com/osyrisrblx/RbxRefresh)
* [Studio Bridge by Vocksel](https://github.com/vocksel/studio-bridge)
* [Elixir by Vocksel](https://github.com/vocksel/elixir)
* [RbxSync by evaera](https://github.com/evaera/RbxSync)
* [CodeSync by MemoryPenguin](https://github.com/MemoryPenguin/CodeSync)
* [rbx-exteditor by MemoryPenguin](https://github.com/MemoryPenguin/rbx-exteditor)

If you use a plugin that _isn't_ Rojo for syncing code, open an issue and let me know why! I'd like Rojo to be the end-all tool so that people stop reinventing solutions to this problem.

## Contributing
The `master` branch is a rewrite known as **Epiphany**. It includes a breaking change to the project configuration format and an infrastructure overhaul.

Pull requests are welcome!

All pull requests are run against a test suite on Travis CI. That test suite should always pass!

## License
Rojo is available under the terms of the Mozilla Public License, Version 2.0. See [LICENSE](LICENSE) for details.