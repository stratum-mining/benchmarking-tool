FROM rust:1.75 as builder
WORKDIR usr/src/pools-latency-calculator
COPY ./pools-latency-calculator/ .
RUN cargo build --release

FROM rust:1.75
COPY --from=builder /usr/src/pools-latency-calculator/ /usr/src/pools-latency-calculator/
WORKDIR /usr/src/pools-latency-calculator/