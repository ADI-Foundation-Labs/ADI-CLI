# ============================================
# Stage 1: Build zkstack CLI
# ============================================
FROM rust:1.93.0 AS builder

ARG ZKSYNC_ERA_COMMIT
ARG ZKSYNC_ERA_OS_INTEGRATION_COMMIT=""
ARG CONTRACTS_COMMIT=""

# Build dependencies only
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    apt-get update && apt-get install -y \
    git \
    clang \
    build-essential \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Clone and checkout zksync-era
WORKDIR /build
RUN git clone https://github.com/matter-labs/zksync-era.git
WORKDIR /build/zksync-era
RUN git checkout ${ZKSYNC_ERA_COMMIT} && \
    git submodule update --init --recursive

# Override contracts submodule to specific commit if provided
RUN if [ -n "$CONTRACTS_COMMIT" ]; then \
    echo "Overriding contracts submodule to ${CONTRACTS_COMMIT}"; \
    git -C contracts checkout "$CONTRACTS_COMMIT" && \
    git add contracts && \
    git -C contracts submodule update --init --recursive; \
    fi

# zksync-os-integration checkout (conditional)
RUN if [ -n "$ZKSYNC_ERA_OS_INTEGRATION_COMMIT" ]; then \
    git fetch && \
    git checkout --force ${ZKSYNC_ERA_OS_INTEGRATION_COMMIT} && \
    git submodule update --init --recursive; \
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

# ============================================
# Stage 2: Install foundry-zksync
# ============================================
FROM debian:trixie-slim AS foundry

ARG FOUNDRY_ZKSYNC_VERSION

RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    ca-certificates \
    git \
    && rm -rf /var/lib/apt/lists/*

RUN curl -L https://raw.githubusercontent.com/matter-labs/foundry-zksync/main/install-foundry-zksync | bash
ENV PATH="/root/.foundry/bin:${PATH}"
RUN foundryup-zksync

# ============================================
# Stage 3: Runtime image
# ============================================
FROM debian:trixie-slim AS runtime

ARG ERA_CONTRACTS_TAG=""
ARG GENESIS_COMMIT

# Runtime dependencies
# Install Docker CLI from official Docker repo (docker.io package in trixie is problematic)
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    curl \
    git \
    nodejs \
    npm \
    python3-pip \
    expect \
    gnupg \
    && rm -rf /var/lib/apt/lists/* \
    && npm install -g yarn \
    && pip3 install --break-system-packages eth-abi "eth-hash[pycryptodome]"

# Install Docker CLI from official Docker repo
RUN install -m 0755 -d /etc/apt/keyrings \
    && curl -fsSL https://download.docker.com/linux/debian/gpg -o /etc/apt/keyrings/docker.asc \
    && chmod a+r /etc/apt/keyrings/docker.asc \
    && echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.asc] https://download.docker.com/linux/debian bookworm stable" > /etc/apt/sources.list.d/docker.list \
    && apt-get update \
    && apt-get install -y --no-install-recommends docker-ce-cli docker-compose-plugin \
    && rm -rf /var/lib/apt/lists/*

# Install cargo (minimal toolchain for zkstack prerequisite check)
# Copy to /usr/local/bin so it's accessible for user-mapped containers
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal \
    && cp /root/.cargo/bin/cargo /usr/local/bin/cargo \
    && cp /root/.cargo/bin/rustc /usr/local/bin/rustc \
    && rm -rf /root/.rustup /root/.cargo

# Copy compiled binaries
COPY --from=builder /usr/local/bin/zkstack /usr/local/bin/zkstack
COPY --from=foundry /root/.foundry/bin/forge /usr/local/bin/forge
COPY --from=foundry /root/.foundry/bin/cast /usr/local/bin/cast

# Copy zksync-era (full repo needed - zkstack runs git commands at runtime)
# Use --chmod to avoid separate chmod layer (saves ~800MB)
COPY --from=builder --chmod=a+w /build/zksync-era /deps/zksync-era

# Download genesis.json
RUN mkdir -p /deps/zksync-era/etc/env/file_based && \
    curl -sSf -o /deps/zksync-era/etc/env/file_based/genesis.json \
    "https://raw.githubusercontent.com/matter-labs/zksync-os-server/${GENESIS_COMMIT}/genesis/genesis.json"

# era-contracts (conditional) - clone fresh with shallow depth
RUN if [ -n "$ERA_CONTRACTS_TAG" ]; then \
    mkdir -p /deps/era-contracts && \
    git clone --depth 1 --branch ${ERA_CONTRACTS_TAG} \
        https://github.com/matter-labs/era-contracts.git /deps/era-contracts && \
    cd /deps/era-contracts && \
    git submodule update --init --recursive; \
    fi

# Configure for non-root users
RUN git config --system --add safe.directory '*' && \
    if [ -d /deps/era-contracts ]; then chmod -R a+w /deps/era-contracts; fi

# Set HOME for non-root users (forge/yarn cache)
ENV HOME=/tmp

WORKDIR /workspace
ENTRYPOINT ["/bin/bash"]
