# Architecture

Rojo is a rather large project with a bunch of moving parts. While it's not too complicated in practice, it tends to be overwhelming because it's a fair bit of Rust and not very clear where to begin.

This document is a "what the heck is going on" level view of Rojo and the codebase that's written to make it more reasonable to jump into something. It won't go too into depth on *how* something is done, but it will go into depth on *what* is being done.

## Overarching

Rojo is divided into two main pieces: the server and the plugin. The server is what's ran on your computer (whether it be via the terminal or the visual studio code extension), with the plugin serving as its main client.

When serving a project, the server gathers data on all of the files in that project, puts it into a nice format, and then sends it to the plugin. Then, when something changes on the file system, it does the same thing for only the changed files and sends them to the plugin.

When it receives a patch (whether it be the initial patch or any subsequent ones), the plugin reads through it and attempts to to apply it. Any sugar (the patch visualizer, as an example) happens on top of the patches received from the server.

## Server

Rojo's server component is divided into a few distinct pieces:

- The web server
- The CLI
- The snapshotting system

### The CLI

The Command Line Interface (CLI) of Rojo is the only interface for the program. It's initialized in `main.rs` but is hosted in `src/cli`.

Each command for the CLI is hosted in its own file, with the `mod.rs` file for the `cli` module handling parsing and running each command. The commands are mostly self-contained, though may also interface with Rojo's other code when necessary.

Specifically, they may interface with the web server and snapshotting system.

### The Snapshotting System

To do what it does, Rojo has to do two main things: it must decide how the file system should map to Roblox and then send changes from the file system to the plugin. To accomplish this, Rojo uses what's referred to as snapshots.

Snapshots are essentially a capture of what a given Instance tree looks like at a given time. Once an initial snapshot is computed and sent to the plugin, any changes to the file system can be turned into a snapshot and compared directly against the previous snapshot, which Rojo can then use to make a set of patches that have to be applied by the plugin.

These patches represent changes, additions, and removals to the Roblox tree that Rojo creates and manages.

When generating snapshots, files are 'transformed' into Roblox objects through what's referred to as the `snapshot middleware`. As an example, this middleware takes files named `init.lua` and transforms them into a `ModuleScript` bearing the name of the parent folder. It's also responsible for things like JSON models and `.rbxm`/`.rbxmx` models being turned into snapshottable trees.

Inquiring minds should look at `snapshot/mod.rs` and `snapshot_middleware` for a more detailed explanation.

Because snapshots are designed to be translated into Instances anyway, this system is also used by the `build` command to turn a Rojo project into a complete file. The backend for serializing a snapshot into a file is provided by `rbx-dom`, which is a different project.

### The Web Server

Rojo uses a small web server to forward changes to the plugin. Once a patch is computed by the snapshot system, it's made available via the server's API. Then, the plugin requests regularly, and if a new patch exists, recieves it and applies it in Studio.

The web server itself is very basic, consisting of around half a dozen endpoints. The bulk of the work is performed by either the snapshot system or the plugin, with the web server acting as a middleman.

## The Plugin

This section of the document is left incomplete.

## Data Structures

Rojo has many data structures and their purpose might not be immediately clear at a glance. To alleviate this, they are documented below.

### Vfs

To learn more, read about [`memofs` architecture](crates/memofs/ARCHITECTURE.md).

### ServeSession

### ChangeProcessor

### RojoTree

### LifeServer

### InstanceSnapshot
