---
name: Tool output passthrough
description: vlint passes tool output through unchanged; no filtering or post-processing in Rust without explicit approval
type: feedback
---

# Tool output passthrough

Never alter, trim, or post-process tool output in Rust code. Tool behavior
is controlled only through config files (defaults/*.args, defaults/*.ini, etc.).

**Why:** vlint is a faithful runner, not a post-processor. Changing tool
output in code obscures what the tool actually did.

**Exception:** Modifications for user experience are allowed only when
absolutely necessary, and require explicit human approval before implementing.

**How to apply:** If tempted to filter, cap, reformat, or suppress tool
output in runner code, stop and ask first.
