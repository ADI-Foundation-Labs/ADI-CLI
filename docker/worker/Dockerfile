FROM rust:1.93.0

ARG ZKSYNC_ERA_COMMIT
ARG ZKSYNC_ERA_OS_INTEGRATION_COMMIT=""
ARG CONTRACTS_COMMIT=""
ARG FOUNDRY_ZKSYNC_VERSION
ARG ERA_CONTRACTS_TAG=""

# Install dependencies
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
  apt-get update && apt-get install -y \
  git \
  clang \
  curl \
  build-essential \
  pkg-config \
  libssl-dev \
  nodejs \
  npm \
  python3-pip \
  docker.io \
  docker-compose \
  expect \
  && rm -rf /var/lib/apt/lists/*

# Install python dependencies for upgrade scripts
RUN --mount=type=cache,target=/root/.cache/pip,sharing=locked \
  pip3 install eth-abi "eth-hash[pycryptodome]" --break-system-packages

# Install yarn globally (needed for era-contracts build)
RUN npm install -g yarn

# Install foundry-zksync
RUN curl -L https://raw.githubusercontent.com/matter-labs/foundry-zksync/main/install-foundry-zksync | bash
ENV PATH="/root/.foundry/bin:${PATH}"
RUN foundryup-zksync

# Install zkstack CLI from specific commit
# Set global cargo target dir for caching
ENV CARGO_TARGET_DIR=/cargo-target/${ZKSYNC_ERA_COMMIT}
WORKDIR /deps
RUN git clone https://github.com/matter-labs/zksync-era.git zksync-era
WORKDIR /deps/zksync-era
RUN git checkout ${ZKSYNC_ERA_COMMIT}
RUN git submodule update --init --recursive
# Override contracts submodule to specific commit if provided
RUN if [ -n "$CONTRACTS_COMMIT" ]; then \
  echo "Overriding contracts submodule to ${CONTRACTS_COMMIT}"; \
  git -C contracts checkout "$CONTRACTS_COMMIT" && \
  git -C contracts submodule update --init --recursive; \
  fi
# Patch yanked parity-scale-codec version
RUN echo "[patch.crates-io]" >> Cargo.toml && \
  echo "parity-scale-codec = { version = \"3.6.12\" }" >> Cargo.toml

# Build zkstack CLI
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
  --mount=type=cache,target=/usr/local/cargo/git,sharing=locked \
  --mount=type=cache,target=/cargo-target \
  --mount=type=cache,target=/usr/local/rustup,sharing=locked \
  cargo build --release --manifest-path zkstack_cli/crates/zkstack/Cargo.toml && \
  cp ${CARGO_TARGET_DIR}/release/zkstack /usr/local/bin/zkstack

# zksync-os-integration and submodules (conditional)
RUN if [ -n "$ZKSYNC_ERA_OS_INTEGRATION_COMMIT" ]; then \
  git fetch && \
  git checkout --force ${ZKSYNC_ERA_OS_INTEGRATION_COMMIT} && \
  git submodule update --init --recursive; \
  fi

# Install era-contracts (for upgrade scripts) - only if ERA_CONTRACTS_TAG is set
RUN --mount=type=cache,target=/root/.cache/yarn,sharing=locked \
  if [ -n "$ERA_CONTRACTS_TAG" ]; then \
    mkdir -p /deps/era-contracts && \
    git clone https://github.com/matter-labs/era-contracts.git /deps/era-contracts && \
    cd /deps/era-contracts && \
    git checkout ${ERA_CONTRACTS_TAG} && \
    git submodule update --init --recursive && \
    yarn install --cache-folder /root/.cache/yarn; \
  fi

WORKDIR /workspace
ENTRYPOINT ["/bin/bash"]
