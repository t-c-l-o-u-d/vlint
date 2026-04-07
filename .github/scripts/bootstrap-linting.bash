#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
set -euo pipefail

# ubuntu-24.04 Noble has shellcheck 0.9.0 which supports --rcfile.
# The runner image may have an older pre-installed version; override it.
sudo apt-get update -qq
sudo apt-get install -y shellcheck
sudo install -m755 /usr/bin/shellcheck /usr/local/bin/shellcheck

# Authenticate to GHCR so podman can pull vlint-images OCI tool containers.
echo "${GITHUB_TOKEN}" | podman login ghcr.io -u "${GITHUB_ACTOR}" --password-stdin
