---
name: implement-spec
description: Implement an approved AWTRIX specification and local ticket dependency graph automatically with isolated Luna subagents and resumable integration.
---

# Implement the ticket graph

## 1. Establish the run

Read AGENTS.md, GLOSSARY.md, relevant ADRs, tracker conventions, the full spec and every ticket. Parse numeric Blocked by edges; validate unique identifiers, existing prerequisites and an acyclic graph. Tickets are the acceptance authority; the spec supplies shared constraints. No new interview or scope expansion is required.

Inspect Git status, branch, worktrees and recent history. Preserve unrelated work. A clean committed baseline is required for isolated worktrees. If a commit is needed, obtain authorization unless the invocation already authorizes local commits. Git identity must already be configured; report missing identity rather than changing Git config.

Create or resume a local integration branch named integration/awtrix-cli. Create a durable run journal at .scratch/awtrix-cli/automation.md recording integration branch, ticket states, dependency graph, branch/worktree assignments, dispatch base SHAs, commit/review/integration evidence, shared contracts and blockers. Record finished tickets with Status: done only after integration and required checks. Active tickets use Status: in-progress. Failed or externally blocked tickets retain that status with an explicit blocker. ready-for-agent means available only when prerequisites are done.

On resume, reconcile the journal against actual Git commits, worktrees, ticket states and check results. A claimed completion without integration evidence must be verified before unlocking dependents.

## 2. Dispatch the frontier

Ticket 01 runs alone and establishes shared contracts. Thereafter select tickets with all prerequisites integrated and verified. Start with at most two implementers; serialize tickets touching the same contracts or files heavily. Do not manufacture dependencies merely to force a linear schedule.

Verify the parent directory before creating .worktrees, then create one Git worktree and branch per ticket from the current integration HEAD. Worktrees live under .worktrees/ticket-NN, branches under ticket/NN-slug. Record each dispatch base SHA. The main working directory remains the integration workspace.

Dispatch only to awtrix-implementer (configured model openai/gpt-6-luna). Supply absolute repository/worktree paths, assigned branch, full spec/ticket references, prerequisite evidence, shared contracts, dispatch base SHA, local commit authorization and exact completion/report requirements. Explain that every tool path/workdir must be explicit. Never substitute another model silently if Luna fails or is unavailable.

Each worker handles one ticket, drives tdd and commits its intended changes. The orchestrator alone updates shared statuses and the run journal. While a worker runs, do not duplicate its implementation in the integration workspace.

## 3. Review and integrate

For each worker result, inspect its actual status, diff, commits and acceptance evidence. Dispatch awtrix-reviewer with the fixed dispatch base SHA and candidate head for an independent Standards + Spec review. Review can run while an unrelated implementation is active. Return blocking findings to the original implementer session; rerun affected tests and review changed areas after fixes.

Merge approved commits into the integration branch in a controlled sequence using ordinary local merges. Preserve unrelated work and history; resolve conflicts deliberately. A worker's passing branch tests do not establish integration success. Run cargo fmt --check, cargo clippy --all-targets --all-features -- -D warnings and cargo test on the integrated Rust tree, plus any ticket-specific required checks. Fix integration issues before unlocking dependents.

Verify every acceptance criterion with concrete evidence. When external prerequisites are unavailable, document what was tested and what remains; mark a ticket blocked when a mandatory criterion cannot be completed. Ticket 19 explicitly permits unavailable physical validations to remain documented as not executed. It does not permit claiming physical compatibility from mocks or headless runs.

After successful integration, record the ticket as done with commit/check/review evidence and commit the journal/status updates locally. Remove a worker worktree only after its work is committed, integrated and recoverable; retain branches and preserve failed/uncommitted work. Recompute the frontier and continue until no further ticket can run.

## 4. Close the run

Run the reproducible end-to-end scenario and final integrated checks, including real headless tests if its executable is available. For a missing AWTRIX executable, first determine whether an official compatible executable or documented local build can be obtained with the available tools; keep automatic acquisition out of the product itself. Record version/source and evidence. Hardware requires an accessible explicitly selected target; do not invent target addresses or hardware results.

Dispatch a final independent awtrix-reviewer review over the original baseline-to-integration diff. Resolve blocking findings, recheck affected behavior and commit intended fixes. Summarize delivered tickets, remaining blockers, integration branch, executed tests, unexecuted headless/hardware validations and how to resume. Completion means all required acceptance checks are satisfied or the run is explicitly reported as partial. A workflow configuration alone is not a completed implementation.

## Operating boundaries

Use only local commits/merges authorized by the invocation. Push, remote publication and firmware/network administration outside the approved spec require their own request. Retain machine state and user changes outside assigned worktrees. Configuration is project-local; worker models are fixed in the agent definitions. The primary orchestrator is Sol; implementers and reviewers are Luna.
