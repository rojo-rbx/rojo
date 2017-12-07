# Rojo Change Log

## Current Master
* Fixed server when using a file as a partition
	* Previously, trailing slashes were put on the end of a partition even if the read request was an empty string. This broke file reading on Windows when a partition pointed to a file instead of a directory!

## 0.2.3
* Plugin only release
* Tightened `init` file rules to only match script files
	* Previously, Rojo would sometimes pick up the wrong file when syncing

## 0.2.2
* Plugin only release
* Fixed broken reconciliation behavior with `init` files

## 0.2.1
* Plugin only release
* Changes default port to 8000

## 0.2.0
* Support for `init.lua` like rbxfs and rbxpacker
* More robust syncing with a new reconciler

## 0.1.0
* Initial release, functionally very similar to [rbxfs](https://github.com/LPGhatguy/rbxfs)