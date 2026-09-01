variable "REGISTRY" {
  default = "harbor-v2.dev.internal.adifoundation.ai/adi-public/cli"
}

variable "PLATFORMS" {
  default = ["linux/amd64", "linux/arm64"]
}

variable "PLATFORM" {
  default = ""  // Single platform override for CI
}

variable "CI_COMMIT_REF_SLUG" {
  default = "main"
}

variable "CACHE_FROM" {
  default = [
    "type=registry,ref=${REGISTRY}/adi-toolkit:cache-${CI_COMMIT_REF_SLUG}",
    "type=registry,ref=${REGISTRY}/adi-toolkit:cache-main"
  ]
}

variable "CACHE_TO_REF" {
  default = ""
}

variable "TAGS" {
  default = null
}

variable "TAG_SUFFIX" {
  default = ""
}

variable "NO_CACHE" {
  default = ""  // Set to any non-empty value to disable remote cache
}

variable "GITLAB_TOKEN" {
  default = ""  // When set, forwarded as BuildKit secret `gitlab_token`
}

variable "ENABLE_SSH" {
  default = ""  // When set, forwards host SSH agent (requires SSH_AUTH_SOCK)
}

target "common" {
  dockerfile = "docker/worker/Dockerfile"
  platforms = PLATFORM != "" ? [PLATFORM] : PLATFORMS
  tags = TAGS != null ? jsondecode(TAGS) : ["${REGISTRY}/adi-toolkit:${CI_COMMIT_REF_SLUG}"]
  cache-from = notequal("", NO_CACHE) ? [] : CACHE_FROM
  cache-to = notequal("", CACHE_TO_REF) ? ["type=registry,ref=${CACHE_TO_REF},mode=max"] : []
  # Credentials for cloning private fee-adjuster-contracts and nox-transaction-filterer:
  # - CI: set GITLAB_TOKEN env -> HTTPS+PAT via BuildKit secret
  # - Local dev: set ENABLE_SSH=1 (with SSH_AUTH_SOCK exported) -> SSH agent
  ssh = notequal("", ENABLE_SSH) ? ["default"] : []
  secret = notequal("", GITLAB_TOKEN) ? ["id=gitlab_token,env=GITLAB_TOKEN"] : []
  # Disable provenance to avoid Harbor compatibility issues (blob upload invalid)
  attests = ["type=provenance,mode=disabled"]
}

group "default" {
  targets = ["toolkit-v0-30-0", "toolkit-v0-30-1", "toolkit-v0-31-0"]
}

target "toolkit-v0-30-0" {
  inherits = ["common"]
  tags = ["${REGISTRY}/adi-toolkit:v0.30.0${TAG_SUFFIX}"]
  args = {
    ZKSYNC_ERA_COMMIT = "79769b7a2c3d5e5e5975e01e499d0e2e8f3fa279"
    CONTRACTS_COMMIT = "a233f74066aed7a9f750fd4ed1efd67b5f5fe9a3"
    FOUNDRY_ZKSYNC_VERSION = "latest"
    GENESIS_COMMIT = "a8e6de4f4f260ab33bb2ac57c441c0bec4a8fb2c"
  }
}

target "toolkit-v0-30-1" {
  inherits = ["common"]
  tags = ["${REGISTRY}/adi-toolkit:v0.30.1${TAG_SUFFIX}"]
  args = {
    ZKSYNC_ERA_COMMIT = "a48fd5f99a3fad0542b514fc9c508094230b35f4"
    CONTRACTS_COMMIT = "9ddc915c85d1f44c79b5d55e160d384138ed5105"
    FOUNDRY_ZKSYNC_VERSION = "latest"
    GENESIS_COMMIT = "a8e6de4f4f260ab33bb2ac57c441c0bec4a8fb2c"
    ERA_CONTRACTS_TAG = "vb-v30.1-upgrade"
  }
}

# v31 upgrade image. The v31 flow is driven by protocol_ops (built from
# era-contracts) plus a standard anvil for its L1 fork; zkstack is unused by the
# upgrade, so its zksync-era/genesis pins mirror v0.30.1. era-contracts is
# pinned by commit on upstream matter-labs/era-contracts, branch
# kl/testnet-v31-calldata (PR #2268, the squash of the mainnet-v31-deploy fork
# branch, identical tree): no tag exists and the branch is unprotected.
target "toolkit-v0-31-0" {
  inherits = ["common"]
  tags = ["${REGISTRY}/adi-toolkit:v0.31.0${TAG_SUFFIX}"]
  args = {
    ZKSYNC_ERA_COMMIT = "a48fd5f99a3fad0542b514fc9c508094230b35f4"
    CONTRACTS_COMMIT = "9ddc915c85d1f44c79b5d55e160d384138ed5105"
    FOUNDRY_ZKSYNC_VERSION = "latest"
    GENESIS_COMMIT = "a8e6de4f4f260ab33bb2ac57c441c0bec4a8fb2c"
    ERA_CONTRACTS_COMMIT = "48177e9b2a6251e3dd555e1bd353f3cf2b9aa6d0"
    BUILD_PROTOCOL_OPS = "1"
  }
}
