# syntax=docker/dockerfile:1.4
# Multi-stage build for minimal production image
# Run scripts/prepare-guestkit-docker.sh before docker build (copies ../guestkit -> ./guestkit).
FROM docker.io/library/rust:1.94-slim-bookworm AS builder

ARG VIRTCTL_VERSION=v1.4.0

RUN apt-get update && apt-get install -y musl-tools curl && rm -rf /var/lib/apt/lists/*
RUN rustup target add x86_64-unknown-linux-musl

# virtctl — pinned layer (no live version probe on every invalidated build)
RUN curl -fsSL "https://github.com/kubevirt/kubevirt/releases/download/${VIRTCTL_VERSION}/virtctl-${VIRTCTL_VERSION}-linux-amd64" \
    -o /usr/local/bin/virtctl \
    && chmod +x /usr/local/bin/virtctl

WORKDIR /build
RUN mkdir -p /out

# GuestKit in-guest agent binary
COPY guestkit/ ./guestkit/
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/build/guestkit/target,sharing=locked \
    cd guestkit \
    && RUSTFLAGS="-C target-feature=+crt-static" cargo build --release --features agent --no-default-features \
       --target x86_64-unknown-linux-musl \
    && install -Dm755 target/x86_64-unknown-linux-musl/release/guestkit /out/guestkit

# Cache Veyron dependency compilation
COPY Cargo.toml Cargo.lock ./
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/build/target,sharing=locked \
    mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs \
    && RUSTFLAGS="-C target-feature=+crt-static" cargo build --release --target x86_64-unknown-linux-musl --bin veyron 2>/dev/null || true \
    && rm -rf src target/x86_64-unknown-linux-musl/release/veyron target/x86_64-unknown-linux-musl/release/deps/veyron-*

COPY src/ src/
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/build/target,sharing=locked \
    touch src/main.rs src/lib.rs \
    && RUSTFLAGS="-C target-feature=+crt-static" cargo build --release --target x86_64-unknown-linux-musl --bin veyron \
    && install -Dm755 target/x86_64-unknown-linux-musl/release/veyron /out/veyron

# Runtime image — alpine for minimal size with debugging capability
FROM docker.io/library/alpine:3.20

RUN apk add --no-cache ca-certificates
COPY --from=builder /out/veyron /usr/local/bin/veyron
RUN ln -sf /usr/local/bin/veyron /usr/local/bin/vmrogue
COPY --from=builder /out/guestkit /usr/local/bin/guestkit
COPY --from=builder /usr/local/bin/virtctl /usr/local/bin/virtctl

USER 10001

ENTRYPOINT ["veyron"]
CMD ["api-serve", "--port", "5151", "--host", "0.0.0.0"]
