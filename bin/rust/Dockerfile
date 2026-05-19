FROM rust:latest AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock* ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /work

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/bpcsr_to_fasta /usr/local/bin/bpcsr_to_fasta

RUN chmod +x /usr/local/bin/bpcsr_to_fasta
