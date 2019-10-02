#!/bin/sh

set -e

cargo test --all --locked
cargo fmt -- --check

touch src/lib.rs # Nudge Rust source to make Clippy actually check things
cargo clippy