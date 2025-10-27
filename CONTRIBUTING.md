# Contributing to the Rojo Project
Rojo is a big project and can always use more help!

Some of the repositories covered are:

* https://github.com/rojo-rbx/rojo
* https://github.com/rojo-rbx/rbx-dom
* https://github.com/rojo-rbx/vscode-rojo
* https://github.com/rojo-rbx/rbxlx-to-rojo

## Code
Code contributions are welcome for features and bugs that have been reported in the project's bug tracker. We want to make sure that no one wastes their time, so be sure to talk with maintainers about what changes would be accepted before doing any work!

You'll want these tools to work on Rojo:

* Latest stable Rust compiler
* Latest stable [Rojo](https://github.com/rojo-rbx/rojo)
* [Rokit](https://github.com/rojo-rbx/rokit)
* [Luau Language Server](https://github.com/JohnnyMorganz/luau-lsp) (Only needed if working on the Studio plugin.)

When working on the Studio plugin, we recommend using this command to automatically rebuild the plugin when you save a change:

*(Make sure you've enabled the Studio setting to reload plugins on file change!)*

```bash
bash scripts/watch-build-plugin.sh
```

You can also run the plugin's unit tests with the following:

*(Make sure you have `run-in-roblox` installed first!)*

```bash
bash scripts/unit-test-plugin.sh
```

## Documentation
Documentation impacts way more people than the individual lines of code we write.

If you find any problems in the documentation, including typos, bad grammar, misleading phrasing, or missing content, feel free to file issues and pull requests to fix them.

## Bug Reports and Feature Requests
Most of the tools around Rojo try to be clear when an issue is a bug. Even if they aren't, sometimes things don't work quite right.

Sometimes there's something that Rojo doesn't do that it probably should.

Please file issues and we'll try to help figure out what the best way forward is.

## Local Development Gotchas

If your build fails with "Error: failed to open file `D:\code\rojo\plugin\modules\roact\src`" you need to update your Git submodules.
Run the command and try building again: `git submodule update --init --recursive`.

## Pushing a Rojo Release
The Rojo release process is pretty manual right now. If you need to do it, here's how:

1. Bump server version in [`Cargo.toml`](Cargo.toml)
2. Bump plugin version in [`plugin/src/Config.lua`](plugin/src/Config.lua)
3. Run `cargo test` to update `Cargo.lock` and run tests
4. Update [`CHANGELOG.md`](CHANGELOG.md)
5. Commit!
    * `git add . && git commit -m "Release vX.Y.Z"`
6. Tag the commit
    * `git tag vX.Y.Z`
7. Publish the CLI
    * `cargo publish`
8. Publish the Plugin
    * `cargo run -- upload plugin --asset_id 6415005344`
9. Push commits and tags
    * `git push && git push --tags`
10. Copy GitHub release content from previous release
    * Update the leading text with a summary about the release
    * Paste the changelog notes (as-is!) from [`CHANGELOG.md`](CHANGELOG.md)
    * Write a small summary of each major feature