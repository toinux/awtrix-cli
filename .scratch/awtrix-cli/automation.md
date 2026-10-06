# AWTRIX automatic implementation

Integration branch: `integration/awtrix-cli`
Original baseline: `b1cfef8`
Models: orchestrator Sol; implementer/reviewer `openai/gpt-6-luna`.
Local commits and merges authorized. Remote publication not authorized.
Concurrency: ticket 01 alone, then at most two implementers.

## Dependency graph (prerequisites)

01: none; 02: 01; 03: 01; 04: 01; 05: 01; 06: 01;
07: 01; 08: 07; 09: 01; 10: 01; 11: 07,09,10;
12: 01; 13: 02,07,08,12; 14: 13; 15: 01;
16: 11,13,15; 17: 16; 18: 01; 19: 03,04,05,06,14,17,18.
Verified unique IDs, existing references, acyclic graph (Luna preflight).

## Ticket state

| Ticket | State | Branch/worktree | Dispatch base | Commit/review/integration evidence |
|---|---|---|---|---|
| 01 | done | ticket/01-diagnostic / .worktrees/ticket-01 | 6991a3b | 0bc68a6 + 5220578; Luna review four blockers corrected; re-review clear; merged; fmt/clippy/test13 passed |
| 02 | done | ticket/02-profils / .worktrees/ticket-02 | 70cb068 | 60d8ed5+907418d; review clear; integrated |
| 07 | done | ticket/07-scripts / .worktrees/ticket-07 | 70cb068 | 3b2a76c+1cc3a28+f4ed0bc; review clear; merge fe5cda1 |
| 09,10 | in-progress | ticket/09-logs, ticket/10-screen | next journal commit | pending |
| 03–06,08,11–19 | ready-for-agent, dependencies enforced | unassigned | — | — |

## Shared contracts

Established by ticket01; authoritative detail in docs/cli-contract.md. Domain sibling
commands; --target/AWTRIX_URL; auth options/environment; timeout default3000ms.
Compact JSON errors/results stdout, concise auxiliary diagnostics stderr; fields selects
top-level keys. Exit0 success,1 transport/general,2 arguments,3 auth,4 timeout,5 HTTP,6 incompatible.
Identity uses /device boardType+soc, not arbitrary strings. Describe offline and connected.

## External prerequisites and blockers

- Rust available: rustc/cargo 1.98.1.
- Real AWTRIX Linux headless executable: acquisition/build feasibility not yet checked.
- Physical ESP32, ESP32-S3, TC002 targets: none explicitly supplied; physical checks cannot be claimed.
- Cross-host distribution validation requires suitable runners; local host is Linux.

## Progress log

- Run initialized from clean committed main baseline. No implementation completed yet.
- Ticket01 integrated after independent review and corrections; 13 CLI tests pass on integration.
- Tickets02/07 integrated after review fixes. Main command registration conflict resolved;
  scripts receive the same resolved/authenticated ApiClient as device commands.
  Integration fe5cda1: fmt/clippy pass; 2 unit +31 CLI tests pass. Combined Luna review clear.
