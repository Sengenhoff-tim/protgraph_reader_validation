FROM rust:latest AS builder

WORKDIR /app

RUN git clone https://github.com/Sengenhoff-tim/sp_embl_builder .

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /work

RUN apt-get update && apt-get install -y ca-certificates procps && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/sp_embl_builder /usr/local/bin/sp_embl_builder
