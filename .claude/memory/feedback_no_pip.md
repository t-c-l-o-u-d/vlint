---
name: No pip installs
description: Never install packages via pip in CI or elsewhere - only use apt.
type: feedback
---

# No pip installs

Never install packages via pip. Use apt only.

**Why:** User preference for system packages over pip.

**How to apply:** When writing CI workflows or install scripts, always use
`sudo apt-get install` rather than `pip install`.
