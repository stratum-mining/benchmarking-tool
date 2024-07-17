############################
# Docker build environment #
############################

FROM node:18.16.1-bookworm-slim AS build

# Upgrade all packages and install dependencies
RUN apt-get update \
    && apt-get upgrade -y
RUN DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    python3 \
    build-essential \
    cmake \
    git \
    ca-certificates \
    iproute2 \
    iputils-ping \
    curl \
    && apt clean && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

RUN git clone https://github.com/benjamin-wilson/public-pool.git

WORKDIR /public-pool

# Build Public Pool using NPM
RUN npm i && npm run build

############################
# Docker final environment #
############################

FROM node:18.16.1-bookworm-slim

# Install necessary packages in the final image
RUN apt-get update && apt-get install -y --no-install-recommends \
        iproute2 \
        iputils-ping \
        curl \
    && apt clean && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

# Expose ports for Stratum and Bitcoin RPC
EXPOSE 3333 3334 8332 48332

WORKDIR /public-pool

# Copy built binaries into the final image
COPY --from=build /public-pool .

# Copy the monitoring script into the container
COPY ./pools-latency-calculator/monitor_and_apply_latency.sh /usr/local/bin/monitor_and_apply_latency.sh

# Make the script executable
RUN chmod +x /usr/local/bin/monitor_and_apply_latency.sh

# Run the monitoring script in the background and start the main application
CMD ["/bin/bash", "-c", "/usr/local/bin/monitor_and_apply_latency.sh 10.5.0.19 2 & exec /usr/local/bin/node dist/main"]
