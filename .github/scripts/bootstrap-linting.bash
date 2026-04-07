#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
set -euo pipefail

# shellcheck: the runner's pre-installed version predates --rcfile (added in
# 0.9.0). Download the latest release to guarantee a compatible version.
gh release download --repo koalaman/shellcheck --pattern 'shellcheck-*.linux.x86_64.tar.xz' -D /tmp
tar -xJ -C /tmp -f /tmp/shellcheck-*.linux.x86_64.tar.xz
sudo install -m755 /tmp/shellcheck-*/shellcheck /usr/local/bin/shellcheck

# shfmt: shell formatter (not pre-installed on the runner)
sudo apt-get install -y shfmt

# markdownlint-cli2: markdown linter (requires Node.js/npm)
npm install --global markdownlint-cli2

# cargo-audit, cargo-deny: Rust security/dependency tooling
cargo install --locked cargo-audit cargo-deny
