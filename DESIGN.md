# Rojo Design
This is a super rough draft that I'm trying to use to lay out of my thoughts.

## API

### POST `/read`
Accepts a `Vec<Route>` of items to read.

Returns `Vec<Option<RbxItem>>`, in the same order as the request.

### POST `/write`
Accepts a `Vec<{ Route, RbxItem }>` of items to write.

I imagine that the `Name` attribute of the top-level `RbxItem` would be ignored in favor of the route name?

## CLI

### Transform Plugins

## Roblox Studio Plugin