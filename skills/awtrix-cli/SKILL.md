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

## 1. Bootstrap the CLI first

**On activation, complete this step before asking about app content or a device
address.** Installing the skill does not install the executable. A missing CLI
is a bootstrap task to perform, not a reason to stop or ask generic project questions.

1. Run `awtrix-cli --version`. If it succeeds, run `awtrix-cli --help` and retain
   that executable for subsequent commands. If it reports `command not found`,
   take the missing-binary branch immediately; another bare `--help` cannot work.
2. Check the normal per-user binary path below (and Cargo's bin directory if
   relevant). If the executable works by absolute path, use it directly.
3. Otherwise announce that you are installing the published CLI and **execute
   the bundled installer now**. Set `SKILL_DIR` to the absolute **Base directory
   for this skill** supplied when it was loaded, not the current project directory.
   The commands below install per-user without sudo and verify SHA-256:
   Keep `$HOME`/`$env:LOCALAPPDATA` as shell expressions; use the installer's
   reported destination as authoritative instead of guessing a user's home path.

   Linux/macOS, in a POSIX shell:

   ```sh
   sh "$SKILL_DIR/scripts/install.sh"
   "$HOME/.local/bin/awtrix-cli" --version
   "$HOME/.local/bin/awtrix-cli" --help
   ```

   Windows, in PowerShell (`$SkillDir` is the loaded skill's absolute directory):

   ```powershell
   & (Join-Path $SkillDir 'scripts\install.ps1')
   & "$env:LOCALAPPDATA\Programs\awtrix-cli\bin\awtrix-cli.exe" --version
   & "$env:LOCALAPPDATA\Programs\awtrix-cli\bin\awtrix-cli.exe" --help
   ```

4. Retain the verified **absolute executable path** in your working context.
   Substitute it for `awtrix-cli` in every example below if the command is not on
   `PATH`. Shell calls can be separate processes: an `export PATH=...` in one call
   may disappear in the next. Reuse the absolute path rather than reinstalling.

**Done means both version and help succeeded.** Then continue with the user's
request, asking only for app requirements or a target that cannot be inferred.
If a tool actually blocks installation, ask specifically to allow that install.
For unsupported hosts, missing scripts, prerequisites, or download errors, read
[installation details and fallbacks](references/installation.md), try the applicable
recovery, and report any remaining blocker explicitly. Preserve the installer's
nonzero status; a failed install is not completion.

## 2. Discover the operation

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

## 3. Establish the target

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
For complex payloads, use `--file` after checking command help.

Load [script and project workflows](references/workflows.md) when deploying
Berry, collecting runtime evidence, managing resources, pruning, or running
headless tests. For settings, inspect `settings --help`, then the specific
operation's help and description; read current values before applying a patch.

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
