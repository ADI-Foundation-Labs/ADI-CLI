variable "REGISTRY" {
  // default = "ghcr.io/adi"
  default = "harbor-v2.dev.internal.adifoundation.ai/adi-chain/cli"
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

target "common" {
  dockerfile = "docker/worker/Dockerfile"
  platforms = PLATFORM != "" ? [PLATFORM] : PLATFORMS
  tags = TAGS != null ? jsondecode(TAGS) : ["${REGISTRY}/adi-toolkit:${CI_COMMIT_REF_SLUG}"]
  cache-from = CACHE_FROM
  cache-to = notequal("", CACHE_TO_REF) ? ["type=registry,ref=${CACHE_TO_REF},mode=max"] : []
  # Disable provenance to avoid Harbor compatibility issues (blob upload invalid)
  attests = ["type=provenance,mode=disabled"]
}

group "default" {
  targets = ["toolkit-v29", "toolkit-v30", "toolkit-v30-0-2"]
}

target "toolkit-v29" {
  inherits = ["common"]
  tags = ["${REGISTRY}/adi-toolkit:v29${TAG_SUFFIX}"]

  args = {
    ZKSYNC_ERA_COMMIT = "7c4c428b1ea3fd75d9884f3e842fb12d847705c1"
    ZKSYNC_ERA_OS_INTEGRATION_COMMIT = "a135c3b09913d49a1323b44ab80e715616934fc7"
    ERA_CONTRACTS_TAG = "zkos-v0.29.11"
    FOUNDRY_ZKSYNC_VERSION = "latest"
  }
}

target "toolkit-v30" {
  inherits = ["common"]
  tags = ["${REGISTRY}/adi-toolkit:v30${TAG_SUFFIX}"]

  args = {
    ZKSYNC_ERA_COMMIT = "a48fd5f99a3fad0542b514fc9c508094230b35f4"
    ERA_CONTRACTS_TAG = "v30-zksync-os-upgrade"
    FOUNDRY_ZKSYNC_VERSION = "latest"
  }
}

target "toolkit-v30-0-2" {
  inherits = ["common"]
  tags = ["${REGISTRY}/adi-toolkit:v30.0.2${TAG_SUFFIX}"]
  args = {
    ZKSYNC_ERA_COMMIT = "a48fd5f99a3fad0542b514fc9c508094230b35f4"
    ERA_CONTRACTS_TAG = "v30-zksync-os-upgrade"
    FOUNDRY_ZKSYNC_VERSION = "latest"
  }
}
