---
description: Implement the AWTRIX specification automatically with isolated GPT-6 Luna workers and local integration.
agent: awtrix-orchestrator
---

Load the implement-spec skill and execute it for $ARGUMENTS. If no argument is provided, use .scratch/awtrix-cli/spec.md and its issues directory. Implement the approved ticket graph with awtrix-implementer and awtrix-reviewer subagents, maximum two simultaneous implementers initially. Local ticket commits and integration merges are authorized by this command; remote pushes and releases require a separate request. Continue through all available tickets, preserving durable progress and clearly reporting prerequisites that cannot be satisfied in this environment.
