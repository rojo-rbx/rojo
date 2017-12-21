# Rojo Design - Protocol Version 1
This is a super rough draft that I'm trying to use to lay out some of my thoughts.

## API

### POST `/read`
Accepts a `Vec<Route>` of items to read.

Returns `Vec<Option<RbxItem>>`, in the same order as the request.

### POST `/write`
Accepts a `Vec<{ Route, RbxItem }>` of items to write.

I imagine that the `Name` attribute of the top-level `RbxItem` would be ignored in favor of the route name?

## CLI

### Transform Plugins
Transform plugins (or filter plugins?) can interject in three places:
* Transform a `VfsItem` that's being read into an `RbxItem`
* Transform an `Rbxitem` that's being written into a `VfsItem`
* Transform a file change into paths that need to be updated

The plan is to have several built-in plugins that can be rearranged/configured in project settings:

* Base plugin
	* Transforms all unhandled files to/from StringValue objects
* Script plugin
	* Transforms `*.lua` files to their appropriate file types
* JSON/rbxmx/rbxlx model plugin
* External binary plugin
	* User passes a binary name (like `moonc`) that modifies file contents

## Roblox Studio Plugin
With the protocol version 1 change, the Roblox Studio plugin got a lot simpler. Notably, the plugin doesn't need to be aware of anything about the filesystem's semantics, which is super handy.