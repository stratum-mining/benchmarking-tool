FROM debian:stable-slim as build

# Install & update base system
RUN apt-get update && apt-get upgrade -y

# Install necessary tools
RUN apt-get install -y wget tar curl jq

# Set environment variables for Bitcoin Core version and installation directory
ENV BITCOIN_VERSION=sv2-tp-0.1.9
ENV BITCOIN_DIR=/bitcoin

# Create the directory where Bitcoin Core will be installed
RUN mkdir -p $BITCOIN_DIR

RUN ARCH=$(dpkg --print-architecture) && \
    if [ "$ARCH" = "amd64" ]; then \
        BITCOIN_URL=https://github.com/Sjors/bitcoin/releases/download/$BITCOIN_VERSION/bitcoin-$BITCOIN_VERSION-x86_64-linux-gnu.tar.gz; \
    elif [ "$ARCH" = "arm64" ]; then \
        BITCOIN_URL=https://github.com/Sjors/bitcoin/releases/download/$BITCOIN_VERSION/bitcoin-$BITCOIN_VERSION-aarch64-linux-gnu.tar.gz; \
    else \
        echo "Unsupported architecture"; exit 1; \
    fi && \
    wget $BITCOIN_URL -O /tmp/bitcoin.tar.gz

# Extract the downloaded tarball
RUN tar -xzvf /tmp/bitcoin.tar.gz -C $BITCOIN_DIR --strip-components=1

# Cleanup
RUN rm /tmp/bitcoin.tar.gz

# Create a volume for blockchain data and configuration files
VOLUME ["/root/.bitcoin"]