# Build stage for sv2-tp (Stratum V2 Template Provider)
FROM debian:bookworm-slim AS builder

# Install build dependencies as per official docs
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    pkgconf \
    python3 \
    git \
    libboost-dev \
    libcapnp-dev \
    capnproto \
    && rm -rf /var/lib/apt/lists/*

# Clone and build sv2-tp
WORKDIR /build
RUN git clone https://github.com/stratum-mining/sv2-tp.git
WORKDIR /build/sv2-tp
RUN git checkout v1.0.5
RUN cmake -B build && \
    cmake --build build -j$(nproc)

# Runtime stage
FROM debian:bookworm-slim

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y \
    libboost-system1.74.0 \
    libboost-filesystem1.74.0 \
    libboost-thread1.74.0 \
    capnproto \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Copy built binary
COPY --from=builder /build/sv2-tp/build/bin/sv2-tp /usr/local/bin/sv2-tp

# Create data directory
RUN mkdir -p /root/.bitcoin

EXPOSE 8442

CMD ["sv2-tp", "-debug=sv2", "-loglevel=sv2:trace"]
