FROM rust:latest AS builder

WORKDIR /app

RUN git clone --branch dev-dfs https://github.com/Sengenhoff-tim/protgraph_bpcsr_reader .

RUN cargo build --release

FROM debian:bookworm-slim

WORKDIR /work

RUN apt-get update && apt-get install -y ca-certificates procps && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/bpcsr_to_fasta /usr/local/bin/bpcsr_to_fasta

