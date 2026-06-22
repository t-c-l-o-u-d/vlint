# Project memories (vlint)

## Overview

- [vlint project overview](project_vlint_overview.md) — Rust rewrite of
  `linter-aio.bash`; backend precedence is PATH → podman → docker
- [Config-driven tool definitions](project_config_driven_tools.md) — catalog
  is hardcoded Rust statics but every field is overridable via `[tool:name]`
  config sections
- [Config precedence system](project_config_merge_plan.md) — first-found
  wins across Walk/EnvVar/Xdg/Home; fallback to vlint default in XDG cache

## Rules

- [Prefer config files over CLI args](feedback_config_over_cli.md) — reach
  for a config file mechanism first; CLI args only when no config option exists
- [Tool output passthrough](feedback_tool_output_passthrough.md) — never
  filter or reformat tool output in Rust; ask first
- [GitHub Actions policy](feedback_github_actions_policy.md) — first-party
  only, SHA-pinned; everything else is a `.github/scripts/` shell script
- [No pip](feedback_no_pip.md) — apt only, never pip
- [Commit message convention](feedback_commit_messages.md) — terse
  single-line lowercase subjects describing the change; no big bodies

## References

- [vlint-images repository](reference_vlint_images.md) — the container
  image repo at ghcr.io/t-c-l-o-u-d/vlint-images
