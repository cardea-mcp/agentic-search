#!/bin/bash

# Find unused dependencies in Cargo.toml
cargo +nightly udeps --all-targets

# Sort dependencies in Cargo.toml alphabetically
cargo sort

# Format code
cargo +nightly fmt --all -- --check

# Clippy
cargo +nightly clippy --all-features -- -D warnings
