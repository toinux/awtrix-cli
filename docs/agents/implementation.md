# Implementation: integration is not delivery

`/implement-spec <issue>` authorizes local commits, ordinary pushes of its dedicated integration branch and a PR into `main`. Its endpoint is **PR ready for human review**, not merge or release. GitHub closes implementation issues through that PR's closing references when the human merges it. The external-PR triage flag in `issue-tracker.md` is unrelated to this publication policy.

## State and ownership

| Run/ticket state | Evidence | Effect |
|---|---|---|
| pending / active / blocked | Assignment, owner, worktree and blocker | Issue stays open; does not unlock dependents. |
| integrated | Candidate reviewed on both axes, acceptance satisfied, commits reachable from integration, integrated checks passed | Issue stays open; may unlock dependents in this run. |
| PR ready | All scoped tickets integrated, final reviews/local checks and live host CI passed on the published HEAD | Human reviews/merges; issues stay open. |
| delivered | PR actually merged into main, merge commit reachable from freshly fetched origin/main | Reconcile automatic closures and parent completion checkboxes. |

Use `integration/spec-<number>` and a dedicated integration worktree below the **selected checkout root**. Worker worktrees also live below that root, making access portable without machine-specific external-directory permissions. Existing branches/worktrees are inspected and resumed, not reset. Ordinary pushes must use exactly `git push -u origin integration/spec-<number>` (first publication) or `git push origin integration/spec-<number>`; the orchestrator verifies the branch, repository remote and ref before each push. This permission pattern is an operational guard, not a shell sandbox.

Before acquiring a run, resolve the absolute common Git directory with `git rev-parse --path-format=absolute --git-common-dir`. Reserve `<common-git-dir>/awtrix-runs/spec-<number>.lock` using atomic `mkdir` after verifying/creating its parent. Store owner session ID, run UUID, timestamp and checkpoint location inside it. Linked worktrees share this reservation. A failed mkdir means an existing owner: inspect it and GitHub's latest checkpoint, then stop rather than double-dispatching. Same-session resume retains the reservation. A different session requires explicit takeover authorization; age or assignment to the same GitHub user is not proof that a session stopped. Keep the reservation while workers run. Release only a reservation whose owner matches this session, after a durable checkpoint and when no workers remain active. An interruption before owner metadata is written requires explicit reconciliation/takeover too.

## Durable checkpoint

GitHub owns scope, dependencies and issue state. Keep an untracked JSON checkpoint in `<common-git-dir>/awtrix-runs/spec-<number>.json` and copy its complete JSON into a parent issue comment at each state transition, prefixed `<!-- awtrix-run:<run-id> -->`. Ticket comments link that parent checkpoint and include dispatch base, branch/worktree, commit SHAs, review and check evidence. The JSON is a recoverable evidence cache, not permission to override the live graph. Do not commit run files, host paths, session data or generated evidence.

Example checkpoint (replace placeholders with actual results):

```json
{
  "run_id": "uuid",
  "spec": 34,
  "base": "original-main-sha",
  "head": "current-integration-sha",
  "pr": 123,
  "required_checks": [
    "host-test (x86_64-unknown-linux-gnu)",
    "host-test (aarch64-apple-darwin)",
    "host-test (x86_64-pc-windows-msvc)"
  ],
  "tickets": [
    {
      "number": 35,
      "in_scope": true,
      "status": "integrated",
      "dependencies": [],
      "dispatch_base": "ticket-dispatch-base-sha",
      "candidate_head": "worker-head-sha",
      "integrated_sha": "checked-integration-sha",
      "acceptance": [
        {"status": "passed", "evidence": "command/result or evidence link"}
      ],
      "reviews": [
        {"axis": "Standards", "base": "ticket-dispatch-base-sha", "head": "worker-head-sha", "approved": true, "evidence": "review report link"},
        {"axis": "Spec", "base": "ticket-dispatch-base-sha", "head": "worker-head-sha", "approved": true, "evidence": "review report link"}
      ],
      "checks": {"head": "checked-integration-sha", "passed": true, "evidence": "commands and results"}
    }
  ],
  "final_reviews": [],
  "final_checks": {}
}
```

Include every required ticket and external prerequisite, preserving live dependency edges. Mark external prerequisites `in_scope: false` and `delivered` only with acceptance/review/check evidence and `delivery: {"pr": <merged-pr>, "merge_sha": "<actual-merge-sha>"}`. The gate queries that PR live and checks its merge on fetched origin/main, supporting squash/rebase delivery. Fetch `refs/pull/<merged-pr>/head` if its original candidate/integration commit objects are missing. Pending/active/blocked entries need only identity, scope, status and dependencies, plus owner/dispatch/blocker metadata as appropriate. A spec without children is its own scoped ticket. Acceptance entries cover **every** criterion; `waived` requires both evidence and an explicit `waiver` pointing to the spec's permission. Local checks are successful only when every required command passed; final checks include the required end-to-end scenario. `final_reviews` use the same two-axis format against the original `base` and current `head`, and `final_checks` use the check format against `head`.

On resume, reconstruct a missing cache from GitHub comments, verify actual worktree status and commit reachability, refresh live children/dependencies, inspect PR state and check runs, and retain the original base/run ID. A newly introduced prerequisite or changed acceptance criterion invalidates affected cached completion. Preserve failed/uncommitted work. For a push rejected due to remote divergence, fetch and reconcile instead of force-pushing. A partial PR remains draft. If ready PR work changes, return it to draft before pushing/revalidating. If the PR was merged with incomplete criteria, record partial delivery and keep/reopen the affected issues; a merge alone does not satisfy acceptance.

## Executable gates

From the assigned **integration worktree**, use the script at the selected checkout's absolute path (Python standard library only):

```sh
python <root>/.opencode/scripts/implementation_gate.py frontier <checkpoint.json>
python <root>/.opencode/scripts/implementation_gate.py ready <checkpoint.json>
python <root>/.opencode/scripts/implementation_gate.py merged <checkpoint.json>
```

Each command exits nonzero on missing/invalid/stale evidence or command failure. `frontier` validates graph/commit evidence and prints runnable scoped pending tickets; it does not require their prerequisites' GitHub issues to be closed. Refresh the live graph and reconcile state before invoking it. Checks and review statements must be backed by their recorded outputs: the gate verifies their structure and SHAs, not the truth of a fabricated report.

`ready` queries the recorded PR live using `gh`, verifies base/branch/published HEAD, GitHub's parsed closing-issue references, and checks at that HEAD. Set `required_checks` to **all** required jobs from `.github/workflows/verify.yml` (the full three-host matrix), plus ticket-specific checks. Empty check lists, pending/failed/missing required checks and failing extra checks are blockers. Optional skipped jobs, such as tag-only releases, do not replace required checks. After the gate passes, use `gh pr ready <number> --repo toinux/awtrix-cli`. Any subsequent commit invalidates ready evidence; repeat final validation and reviews against the new HEAD.

`merged` requires the recorded PR to be merged into main at the verified HEAD, its merge commit reachable from freshly fetched origin/main, all scoped acceptance/integration evidence and the final reviews/checks. Only then reconcile automatic closures and parent checkboxes; the orchestrator has no direct `gh issue close` permission. Squash/rebase merge is supported through GitHub's recorded merge commit, rather than assuming original worker SHAs must survive on main. If GitHub did not close an issue automatically, report the missing closure to the human with merge evidence.

## Configuration and verification

The primary orchestrator is Sol; implementation and both review axes use the configured Luna agents. Both review sessions are dispatched directly by the primary agent so `subagent_depth: 1` is sufficient. Workers own their local commits; the orchestrator refreshes/integrates against the latest tip and performs integrated verification. Code-review's two axes and Fowler baseline are retained, while a separate merger agent is deliberately unnecessary.

After changing OpenCode configuration, quit and restart OpenCode. Verify loaded command/agents with `opencode debug config` and `opencode debug agent <name>`, and verify the configured Luna model is available before dispatch. Do not silently substitute a model. Run `python tests/agent-workflow/test_implementation_gate.py -v` for the publication/frontier guards; CI runs it on all three hosts.

Global MCP configuration must use direct named entries, e.g. `"mcp": {"parallel-search": {"type": "remote", "url": "https://search.parallel.ai/mcp"}}`. The nested `mcp.servers` shape is not the published OpenCode schema. Machine-global changes are outside the versioned PR; inspect the effective configuration after restarting.
