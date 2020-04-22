FROM rust:1.40 as build

ENV RUSTUP_TOOLCHAIN="stable-x86_64-unknown-linux-gnu"
ENV RUST_BACKTRACE=full

RUN rustup install stable
# rustup component add rustfmt && \
# rustup component add clippy

WORKDIR /opt/app

ADD . .
# run build
RUN cargo build \
            --bin dvm \
            --bin compiler \
            --bin stdlib-builder \
            --release
# # tests are going on release workflow
# RUN cargo test --all -- --test-threads=4 && \
#     cargo +stable fmt --all -- --check && \
#     cargo +stable clippy --all --tests --examples -- -Dwarnings

RUN ls -la ./target/release

FROM ubuntu:18.04
WORKDIR /opt/app
RUN apt-get update -y && \
    apt-get install -y libssl1.1
COPY ./lang/stdlib/ ./stdlib/
COPY --from=build \
    /opt/app/target/release/compiler \
    /opt/app/target/release/dvm \
    /opt/app/target/release/stdlib-builder \
    /opt/app/

STOPSIGNAL SIGTERM
