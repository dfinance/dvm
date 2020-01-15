#!/usr/bin/env bash
cargo clippy --all --tests --examples
cargo fmt --all
cargo test --all --tests