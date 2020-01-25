#!/usr/bin/env bash
set -e

# RustFMT
# install: rustup component add rustfmt --toolchain stable
cargo +stable fmt --all

# Clippy
# install: rustup component add clippy --toolchain stable
# clippy available in stable channel only
cargo +stable clippy --all --tests --examples -- -Dwarnings

cargo test --all --tests
