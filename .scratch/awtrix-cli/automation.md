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
| 01 | in-progress | ticket/01-diagnostic / .worktrees/ticket-01 | pending assignment | implementation pending |
| 02–19 | ready-for-agent, dependencies enforced | unassigned | — | — |

## Shared contracts

To be established by ticket 01. CLI boundary tests; compact JSON and human output;
stdout results/stderr auxiliary diagnostics; stable error/exit codes; capability-aware HTTP.

## External prerequisites and blockers

- Rust available: rustc/cargo 1.98.1.
- Real AWTRIX Linux headless executable: acquisition/build feasibility not yet checked.
- Physical ESP32, ESP32-S3, TC002 targets: none explicitly supplied; physical checks cannot be claimed.
- Cross-host distribution validation requires suitable runners; local host is Linux.

## Progress log

- Run initialized from clean committed main baseline. No implementation completed yet.
