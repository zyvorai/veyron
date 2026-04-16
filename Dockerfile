# Multi-stage build for minimal production image
FROM rust:1.87-slim-bookworm AS builder

RUN apt-get update && apt-get install -y musl-tools curl && rm -rf /var/lib/apt/lists/*
RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /build

# Cache dependency compilation
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo "fn main() {}" > src/main.rs && echo "" > src/lib.rs \
    && RUSTFLAGS="-C target-feature=+crt-static" cargo build --release --target x86_64-unknown-linux-musl 2>/dev/null || true \
    && rm -rf src target/x86_64-unknown-linux-musl/release/vmrogue target/x86_64-unknown-linux-musl/release/deps/vmrogue-*

# Build actual source
COPY src/ src/
RUN touch src/main.rs src/lib.rs \
    && RUSTFLAGS="-C target-feature=+crt-static" cargo build --release --target x86_64-unknown-linux-musl \
    && strip target/x86_64-unknown-linux-musl/release/vmrogue

# Download virtctl for VNC console support
RUN VIRTCTL_VERSION=$(curl -sL https://storage.googleapis.com/kubevirt-prow/release/kubevirt/kubevirt/stable.txt 2>/dev/null || echo "v1.4.0") \
    && curl -sL "https://github.com/kubevirt/kubevirt/releases/download/${VIRTCTL_VERSION}/virtctl-${VIRTCTL_VERSION}-linux-amd64" -o /usr/local/bin/virtctl \
    && chmod +x /usr/local/bin/virtctl

# Runtime image — alpine for minimal size with debugging capability
FROM alpine:3.20

RUN apk add --no-cache ca-certificates
COPY --from=builder /build/target/x86_64-unknown-linux-musl/release/vmrogue /usr/local/bin/vmrogue
COPY --from=builder /usr/local/bin/virtctl /usr/local/bin/virtctl

USER 10001

ENTRYPOINT ["vmrogue"]
CMD ["api-serve", "--port", "5151", "--host", "0.0.0.0"]
