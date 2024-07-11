FROM rust:1.75 as builder
WORKDIR usr/src/sv2-custom-proxy
COPY ./sv2-custom-proxy .
#WORKDIR custom-proxies
RUN cargo build --release

FROM rust:1.75
COPY --from=builder usr/src/sv2-custom-proxy/ usr/src/sv2-custom-proxy/
WORKDIR /usr/src/sv2-custom-proxy/