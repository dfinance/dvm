#!/usr/bin/env bash
set -e

cargo fmt --all
cargo clippy --all --tests --examples -- -Dwarnings
cargo test --all --tests