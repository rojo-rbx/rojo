# memofs Changelog

## Unreleased Changes
* Changed `StdBackend` file watching component to use minimal recursive watches. [#830]

[#830]: https://github.com/rojo-rbx/rojo/pull/830

## 0.2.0 (2021-08-23)
* Updated to `crossbeam-channel` 0.5.1.

## 0.1.3 (2020-11-19)
* Added `set_watch_enabled` to `Vfs` and `VfsLock` to allow turning off file watching.

## 0.1.2 (2020-03-29)
* `VfsSnapshot` now implements Serde's `Serialize` and `Deserialize` traits.

## 0.1.1 (2020-03-18)
* Improved error messages using the [fs-err](https://crates.io/crates/fs-err) crate.

## 0.1.0 (2020-03-10)
* Initial release
