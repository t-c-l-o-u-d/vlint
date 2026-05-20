---
name: Prefer config files over CLI args
description: When configuring linter behavior, always prefer config files over CLI args where both are possible.
type: feedback
---

# Prefer config files over CLI args

Prefer linter config files (editorconfig, rc files, yaml configs, etc.)
over CLI argument flags for setting linter options.

**Why:** Config files are more explicit, user-overridable, and consistent
with the config-driven tool definition philosophy. See [[project_config_driven_tools]].

**How to apply:** When adding or adjusting linter behavior, reach for a
config file mechanism first. Only use CLI args as a fallback when the tool
provides no config file option for that setting (e.g. shfmt has no flag to
specify an editorconfig path, so --indent is an accepted exception).
