# Agent Configuration

This file configures how AI agents should interact with this repository.

## Agent skills

### Issue tracker

Issues and specs live in GitHub Issues for `toinux/awtrix-cli`; use `gh`. See `docs/agents/issue-tracker.md`.

### Implementation

For specs, ticket dependencies, implementation PRs or interrupted runs, follow `docs/agents/implementation.md`. `/implement-spec` delivers a verified PR for human merge; integration unlocks dependents, and merge into `main` completes issues.

### Triage labels

Use the five canonical labels: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout: `GLOSSARY.md` at root, `docs/adr/` for decisions. See `docs/agents/domain.md`.
