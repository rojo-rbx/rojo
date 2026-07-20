# Prism 0.1

Prism is a standalone Roblox developer tool derived from Rojo. It retains Rojo project-file and sync-protocol compatibility while adding a trusted, local automation path for a connected Prism Studio plugin.

Prism 0.1 currently supports:

- `prism serve [PROJECT]`
- `prism exec SCRIPT.lua`
- `prism inspect TARGET`
- `prism plugin install`
- `prism plugin list`
- `prism --help` and `prism --version`

It does not currently provide selection, camera control, focus, screenshots, search, play/stop control, snapshots or diffs, preview/watch automation, profiling automation, or playtesting automation.

## Build and installation

Install Rust 1.88 or newer, initialize the repository submodules, and install the pinned tools with Rokit:

```text
git submodule update --init --recursive
rokit install
cargo build --locked
```

The primary development binary is `target/debug/prism` (`target\debug\prism.exe` on Windows). Build an optimized binary with `cargo build --release --locked`; it is written to `target/release/prism` or `target\release\prism.exe`.

Install the bundled Studio plugin after building:

```text
cargo run --bin prism -- plugin install
```

Prism writes `PrismManagedPlugin.rbxm` to Roblox Studio's local Plugins directory and prints the exact path. Use `prism plugin list` to inspect likely Prism and Rojo-like local plugin files. Marketplace-installed copies must be disabled separately through Studio if they conflict.

## Serving a project

Pass a Rojo-compatible project file or directory explicitly:

```text
prism serve default.project.json
```

With no path, Prism checks only the current directory. It prefers `default.project.json`; otherwise it uses the sole `*.project.json` file. It errors when no candidate exists or when several candidates exist, listing multiple candidates in deterministic order. It does not search parent or descendant directories.

## Trusted local automation

Start `prism serve`, open Roblox Studio with the Prism plugin installed, connect the plugin, and then invoke trusted local commands from another terminal:

```text
prism exec script.lua
prism inspect Workspace
prism inspect Workspace.Map --depth 2 --properties --json
```

`prism exec` runs the supplied Luau as trusted code inside the connected Studio edit session. It can mutate the open place. Run it only against a local server and only with scripts you have reviewed and trust. The serve and automation APIs are not a sandbox and should not be exposed to untrusted networks or users.

`prism inspect` reads a bounded typed view of a DataModel path through the connected plugin. The Studio plugin is required for both commands, and automation is available only in Studio edit mode.

## Compatibility and attribution

Prism keeps Rojo-compatible `.project.json` formats, the `/api/rojo` compatibility route, existing sync routes, exec job routes such as `/api/exec/jobs`, the packaged `Rojo` model root, and compatibility-sensitive Studio IDs. The Rust package remains named `rojo`, and its public library remains `librojo`; `prism` is the primary executable and `rojo` is retained as a compatibility executable.

Prism is derived from Rojo and retains its Mozilla Public License 2.0 attribution and licensing. See `LICENSE.txt` and the upstream history in `CHANGELOG.md`.
