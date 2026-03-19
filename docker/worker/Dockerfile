# ============================================
# Stage 1: Build zkstack CLI
# ============================================
FROM rust:1.93-slim-bookworm AS builder

ARG ZKSYNC_ERA_COMMIT
ARG ZKSYNC_ERA_OS_INTEGRATION_COMMIT=""
ARG CONTRACTS_COMMIT=""
ARG ERA_CONTRACTS_TAG=""
ARG GENESIS_COMMIT

# Build dependencies only
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    apt-get update && apt-get install -y \
    git \
    clang \
    make \
    curl \
    ca-certificates \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Clone and checkout zksync-era
WORKDIR /build
RUN git clone https://github.com/matter-labs/zksync-era.git
WORKDIR /build/zksync-era
RUN git checkout ${ZKSYNC_ERA_COMMIT} && \
    git submodule update --init --recursive --jobs "$(nproc || echo 4)"

# Override contracts submodule to specific commit if provided
RUN if [ -n "$CONTRACTS_COMMIT" ]; then \
    echo "Overriding contracts submodule to ${CONTRACTS_COMMIT}"; \
    git -C contracts checkout "$CONTRACTS_COMMIT" && \
    git add contracts && \
    git -C contracts submodule update --init --recursive --jobs "$(nproc || echo 4)"; \
    fi

# zksync-os-integration checkout (conditional)
RUN if [ -n "$ZKSYNC_ERA_OS_INTEGRATION_COMMIT" ]; then \
    git fetch && \
    git checkout --force ${ZKSYNC_ERA_OS_INTEGRATION_COMMIT} && \
    git submodule update --init --recursive --jobs "$(nproc || echo 4)"; \
    fi

# Patch yanked parity-scale-codec version
RUN echo "[patch.crates-io]" >> Cargo.toml && \
    echo "parity-scale-codec = { version = \"3.6.12\" }" >> Cargo.toml

# Build zkstack CLI
ENV CARGO_TARGET_DIR=/cargo-target
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/cargo-target,sharing=locked \
    --mount=type=cache,target=/usr/local/rustup,sharing=locked \
    cargo build --release --manifest-path zkstack_cli/crates/zkstack/Cargo.toml && \
    cp /cargo-target/release/zkstack /usr/local/bin/zkstack

# Clone era-contracts (conditional) - needed for upgrade scripts
RUN mkdir -p /build/era-contracts && \
    if [ -n "$ERA_CONTRACTS_TAG" ]; then \
    git clone --depth 1 --branch ${ERA_CONTRACTS_TAG} \
        https://github.com/matter-labs/era-contracts.git /build/era-contracts && \
    cd /build/era-contracts && \
    git submodule update --init --recursive --jobs "$(nproc || echo 4)"; \
    fi

# Download genesis.json
RUN mkdir -p /build/zksync-era/etc/env/file_based && \
    curl -sSf -o /build/zksync-era/etc/env/file_based/genesis.json \
    "https://raw.githubusercontent.com/matter-labs/zksync-os-server/${GENESIS_COMMIT}/genesis/genesis.json"

# Install foundry-zksync
RUN curl -L https://raw.githubusercontent.com/matter-labs/foundry-zksync/main/install-foundry-zksync | bash
ENV PATH="/root/.foundry/bin:${PATH}"
RUN foundryup-zksync

# Remove .git to save ~542MB (fake git in runtime handles zkstack)
RUN find /build -type d -name .git -prune -exec rm -rf '{}' + && \
    find /build -type f -name .git -delete

# ============================================
# Stage 2: Docker CLI binaries
# ============================================
FROM docker:27-cli AS docker-bins

# ============================================
# Stage 3: Runtime image
# ============================================
FROM debian:trixie-slim AS runtime

# Runtime dependencies (minimal)
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    apt-get update && apt-get install -y --no-install-recommends \
    nodejs \
    npm \
    expect \
    && rm -rf /var/lib/apt/lists/*
RUN npm install -g yarn

# Copy Docker CLI from official image
COPY --from=docker-bins /usr/local/bin/docker /usr/local/bin/docker
COPY --from=docker-bins /usr/local/libexec/docker/cli-plugins/docker-compose /usr/local/lib/docker/cli-plugins/docker-compose

# Copy cargo/rustc from builder (zkstack prerequisite check)
COPY --from=builder /usr/local/cargo/bin/cargo /usr/local/bin/cargo
COPY --from=builder /usr/local/cargo/bin/rustc /usr/local/bin/rustc

# Copy compiled binaries
COPY --from=builder /usr/local/bin/zkstack /usr/local/bin/zkstack
COPY --from=builder /root/.foundry/bin/forge /usr/local/bin/forge
COPY --from=builder /root/.foundry/bin/cast /usr/local/bin/cast

# Fake git command (zkstack checks for git but doesn't need real output)
RUN printf '#!/bin/sh\nexit 0\n' > /usr/local/bin/git && \
    chmod +x /usr/local/bin/git

# Copy zksync-era (.git removed, genesis.json included from builder)
# Use --chmod to avoid separate chmod layer (saves ~800MB)
COPY --from=builder --chmod=a+w /build/zksync-era /deps/zksync-era

# Copy era-contracts from builder (if it was cloned)
COPY --from=builder --chmod=a+w /build/era-contracts /deps/era-contracts

# Set HOME for non-root users (forge/yarn cache)
ENV HOME=/tmp

WORKDIR /workspace
ENTRYPOINT ["/bin/bash"]
