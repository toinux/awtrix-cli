# Issue tracker: GitHub

Issues and specifications for this repository live in GitHub Issues for `toinux/awtrix-cli`. Use the `gh` CLI. GitHub issue state, labels, comments, sub-issues and native dependencies are authoritative.

## Conventions

- Create: `gh issue create --repo toinux/awtrix-cli --title "..." --body-file <file>`.
- Read: `gh issue view <number> --repo toinux/awtrix-cli --json number,title,body,state,labels,assignees,comments`.
- List: `gh issue list --repo toinux/awtrix-cli --state open --limit 100 --json number,title,body,state,labels,assignees`, with appropriate label filters. Paginate when needed.
- Comment: `gh issue comment <number> --repo toinux/awtrix-cli --body-file <file>`.
- Apply/remove labels: `gh issue edit <number> --repo toinux/awtrix-cli --add-label "..."` / `--remove-label "..."`. Use the strings in `triage-labels.md`.
- Claim: `gh issue edit <number> --repo toinux/awtrix-cli --add-assignee @me`.
- Complete: append acceptance and integration evidence, then `gh issue close <number> --repo toinux/awtrix-cli --reason completed`. Reopen with `gh issue reopen` when further work is required.

When a skill says **publish to the issue tracker**, create a GitHub issue. When it says **fetch the relevant ticket**, read the live issue and its comments. Bare issue numbers refer to GitHub issue numbers.

## Specifications and dependencies

- A specification is a parent issue labelled `spec`, with a checklist of implementation issues.
- Link a child using `gh api --method POST repos/toinux/awtrix-cli/issues/<parent>/sub_issues -F sub_issue_id=<child-database-id>`; fetch children with `gh api --paginate repos/toinux/awtrix-cli/issues/<parent>/sub_issues`.
- Fetch an issue's database ID with `gh api repos/toinux/awtrix-cli/issues/<number> --jq .id`. It differs from both the issue number and node ID.
- Add a blocker using `gh api --method POST repos/toinux/awtrix-cli/issues/<child>/dependencies/blocked_by -F issue_id=<blocker-database-id>`; read blockers with `gh api --paginate repos/toinux/awtrix-cli/issues/<child>/dependencies/blocked_by`.
- Native dependencies are canonical. If unavailable, use a `Blocked by: #<number>, ...` body line and inspect each blocker's live state.
- The ready frontier contains open, unclaimed issues whose blockers are all completed. Inspect closed blockers' completion reason/evidence; `ready-for-agent` means specified, not necessarily unblocked.
- Update the parent checklist after completing a child. Close the parent when all required children and final acceptance checks are complete.

## Pull requests as a triage surface

**PRs as a request surface: no.** Set to `yes` if external pull requests should enter triage. GitHub shares issue and PR numbers; use `gh pr view` when resolving an ambiguous number.

## Wayfinding

- A map is a parent issue labelled `wayfinder:map`; its children are sub-issues labelled `wayfinder:<type>` (`research`, `prototype`, `grilling`, or `task`).
- The frontier contains open, unassigned map children with satisfied dependencies, in map order.
- Claim a child with `gh issue edit <number> --add-assignee @me`.
- Resolve it by commenting with the answer, closing it as completed, and appending a gist/link to the map's Decisions-so-far.
