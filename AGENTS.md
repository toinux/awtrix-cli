# Agent Configuration

This file configures how AI agents should interact with this repository.

## Agent skills

### Issue tracker

Issues and specs live in GitHub Issues (`toinux/awtrix-cli`). See `docs/agents/issue-tracker.md` for operations and migrated ticket references.

### Triage labels

Default canonical labels: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout: `GLOSSARY.md` at root, `docs/adr/` for decisions. See `docs/agents/domain.md`.

### Automatic implementation

For implementing the complete ticket graph, use `/implement-spec` with the project-local Luna agents. See `docs/agents/automation.md` for launch and resume rules.
