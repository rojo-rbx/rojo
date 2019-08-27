# Contributing to the Rojo Project
Rojo is a big project and can always use more help! This guide covers all repositories underneath the [rojo-rbx organization on GitHub](https://github.com/rojo-rbx).

Some of the repositories covered are:

* https://github.com/rojo-rbx/rojo
* https://github.com/rojo-rbx/rbx-dom
* https://github.com/rojo-rbx/vscode-rojo
* https://github.com/rojo-rbx/rbxlx-to-rojo

## Code
Code contributions are welcome for features and bugs that have been reported in the project's bug tracker. We want to make sure that no one wastes their time, so be sure to talk with maintainers about what changes would be accepted before doing any work!

## Documentation
Documentation impacts way more people than the individual lines of code we write.

If you find any problems in documentation, including typos, bad grammar, misleading phrasing, or missing content, feel free to file issues and pull requests to fix them.

## Bug Reports and Feature Requests
Most of the tools around Rojo try to be clear when an issue is a bug. Even if they aren't, sometimes things don't work quite right.

Sometimes there's something that Rojo doesn't do that it probably should.

Please file issues and we'll try to help figure out what the best way forward is.

## Pushing a Rojo Release
The Rojo release process is pretty manual right now. If you need to do it, here's how:

1. Bump server version in [`server/Cargo.toml`](server/Cargo.toml)
2. Bump plugin version in [`plugin/src/Config.lua`](plugin/src/Config.lua)
3. Run `cargo test` to update `Cargo.lock` and double-check tests
4. Update [`CHANGELOG.md`](CHANGELOG.md)
5. Commit!
    * `git add . && git commit -m "Release vX.Y.Z"`
6. Tag the commit with the version from `Cargo.toml` prepended with a v, like `v0.4.13`
7. Build Windows release build of CLI
    * `cargo build --release`
7. Publish the CLI
    * `cargo publish`
8. Build and upload the plugin
    * `rojo build plugin -o Rojo.rbxm`
    * Upload `Rojo.rbxm` to Roblox.com, keep it for later
9. Push commits and tags
    * `git push && git push --tags`
10. Copy GitHub release content from previous release
    * Update the leading text with a summary about the release
    * Paste the changelog notes (as-is!) from [`CHANGELOG.md`](CHANGELOG.md)
    * Write a small summary of each major feature