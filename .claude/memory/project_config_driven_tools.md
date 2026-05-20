---
name: Config-driven tool definitions
description: Tool catalog lives in hardcoded Rust statics (src/catalog/mod.rs) but every field is overridable per-tool via [tool:name] config sections.
type: project
---

# Config-driven tool definitions

The tool catalog (src/catalog/mod.rs) defines all tools as static Rust
arrays. These are the vlint defaults. Users can override any field under
`[tool:name]` sections in their config file.

Supported per-tool overrides: binary_name, container_image, lint_args,
format_print_args, format_args, env_vars, container_env_vars.

**Why:** Users need to customize linter behavior per-project without
rebuilding vlint. Decision made 2026-04-02.

**How to apply:** New tool behavior (args, flags, env vars) goes in the
defaults/ config files, not hardcoded in Rust. Any field a user sets in
[tool:name] config overrides the default. See also [[feedback_config_over_cli]].
