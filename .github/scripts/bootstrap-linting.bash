#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
set -euo pipefail

# shfmt: shell formatter (not pre-installed on the runner)
sudo apt-get install -y shfmt

# markdownlint-cli2: markdown linter (requires Node.js/npm)
npm install --global markdownlint-cli2

# cargo-audit, cargo-deny: Rust security/dependency tooling
cargo install --locked cargo-audit cargo-deny
