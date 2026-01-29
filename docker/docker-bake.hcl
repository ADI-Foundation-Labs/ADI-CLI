variable "REGISTRY" {
  default = "harbor.sde.adifoundation.ai/adi-chain/cli"
}

variable "PLATFORMS" {
  default = ["linux/amd64", "linux/arm64"]
}

group "default" {
  targets = ["toolkit-v29", "toolkit-v30"]
}

target "toolkit-v29" {
  dockerfile = "docker/worker/Dockerfile"
  platforms = PLATFORMS
  tags = ["${REGISTRY}/adi-toolkit:v29.0.11"]
  args = {
    ZKSYNC_ERA_COMMIT = "7c4c428b1ea3fd75d9884f3e842fb12d847705c1"
    ZKSYNC_ERA_OS_INTEGRATION_COMMIT = "a135c3b09913d49a1323b44ab80e715616934fc7"
    ERA_CONTRACTS_TAG = "zkos-v0.29.11"
    FOUNDRY_ZKSYNC_VERSION = "latest"
  }
}

target "toolkit-v30" {
  dockerfile = "docker/worker/Dockerfile"
  platforms = PLATFORMS
  tags = ["${REGISTRY}/adi-toolkit:v30"]
  args = {
    ZKSYNC_ERA_COMMIT = "a48fd5f99a3fad0542b514fc9c508094230b35f4"
    ERA_CONTRACTS_TAG = "v30-zksync-os-upgrade"
    FOUNDRY_ZKSYNC_VERSION = "latest"
  }
}
