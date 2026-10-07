---
description: Implement the AWTRIX specification automatically with isolated GPT-6 Luna workers and local integration.
agent: awtrix-orchestrator
---

Load the implement-spec skill and execute it for $ARGUMENTS. If no argument is provided, use https://github.com/toinux/awtrix-cli/issues/1 and its GitHub sub-issues. Resolve legacy local paths through the migration table in .scratch/awtrix-cli/README.md, then fetch live GitHub issues and native dependencies. Implement the approved ticket graph with awtrix-implementer and awtrix-reviewer subagents, maximum two simultaneous implementers initially. Local ticket commits, integration merges and GitHub ticket comments/assignment/state/checklist updates are authorized by this command; remote code pushes and releases require a separate request. Continue through all available tickets, preserving durable progress and clearly reporting prerequisites that cannot be satisfied in this environment.
