# Rojo as a C Library
This is an experiment to expose a C API for Rojo that would be suitable for embedding it into an existing C/C++ application.

I'm hoping to expand it to drop the HTTP layer and communicate through a channel, which could make it feasible to embed into an existing Roblox IDE with minimal changes or additional code.

## Building
This project is currently not built by default and could break/disappear at any time.

```bash
cargo build -p clibrojo
```

On Windows, Cargo will generate a `clibrojo.dll` and associated `.lib` file. Link these into your project.

To generate the associated C header file to include in the project, use [cbindgen](https://github.com/eqrion/cbindgen):

```bash
cbindgen --crate clibrojo --output include/rojo.h
```