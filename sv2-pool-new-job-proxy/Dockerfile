FROM rust:1.75 as builder
WORKDIR usr/src/sv2-pool-new-job-proxy
COPY ./sv2-pool-new-job-proxy .
RUN cargo build --release

FROM rust:1.75
COPY --from=builder usr/src/sv2-pool-new-job-proxy/ usr/src/sv2-pool-new-job-proxy/
WORKDIR /usr/src/sv2-pool-new-job-proxy/