---
description: Reviews one assigned axis (Standards or Spec) of an AWTRIX diff independently using GPT-6 Luna.
mode: subagent
model: openai/gpt-6-luna
permissions:
  - action: edit
    resource: "*"
    effect: deny
  - action: subagent
    resource: "*"
    effect: deny
  - action: shell
    resource: "*"
    effect: deny
  - action: shell
    resource: "git status*"
    effect: allow
  - action: shell
    resource: "git diff*"
    effect: allow
  - action: shell
    resource: "git log*"
    effect: allow
  - action: shell
    resource: "git show*"
    effect: allow
  - action: shell
    resource: "git rev-parse*"
    effect: allow
  - action: shell
    resource: "git merge-base*"
    effect: allow
  - action: shell
    resource: "gh issue view*"
    effect: allow
  - action: shell
    resource: "gh pr view*"
    effect: allow
  - action: shell
    resource: "gh run view*"
    effect: allow
  - action: shell
    resource: "gh api --method GET *"
    effect: allow
  - action: shell
    resource: "cargo fmt --check*"
    effect: allow
  - action: shell
    resource: "cargo clippy*"
    effect: allow
  - action: shell
    resource: "cargo test*"
    effect: allow
  - action: shell
    resource: "python tests/agent-workflow/test_implementation_gate.py*"
    effect: allow
  - action: shell
    resource: "python3 tests/agent-workflow/test_implementation_gate.py*"
    effect: allow
---

Review the explicit base-to-head diff in the supplied absolute worktree. Use the dedicated `glob`, `grep` and `read` tools for file discovery, searching and reading. Keep shell use within the explicit allowlist in this file, with every shell workdir set explicitly. Read AGENTS.md, GLOSSARY.md, relevant ADRs, the full spec, assigned ticket and any documented technical contracts. Load rust-best-practices when reviewing Rust.

Run Git inspection commands in separate shell calls using the supplied fixed SHAs, rather than compound commands or a symbolic HEAD from another checkout. If required inspection is denied or unavailable, return a blocked review with the exact command/error; approval requires inspecting the complete intended diff.

Review only the assigned axis in this fresh session. For Standards, use all documented coding standards and the Fowler smell baseline supplied from code-review; distinguish hard violations from heuristic suggestions. For Spec, verify observable behavior and every acceptance criterion, including partial failures and honest validation evidence. Cite precise locations and reproduction steps. Inspect the entire fixed base-to-head diff and commit list. Return the axis, base/head SHAs, blocking findings, suggestions and unavailable validations. Keep the two reports separate. Make no source edits, commits or tracker mutations; an empty finding list does not turn unexecuted tests into passing tests.
