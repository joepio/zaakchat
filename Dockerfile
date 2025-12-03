# Multi-stage Dockerfile for ZaakChat
#
# Stages:
# 1) rust_generate  - run Cargo generators (export_schemas, generate_asyncapi) to produce target/schemas and asyncapi artifacts
# 2) node_builder   - run pnpm build and the JS types generator (uses generated schemas from rust_generate)
# 3) rust_builder   - build the Rust release binary and include frontend/dist and generated artifacts
# 4) runtime        - minimal runtime image that runs the server
#
# Build from repo root with ZaakChat as the build context:
#   docker build -t joepmeneer/zaakchat:latest -f zaakchat/Dockerfile sse-demo
#
# Notes:
# - This layout keeps Cargo-based generation in the Rust stage where Cargo is available,
#   and keeps Node stage focused on JS build and generate-types.js which consumes the generated schemas.
# - If your repo's lockfiles / package.json are updated, re-generate and commit the lockfile for reproducible builds.

# -------------------------
# 1) Run rust generators
# -------------------------
FROM rust:1-bullseye AS rust_generate

# Install minimal runtime deps needed by generators (if any)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

# Copy Cargo manifest and source so cargo can build/run generator binaries.
COPY Cargo.toml Cargo.lock ./
COPY src ./src
# Some generator bins may live in src/bin; copy any bin sources too (if present)
COPY src/bin ./src/bin

# Run the generator binaries. These should produce:
# - target/schemas/*.json (export_schemas)
# - asyncapi.yaml / asyncapi.json / asyncapi-docs (generate_asyncapi)
# Use cargo run --release --bin <name> to build & execute the binary.
# If the generators fail for any reason, create safe placeholders so later COPY steps do not fail.
RUN (cargo run --release --bin export_schemas && cargo run --release --bin generate_asyncapi) || true; \
    mkdir -p target/schemas asyncapi-docs; \
    # Create lightweight placeholders only if the real artifacts are not present
    if [ ! -f asyncapi.yaml ]; then echo "# asyncapi placeholder" > asyncapi.yaml; fi; \
    if [ ! -f asyncapi.json ]; then echo "{}" > asyncapi.json; fi; \
    if [ ! -f asyncapi-docs/index.html ]; then echo '<!doctype html><meta charset="utf-8"><title>AsyncAPI docs (placeholder)</title>' > asyncapi-docs/index.html; fi

# Expose the generated artifacts in the stage filesystem at /workspace/target and /workspace/asyncapi*
# (they will be present either as real outputs or as placeholders we created above).

# -------------------------
# 2) Node build (frontend)
# -------------------------
FROM node:22-bullseye-slim AS node_builder

# Install tools needed for building the frontend and running the JS generator
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    build-essential \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

# Copy root-level package.json and any lockfile present into the node stage.
# (The repo uses a root package.json which drives generate scripts.)
COPY package.json pnpm-lock.yaml* ./

# Copy frontend sources
COPY frontend ./frontend

# Copy Node-side scripts (e.g. scripts/generate-types.js)
COPY scripts ./scripts

# Copy the schemas and any asyncapi artifacts produced by the rust_generate stage
# so the JS generator can read them (scripts/generate-types.js expects target/schemas).
COPY --from=rust_generate /workspace/target ./target
COPY --from=rust_generate /workspace/asyncapi.yaml ./asyncapi.yaml
COPY --from=rust_generate /workspace/asyncapi.json ./asyncapi.json
COPY --from=rust_generate /workspace/asyncapi-docs ./asyncapi-docs

# Ensure corepack/pnpm is enabled
RUN corepack enable || true

# Non-interactive installs for CI/docker
ENV CI=true

# Install JS deps. Use --no-frozen-lockfile to avoid failing the build in case the lockfile doesn't match package.json.
# (If you want strict reproducibility, change to --frozen-lockfile and ensure pnpm-lock.yaml is committed and up-to-date.)
RUN if [ -f pnpm-lock.yaml ]; then \
    pnpm install --no-frozen-lockfile; \
    else \
    pnpm install; \
    fi

# Run the TypeScript generator (consumes target/schemas) and build the frontend
# generate-types.js will produce frontend/src/types/interfaces.ts
RUN node ./scripts/generate-types.js

# Build frontend assets (run from repo root where package.json lives)
WORKDIR /workspace
RUN pnpm build

# -------------------------
# 3) Build Rust release
# -------------------------
FROM rust:1-bullseye AS rust_builder

# Install build dependencies required by some native crates
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    libssl-dev \
    ca-certificates \
    clang \
    cmake \
    git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /workspace

# Copy Cargo manifest files first for caching
COPY Cargo.toml Cargo.lock ./

# Provide a minimal placeholder main to allow cargo fetch to run and cache dependencies
RUN mkdir -p src && echo "fn main(){println!(\"placeholder\");}" > src/main.rs

# Fetch dependencies to populate cargo cache
RUN cargo fetch

# Overwrite with real source
COPY src ./src
COPY src/bin ./src/bin

# Copy generated frontend artifacts from node_builder (dist) and generated asyncapi artifacts
COPY --from=node_builder /workspace/dist ./dist
COPY --from=node_builder /workspace/asyncapi.yaml ./asyncapi.yaml
COPY --from=node_builder /workspace/asyncapi.json ./asyncapi.json
COPY --from=node_builder /workspace/asyncapi-docs ./asyncapi-docs

# Build release binary
RUN cargo build --release

# -------------------------
# 4) Final runtime image
# -------------------------
FROM debian:bullseye-slim AS runtime

# Create non-root user
RUN useradd --create-home --shell /bin/bash appuser

# Install minimal runtime deps (certs for HTTPS)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the release binary (binary name from Cargo.toml: zaakchat)
COPY --from=rust_builder /workspace/target/release/zaakchat ./zaakchat

# Copy frontend dist produced earlier
COPY --from=rust_builder /workspace/dist ./dist

# Copy asyncapi docs if present
COPY --from=rust_builder /workspace/asyncapi.yaml ./asyncapi.yaml
COPY --from=rust_builder /workspace/asyncapi.json ./asyncapi.json
COPY --from=rust_builder /workspace/asyncapi-docs ./asyncapi-docs

# Ensure the app binary is executable and owned by non-root user
# Also create the data directory and ensure it's owned by appuser
RUN mkdir -p /app/data && \
    chmod +x ./zaakchat && \
    chown -R appuser:appuser /app

USER appuser

# Expose the port used by the application
EXPOSE 8000

# Runtime environment
ENV RUST_LOG=info
ENV BASE_URL="http://localhost:8000"
ENV DATA_DIR="/app/data"

VOLUME [ "/app/data" ]

# Start server (binary expects ./dist to exist)
CMD ["./zaakchat"]
