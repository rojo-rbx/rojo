# memofs Changelog

## Unreleased Changes
* Added `set_watch_enabled` to `Vfs` and `VfsLock` to allow turning off file watching.

## 0.1.2 (2020-03-29)
* `VfsSnapshot` now implements Serde's `Serialize` and `Deserialize` traits.

## 0.1.1 (2020-03-18)
* Improved error messages using the [fs-err](https://crates.io/crates/fs-err) crate.

## 0.1.0 (2020-03-10)
* Initial release