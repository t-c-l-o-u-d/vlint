---
name: linter-images reference project
description: ~/git/linter-images/ is the bash-based inspiration project for vlint. Contains detection engine spec, container images, and per-linter tool configs.
type: reference
---

# linter-images reference project

The linter-images project at ~/git/linter-images/ is the primary reference for vlint development.

Key files:

- `linter-aio.bash` - main bash script with detection engine and container execution
- `docs/detection-engine.md` - authoritative spec for weighted consensus algorithm with 16 test cases
- `images/lint-*/lint.bash` - per-linter tool orchestration and config lookup patterns
- `images/lint-*/fix.bash` - per-linter auto-fix scripts
- `docs/images.md` - catalog of all 15 linter images and their tools
- `todo.md` - explicitly lists "Rewrite as a single Rust binary" as a goal

Registry: ghcr.io/t-c-l-o-u-d/linter-images
