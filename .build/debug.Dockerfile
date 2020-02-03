# FROM rust:1.40 as build

# ENV RUSTUP_TOOLCHAIN="stable-x86_64-unknown-linux-gnu"
# ENV RUST_BACKTRACE=full
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
#     /opt/app/target/debug/client.d \
#     /opt/app/target/debug/server \
#     /opt/app/target/debug/server.d \
#     /opt/app/target/debug/verify \
#     /opt/app/target/debug/verify.d \
#     /opt/app/

FROM ubuntu:18.04
WORKDIR /opt/app
COPY \
    ./target/debug/client \
    ./target/debug/server \
    ./target/debug/verify \
    /opt/app/
