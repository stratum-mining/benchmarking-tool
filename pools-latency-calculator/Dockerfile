# Build stage
FROM rust:1.75-alpine AS builder

WORKDIR /usr/src/pools-latency-calculator

# Install necessary dependencies for building
RUN apk add --no-cache musl-dev pkgconfig libressl-dev

# Copy the source code into the container
COPY pools-latency-calculator/ .

# Build the project in release mode
RUN cargo build --release

# Final stage
FROM alpine:latest

# Install necessary runtime dependencies
RUN apk update && apk add --no-cache \
        iproute2 \
        iputils-ping \
        iptables \
        curl

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/pools-latency-calculator/target/release/pools-latency-calculator /usr/local/bin/pools-latency-calculator

# Set the working directory
WORKDIR /usr/local/bin/
