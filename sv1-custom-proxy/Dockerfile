FROM rust:1.75 as builder
WORKDIR usr/src/sv1-custom-proxy
COPY ./sv1-custom-proxy .
RUN cargo build --release

FROM rust:1.75
COPY --from=builder usr/src/sv1-custom-proxy/ usr/src/sv1-custom-proxy/
RUN apt-get update && apt-get install -y --no-install-recommends \
        iproute2 \
        iputils-ping \
        iptables \
        curl 
WORKDIR /usr/src/sv1-custom-proxy/
COPY ./pools-latency-calculator/monitor_and_apply_latency.sh /usr/local/bin/monitor_and_apply_latency.sh
RUN chmod +x /usr/local/bin/monitor_and_apply_latency.sh