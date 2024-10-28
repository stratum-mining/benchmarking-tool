# Build stage
FROM rust:1.75-alpine AS builder

WORKDIR /usr/src/stratum/

# Install git and necessary build dependencies
RUN apk add --no-cache git musl-dev pkgconfig libressl-dev

# Clone the repository and checkout the main branch
RUN git clone https://github.com/stratum-mining/stratum.git .

# Build the project in release mode
WORKDIR /usr/src/stratum/roles/
RUN cargo build --release

# Final stage
FROM alpine:latest

# Install necessary runtime dependencies
RUN apk update && apk add --no-cache \
        iproute2 \
        iputils-ping \
        iptables \
        curl

# Copy only the compiled binaries from the builder stage
COPY --from=builder /usr/src/stratum/roles/target/release/pool_sv2 /usr/local/bin/pool_sv2
COPY --from=builder /usr/src/stratum/roles/target/release/jd_server /usr/local/bin/jd_server
COPY --from=builder /usr/src/stratum/roles/target/release/jd_client /usr/local/bin/jd_client
COPY --from=builder /usr/src/stratum/roles/target/release/translator_sv2 /usr/local/bin/translator_sv2

# Set the working directory
WORKDIR /usr/local/bin/

# Copy the script and make it executable
COPY ./pools-latency-calculator/monitor_and_apply_latency.sh /usr/local/bin/monitor_and_apply_latency.sh
RUN chmod +x /usr/local/bin/monitor_and_apply_latency.sh