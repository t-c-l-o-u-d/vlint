---
name: GitHub Actions policy
description: Only first-party (actions/*) GitHub Actions allowed, all SHA-pinned. Everything else must be a script in .github/scripts/.
type: feedback
---

# GitHub Actions policy

Only first-party GitHub Actions (actions/checkout, actions/upload-artifact,
actions/download-artifact, etc.) are allowed. All must be SHA-pinned to
their latest version. Dependabot monitors for updates.

Everything that isn't a first-party action must be a bash script in
`.github/scripts/`.

**Why:** Security and supply chain control - no third-party actions.

**How to apply:** Never use third-party actions. If you need functionality,
write a script. Pin all actions to full commit SHA with a version comment.
