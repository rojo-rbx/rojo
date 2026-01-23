# memofs Changelog

## Unreleased Changes
* Added `Vfs::canonicalize`. [#1201]

## 0.3.1 (2025-11-27)
* Added `Vfs::exists`. [#1169]
* Added `create_dir` and `create_dir_all` to allow creating directories. [#937]

[#1169]: https://github.com/rojo-rbx/rojo/pull/1169
[#937]: https://github.com/rojo-rbx/rojo/pull/937

## 0.3.0 (2024-03-15)
* Changed `StdBackend` file watching component to use minimal recursive watches. [#830]
* Added `Vfs::read_to_string` and `Vfs::read_to_string_lf_normalized` [#854]

[#830]: https://github.com/rojo-rbx/rojo/pull/830
[#854]: https://github.com/rojo-rbx/rojo/pull/854

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
