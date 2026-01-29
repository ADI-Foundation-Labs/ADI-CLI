FROM rust:1.93.0

ARG ZKSYNC_ERA_COMMIT
ARG ZKSYNC_ERA_OS_INTEGRATION_COMMIT=""
ARG ERA_CONTRACTS_TAG
ARG FOUNDRY_ZKSYNC_VERSION

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
  && rm -rf /var/lib/apt/lists/*

# Install python dependencies for upgrade scripts
RUN pip3 install eth-abi "eth-hash[pycryptodome]" --break-system-packages

# Install yarn globally (needed for era-contracts build)
RUN npm install -g yarn

# Install foundry-zksync
RUN curl -L https://raw.githubusercontent.com/matter-labs/foundry-zksync/main/install-foundry-zksync | bash
ENV PATH="/root/.foundry/bin:${PATH}"
RUN foundryup-zksync

# Install zkstack CLI from specific commit
WORKDIR /deps
RUN git clone https://github.com/matter-labs/zksync-era.git zksync-era
WORKDIR /deps/zksync-era
RUN git checkout ${ZKSYNC_ERA_COMMIT}
RUN --mount=type=cache,target=/usr/local/cargo/registry,sharing=locked \
  cargo install --path zkstack_cli/crates/zkstack --force --locked

# zksync-os-integration and submodules (conditional)
RUN if [ -n "$ZKSYNC_ERA_OS_INTEGRATION_COMMIT" ]; then \
  git fetch && \
  git checkout --force ${ZKSYNC_ERA_OS_INTEGRATION_COMMIT} && \
  git submodule update --init --recursive; \
  fi

# Install era-contracts (for upgrade scripts)
WORKDIR /deps/era-contracts
RUN git clone https://github.com/matter-labs/era-contracts.git .
RUN git checkout ${ERA_CONTRACTS_TAG}
RUN git submodule update --init --recursive
# Pre-install dependencies for faster runtime
RUN yarn install

WORKDIR /workspace
ENTRYPOINT ["/bin/bash"]
