FROM rust:1.75 as builder
WORKDIR usr/src/stratum/
RUN git clone https://github.com/stratum-mining/stratum.git .
WORKDIR /usr/src/stratum/roles/
RUN cargo build --release

FROM rust:1.75
COPY --from=builder /usr/src/stratum/roles/ /usr/src/stratum/roles
RUN apt-get update && apt-get install -y --no-install-recommends \
        iproute2 \
        iputils-ping \
        iptables \
        curl 
        
WORKDIR /usr/src/stratum/roles
COPY ./pools-latency-calculator/monitor_and_apply_latency.sh /usr/local/bin/monitor_and_apply_latency.sh
RUN chmod +x /usr/local/bin/monitor_and_apply_latency.sh