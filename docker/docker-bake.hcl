variable "REGISTRY" {
  default = "harbor.sde.adifoundation.ai/adi-chain/cli"
}

variable "PLATFORMS" {
  default = ["linux/amd64", "linux/arm64"]
}

group "default" {
  targets = ["toolkit-v30-0-2"]
}

target "toolkit-v30-0-2" {
  dockerfile = "docker/worker/Dockerfile"
  platforms = PLATFORMS
  tags = ["${REGISTRY}/adi-toolkit:v30.0.2"]
  args = {
    ZKSYNC_ERA_COMMIT = "a48fd5f99a3fad0542b514fc9c508094230b35f4"
    ERA_CONTRACTS_TAG = "v30-zksync-os-upgrade"
    FOUNDRY_ZKSYNC_VERSION = "latest"
  }
}
