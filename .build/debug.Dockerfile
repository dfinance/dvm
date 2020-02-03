FROM rust:1.40 as build


ENV RUSTUP_HOME=/opt/app/cache/rustup \
    CARGO_HOME=/opt/app/cache/cargo \
    PATH=/opt/app/cache/cargo/bin:$PATH \
    RUSTUP_TOOLCHAIN="stable-x86_64-unknown-linux-gnu" \
    RUST_BACKTRACE=full

RUN rustup install stable && \
    rustup component add rustfmt && \
    rustup component add clippy

WORKDIR /opt/app
COPY . .
RUN cargo build --all


FROM ubuntu:18.04
WORKDIR /opt/app
COPY --from=build \
    /opt/app/target/debug/client \
    /opt/app/target/debug/server \
    /opt/app/target/debug/verify \
    /opt/app/
    # /opt/app/target/debug/client.d \
    # /opt/app/target/debug/server.d \
    # /opt/app/target/debug/verify.d \

# FROM ubuntu:18.04
# WORKDIR /opt/app
# COPY \
#     ./target/debug/client \
#     ./target/debug/server \
#     ./target/debug/verify \
#     /opt/app/
