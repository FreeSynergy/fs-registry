# fs-registry — multi-stage container build
# Stage 1: Build
FROM docker.io/rust:1.83-slim AS builder

WORKDIR /build

RUN apt-get update && apt-get install -y --no-install-recommends \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

COPY fs-libs/    fs-libs/
COPY fs-bus/     fs-bus/
COPY fs-registry/ fs-registry/

WORKDIR /build/fs-registry
RUN cargo build --release --bin fs-registry

# Stage 2: Runtime
FROM docker.io/debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

RUN mkdir -p /var/lib/freesynergy

COPY --from=builder /build/fs-registry/target/release/fs-registry /usr/local/bin/

ENV FS_REGISTRY_DB=/var/lib/freesynergy/registry.db
ENV FS_GRPC_PORT=50060
ENV FS_REST_PORT=8060

# gRPC
EXPOSE 50060
# REST + Swagger UI
EXPOSE 8060

ENTRYPOINT ["/usr/local/bin/fs-registry"]
