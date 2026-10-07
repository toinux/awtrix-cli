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
| 09 | done | ticket/09-logs / .worktrees/ticket-09 | 02cf98c | 698ccf4+753ddf3; review clear; integrated |
| 10 | done | ticket/10-screen / .worktrees/ticket-10 | 02cf98c | 76abdab+af4fd8c+0dde76d+41b0444; final review clear; integrated240dab5 |
| 08 | done | ticket/08-lifecycle / .worktrees/ticket-08 | c0dac40 | 7809dac+322528f; review clear; integrated102db6d |
| 12 | done | ticket/12-resources / .worktrees/ticket-12 | c0dac40 | 40df070+1ed2174; review clear; integrated102db6d |
| 11 | done | ticket/11-verify / .worktrees/ticket-11 | 6d26718 | ed3f70c+95a61ee+f005294+d63840b; review clear; integrated5463ee7 |
| 15 | done | ticket/15-headless / .worktrees/ticket-15 | 6d26718 | a7f43ae+67d3bb7+c3491b4; review clear; integrated5463ee7; realheadless passed |
| 13 | done | ticket/13-project / .worktrees/ticket-13 | 445270f | 8380053+fcea5b1+e36b6b4; preflight review clear; real protected example create/update/conflict passed; integrated |
| 18 | in-progress (external blocker) | ticket/18-dist / .worktrees/ticket-18 | 445270f | 41af415; software review clear; Linux release/HTTP checks pass; integratedf27aa33; native macOS/Windows pending |
| 03 | done | ticket/03-pushed / .worktrees/ticket-03 | 6ac6db5 | 98ba2df+519ce6c; review clear; realheadless pushed create/update/delete; integrated |
| 14 | done | ticket/14-tracking / .worktrees/ticket-14 | 6ac6db5 | c97c366+1ba7e44+e9a36c1; review clear; integratedffff1a5 |
| 04,16 | in-progress | ticket/04-notify, ticket/16-tests | next journal commit | pending |
| 05–06,17,19 | ready-for-agent, dependencies enforced | unassigned | — | — |

## Shared contracts

Established by ticket01; authoritative detail in docs/cli-contract.md. Domain sibling
commands; --target/AWTRIX_URL; auth options/environment; timeout default3000ms.
Compact JSON errors/results stdout, concise auxiliary diagnostics stderr; fields selects
top-level keys. Exit0 success,1 transport/general,2 arguments,3 auth,4 timeout,5 HTTP,6 incompatible.
Identity uses /device boardType+soc, not arbitrary strings. Describe offline and connected.

## External prerequisites and blockers

- Rust available: rustc/cargo 1.98.1.
- Real AWTRIX Linux headless executable: built /tmp/opencode/awtrix-ng-build/awtrix-linux,
  official source a02f3ab66cd88cbf5c5f08f3dcc47eb231ae0188, version1.2.2.
  CMake/platformio provided transiently via uv; required awtrix-linux target built.
  Default upstream all-target build GCC16 Werror fails; selected executable target succeeds.
- Physical ESP32, ESP32-S3, TC002 targets: none explicitly supplied; physical checks cannot be claimed.
- Cross-host distribution validation requires suitable runners; local host is Linux.
- Ticket18 external blocker: native macOS ARM64 and Windows x86_64 MSVC help/version/HTTP/artifact
  execution pending. CI workflow prepared but not triggered (no remote publication authorized).
  Ticket19 cannot start until18 actually accepted. Continue other independent tickets.

## Progress log

- Run initialized from clean committed main baseline. No implementation completed yet.
- Ticket01 integrated after independent review and corrections; 13 CLI tests pass on integration.
- Tickets02/07 integrated after review fixes. Main command registration conflict resolved;
  scripts receive the same resolved/authenticated ApiClient as device commands.
  Integration fe5cda1: fmt/clippy pass; 2 unit +31 CLI tests pass. Combined Luna review clear.
- Tickets09/10 integrated240dab5. Logs finite network budget, JSONL error/end resume;
  capture official /display/screen packedRGB, bounded read and atomic replacement.
  Screen worker accidentally edited root initially: provenance confirmed, redundant stray
  changes removed before integration. Merge conflicts resolved preserving both domains.
  Integration fmt/clippy pass; 2 unit+46 CLI tests pass; Luna review clear.
- Tickets08/12 integrated102db6d after review fixes: lifecycle/config/data and
  APPLIED_NOT_SAVED507; module leadingheader and multipart resources. Generic file
  download absent from official API returns explicit UNSUPPORTED, no guessed endpoint.
  Resource worker root-stray edits confirmed/removed; assignment isolation reiterated.
  Integrated fmt/clippy pass, 2unit+61CLI tests. Luna review clear.
- Tickets11/15 integrated5463ee7: bounded verification/partialreport and protecteddeploychain;
  headless owns child listener+pidfd, cleans failures, preserves external services.
  Required real AWTRIX1.2.2 start/HTTP/status/stop passed (documented provenance).
  Integrated fmt/clippy pass, 2unit+80CLI tests; independent re-review clear.
  Nonblocking stderr collection diagnostic label issue retained for final hardening.
- Ticket13 integrated after preflight corrections and real protectedheadless checks.
  Initial real422 diagnosed as invalid Berry starter (return true,1000); corrected app
  class/draw/terminalreturn initializer+example. Exactsource update and stale conflict proven.
  Ticket18 pipeline integratedf27aa33, not declared done. Rustls replaces nativeOpenSSL.
  Lockfile merge resolved; integrated fmt/clippy pass,2unit+87CLI+2distribution tests.
- Tickets03/14 integratedffff1a5: pushed upserts with pervariantlimits, 35fieldkind validators;
  durable perproject/endpoint/device ownership, pruneonlymanaged and conservative explicit
  uncertainty recovery releases ownership without remote mutation. Final Luna review clear.
  Integrated fmt/clippy pass,6unit+99CLI+2distribution tests.
