#!/usr/bin/env bash
# Installer script for the adi CLI.
#
# Usage:
#   export GITLAB_TOKEN="glpat-..."
#   curl -fsSL --header "PRIVATE-TOKEN: $GITLAB_TOKEN" "https://gitlab.sre.ideasoft.io/api/v4/projects/348/repository/files/install.sh/raw?ref=main" | bash
#   curl -fsSL --header "PRIVATE-TOKEN: $GITLAB_TOKEN" "https://gitlab.sre.ideasoft.io/api/v4/projects/348/repository/files/install.sh/raw?ref=main" | bash -s -- v0.1.0
#
# Environment variables:
#   GITLAB_TOKEN  — GitLab personal or project access token (required, needs `api` scope)
#
# Create a token at:
#   https://gitlab.sre.ideasoft.io/-/user_settings/personal_access_tokens?name=adi-cli-install&scopes=api
#
set -euo pipefail

GITLAB_URL="https://gitlab.sre.ideasoft.io"
PROJECT_ID="348"
PACKAGE_NAME="adi-cli"
INSTALL_DIR="${HOME}/.cargo/bin"
BINARY_NAME="adi"
TOKEN="${GITLAB_TOKEN:-}"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

info()  { printf '\033[1;34m==>\033[0m %s\n' "$*"; }
error() { printf '\033[1;31merror:\033[0m %s\n' "$*" >&2; exit 1; }

need_cmd() {
  if ! command -v "$1" > /dev/null 2>&1; then
    error "Required command '$1' not found. Please install it and try again."
  fi
}

# Authenticated curl wrapper
auth_curl() {
  curl --header "PRIVATE-TOKEN: ${TOKEN}" "$@"
}

# ---------------------------------------------------------------------------
# Detect platform
# ---------------------------------------------------------------------------

detect_os() {
  case "$(uname -s)" in
    Linux*)  echo "linux"  ;;
    Darwin*) echo "darwin" ;;
    *)       error "Unsupported operating system: $(uname -s)" ;;
  esac
}

detect_arch() {
  case "$(uname -m)" in
    x86_64|amd64)  echo "amd64" ;;
    aarch64|arm64)  echo "arm64" ;;
    *)              error "Unsupported architecture: $(uname -m)" ;;
  esac
}

# ---------------------------------------------------------------------------
# Resolve version
# ---------------------------------------------------------------------------

get_latest_version() {
  local api_url="${GITLAB_URL}/api/v4/projects/${PROJECT_ID}/releases"
  local tag
  tag="$(auth_curl -fsSL "${api_url}" | grep -o '"tag_name":"[^"]*"' | head -1 | cut -d'"' -f4)"
  if [ -z "${tag}" ]; then
    error "Could not determine the latest release. Check your GITLAB_TOKEN or specify a version explicitly: bash -s -- v0.1.0"
  fi
  echo "${tag}"
}

# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

main() {
  need_cmd curl
  need_cmd tar
  need_cmd uname

  if [ -z "${TOKEN}" ]; then
    error "GITLAB_TOKEN is required. Export a personal access token with 'api' scope:

  export GITLAB_TOKEN=\"glpat-...\"

  Create one at: ${GITLAB_URL}/-/user_settings/personal_access_tokens?name=adi-cli-install&scopes=api"
  fi

  local version="${1:-}"
  if [ -z "${version}" ]; then
    info "Detecting latest release..."
    version="$(get_latest_version)"
  fi

  # Strip leading 'v' for the package registry URL
  local semver="${version#v}"

  local os arch archive_name download_url
  os="$(detect_os)"
  arch="$(detect_arch)"
  archive_name="${BINARY_NAME}-${os}-${arch}.tar.gz"
  download_url="${GITLAB_URL}/api/v4/projects/${PROJECT_ID}/packages/generic/${PACKAGE_NAME}/${semver}/${archive_name}"

  info "Installing ${BINARY_NAME} ${version} (${os}/${arch})..."

  # Create install directory
  mkdir -p "${INSTALL_DIR}"

  # Download and extract
  TMP_DIR="$(mktemp -d)"
  trap 'rm -rf "${TMP_DIR}"' EXIT

  info "Downloading ${archive_name}..."
  auth_curl -fSL --progress-bar -o "${TMP_DIR}/${archive_name}" "${download_url}" \
    || error "Download failed. Check that version '${version}' exists and your GITLAB_TOKEN has 'api' scope."

  info "Extracting to ${INSTALL_DIR}..."
  tar -xzf "${TMP_DIR}/${archive_name}" -C "${TMP_DIR}"
  install -m 755 "${TMP_DIR}/${BINARY_NAME}" "${INSTALL_DIR}/${BINARY_NAME}"

  # Verify
  if "${INSTALL_DIR}/${BINARY_NAME}" version > /dev/null 2>&1; then
    info "Successfully installed ${BINARY_NAME} ${version} to ${INSTALL_DIR}/${BINARY_NAME}"
  else
    info "Installed ${BINARY_NAME} to ${INSTALL_DIR}/${BINARY_NAME} (could not verify — binary may require additional dependencies)"
  fi

  # PATH hint
  case ":${PATH}:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
      printf '\n'
      info "Add %s to your PATH:\n" "${INSTALL_DIR}"
      printf '  export PATH="%s:$PATH"\n\n' "${INSTALL_DIR}"
      printf '  Or add the line above to your shell profile (~/.bashrc, ~/.zshrc, etc.)\n'
      ;;
  esac
}

main "$@"
