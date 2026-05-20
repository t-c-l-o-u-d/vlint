---
name: vlint project overview
description: vlint is a Rust binary that runs linters, replacing the bash linter-aio.bash from ~/git/linter-images/. Supports native PATH, podman, and docker backends with weighted consensus file detection.
type: project
---

# vlint project overview

vlint is a Rust rewrite of the bash-based linter runner from ~/git/linter-images/.
See [[reference_linter_images]] for the source-of-truth spec,
[[reference_vlint_images]] for the container image repo.

**Config precedence (highest to lowest):**

1. CLI flag
2. User config ($XDG_CONFIG_HOME/vlint/config.ini)

- Workspace/project-level config is not yet implemented (_workspace parameter is stubbed in load_config)
- No global /etc config (user explicitly dropped it)

**Backend discovery precedence:**

1. Native PATH
2. Podman (OCI)
3. Docker (OCI)

nspawn backend was removed 2026-04-10 - it can't pull from arbitrary registries,
making it unusable with the ghcr.io image distribution model.

**How to apply:** Discovery chain is PATH -> podman -> docker.
