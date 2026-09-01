# ============================================
# Stage 1: Build zkstack CLI
# ============================================
FROM rust:1.93-slim-bookworm AS builder

ARG ZKSYNC_ERA_COMMIT
ARG ZKSYNC_ERA_OS_INTEGRATION_COMMIT=""
ARG CONTRACTS_COMMIT=""
# era-contracts checkout: ERA_CONTRACTS_COMMIT pins a full 40-hex commit SHA
# (fetched directly, so it does not have to be a branch tip); ERA_CONTRACTS_TAG
# clones the head of a branch or tag. The commit wins when both are set.
ARG ERA_CONTRACTS_TAG=""
ARG ERA_CONTRACTS_COMMIT=""
ARG GENESIS_COMMIT
# When set (v31 image), build the era-contracts `protocol_ops` CLI and install a
# standard anvil: the v31 upgrade flow drives protocol_ops against an L1 fork.
ARG BUILD_PROTOCOL_OPS=""
# Fee-adjuster contracts source. SSH URL is used so the host SSH agent forwarded
# via `--ssh default` (BuildKit) provides credentials — no tokens are baked into
# image layers. Override REF to pin a branch/tag/commit.
ARG FEE_ADJUSTER_REPO="git@gitlab.sre.ideasoft.io:adi-foundation/adi-chain/fee-adjuster-contracts.git"
ARG FEE_ADJUSTER_REF="main"

ARG NOX_TRANSACTION_FILTERER_REPO="git@gitlab.sre.ideasoft.io:adi-foundation/adi-chain/nox/nox-transaction-filterer.git"
ARG NOX_TRANSACTION_FILTERER_REF="main"
# Pin forge-std for reproducible script builds.
ARG FORGE_STD_TAG="v1.9.4"

# Build dependencies only
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    apt-get update && apt-get install -y \
    git \
    openssh-client \
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
    if [ -n "$ERA_CONTRACTS_COMMIT" ]; then \
    git -C /build/era-contracts init -q && \
    git -C /build/era-contracts remote add origin https://github.com/matter-labs/era-contracts.git && \
    git -C /build/era-contracts fetch --depth 1 origin "$ERA_CONTRACTS_COMMIT" && \
    git -C /build/era-contracts checkout -q FETCH_HEAD; \
    elif [ -n "$ERA_CONTRACTS_TAG" ]; then \
    git clone --depth 1 --branch ${ERA_CONTRACTS_TAG} \
        https://github.com/matter-labs/era-contracts.git /build/era-contracts; \
    fi && \
    if [ -e /build/era-contracts/.git ]; then \
    git -C /build/era-contracts submodule update --init --recursive --jobs "$(nproc || echo 4)"; \
    fi

# Download genesis.json
RUN mkdir -p /build/zksync-era/etc/env/file_based && \
    curl -sSf -o /build/zksync-era/etc/env/file_based/genesis.json \
    "https://raw.githubusercontent.com/matter-labs/zksync-os-server/${GENESIS_COMMIT}/genesis/genesis.json"

# Install foundry-zksync
RUN curl -L https://raw.githubusercontent.com/matter-labs/foundry-zksync/main/install-foundry-zksync | bash
ENV PATH="/root/.foundry/bin:${PATH}"
RUN foundryup-zksync

# Install Node.js (needed to fetch OpenZeppelin sources for contract builds).
RUN curl -fsSL https://deb.nodesource.com/setup_20.x | bash - && \
    apt-get update && apt-get install -y --no-install-recommends nodejs && \
    rm -rf /var/lib/apt/lists/*

# v31: build protocol_ops (drives the upgrade) and a standard anvil (its L1
# fork). Binaries land in /build/extra-bins and are copied onto the runtime
# PATH below. The yarn install primes the era-contracts deps so protocol_ops can
# forge build l1-contracts at runtime.
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
    --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
    --mount=type=cache,target=/cargo-target,sharing=locked \
    --mount=type=cache,target=/usr/local/rustup,sharing=locked \
    mkdir -p /build/extra-bins && \
    if [ -n "$BUILD_PROTOCOL_OPS" ]; then \
      export FOUNDRY_DIR=/opt/foundry-std && \
      curl -L https://foundry.paradigm.xyz | bash && \
      /opt/foundry-std/bin/foundryup && \
      cp /opt/foundry-std/bin/anvil /build/extra-bins/anvil && \
      npm install -g yarn && \
      (cd /build/era-contracts && (yarn install --frozen-lockfile || yarn install) && \
       yarn --cwd da-contracts build:foundry && yarn --cwd l1-contracts build:foundry) && \
      cd /build/era-contracts/protocol-ops && \
      cargo build --release && \
      cp /cargo-target/release/protocol_ops /build/extra-bins/protocol_ops; \
    fi

# Clone repositories and setup SSH.
RUN --mount=type=ssh \
    --mount=type=secret,id=gitlab_token,target=/run/secrets/gitlab_token,required=false \
    set -eu; \
    mkdir -p ~/.ssh && \
    ssh-keyscan -t rsa,ed25519 gitlab.sre.ideasoft.io >> ~/.ssh/known_hosts 2>/dev/null || true; \
    git clone --depth 1 --branch ${FORGE_STD_TAG} \
        https://github.com/foundry-rs/forge-std.git /build/forge-std; \
    if [ -s /run/secrets/gitlab_token ]; then \
        TOKEN="$(cat /run/secrets/gitlab_token)"; \
        FEE_CLONE_URL="https://oauth2:${TOKEN}@$(echo "$FEE_ADJUSTER_REPO" | sed 's#^git@##; s#:#/#')"; \
        NOX_CLONE_URL="https://oauth2:${TOKEN}@$(echo "$NOX_TRANSACTION_FILTERER_REPO" | sed 's#^git@##; s#:#/#')"; \
        echo "Cloning private repositories via HTTPS+token"; \
    else \
        FEE_CLONE_URL="$FEE_ADJUSTER_REPO"; \
        NOX_CLONE_URL="$NOX_TRANSACTION_FILTERER_REPO"; \
        echo "Cloning private repositories via SSH agent"; \
    fi; \
    git clone --depth 1 --branch ${FEE_ADJUSTER_REF} "$FEE_CLONE_URL" /build/fee-adjuster-contracts; \
    git clone --depth 1 --branch ${NOX_TRANSACTION_FILTERER_REF} "$NOX_CLONE_URL" /build/nox-transaction-filterer; \
    git -C /build/nox-transaction-filterer submodule update --init --recursive || echo "Submodule update failed"

# Build fee-adjuster-contracts.
RUN --mount=type=ssh \
    cd /build/fee-adjuster-contracts && \
    npm ci --no-audit --no-fund && \
    rm -rf lib/forge-std && mkdir -p lib && cp -a /build/forge-std lib/forge-std && \
    forge build && \
    rm -rf node_modules/.cache cache_forge

# Build nox-transaction-filterer.
RUN cd /build/nox-transaction-filterer && \
    rm -rf lib/forge-std && mkdir -p lib && cp -a /build/forge-std lib/forge-std && \
    forge build && \
    rm -rf cache_forge

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

# Copy v31 binaries (protocol_ops + anvil). The dir is empty for non-v31
# targets, so this is a no-op there.
COPY --from=builder /build/extra-bins/ /usr/local/bin/

# Copy fee-adjuster-contracts (sources + prebuilt out/) from builder.
# Forge needs to write broadcast/ + cache files at runtime as a non-root user,
# so ensure every directory in the tree is world-rwx (mode 777) and files are
# world-rw (mode 666).
COPY --from=builder /build/fee-adjuster-contracts /deps/fee-adjuster-contracts
RUN find /deps/fee-adjuster-contracts -type d -exec chmod 777 {} + && \
    find /deps/fee-adjuster-contracts -type f -exec chmod 666 {} +

# Copy nox-transaction-filterer from builder.
COPY --from=builder /build/nox-transaction-filterer /deps/nox-transaction-filterer
RUN find /deps/nox-transaction-filterer -type d -exec chmod 777 {} + && \
    find /deps/nox-transaction-filterer -type f -exec chmod 666 {} +

# Set HOME for non-root users (forge/yarn cache)
ENV HOME=/tmp

WORKDIR /workspace
ENTRYPOINT ["/bin/bash"]
