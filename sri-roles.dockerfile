FROM rust:1.75 as builder
WORKDIR usr/src/stratum/
RUN git clone https://github.com/stratum-mining/stratum.git .
#COPY . .
#WORKDIR roles/
WORKDIR /usr/src/stratum/roles/
RUN cargo build --release

FROM rust:1.75
COPY --from=builder /usr/src/stratum/roles/ /usr/src/stratum/roles
WORKDIR /usr/src/stratum/roles
