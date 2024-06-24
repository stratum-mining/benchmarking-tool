FROM rust:1.75 as builder
WORKDIR usr/src/stratum/
COPY . .
WORKDIR roles/
RUN cargo build --release

FROM rust:1.75
COPY --from=builder /usr/src/stratum/roles/ /usr/src/stratum/roles
WORKDIR /usr/src/stratum/roles
