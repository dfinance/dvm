#!/usr/bin/env bash
cargo clippy --all --tests --examples
cargo fmt --all -- --check
cargo test --all --tests