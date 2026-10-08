---
name: implement-spec
description: Implement an approved AWTRIX specification with isolated Luna workers, resumable ticket integration and a verified PR for human merge.
---

# Implement the ticket graph

## 1. Establish or resume the run

Read AGENTS.md, GLOSSARY.md, relevant ADRs, tracker conventions and `docs/agents/implementation.md`. The latter defines ownership, checkpoints and publication gates. Fetch the explicit spec issue and its comments; ask if absent or ambiguous. Fetch all native sub-issues and blocked-by edges with pagination. Use body references only if native relationships are unavailable, not when they return an empty list. If the spec has no children, treat the spec itself as the implementation ticket. Validate the complete graph, including external prerequisites, for missing issues and cycles. Tickets supply acceptance criteria; the spec supplies shared constraints and agreed test seams.

Inspect status, branch, worktrees, identity and recent history. Resolve the selected repository root with git rev-parse --show-toplevel. Preserve unrelated tracked/untracked work; use a committed base and a clean dedicated integration worktree, rather than requiring the user's checkout to be clean. Fetch origin/main and freeze the original base SHA. Acquire the shared per-spec ownership reservation before changing run state. Resume existing evidence/branches/worktrees; a conflicting owner is a blocker until explicit takeover is authorized.

Create integration/spec-<spec-number> from the frozen main base in <root>/.worktrees/spec-<spec-number>/integration. On resume, reconcile GitHub checkpoints, actual commits, remote branch, PR and current checks before dispatch. Retain the original base. Verify completion claims against main or the current integration history; closure as not planned supplies no prerequisite evidence.

## 2. Dispatch the frontier

Run the frontier gate from `docs/agents/implementation.md`. Open prerequisites may unlock a dependent when their reviewed, acceptance-verified commits are actually integrated in this run. Skip tickets already integrated or delivered; resume a claimed ticket only after reconciling its owner and work. Start with at most two workers and serialize overlapping contracts/files. Recompute the graph from live issues between waves.

Verify the parent directory, then create each worker worktree below <root>/.worktrees/spec-<spec-number>/ticket-<ticket-number>, with branch ticket/<spec-number>/<ticket-number>-slug, from the current integration HEAD. Record its dispatch base before assigning @me and dispatching. Supply absolute paths, branch, spec/ticket bodies and comments, prerequisite evidence, shared contracts, agreed test seams, dispatch SHA and local commit authorization.

Dispatch only awtrix-implementer (openai/gpt-6-luna). Each worker verifies its base, uses tdd at the approved seams and returns commits with criterion-by-criterion evidence and test results. If seams are not yet agreed, obtain one shared agreement before dispatch rather than repeating the interview per worker. An unavailable model or harness failure is a checkpointed blocker, not permission to switch models. The orchestrator owns merges, publication and tracker mutations.

## 3. Review and integrate

Inspect each result's actual status, full diff and commits. Launch two fresh awtrix-reviewer sessions in parallel: one Standards and one Spec, both against the same fixed dispatch base and candidate HEAD. Supply the code-review skill's standards discovery rules and Fowler baseline to the Standards session, and full acceptance criteria to the Spec session. Keep reports separate and return blocking findings to the implementer. Recheck and re-review each changed candidate.

Merge approved commits sequentially into the integration worktree. This deliberately assigns the upstream worker-refresh/merger responsibilities to the orchestrator: workers do not merge shared branches. Resolve conflicts explicitly and review any substantive resolution. Run cargo fmt --check, cargo clippy --all-targets --all-features --locked -- -D warnings, cargo test --locked and ticket-specific checks against the integrated SHA. Passing worker tests alone do not unlock dependents.

Record each verified integration in the checkpoint and GitHub comments, leaving issues and completion checkboxes open. Run the frontier gate again before dispatching dependents. Keep mandatory unavailable criteria blocked; only the explicit spec can waive a validation. Report headless/hardware checks as executed, waived or unavailable with evidence, never inferred from mocks. Preserve uncommitted, failed or unintegrated worktrees.

After the first successful merge, publish only integration/spec-<spec-number> and create a draft PR into main with closing references to the spec and implementation tickets. Reuse its recorded PR on resume. Keep a partial run draft and report blockers without closing issues.

## 4. Deliver the PR

After all criteria are satisfied, run the required end-to-end scenario and integrated checks. Load code-review and dispatch its two axes directly from the primary orchestrator using awtrix-reviewer; depth 1 and reviewer task: deny are intentional. Freeze the original base and final HEAD for both sessions. Supply the full integration diff, commit list, standards sources/baseline and spec. Resolve blockers in one assigned implementer, then rerun affected checks and both final axes for the new HEAD.

Push the final integration branch. Wait for the actual PR HEAD's full host CI matrix. Store the exact required check names, local validation and separate final review evidence in the checkpoint. Run the ready gate; missing, pending, failed or stale evidence keeps the PR draft. If checks pass, mark the existing PR ready for human review and post its URL, HEAD, delivered tickets, checks and resume details to the parent. This invocation ends at PR ready; all implementation issues remain open pending human merge.

If resumed after human merge, fetch main and run the merged gate before reconciling GitHub's automatic closures and checking completed children in the parent. If the PR is closed without merge, report partial work and preserve the branch. Clean up only clean worker worktrees whose committed HEAD is reachable from the published integration branch (or main after merge). Keep the integration workspace and durable checkpoint recoverable. Release only this session's reservation after recording a stable checkpoint.
