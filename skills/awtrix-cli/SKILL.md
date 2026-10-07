---
name: awtrix-cli
description: >-
  Use when a task involves an AWTRIX NG display: send build or job notifications,
  show progress with temporary apps, inspect device capabilities or settings,
  capture the screen, deploy and debug persistent Berry scripts, manage their
  modules and resources, or validate and test awtrix.toml projects. Guides use
  and installation of the awtrix-cli executable, structured output, protected
  updates, and bounded runtime verification.
---

# AWTRIX CLI

`awtrix-cli` is the primary interface and source of truth. This skill teaches
efficient use; it does not duplicate CLI documentation or require rediscovering
it on every task. Execute the CLI rather than reimplementing its HTTP operations.

## 1. Select the workflow

Match the user's intent to the smallest suitable capability:

| Intent | Capability |
| --- | --- |
| Temporary content or progress | `apps` |
| One-off notification | `notify` |
| Inspect/change an existing script; quick standalone experiment | `script` |
| Develop/maintain source files; coordinate resources or configuration | Consider `project` |
| Validate/deploy an `awtrix.toml` project | `project` |
| Automated assertions | `test` |
| Device information, settings, image, diagnostics, resources | `device`, `settings`, `screen`, `logs`, `resources` respectively |

Standalone persistent scripts are legitimate. Consider `project` when creating
files to retain/edit, managing multiple resources or configuration, iterating,
or planning future maintenance. If both fit, briefly propose a project before
creating files. Use `.ax` for new AWTRIX script sources.

Load [script/project workflows](references/workflows.md) only for script work,
project scaffolding/deployment, resources, runtime evidence, cleanup, or tests.
For a project, prefer `project init` → edit scaffold → `project validate` →
`project deploy` → verification if needed. First make a bounded check for an
existing `awtrix.toml` in the intended workspace/parent context; reuse a relevant
project rather than creating a nested duplicate. The generated scaffold is the
starting point, not an unrelated device script fetched to learn Berry structure.

## 2. Use known commands; discover only missing information

Before any discovery or inspection, identify the concrete unresolved question
required to complete the task. Reuse information in this skill, a loaded
reference, previous command output, or an explicit example. Documented examples
are reliable enough to execute directly when they match the need.

- Use `--help` only for unknown exact syntax or required options. If the operation
  itself is unknown, inspect its family, e.g. `awtrix-cli script --help`.
- Use `describe "<topic>"` for missing semantics, schemas, capabilities, or
  structured details, e.g. `awtrix-cli --json describe "apps create"`.
- Stop discovery once the question is answered. Do not pair help and describe
  mechanically, repeat help in the same session without a reason, or use help
  to confirm syntax already supplied by the skill or a loaded reference.
- Never guess subcommands or probe supposed names. Discover only when the
  required operation is actually unknown.
- Reuse inspection results while relevant; refresh only after a change, stale
  evidence, or a new unresolved question. Fetch only task-relevant sources/data.

Descriptions are offline unless a target is supplied (including `AWTRIX_URL`).
Connected capabilities take precedence. Some topics are family-level (`script`,
`screen`); consult subcommand help only if exact flags remain unknown.

## 3. Ensure the executable and target

If availability is not already established, `awtrix-cli --version` is sufficient;
help is not an installation check. If missing, follow
[installation and recovery](references/installation.md). Retain the verified
executable path across calls, using its absolute path if absent from `PATH`.

Use the user's device URL or existing personal profile. Inspect profiles with
`awtrix-cli --json profile list` when needed. Replace `http://awtrix.local`
below with the intended endpoint; it is an example, not an auto-discovery address.

Use `device diagnose` for an unresolved reachability/identity question, or
`device capabilities` for required capability information not already available.
A known notification/app operation needs no routine diagnostic preflight.
For ordinary device commands, selection precedence is `--target`, `AWTRIX_URL`,
`--profile`/`AWTRIX_PROFILE`, project profile (project operations), default profile.
An environment URL can therefore override a named profile. Basic authentication
uses `AWTRIX_USERNAME` and `AWTRIX_PASSWORD` or personal profile credentials;
keep credentials out of project manifests and reports. `--timeout` is a per-request
bound in milliseconds, not an observation duration.

## 4. Perform the requested operation

For an authorized notification or named temporary status app:

```sh
awtrix-cli --target http://awtrix.local --json notify send \
  --payload '{"text":"Build passed"}'
awtrix-cli --target http://awtrix.local --json apps create build \
  --payload '{"text":"Building...","lifetimeMs":60000}'
awtrix-cli --target http://awtrix.local --json apps update build \
  --payload '{"text":"Tests: OK","lifetimeMs":60000}'
awtrix-cli --target http://awtrix.local --json --fields apps apps list
awtrix-cli --target http://awtrix.local --json screen capture --output screen.png
```

`apps create` and `apps update` both create or replace the named pushed app;
use a task-owned name. Pushed apps can expire and disappear on reboot.
Prefer updating a named status app over repeatedly queueing notifications.
For complex payloads, use `--file PATH` instead of `--payload JSON`.

For settings, read relevant current values before a patch; apply the discovery
rules above only for syntax or semantics still missing.

## 5. Interpret evidence and failures

- Use `--json` for machine output. `--fields a,b` selects known top-level result
  fields; choose them from `describe` or a complete result, preserving evidence
  needed for the task. `logs follow` emits JSONL and does not accept `--fields`.
- Preserve stdout, stderr, and the process exit status. Failures usually include
  `error.code`/`error.message` on JSON stdout and diagnostics on stderr; verification
  and test failures can instead return a detailed report. Read both streams.
- Exit statuses: `0` success, `1` general/transport/runtime/test failure,
  `2` arguments/target/fields, `3` authentication, `4` timeout,
  `5` generic HTTP failure, `6` incompatibility/protection unavailable.
  Inspect the error code as well: specific HTTP-derived errors can exit `1`.
- After an uncertain write (`OUTCOME_UNKNOWN`, transport interruption, or
  `APPLIED_STATE_UNKNOWN`), inspect device state before deciding what to do next.
  Replaying a notification can duplicate it; its name is not an idempotency key.
- Protected script updates require `scriptUpdates`. A conflict or unavailable
  protection calls for inspecting the source/capabilities, not automatic `--force`.
- Scope deletion, reboot, power changes, pruning, and unconditional replacement
  to the user's requested operation. Script deletion also removes persisted data.
  Capture to a task-owned path: PNG output replaces an existing destination.
- HTTP acceptance or saved source is not proof of visibility or runtime health.
  Use bounded verification/capture when evidence is required, and report unavailable
  evidence explicitly. Logs retain only 34 lines; successful observation is not
  a guarantee of future correctness.

Finish by reporting the selected target, what changed, observed evidence/artifact
paths, and any error or uncertain outcome.
