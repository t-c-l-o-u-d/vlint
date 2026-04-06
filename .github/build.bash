#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Build release binary

set -euo pipefail

step() {
  echo "==> $1"
}

main() {
  local version="${1:?usage: build.bash VERSION ARCH PLATFORM}"
  local arch="${2:-x86_64}"
  local platform="${3:-linux}"

  step "Building release binary"
  cargo build --release

  step "Preparing binary artifact"
  cp target/release/vlint "vlint-${version}-${arch}-${platform}"

  step "Build completed successfully"
}

main "$@"
