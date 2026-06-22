---
name: Commit message convention
description: Keep commit subjects terse, single-line, lowercase imperative describing the change; no body for small changes, no process-framed prefixes.
type: feedback
---

# Commit message convention

Write terse, single-line commit subjects in lowercase imperative mood that
describe the change itself (e.g. `quiet non-verbose output and exit cleanly
on SIGPIPE`, `exit 1 when format-apply modifies files`, `tidy output`).
Conventional-type prefixes are used where they fit (`build(deps):`, `fix:`,
`feat:`, `style:`, `docs`). Avoid multi-paragraph bodies for ordinary
changes — even a large diff usually gets one line.

**Why:** Matches the repo's git history and the maintainer's preference.
Verbose multi-paragraph bodies and process-framed subjects (describing the
review/workflow rather than the change) are not wanted.

**How to apply:** Default to a single subject line that names the change.
Add a short body only when genuinely needed (the `build(deps): cargo update`
commit's one-line body is the ceiling). To reword an earlier commit when
interactive rebase is unavailable in this environment, rebuild via detached
cherry-pick with `-c core.hooksPath=/dev/null`, then verify the tree is
unchanged (`git diff --quiet <old-tip> HEAD`) before moving the branch.
