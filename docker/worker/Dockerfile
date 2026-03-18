FROM rust:1.93-slim-bookworm AS build

ARG ZKSYNC_ERA_COMMIT
ARG ZKSYNC_ERA_OS_INTEGRATION_COMMIT=""
ARG CONTRACTS_COMMIT=""
ARG FOUNDRY_ZKSYNC_VERSION
ARG ERA_CONTRACTS_TAG=""
ARG GENESIS_COMMIT

# Install build dependencies
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
  apt-get update && apt-get install -y \
  git \
  bash \
  ca-certificates \
  curl \
  pkg-config \
  libssl-dev \
  python3-pip \
  nodejs \
  npm \
  && rm -rf /var/lib/apt/lists/*

# Install python dependencies for upgrade scripts
RUN --mount=type=cache,target=/root/.cache/pip,sharing=locked \
  pip3 install eth-abi "eth-hash[pycryptodome]" --break-system-packages

# Install yarn globally (needed for era-contracts build)
RUN npm install -g yarn

# Install foundry-zksync and copy to system path (accessible by non-root users)
RUN curl -L https://raw.githubusercontent.com/matter-labs/foundry-zksync/main/install-foundry-zksync | bash
ENV PATH="/root/.foundry/bin:${PATH}"
RUN foundryup-zksync \
    && cp /root/.foundry/bin/* /usr/local/bin/

# Install zkstack CLI from specific commit
# Set global cargo target dir for caching
ENV CARGO_TARGET_DIR=/cargo-target/${ZKSYNC_ERA_COMMIT}
WORKDIR /deps
RUN git clone https://github.com/matter-labs/zksync-era.git zksync-era
WORKDIR /deps/zksync-era
RUN git checkout ${ZKSYNC_ERA_COMMIT}
RUN git submodule update --init --recursive --jobs "$(nproc || echo 4)"
# Override contracts submodule to specific commit if provided
RUN if [ -n "$CONTRACTS_COMMIT" ]; then \
  echo "Overriding contracts submodule to ${CONTRACTS_COMMIT}"; \
  git -C contracts checkout "$CONTRACTS_COMMIT" && \
  git add contracts && \
  git -C contracts submodule update --init --recursive --jobs "$(nproc || echo 4)"; \
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
  git submodule update --init --recursive --jobs "$(nproc || echo 4)"; \
  fi

# Download genesis.json from zksync-os-server repository
RUN mkdir -p /deps/zksync-era/etc/env/file_based && \
  curl -sSf -o /deps/zksync-era/etc/env/file_based/genesis.json \
  "https://raw.githubusercontent.com/matter-labs/zksync-os-server/${GENESIS_COMMIT}/genesis/genesis.json"

WORKDIR /deps/zksync-era

# Install era-contracts (for upgrade scripts) - only if ERA_CONTRACTS_TAG is set
RUN --mount=type=cache,target=/root/.cache/yarn,sharing=locked \
  mkdir -p /deps/era-contracts && \
  if [ -n "$ERA_CONTRACTS_TAG" ]; then \
    git clone https://github.com/matter-labs/era-contracts.git /deps/era-contracts && \
    cd /deps/era-contracts && \
    git checkout ${ERA_CONTRACTS_TAG} && \
    git submodule update --init --recursive --jobs "$(nproc || echo 4)" && \
    yarn install --cache-folder /root/.cache/yarn; \
  fi

# Drop VCS metadata before copying repositories into runtime image.
RUN find /deps/zksync-era /deps/era-contracts -type d -name .git -prune -exec rm -rf '{}' + && \
    find /deps/zksync-era /deps/era-contracts -type f -name .git -delete

# Allow non-root users to write to deps (needed for --user mapping)
RUN git config --system --add safe.directory '*' \
    && chmod -R a+w /deps

# Set HOME for non-root users (forge/yarn cache)
ENV HOME=/tmp

FROM docker:27-cli AS docker-bins
FROM rust:1.93-slim-bookworm AS runtime

# Copy docker CLI and compose plugin from the official docker image.
COPY --from=docker-bins /usr/local/bin/docker /usr/local/bin/docker
COPY --from=docker-bins /usr/local/libexec/docker/cli-plugins/docker-compose /usr/local/lib/docker/cli-plugins/docker-compose

# Install nodejs + npm for yarn.
RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
  apt-get update && apt-get install -y \
  nodejs \
  npm \
  expect \
  && rm -rf /var/lib/apt/lists/*
RUN npm install -g yarn

# Runtime tooling exposed from /usr/local/bin.
COPY --from=build /usr/local/bin/zkstack /usr/local/bin/zkstack
COPY --from=build /usr/local/bin/forge /usr/local/bin/forge
COPY --from=build /usr/local/bin/cast /usr/local/bin/cast

# Fake git command (returns 0 for any invocation).
RUN echo '#!/bin/bash' > /usr/local/bin/git && \
    echo 'exit 0' >> /usr/local/bin/git && \
    chmod +x /usr/local/bin/git

# Upgrade scripts and supporting files.
COPY --from=build /deps/era-contracts /deps/era-contracts
COPY --from=build /deps/zksync-era /deps/zksync-era

# Keep writable deps for --user mapping.
RUN chmod -R a+w /deps

# Set HOME for non-root users (forge/yarn cache)
ENV HOME=/tmp

WORKDIR /workspace
ENTRYPOINT ["/bin/bash"]
