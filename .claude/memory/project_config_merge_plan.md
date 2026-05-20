---
name: Config precedence system
description: Implemented unified config precedence system - first-found wins across Walk/EnvVar/Xdg/Home locations, fallback to vlint default in XDG cache.
type: project
---

# Config precedence system

The config precedence system is implemented in src/catalog/linter.rs
(types) and src/config/resolve.rs (resolution logic).

**Key design decisions (2026-04-03):**

- WorkspacePath location type was dropped - no writing to workspace
- Fallback default is written to $XDG_CACHE_HOME/vlint/<cache_filename>
- $HOME is mounted read-only in all container backends so configs under
  $HOME are accessible at their original paths
- .linter/ and .linters/ directory support was dropped

**Tool precedence chains:**

yamllint: Walk(".yamllint", Home), Walk(".yamllint.yaml", Home),
Walk(".yamllint.yml", Home), EnvVar("YAMLLINT_CONFIG_FILE"),
Xdg("yamllint/config"), Home(".config/yamllint/config")

shellcheck: Walk(".shellcheckrc", Root), Xdg("shellcheck/shellcheckrc"),
Home(".shellcheckrc")

markdownlint-cli2: Walk(".markdownlint-cli2.yaml", Root),
Walk(".markdownlint-cli2.jsonc", Root)

cargo-deny: Walk(".cargo/deny.toml", Root), Walk("deny.toml", Root) -
no default, first found only

shfmt: no ConfigPrecedence - uses CLI args (--indent, --binary-next-line,
etc.) because shfmt has no --config flag for editorconfig

All cargo-* tools: config_precedence = None
