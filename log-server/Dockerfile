FROM rust:1.75 as builder
WORKDIR usr/src/log-server
COPY ./log-server .
RUN cargo build --release

FROM rust:1.75
COPY --from=builder usr/src/log-server/ usr/src/log-server/
RUN apt-get update && apt-get install -y --no-install-recommends \
        iproute2 \
        iputils-ping \
        iptables \
        curl 
WORKDIR /usr/src/log-server/