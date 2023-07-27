# Architecture

As the `README.md` says, this is an incomplete library that's dedicated to serving Rojo's purposes for the time being.
Meaning, there's still plenty of work to be done and the API will need to change.

For the time being, this documents the current state of affairs.

## VFS Interface

The predominant object of `memofs`, it provides an interface for an abstract filesystem (known as a backend in `memofs`).

### Backends

Instead of `Vfs` providing a backend for you, it's flipped that it consumes a backend to use.
Essentially, [late-binding](https://ericlippert.com/2012/02/06/what-is-late-binding/) the backend so it's decided at runtime instead of compile-time.

This is useful because you can implement any arbitrary backend and pass it along to `Vfs` without any changes to your project!

For example, if you want to use a network drive instead of a local file system,
all that's required is to implement a `VfsBackend` to interface with the network drive.
Now, you can swap your local file system for the new one!

There are common use cases for this feature, hence `memofs` provides several backends. 

#### In-memory

As the name implies, it keeps all files and directories in memory.
This is particularly useful for testing, as it's easy to build, snapshot, and teardown.

To help, `memofs` provides a `VfsSnapshot` object to snapshot the filesystem. The in-memory backend has methods to load from and save to a `VfsSnapshot`.

#### Noop

As the name implies, it does nothing.
As the name doesn't imply, every operation will error.
This is useful if you want to verify that your software doesn't perform any read/write operations.

#### Std

As the name implies, it provides an interface to the `std`'s filesystem API. Particularly, `fs_err` for nicer error messages!

### Filesystem events

`Vfs` additionally provides an event bus via `Vfs::event_receiver()`.
For any changes detected by the backend, it will be sent down that channel.
Only `std` actually provides any events.

Additionally, there is a `Vfs::commit_event()` method, which will unwatch a path if a remove event is passed.
