FROM rust:1.40 as build

ENV RUSTUP_TOOLCHAIN="stable-x86_64-unknown-linux-gnu"
ENV RUST_BACKTRACE=full

RUN rustup install stable && \
    rustup component add rustfmt && \
    rustup component add clippy

WORKDIR /opt/app

# install deps
# ADD Cargo.toml Cargo.lock ./
# RUN mkdir src/ && \
#     echo "fn main() {println!(\"if you see this, the build broke\")}" > src/main.rs && \
#     echo "fn main() {println!(\"if you see this, the build broke\")}" > src/build.rs && \
#     cargo build --release
# # RUN ls -la target/release/
# # RUN ls -la target/release/deps/
# # RUN ls -la target/release/build/
# RUN rm src/main.rs src/build.rs
# #  && \
# #     find -name 'move-vm-in-cosmos*' | xargs rm -rf

ADD . .
# run build
RUN cargo build \
            --bin server \
            --bin compiler \
            --bin stdlib-builder \
            --release
# run tests
RUN cargo test --all && \
    cargo +stable fmt --all -- --check && \
    cargo +stable clippy --all --tests --examples -- -Dwarnings

RUN ls -la ./target/release

FROM ubuntu:18.04
WORKDIR /opt/app
COPY ./stdlib ./
COPY --from=build \
    /opt/app/target/release/compiler \
    /opt/app/target/release/server \
    /opt/app/target/release/stdlib-builder \
    /opt/app/
