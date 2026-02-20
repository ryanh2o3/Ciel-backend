FROM rust:1-bookworm AS builder

WORKDIR /app
COPY Cargo.toml ./
COPY src ./src

RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
 && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /bin/false ciel

WORKDIR /app
COPY --from=builder /app/target/release/ciel /app/ciel
COPY migrations ./migrations

USER ciel

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
    CMD curl -f http://localhost:8080/health || exit 1

CMD ["/app/ciel"]
