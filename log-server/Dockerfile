# Build stage
FROM rust:1.75-alpine AS builder

WORKDIR /usr/src/log-server
COPY . .

# Install necessary dependencies
RUN apk add musl-dev pkgconfig libressl-dev

# Build the project in release mode
RUN cargo build --release

# Final stage
FROM alpine:latest

# Copy only the binary from the builder image
COPY --from=builder /usr/src/log-server/target/release/log-server /usr/local/bin/log-server

# Set the working directory
WORKDIR /usr/local/bin/