# Multi-stage build for minimal production image
FROM rust:1.80-slim-bookworm AS builder

WORKDIR /build

# Cache dependency compilation
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs \
    && cargo build --release --locked 2>/dev/null || true \
    && rm -rf src

# Build actual source
COPY src/ src/
RUN cargo build --release --locked && strip target/release/vmrogue

# Runtime image
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/vmrogue /usr/local/bin/vmrogue

RUN useradd -u 10001 -m vmrogue
USER vmrogue

# Health checking is handled by Kubernetes probes (livenessProbe/readinessProbe)
# targeting /api/v1/health on the running server.

ENTRYPOINT ["vmrogue"]
CMD ["api-serve"]
