# syntax=docker/dockerfile:1.7
FROM rust:1-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release -p tskr-writer

# TODO: switch to alpine
FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/tskr-writer /usr/local/bin/tskr-writer
EXPOSE 8090
ENTRYPOINT ["/usr/local/bin/tskr-writer"]
