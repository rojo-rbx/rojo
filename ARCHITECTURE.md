# Architecture

Rojo is a rather large project with a bunch of moving parts. While it's not too complicated in practice, it tends to be overwhelming because it's a fair bit of Rust and not very clear where to begin.

This document is a "what the heck is going on" level view of Rojo and the codebase that's written to make it more reasonable to jump into something. It won't go too into depth on *how* something is done, but it will go into depth on *what* is being done.

## Overarching

Rojo is divided into several components, each layering on top of each other to provide Rojo's functionality.

At the core of Rojo lies [`ServeSession`](#servesession). As the name implies, it contains all of the components to keep a persistent DOM, react to events to update the DOM, and serve the DOM to consumers.

Most of Rojo's uses are built upon `ServeSession`! For example, the [`sourcemap` command](#sourcemapcommand) uses `ServeSession` to generate the DOM and read it to build the `sourcemap.json` file.

### The Serve Command

There are two main pieces in play when serving: the server and the Studio plugin.

The server runs a local [`LiveServer`](#liveserver) with access to your filesystem (whether it be via the terminal, the visual studio code extension, or a remote machine). It consumes a `ServeSession` and attaches a web server on top. The web server itself is very basic, consisting of around half a dozen endpoints. Generally, [`LiveServer`](#liveserver) acts as a middleman with the bulk of the work is performed by either the underlying `ServeSession` or the plugin. 

To serve a project to a connecting plugin, the server gathers data on all of the files in that project, puts it into a nice format, and then sends it to the plugin. After that, when something changes on the file system, the underlying `ServeSession` emits new patches. The web server has an endpoint the plugin [long polls](https://en.wikipedia.org/wiki/Push_technology#Long_polling) to receive the patches from the server and apply them to the datamodel in Studio.

When the plugin receives a patch it reads through the patch contents and attempts to to apply the changes described by it. Any sugar (the patch visualizer, as an example) happens on top of the patches received from the server.

### The Sourcemap Command
### The Build Command
### The Upload Command

### The Snapshotting System

To do what it does, Rojo has to do two main things: it must decide how the file system should map to Roblox and then send changes from the file system to the plugin. To accomplish this, Rojo uses what's referred to as snapshots.

Snapshots are essentially a capture of what a given Instance tree looks like at a given time. Once an initial snapshot is computed and sent to the plugin, any changes to the file system can be turned into a snapshot and compared directly against the previous snapshot, which Rojo can then use to make a set of patches that have to be applied by the plugin.

These patches represent changes, additions, and removals to the Roblox tree that Rojo creates and manages.

When generating snapshots, files are 'transformed' into Roblox objects through what's referred to as the `snapshot middleware`. As an example, this middleware takes files named `init.lua` and transforms them into a `ModuleScript` bearing the name of the parent folder. It's also responsible for things like JSON models and `.rbxm`/`.rbxmx` models being turned into snapshottable trees.

Inquiring minds should look at `snapshot/mod.rs` and `snapshot_middleware` for a more detailed explanation.

Because snapshots are designed to be translated into Instances anyway, this system is also used by the `build` command to turn a Rojo project into a complete file. The backend for serializing a snapshot into a file is provided by `rbx-dom`, which is a different project.

## The Plugin

As mentioned above, the Studio plugin uses the web server to receive patches that contain the changes, additions, and removals to the Studio datamodel. The patch does not actually point at instances directly but rather uses string IDs. The plugin has an InstanceMap that is a bidirectional map between instance IDs and Roblox instances. It lets us keep track of every instance that Rojo knows about.

To actually apply the patch to the Studio datamodel, the plugin has a Reconciler. The Reconciler uses the InstanceMap to find the instances mentioned by the patch, and then it attempts to modify/create/destroy instances as specified in the patch. It will also create and return a new patch that contains all of the operations that failed to apply (effectively creating a patch that if applied, would fix the failures), which is used to display warnings to the end user.

The first time the plugin connects to the server is a special case, since there are no changes yet. The server sends the plugin the initial state, which is first used to hydrate the InstanceMap with all the currently known instances. Then, the Reconciler computes a patch by diffing the datamodel with the server state. This is known as the "catch up" patch, and that is what is shown in the Confirming page in the plugin. When that catch up patch is applied, the Studio datamodel should match the Rojo server and subsequent patches will be able to apply cleanly.

## Data Structures

Rojo has many data structures and their purpose might not be immediately clear at a glance. To alleviate this, they are documented below.

### Vfs

To learn more, read about [`memofs` architecture](crates/memofs/ARCHITECTURE.md).

### LiveServer

LiveServer underlies the [`serve` command](#the-serve-command) and provides the web server which clients (such as the Studio plugin) can use to interface with a [`ServeSession`](#servesession).

The web server has two components: a UI and the API used by clients.

The UI provides information about the current project's [tree](#rojotree), including metadata. It also shows the project name, up-time, and version its Rojo is on.

The API provides a simple JSON protocol to interact with and receive changes from the underlying [`ServeSession`](#servesession). Checkout the [`api.rs` file under the web module](src/web/api.rs) to learn more.

### ServeSession

The `ServeSession` is the core of Rojo. It contains all of the required components to serve a given project file.

Generally, to serve means:

- Rojo maintains a DOM and exposes it to consumers;
- Rojo is able to accept events to cause changes to the DOM;
- Rojo is able to emit changes to the DOM to consumer.

It depends on:

- [`RojoTree`](#rojotree) to represent the DOM;
- `Project` to represent your root project file (e.g. `default.project.json`);
- [`Vfs`](#vfs) to provide a filesystem and emit events on changes;
- [`ChangeProcessor`](#changeprocessor) to process filesystem events from `Vfs` and consequently update the DOM through the [snapshotting system](#the-snapshotting-system);

It also provides an API for the higher level components so it can be used with the outside world:

- There is a [`MessageQueue`](#messagequeue) consumers listen on for patches applied to the DOM;
- There is a channel to send patches and update the DOM;
- And a `SessionId` to uniquely identify the `ServeSession`.

The primary interface for a `ServeSession` is the web server used by the [`serve` command](#servecommand). Additionally, several of Rojo's commands are implemented using `ServeSession`.

### MessageQueue

`MessageQueue` manages a persistent history of messages and a way to subscribe asynchronously to messages past or future.

It does this primarily by having the subscribers keep a cursor of the message they're currently on. So, when a subscription does occurs:

- If the cursor is behind the latest message, it will be immediately be fired with all messages past the cursor;
- If the cursor is on the latest message, it will fire on the next message given;
- If the cursor is ahead of the latest message, it will fire on the message after the cursor.

### SessionId

### ChangeProcessor

### RojoTree

### InstanceSnapshot
