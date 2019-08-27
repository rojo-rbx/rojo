#!/bin/sh

set -ev

cargo test --locked --verbose
cargo fmt -- --check
cargo clippy