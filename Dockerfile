FROM lukemathwalker/cargo-chef:latest-rust-1 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin placedog

FROM debian:bookworm-slim AS runtime
WORKDIR /app
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/placedog /usr/local/bin
COPY ./images ./images

RUN mkdir ./.cache

RUN chmod +rx /usr/local/bin/placedog && \
    chmod -R +r ./images && \
    chmod -R 777 ./.cache

RUN useradd -m -u 1000 placedog
RUN chown -R placedog:placedog ./.cache

USER placedog

EXPOSE 8033

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8033/health || exit 1

ENTRYPOINT ["/usr/local/bin/placedog"]
