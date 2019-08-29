#!/bin/sh

set -ev

cargo test --all --locked --verbose
cargo fmt -- --check
cargo clippy