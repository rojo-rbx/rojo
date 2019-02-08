This document aims to give a general overview of how Rojo works. It's intended for people who want to contribute to the project as well as anyone who's just curious how the tool works!

[TOC]

## CLI

### RbxTree
Rojo uses a library named [`rbx_tree`](https://github.com/LPGhatguy/rbx-tree) as its implementation of the Roblox DOM. It serves as a common format for serialization to all the formats Rojo supports!

Rojo uses two related libraries to deserialize instances from Roblox's file formats, `rbx_xml` and `rbx_binary`.

### In-Memory Filesystem (IMFS)
Relevant source files:

* [`server/src/imfs.rs`](https://github.com/LPGhatguy/rojo/blob/master/server/src/imfs.rs)
* [`server/src/fs_watcher.rs`](https://github.com/LPGhatguy/rojo/blob/master/server/src/fs_watcher.rs)

Rojo keeps an in-memory copy of all files that it needs reasons about. This enables taking fast, stateless, tear-tree snapshots of files to turn them into instances.

Keeping an in-memory copy of file contents will also enable Rojo to debounce changes that are caused by Rojo itself. This'll happen when two-way sync finally happens.

### Snapshot Reconciler
Relevant source files:

* [`server/src/snapshot_reconciler.rs`](https://github.com/LPGhatguy/rojo/blob/master/server/src/snapshot_reconciler.rs)
* [`server/src/rbx_snapshot.rs`](https://github.com/LPGhatguy/rojo/blob/master/server/src/rbx_snapshot.rs)
* [`server/src/rbx_session.rs`](https://github.com/LPGhatguy/rojo/blob/master/server/src/rbx_session.rs)

To simplify incremental updates of instances, Rojo generates lightweight snapshots describing how files map to instances. This means that Rojo can treat file change events similarly to damage painting as opposed to trying to surgically update the correct instances.

This approach reduces the number of desynchronization bugs, reduces the complexity of important pieces of the codebase, and makes writing plugins a lot easier.

### HTTP API
Relevant source files:

* [`server/src/web.rs`](https://github.com/LPGhatguy/rojo/blob/master/server/src/web.rs)

The Rojo live-sync server and Roblox Studio plugin communicate via HTTP.

Requests sent from the plugin to the server are regular HTTP requests.

Messages sent from the server to the plugin are delivered via HTTP long-polling. This is an approach that uses long-lived HTTP requests that restart on timeout. It's largely been replaced by WebSockets, but Roblox doesn't have support for them.

## Roblox Studio Plugin
TODO