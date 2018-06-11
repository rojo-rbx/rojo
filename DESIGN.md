# Rojo - Protocol v2 Design
This spec is a work in progress.

The aim is to rewrite most of Rojo to drop the notion of routes almost entirely in favor of generated IDs and make the server much more stateful. This should make change description completely robust in all cases, including models with many duplicated instance names.

## Base Terms

* **Server**: The binary portion of Rojo, in Rust.
* **Client**: The Roblox Studio plugin portion of Rojo, in Lua.
* **Middleware**: Part of the server, transforms files to Roblox representations and back.
* **VFS**: The Virtual File System specifically that middleware deal with; it is in-memory.
* **Roblox Instance**: An actual Roblox object derived from `Instance` on the client.

## Initialization
On server initialization, Rojo reads all files in each partition and runs each through our middleware stack (renamed from plugins). Middleware transform a `VfsItem` into a set of `RbxInstance` objects with these properties:

* name: `String`
* class_name: `String`
* parent: `Option<Id>`
* properties: `HashMap<String, RbxValue>`
* file_route: `FileRoute`
* id: `Id`
* rbx_referent: `Option<RbxReferent>`

The server should keep these book-keeping maps:

* partitions: `HashMap<String, Partition>`, to read partitions from disk
* partition_instances: `HashMap<String, Id>`, to track partition instances
* partition_files: `HashMap<String, VfsItem>`, to serve as an in-memory filesystem
* instances: `HashMap<Id, RbxInstance>`, to serve as an index
* instances_by_route: `HashMap<FileRoute, Id>`, to handle filesystem changes

As a future extension, the server should also track:

* `referent_to_id`: `HashMap<RbxReferent, Id>`, when loading `rbxmx` format models from disk.

`Partition` is defined as:

* path: `PathBuf`
* target: `Vec<String>`

On initial sync, the client should receive:

* A list of every `RbxInstance` the server knows of.
* A map from `Id` to a Roblox route, containing an entry for every partition and where they should be mounted.

## File Changes
Whenever a file is changed, the server should:

* Convert its path to a `FileRoute`.
* Read the file into a `VfsItem` object.
* Compare the contents of the file with the last-seen contents from the map from `FileRoute` to `String`.
	* If they're the same, that means this change was caused by the server itself, and should be discarded.
	* Else, clear the value from the map.
* Pass the `VfsItem` through the middleware chain to create a tree of `RbxInstance` objects.
* Check the existing map from `FileRoute` to `Id`.
	* If an existing instance exists with the root's `Id`, reuse it and replace the root's `Id`.
	* Else, generate a new ID.
* For each child instance:
	* If an `RbxReferent` value is present on the instance, attempt to map it to an existing `Id`, falling back to creating a new `Id`.
* Log a change event `(Timestamp, Id)` for the root instance into a sorted list that's client queryable.

The client will then:

* Receive notice of changes by receiving a list of `Id`s that have changed.
* Ask the server for the contents of each `Id` that has changed.
* Apply the properties the server returns to each instance.

If an `Id` was not previously tracked by the client, the associated `RbxInstance`'s `parent` field will point to another `Id`. The client will either have knowledge of that `Id`, or request it from the server if it isn't known. This can potentially continue up the tree until the client reaches a partition mount point, which should be known by the client due to the initialization process.

## Client Changes
The client should keep list of mutations to track any uncommitted changes to Roblox Instances tracked by Rojo.

Those changes should come in three flavors:

* Changed a property (need `id`, `key`, and `value`)
* Deleted a known instance (need `id`)
* Created a new instance (need `clientId`)

Every sync period, which could be around 50ms, the plugin should send a request with every Roblox Instance mutation all at once.

In order to correctly serialize changes received from the client, some care needs to be taken.

For every *Changed* or *Deleted* mutation received from the client, the server should:

* Attempt to locate the `RbxInstance` associated with the `id`.
	* If it doesn't exist, abort the change and output a warning to the console.
* Modify the `RbxInstance` in-place.
* Traverse up the tree by following the `parent` property until an `RbxInstance` is found that has a `route` property.
* Use the middleware chain to serialize these nodes to a single `VfsItem` object.
* Insert the contents of the `VfsItem` into the server's map from `Route` to `String`, which is used by the "File Changes" process.
* Write the file or directory to the disk atomically using *write-and-rename*.

For every *Created* mutation received from the client, the server should:

* Attempt to locate the `RbxInstance` associated with the instance's `parent` `Id`
	* If it doesn't exist, abort the change and output a warning to the console.
* Generate a new `Id` for the instance being created.
* Use the middleware chain to process the added instance, its parent, and generate a list of `VfsItem` objects to commit to disk.
* Insert the `RbxInstance` into all of the server's relevant maps.
* Yield the generated `Id` to the client, indexed by the request's `clientId`, which can be used for later updates in both directions.

## State and Session Restart
Previously, only the client had meaningful state with regards to what instances were loaded; the server in 0.3.x only tracks a list of changes by route.

Both the Rojo server and client in the Stateful IDs redesign have a significant amount of state, which requires using a field already present, but unused, in the sync protocol.

During initialization and initial sync, the server should send the client two values:
* A server ID value generated randomly on each startup
* A project name, specified in `rojo.json`

The client should store the server ID as connection metadata, and the project name as place metadata that it may persist.

Every request from the client should attach both the server ID and the project name. As a first validation check, the server must compare both values to its own.

If a mismatch is detected, the server should immediately abort the request and return an error containing its server ID and loaded project name.

If the client detects an error in this way, it should delete any connection metadata, including `Id` mappings -- this data is no longer relevant to any new connections.

If the returned project name matches the value that the client was using before, the client should begin a *session restart* by reinitializing the connection to the server and creating new metadata.

## Other Changes
These changes aren't directly related to this refactor, but are changes that need to be made to Rojo.

* Unknown instances will no longer sync as `StringValue` instances
* `rojo init` generates project files with random port numbers