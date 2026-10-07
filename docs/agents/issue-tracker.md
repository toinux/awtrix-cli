# Issue tracker: GitHub

Issues and specs live in GitHub Issues for `toinux/awtrix-cli`. Use the `gh` CLI. GitHub issue state, labels, comments and native dependencies are authoritative.

## Operations

- Create: `gh issue create --repo toinux/awtrix-cli --title "..." --body-file <file>`.
- Read: `gh issue view <number> --repo toinux/awtrix-cli --json number,title,body,state,labels,assignees,comments`.
- List: `gh issue list --repo toinux/awtrix-cli --state open --limit 100 --json number,title,body,state,labels,assignees`, with appropriate label filters. Paginate API queries when more results are needed.
- Comment: `gh issue comment <number> --repo toinux/awtrix-cli --body-file <file>`.
- Label: `gh issue edit <number> --repo toinux/awtrix-cli --add-label "..."` / `--remove-label "..."`. See `triage-labels.md`.
- Claim: `gh issue edit <number> --repo toinux/awtrix-cli --add-assignee @me`.
- Complete: append acceptance/integration evidence, then `gh issue close <number> --repo toinux/awtrix-cli --reason completed`. Reopen with `gh issue reopen` when further work is required.

When a skill says **publish to the issue tracker**, create a GitHub issue. When it says **fetch the relevant ticket**, read the live issue and its comments. Resolve bare numbers in the GitHub number space; local ticket numbers are historical identifiers.

## Specifications and dependencies

- A specification is a parent issue labelled `spec`, with a checklist of implementation issues.
- Link children using `gh api --method POST repos/toinux/awtrix-cli/issues/<parent>/sub_issues -F sub_issue_id=<child-database-id>`.
- Fetch children using `gh api --paginate repos/toinux/awtrix-cli/issues/<parent>/sub_issues`.
- Fetch an issue's database ID using `gh api repos/toinux/awtrix-cli/issues/<number> --jq .id`. The database ID differs from the issue number and node ID.
- Add a dependency using `gh api --method POST repos/toinux/awtrix-cli/issues/<child>/dependencies/blocked_by -F issue_id=<blocker-database-id>`.
- Read dependencies using `gh api --paginate repos/toinux/awtrix-cli/issues/<child>/dependencies/blocked_by`.
- Native dependencies are canonical. A `Blocked by: #<number>, ...` body line provides readable pointers; keep it aligned when editing dependencies. If native dependencies are unavailable, use that line as the fallback graph and inspect each blocker's live state.
- The ready frontier contains open, unclaimed issues whose blockers are all completed. `issue_dependencies_summary.blocked_by` counts open blockers; inspect closed blockers' completion evidence/reason before treating them as satisfied. `ready-for-agent` describes ticket readiness, not prerequisite completion.
- Update the parent checklist after completing a child. Close the parent when all required children and final acceptance checks are complete.

## Pull requests as a triage surface

**PRs as a request surface: no.** Set to `yes` if external pull requests should enter triage. GitHub shares issue and PR numbers; use `gh pr view` for PRs.

## Wayfinding operations

- Map: a parent issue labelled `wayfinder:map`, holding Notes / Decisions-so-far / Fog.
- Children: sub-issues labelled `wayfinder:<type>` (`research`, `prototype`, `grilling`, `task`). Create labels as needed.
- Frontier: map children that are open, unassigned and have satisfied dependencies; use map order.
- Claim: assign the child to the driving developer before work.
- Resolve: comment with the answer, close as completed and append a gist/link to the map's Decisions-so-far.

## Local tracker migration — 2026-10-07

The original specification is [#1](https://github.com/toinux/awtrix-cli/issues/1). Local tickets 01–18 were imported as completed issues #2–#19; local ticket 19 was completed as [#20](https://github.com/toinux/awtrix-cli/issues/20). Each issue retains its original content, source marker and dependency links; completed issues have imported evidence comments.

See `.scratch/awtrix-cli/README.md` for the full correspondence. Local spec/ticket files are optional historical archives, ignored by Git; fresh clones read their content from GitHub. The correspondence and `.scratch/awtrix-cli/automation.md` remain versioned for migration references and local integration evidence; live ticket status is read and updated on GitHub.
