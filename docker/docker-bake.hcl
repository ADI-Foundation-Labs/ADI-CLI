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

variable "NO_CACHE" {
  default = ""  // Set to any non-empty value to disable remote cache
}

target "common" {
  dockerfile = "docker/worker/Dockerfile"
  platforms = PLATFORM != "" ? [PLATFORM] : PLATFORMS
  tags = TAGS != null ? jsondecode(TAGS) : ["${REGISTRY}/adi-toolkit:${CI_COMMIT_REF_SLUG}"]
  cache-from = notequal("", NO_CACHE) ? [] : CACHE_FROM
  cache-to = notequal("", CACHE_TO_REF) ? ["type=registry,ref=${CACHE_TO_REF},mode=max"] : []
  # Disable provenance to avoid Harbor compatibility issues (blob upload invalid)
  attests = ["type=provenance,mode=disabled"]
}

group "default" {
  targets = ["toolkit-v0-30-1"]
}

target "toolkit-v0-30-1" {
  inherits = ["common"]
  tags = ["${REGISTRY}/adi-toolkit:v0.30.1${TAG_SUFFIX}"]
  args = {
    ZKSYNC_ERA_COMMIT = "a48fd5f99a3fad0542b514fc9c508094230b35f4"
    CONTRACTS_COMMIT = "9ddc915c85d1f44c79b5d55e160d384138ed5105"
    FOUNDRY_ZKSYNC_VERSION = "latest"
  }
}
