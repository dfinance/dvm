FROM rust:1.40 as build

ENV RUSTUP_TOOLCHAIN="stable-x86_64-unknown-linux-gnu"
ENV RUST_BACKTRACE=full

RUN rustup install stable && \
    rustup component add rustfmt

WORKDIR /opt/app

ADD . .
# run build
RUN cargo build \
            --bin dvm \
            --bin movec \
            --bin status-table \
            --bin stdlib-builder \
            --release

RUN ls -la ./target/release

FROM ubuntu:18.04
WORKDIR /opt/app
RUN apt-get update -y && \
    apt-get install -y libssl1.1 bash jq && \
    addgroup --gid 1000 user && \
    adduser --disabled-password --uid 1000 --home /home/user --gecos "user" --shell /bin/bash --ingroup user user && \
    chown -R 1000:1000 /opt/app
USER user
COPY ./stdlib/modules/ ./stdlib/
COPY --from=build \
    /opt/app/target/release/dvm \
    /opt/app/target/release/movec \
    /opt/app/target/release/status-table \
    /opt/app/target/release/stdlib-builder \
    /opt/app/

STOPSIGNAL SIGTERM
