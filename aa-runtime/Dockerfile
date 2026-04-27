# Builder stage: compiles aa-runtime for the target musl architecture
FROM rust:alpine AS builder

# Install build-time dependencies
RUN apk add --no-cache \
    musl-dev \
    protobuf-dev \
    openssl-dev \
    pkgconfig

# Select the correct musl target based on Docker's TARGETARCH build arg,
# add the target, build the release binary, strip it, and place it at a
# fixed path so the final stage can copy it regardless of architecture.
ARG TARGETARCH
RUN case "$TARGETARCH" in \
      amd64) TARGET=x86_64-unknown-linux-musl ;; \
      arm64) TARGET=aarch64-unknown-linux-musl ;; \
      *) echo "Unsupported arch: $TARGETARCH" && exit 1 ;; \
    esac && \
    rustup target add "$TARGET" && \
    echo "$TARGET" > /tmp/rust_target

WORKDIR /app

COPY . .

RUN TARGET=$(cat /tmp/rust_target) && \
    cargo build --release --target "$TARGET" -p aa-runtime && \
    strip "target/$TARGET/release/aa-runtime" && \
    cp "target/$TARGET/release/aa-runtime" /app/aa-runtime

# Final stage: minimal distroless image running as non-root
FROM gcr.io/distroless/static:nonroot AS runtime

COPY --from=builder /app/aa-runtime /aa-runtime

LABEL org.opencontainers.image.source="https://github.com/AI-agent-assembly/agent-assembly" \
      org.opencontainers.image.description="aa-runtime sidecar — AI agent assembly runtime" \
      org.opencontainers.image.licenses="Apache-2.0"

ENTRYPOINT ["/aa-runtime"]
