# Automatic implementation

The project-local command `/implement-spec https://github.com/toinux/awtrix-cli/issues/1` runs the approved GitHub ticket graph through the `awtrix-orchestrator` primary agent. The same specification is the default when no argument is supplied. Implementation and independent review use `openai/gpt-6-luna`; orchestration uses `openai/gpt-6.1-sol`.

The workflow lives in the project `implement-spec` skill. It owns isolated Git worktrees, a maximum of two simultaneous implementers initially, acceptance checks, local integration and a durable journal. The runtime journal is created when implementation starts; configuration alone does not complete any ticket.

Restart OpenCode after changing agents, commands, skills or configuration. Then invoke the command in the project directory. It authorizes local implementation commits and merges plus GitHub ticket comments, assignment, state and parent-checklist updates. Code pushes and releases require a separate request. A stopped run is resumed using the same command and its saved journal, reconciled against live GitHub issues.

GitHub is authoritative for ticket state and dependencies; see `issue-tracker.md`. The journal stays at `.scratch/awtrix-cli/automation.md` for local integration evidence. Historical spec/ticket files are archives with the migration map in `.scratch/awtrix-cli/README.md`. Local tickets 01–19 are completed GitHub issues #2–#20. Distinguish historical ticket IDs from GitHub issue numbers when resuming branches/worktrees.

The installed `implement` skill is intended for a single-ticket workflow and contains its own commit/review steps. The automatic workflow uses dedicated worker/reviewer agents instead, keeping graph status and integration in the orchestrator.

Real headless tests require an AWTRIX Linux executable and a compatible host. Physical checks require accessible explicitly chosen hardware. Reports must distinguish executed checks from unavailable validations.
