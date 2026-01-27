# Build stage
FROM rust:1.83-alpine AS builder

WORKDIR /usr/src
# Create a minimal workspace Cargo.toml for Docker build (only sv2-custom-proxy)
RUN printf '[workspace]\nresolver="2"\nmembers = ["sv2-custom-proxy"]\n' > Cargo.toml
# Copy Cargo.lock if it exists
COPY Cargo.lock* ./
# Copy the sv2-custom-proxy package
COPY ./sv2-custom-proxy ./sv2-custom-proxy

# Install necessary dependencies for building
RUN apk update && apk add --no-cache musl-dev pkgconf libressl-dev

# Build the project in release mode from workspace root
RUN cargo build --package sv2-custom-proxy --release

# Final stage
FROM alpine:latest

# Install necessary runtime dependencies
RUN apk update && apk add --no-cache \
        iproute2 \
        iputils-ping \
        iptables \
        curl

# Copy the binary from the builder stage
COPY --from=builder /usr/src/target/release/sv2-custom-proxy /usr/local/bin/sv2-custom-proxy

# Set the working directory
WORKDIR /usr/local/bin/

# Copy the script and make it executable
COPY ./pools-latency-calculator/monitor_and_apply_latency.sh /usr/local/bin/monitor_and_apply_latency.sh
RUN chmod +x /usr/local/bin/monitor_and_apply_latency.sh