# Automatic implementation

The project-local command `/implement-spec .scratch/awtrix-cli/spec.md` runs the approved ticket graph through the `awtrix-orchestrator` primary agent. Implementation and independent review use `openai/gpt-6-luna`; orchestration uses `openai/gpt-6.1-sol`.

The workflow lives in the project `implement-spec` skill. It owns isolated Git worktrees, a maximum of two simultaneous implementers initially, acceptance checks, local integration and a durable journal. The runtime journal is created when implementation starts; configuration alone does not complete any ticket.

Restart OpenCode after changing agents, commands, skills or configuration. Then invoke the command in the project directory. It authorizes local implementation commits and merges, without remote publication. A stopped run is resumed using the same command and its saved journal.

The installed `implement` skill is intended for a single-ticket workflow and contains its own commit/review steps. The automatic workflow uses dedicated worker/reviewer agents instead, keeping graph status and integration in the orchestrator.

Real headless tests require an AWTRIX Linux executable and a compatible host. Physical checks require accessible explicitly chosen hardware. Reports must distinguish executed checks from unavailable validations.
