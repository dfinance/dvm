FROM rust:1.40

ENV RUSTUP_TOOLCHAIN="stable-x86_64-unknown-linux-gnu"
RUN rustup install stable && \
    rustup component add rustfmt && \
    rustup component add clippy

WORKDIR /opt/app
COPY . .

ENV RUST_BACKTRACE=full

# run build
RUN cargo build --all
# run tests
RUN cargo test --all && \
    cargo +stable fmt --all && \
    cargo +stable clippy --all --tests --examples -- -Dwarnings

