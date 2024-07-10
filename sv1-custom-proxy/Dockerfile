FROM rust:1.75 as builder
WORKDIR usr/src/sv1-custom-proxy
COPY ./sv1-custom-proxy .
#RUN cd sv1-custom-proxy/
RUN cargo build --release

FROM rust:1.75
COPY --from=builder usr/src/sv1-custom-proxy/ usr/src/sv1-custom-proxy/
WORKDIR /usr/src/sv1-custom-proxy/