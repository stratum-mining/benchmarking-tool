FROM debian:stable-slim as build

# Install & update base system
RUN apt-get update && apt-get upgrade -y

# Install necessary tools
RUN apt-get install -y wget tar curl jq

# Set environment variables for versions and installation directory
ENV SV2_TP_VERSION=v1.0.2
ENV BITCOIN_VERSION=30.0rc1
ENV BITCOIN_DIR=/bitcoin

# Create the directory where Bitcoin Core will be installed
RUN mkdir -p $BITCOIN_DIR

# Download and install sv2-tp
RUN ARCH=$(dpkg --print-architecture) && \
    if [ "$ARCH" = "amd64" ]; then \
        SV2_TP_URL=https://github.com/Sjors/sv2-tp/releases/download/$SV2_TP_VERSION/sv2-tp-1.0.2-x86_64-linux-gnu.tar.gz; \
    elif [ "$ARCH" = "arm64" ]; then \
        SV2_TP_URL=https://github.com/Sjors/sv2-tp/releases/download/$SV2_TP_VERSION/sv2-tp-1.0.2-aarch64-linux-gnu.tar.gz; \
    else \
        echo "Unsupported architecture"; exit 1; \
    fi && \
    wget $SV2_TP_URL -O /tmp/sv2-tp.tar.gz && \
    tar -xzvf /tmp/sv2-tp.tar.gz -C /tmp && \
    mkdir -p $BITCOIN_DIR/bin && \
    cp /tmp/sv2-tp-*/bin/* $BITCOIN_DIR/bin/ 2>/dev/null || cp /tmp/sv2-tp-*/sv2-tp $BITCOIN_DIR/bin/ && \
    rm -rf /tmp/sv2-tp.tar.gz /tmp/sv2-tp*

# Download and install Bitcoin Core v30.0rc1
RUN ARCH=$(dpkg --print-architecture) && \
    if [ "$ARCH" = "amd64" ]; then \
        BITCOIN_CORE_URL=https://bitcoincore.org/bin/bitcoin-core-30.0/test.rc1/bitcoin-30.0rc1-x86_64-linux-gnu.tar.gz; \
    elif [ "$ARCH" = "arm64" ]; then \
        BITCOIN_CORE_URL=https://bitcoincore.org/bin/bitcoin-core-30.0/test.rc1/bitcoin-30.0rc1-aarch64-linux-gnu.tar.gz; \
    else \
        echo "Unsupported architecture"; exit 1; \
    fi && \
    wget $BITCOIN_CORE_URL -O /tmp/bitcoin-core.tar.gz && \
    tar -xzvf /tmp/bitcoin-core.tar.gz -C /tmp && \
    mkdir -p $BITCOIN_DIR/bin && \
    cp /tmp/bitcoin-$BITCOIN_VERSION/bin/* $BITCOIN_DIR/bin/ && \
    cp /tmp/bitcoin-$BITCOIN_VERSION/libexec/* $BITCOIN_DIR/bin/ && \
    rm -rf /tmp/bitcoin-core.tar.gz /tmp/bitcoin-$BITCOIN_VERSION

# Create a volume for blockchain data and configuration files
VOLUME ["/root/.bitcoin"]