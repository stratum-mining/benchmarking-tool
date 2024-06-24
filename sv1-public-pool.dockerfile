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
    && apt clean && rm -rf /var/lib/apt/lists/* /tmp/* /var/tmp/*

RUN git clone https://github.com/benjamin-wilson/public-pool.git

WORKDIR /public-pool

#COPY . .

# Build Public Pool using NPM
RUN npm i && npm run build

############################
# Docker final environment #
############################

FROM node:18.16.1-bookworm-slim

# Expose ports for Stratum and Bitcoin RPC
EXPOSE 3333 3334 8332 48332

WORKDIR public-pool

# Copy built binaries into the final image
COPY --from=build /public-pool .
#COPY .env.example .env



CMD ["/usr/local/bin/node", "dist/main"]
