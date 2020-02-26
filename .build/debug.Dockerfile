# FROM buildpack-deps:buster as build
# # FROM rust:1.40 as build

# ENV RUSTUP_HOME=/opt/app/cache/rustup \
#     CARGO_HOME=/opt/app/cache/cargo \
#     PATH=/opt/app/cache/cargo/bin:$PATH \
#     RUSTUP_TOOLCHAIN="stable-x86_64-unknown-linux-gnu" \
#     RUST_BACKTRACE=full

# # RUN apt-get update && \
# #     apt-get install -y --no-install-recommends \
# #         cargo build-essential curl wget jq bash g++ gcc
# RUN curl https://sh.rustup.rs -sSf | bash -s -- -y
# RUN rustup install stable && \
#     rustup component add rustfmt && \
#     rustup component add clippy
# WORKDIR /opt/app
# COPY . .
# RUN cargo build --all


# FROM ubuntu:18.04
# WORKDIR /opt/app
# COPY --from=build \
#     /opt/app/target/debug/client \
#     /opt/app/target/debug/server \
#     /opt/app/target/debug/verify \
#     /opt/app/
#     # /opt/app/target/debug/client.d \
#     # /opt/app/target/debug/server.d \
#     # /opt/app/target/debug/verify.d \

FROM ubuntu:18.04
WORKDIR /opt/app
COPY \
    ./target/debug/* \
    /opt/app/
