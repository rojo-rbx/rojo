# memofs
[![Crates.io](https://img.shields.io/crates/v/memofs.svg)](https://crates.io/crates/memofs)

Implementation of a virtual filesystem with a configurable backend and file
watching.

memofs is currently an unstable minimum viable library. Its primary consumer is
[Rojo](https://github.com/rojo-rbx/rojo), a build system for Roblox.

### Current Features
* API similar to `std::fs`
* Configurable backends
    * `StdBackend`, which uses `std::fs` and the `notify` crate
    * `NoopBackend`, which always throws errors
    * `InMemoryFs`, a simple in-memory filesystem useful for testing

### Future Features
* Hash-based hierarchical memoization keys (hence the name)
* Configurable caching (write-through, write-around, write-back)

## License
memofs is available under the terms of the MIT license. See [LICENSE.txt](LICENSE.txt) or <https://opensource.org/licenses/MIT> for more details.