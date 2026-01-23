#!/usr/bin/env bash
set -euo pipefail

have() { command -v "$1" >/dev/null 2>&1; }

echo "[bootstrap] Ensuring pre-commit…"
if ! have pre-commit; then
  if have pipx; then
    pipx install pre-commit
  elif have brew; then
    brew install pre-commit
  elif have pip3; then
    pip3 install --user pre-commit
    echo ">> Added via pip3 --user. Ensure ~/.local/bin is on PATH."
  else
    echo "Install python3 + pipx or Homebrew first. Aborting."; exit 1
  fi
fi


echo "[bootstrap] Installing git hooks…"
pre-commit install --install-hooks
pre-commit autoupdate # keep hook repos pinned to latest

echo "✅ pre-commit ready. Try: pre-commit run --all-files"
