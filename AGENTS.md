# Agent Configuration

This file configures how AI agents should interact with this repository.

## Agent skills

### Issue tracker

Issues and specs live in GitHub Issues for `toinux/awtrix-cli`; use `gh`. See `docs/agents/issue-tracker.md`.

### Implementation

For specs, ticket dependencies, implementation PRs or interrupted runs, follow `docs/agents/implementation.md`. `/implement-spec` delivers a verified PR for human merge; integration unlocks dependents, and merge into `main` completes issues.

### Releases

When asked to cut a release, follow `docs/agents/release.md`, including the mandatory English `CHANGELOG.md` update for every release. A release requires an explicit human confirmation of the proposed version and notes before creating or pushing the version commit and annotated tag.

### Triage labels

Use the five canonical labels: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context layout: `GLOSSARY.md` at root, `docs/adr/` for decisions. See `docs/agents/domain.md`.

### Feature coverage

`docs/feature-coverage.md` records what the AWTRIX NG device API offers and what
this CLI covers. Re-check it with the `awtrix-coverage` project skill before
claiming a feature is uncovered or after an AWTRIX NG release.

### CLI discoverability

When changing CLI behavior, help, or schemas, treat executable discovery as the source of truth: execution `--help` provides syntax; `describe --list` catalogs topics; `describe <topic> --offline` provides local semantics, schemas, outputs, and examples. Keep help, descriptions, and emitted results truthful and aligned, and maintain meaningful discovery tests for the changed contract. See `docs/cli-contract.md` for CLI contracts.
