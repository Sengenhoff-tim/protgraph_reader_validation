# Stage 1: build
FROM ubuntu:22.04 AS build
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update && \
    apt-get install -y --no-install-recommends build-essential cmake g++ ca-certificates && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /src
COPY . .    

RUN mkdir -p build && cd build && \
    cmake .. -DCMAKE_BUILD_TYPE=Release && \
    cmake --build . --config Release -j$(nproc)

# Stage 2: runtime
FROM ubuntu:22.04
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=build /src/build/protgraphtraverseintvarlimitter /usr/local/bin/protgraphtraverseintvarlimitter
RUN chmod +x /usr/local/bin/protgraphtraverseintvarlimitter

WORKDIR /work

