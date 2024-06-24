# Compile Bitcoin Core from source
FROM debian:stable-slim as build

# Install & update base system
RUN apt-get update && apt-get upgrade -y

# Install git and build tools
RUN apt-get install -y git build-essential libtool autotools-dev autoconf pkg-config libssl-dev libboost-all-dev bsdmainutils libzmq3-dev


# Set environment variables for Bitcoin Core version and installation directory
ENV BITCOIN_DIR=/bitcoin

# Install necessary dependencies
#RUN apk --no-cache add build-base autoconf automake libtool boost-dev openssl-dev db-c++ db-dev miniupnpc-dev libevent-dev git

# Download the selected Stratum source code zip file
RUN git clone https://github.com/Sjors/bitcoin.git

# Build Bitcoin Core
WORKDIR $BITCOIN_DIR

RUN git switch sv2

# Run autogen.sh
RUN ./autogen.sh

# Configure
RUN ./configure --enable-suppress-external-warnings --disable-bench --disable-tests --disable-fuzz-binary --without-gui --disable-wallet

# Build with parallel jobs (use "-j N" for N parallel jobs)
RUN make -j 4

# Copy the custom bitcoin.conf file into the container
# COPY sri-configs/bitcoin.conf /root/.bitcoin/bitcoin.conf

# Create a volume for blockchain data and configuration files
# docker run -v /path/to/host/directory:/root/.bitcoin bitcoin-sv2
VOLUME ["/root/.bitcoin"]

# Expose Bitcoin P2P and RPC ports (optional)
#EXPOSE 8333 8332 8442 18333

# Use the entrypoint to start bitcoind with the custom configuration
#ENTRYPOINT ["/bitcoin/src/bitcoind", "-sv2", "-sv2port=8442", "-sv2interval=10", "-sv2feedelta=100"] 