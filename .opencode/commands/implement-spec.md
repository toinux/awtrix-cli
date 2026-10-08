---
description: Implement the AWTRIX specification automatically with isolated GPT-6 Luna workers and local integration.
agent: awtrix-orchestrator
---

Load the `implement-spec` skill and execute it for `$ARGUMENTS`. Require an explicit GitHub spec issue URL or number; if none is supplied, ask rather than defaulting to completed issue #1. Follow the live issue graph through implementation, review, integration and required checks, using the configured Luna workers. This command authorizes local commits/merges and GitHub issue comments, assignment, state and checklist updates. Code pushes and releases require a separate request. Record progress in GitHub comments and report blockers.
