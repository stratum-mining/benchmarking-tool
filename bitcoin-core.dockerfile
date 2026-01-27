# Build Bitcoin Core from source
FROM debian:stable-slim AS builder

# Install build dependencies
# According to https://github.com/bitcoin/bitcoin/blob/master/doc/build-unix.md
RUN apt-get update && apt-get install -y \
    build-essential \
    cmake \
    pkgconf \
    python3 \
    git \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set Bitcoin Core version
ENV BITCOIN_VERSION=30.0
ENV BITCOIN_DIR=/bitcoin

# Clone Bitcoin Core repository
WORKDIR /tmp
RUN git clone https://github.com/bitcoin/bitcoin.git bitcoin-core

# Checkout the specific version
WORKDIR /tmp/bitcoin-core
RUN git checkout v${BITCOIN_VERSION}

RUN apt-get update && apt-get install -y \
    libevent-dev \
    libboost-dev \
    libzmq3-dev \
    libcapnp-dev \
    capnproto \
    && rm -rf /var/lib/apt/lists/*

# Build Bitcoin Core using CMake
# According to https://github.com/bitcoin/bitcoin/blob/master/doc/build-unix.md
RUN cmake -B build \
    -DCMAKE_INSTALL_PREFIX=${BITCOIN_DIR} \
    -DENABLE_WALLET=OFF \
    -DWITH_MULTIPROCESS=ON && \
    cmake --build build -j$(nproc) && \
    cmake --install build

# Final stage
FROM debian:stable-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    libboost-system1.83.0 \
    libboost-filesystem1.83.0 \
    libboost-chrono1.83.0 \
    libboost-thread1.83.0 \
    libevent-2.1-7t64 \
    libevent-core-2.1-7t64 \
    libevent-extra-2.1-7t64 \
    libevent-pthreads-2.1-7t64 \
    libzmq5 \
    libssl3t64 \
    capnproto \
    ca-certificates \
    wget \
    curl \
    jq \
    && rm -rf /var/lib/apt/lists/*

# Copy Bitcoin Core binaries from builder
COPY --from=builder /bitcoin/bin/* /usr/local/bin/
COPY --from=builder /bitcoin/libexec/* /usr/local/libexec/

# Create volume for blockchain data
VOLUME ["/root/.bitcoin"]

# Set working directory
WORKDIR /root

# Default command - launch node with IPC support
CMD ["bitcoin", "-m", "node", "-ipcbind=unix"]

