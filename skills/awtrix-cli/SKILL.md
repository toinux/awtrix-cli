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

Control an AWTRIX NG device through its HTTP(S) API using `awtrix-cli`.
Use temporary pushed apps for status content; use persistent Berry scripts for
on-device logic. Execute the CLI rather than reimplementing its HTTP operations.

## 1. Check availability and discover the operation

Run `awtrix-cli --version` and `awtrix-cli --help`. If the executable is missing,
read [installation](references/installation.md) and run the bundled installer
for the host. It downloads the matching GitHub release binary, verifies SHA-256,
and checks version/help before installing. Add its directory to the current
shell's `PATH`, then repeat both checks. Installing this skill alone does not
execute an installer. Shell examples here use POSIX quoting; adapt them for PowerShell.

Load only the operation needed:

```sh
awtrix-cli apps create --help
awtrix-cli --json describe "apps create"
```

Use `--help` for exact syntax and `describe "<topic>"` for inputs, schemas,
outputs, and examples. Descriptions use an offline reference unless a target
is supplied, including through `AWTRIX_URL`; connected capabilities take
precedence over offline assumptions. Some description topics are family-level
(for example `script`, `screen`); use the subcommand's help for exact flags.

## 2. Establish the target

Use the user's device URL or existing personal profile. Inspect profiles with
`awtrix-cli --json profile list` when needed. Replace `http://awtrix.local`
below with the intended endpoint; it is an example, not an auto-discovery address.

```sh
awtrix-cli --target http://awtrix.local --json device diagnose
awtrix-cli --target http://awtrix.local --json device capabilities
```

Confirm the reported target and capabilities before changing device content.
For ordinary device commands, selection precedence is `--target`, `AWTRIX_URL`,
`--profile`/`AWTRIX_PROFILE`, project profile (project operations), default profile.
An environment URL can therefore override a named profile. Basic authentication
uses `AWTRIX_USERNAME` and `AWTRIX_PASSWORD` or personal profile credentials;
keep credentials out of project manifests and reports. `--timeout` is a per-request
bound in milliseconds, not an observation duration.

## 3. Perform the requested operation

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
For complex payloads, use `--file` after checking command help.

Load [script and project workflows](references/workflows.md) when deploying
Berry, collecting runtime evidence, managing resources, pruning, or running
headless tests. For settings, inspect `settings --help`, then the specific
operation's help and description; read current values before applying a patch.

## 4. Interpret evidence and failures

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
