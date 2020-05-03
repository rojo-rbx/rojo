#!/bin/sh
set -e

cargo build --release

echo "Known good:"
time rojo build ../uiblox/test-place.project.json -o UIBlox.rbxlx

echo "Current:"
time ./target/release/rojo build ../uiblox/test-place.project.json -o UIBlox.rbxlx