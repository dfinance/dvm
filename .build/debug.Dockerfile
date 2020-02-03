FROM ubuntu:18.04
WORKDIR /opt/app
COPY \
    /opt/app/target/debug/client \
    /opt/app/target/debug/server \
    /opt/app/target/debug/verify \
    /opt/app/
    # /opt/app/target/debug/client.d \
    # /opt/app/target/debug/server.d \
    # /opt/app/target/debug/verify.d \
