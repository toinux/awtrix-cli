---
description: Implement an AWTRIX spec with GPT-6 Luna workers and deliver a verified PR for human merge.
agent: awtrix-orchestrator
---

Load the `implement-spec` skill and execute it for `$ARGUMENTS`. Require an explicit GitHub spec issue URL or number. This invocation authorizes scoped local commits/merges, issue comments/assignment, ordinary pushes of the spec integration branch, and creation/update of its draft PR towards `main`. Deliver a verified PR ready for human review. Keep implementation issues open until that PR is merged into `main`; use the publication gates in `docs/agents/implementation.md`. The human owns the merge. Releases, direct pushes to `main`, force pushes and unrelated machine changes require a separate request. Report the PR URL or the exact blocker and resume instructions.
