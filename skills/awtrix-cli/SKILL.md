---
name: awtrix-cli
description: >-
  Use for AWTRIX context: questions or tasks about the awtrix-cli version,
  availability, location, installation, update, use, or troubleshooting, as well
  as AWTRIX NG devices, current app, app inventory and rotation, displays,
  firmware, notifications and progress,
  capabilities and settings, screen capture, persistent Berry scripts, their
  modules and resources, or awtrix.toml projects. Not for an incidental AWTRIX
  mention or an unrelated generic CLI request. Guides use and installation of
  the executable, structured output, protected updates, and bounded runtime
  verification.
---

# AWTRIX CLI

`awtrix-cli` is the primary interface and source of truth. This skill teaches
efficient use; it does not duplicate CLI documentation or require rediscovering
it on every task. Execute the CLI rather than reimplementing its HTTP operations.

## 1. Select the workflow

Match the user's intent to the smallest suitable capability:

| Intent | Capability |
| --- | --- |
| Identify the currently displayed app | `apps active-get` → `current_app` |
| List installed/pushed apps and their enabled/loop state | `apps list` |
| Select an app or inspect/change rotation | `apps select`, `apps order-get`, `apps order-set` |
| See rendered pixels or save an image | `screen capture` |
| Temporary content or progress | `apps` |
| One-off notification | `notify` |
| Minify a local Berry source file | `minify` (local, top-level command) |
| Inspect/change an existing script; quick standalone experiment | `script` |
| Read or manually patch a script's persisted store | `script data`, `script data-patch` |
| Inspect temporary cross-script publications | `script shared` |
| Develop/maintain source files; coordinate resources or configuration | Consider `project` |
| Validate/deploy an `awtrix.toml` project | `project` |
| Automated assertions | `test` |
| Device identity/state, settings, connection diagnostics, logs, resources | `device`, `settings`, `device diagnose`, `logs`, `resources` respectively |

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

### Berry coding cooperation

For a request to create or change Berry code for an AWTRIX NG app/script, use an
available scripting skill whose declared purpose covers writing, maintaining, or
debugging that code. This includes correcting code and applies whether that skill
is already loaded or is identified later; use its normal harness loading mechanism
when needed. Judge by its declared intent, not its name, path, or author. Use only
the skills exposed by the harness; do not search the filesystem or load candidates
just to compare them. Device-only work (including logs, diagnosis, reboot, capture,
and deployment of an existing file) does not trigger scripting-skill discovery.

The scripting skill supplies Berry language, embedded API, lifecycle, and hardware
knowledge. This skill remains authoritative for operations supported by
`awtrix-cli`, including its protections and verification. Its REST examples do not
override the CLI workflow. Use REST only for a needed operation not covered by the
CLI, after a targeted coverage check. These are workflow responsibilities, not a
change to harness instruction priority.

If code work needs Berry guidance and no suitable skill is identified among those
available, briefly offer the official entry point:
<https://blueforcer.github.io/awtrix-ng/esp32/guides/ai-prompt/>. Do not install,
download, or generate a skill. If the exposed inventory may be partial, say none
was identified among available skills rather than asserting none exists. Continue
with CLI-supported work and state any Berry-knowledge limitation; do not repeat the
offer after the user has decided.

When the user can access local files, create or modify the complete `.ax` source
file and report its path, useful changes, and checks actually performed. Do not
intentionally print the full source again after editing, correcting, or deploying;
read only portions needed to diagnose, while a justified full read remains valid.
Return one complete source block only when explicitly requested or when the user
cannot access the file. Keep diagnostics bounded and task-relevant, preserving
errors, uncertainty, and truncation. A saved or accepted source is not proof of
runtime health. This cooperation avoids intentional source repetition, not the
initial context cost of loading a scripting skill.

## 2. Use known commands; discover only missing information

Before any discovery or inspection, identify the concrete unresolved question
required to complete the task. Reuse information in this skill, a loaded
reference, previous command output, or an explicit example. Documented examples
are reliable enough to execute directly when they match the need.

- Use `--help` only for unknown exact syntax or required options. If the operation
  itself is unknown, inspect the known family help, or `awtrix-cli --help` when
  its family is unknown. Discover description topics independently with
  `describe --list` when needed.
- Use `describe <topic> --offline` for missing semantics, schemas, outputs, or
  examples, e.g. `awtrix-cli --json describe apps create --offline`. Topics accept
  separate words or a quoted string. Use `describe --list` only when the required
  topic is unknown.
- Stop discovery once the question is answered. Do not pair help and describe
  mechanically, repeat help in the same session without a reason, or use help
  to confirm syntax already supplied by the skill or a loaded reference.
- Never guess subcommands or probe supposed names. Discover only when the
  required operation is actually unknown.
- Reuse inspection results while relevant; refresh only after a change, stale
  evidence, or a new unresolved question. A question about the current app needs
  a fresh observation because rotation can change it without an agent write.
  Fetch only task-relevant sources/data.

For local minification, use `awtrix-cli --json minify ./src/main.ax`.
It writes sibling `main.min.ax` and leaves the source unchanged.
Its description topic is `minify`:
`awtrix-cli --json describe minify --offline`.
`script re-minify` instead modifies an installed remote script.

Default to `--offline` for local documentation, even when `AWTRIX_URL` is set.
Descriptions document operations without executing them or validating payloads.
Omitting `--offline` with `--target` or `AWTRIX_URL` optionally fetches connected
capabilities; failures propagate and connected capability names are authoritative.
Named/default profiles alone do not connect. `--list`, profile descriptions, and
`describe update` are always local. Offline documentation is not proof of device
support. `script get`, `script deploy`, and `script re-minify` share the script
family description; `screen capture` accepts the screen description. Consult
execution help only if exact flags remain unknown.

## 3. Ensure the executable and target

If availability is not already established, `awtrix-cli --version` is sufficient;
help is not an installation check. If missing, follow
[installation and recovery](references/installation.md). Retain the verified
executable path across calls, using its absolute path if absent from `PATH`.
For an existing installation, run `awtrix-cli update` only when the user chooses
to upgrade; release notices only announce availability and never install updates.

Use the user's device URL or existing personal profile. Inspect profiles with
`awtrix-cli --json profile list` when needed. Replace `http://awtrix.local`
below with the intended endpoint; it is an example, not an auto-discovery address.

Use `device diagnose` for an unresolved reachability/identity question, or
`device capabilities` for required capability information not already available.
A known notification/app operation needs no routine diagnostic preflight.
For ordinary device commands, selection precedence is `--target`, `AWTRIX_URL`,
`--profile`/`AWTRIX_PROFILE`, project profile (project operations), default profile.
An environment URL can therefore override a named profile. Basic authentication
uses `--username`/`--password`, then `AWTRIX_USERNAME`/`AWTRIX_PASSWORD`, then
credentials of the selected profile only. URL overrides do not inherit profile
credentials. `profile update` fully replaces the entry and clears omitted
credentials; `profile add` also replaces an existing entry. Supply credentials
again when retaining them, and keep them out of project manifests and reports.
`--timeout` is a per-request bound in milliseconds, not an observation duration.

## 4. Perform the requested operation

### Read the current app or display

For “which app is currently displayed?”, read the structured device state:

```sh
awtrix-cli --json apps active-get
```

Use the resolved target from section 3. Answer with `current_app`, preserving
its exact name, then stop when that answers the question.
If the value is null or the request fails, report that the current app could not
be determined. `apps list` reports inventory, not which app is displayed;
enabled, present, and in-loop flags do not establish current visibility.

`current_app` reads device `currentApp`; a notification can cover the display
without changing it. Use `screen capture` for a request about rendered content,
a screenshot, or a visual discrepancy. A capture establishes pixels at capture
time, not an app's identity; state and capture are separate observations and
rotation can occur between them.

When deploying a script whose new rendering needs visual inspection, consider
`script deploy --capture-dir DIR` for one or more new marker-triggered PNGs; see the
bounded workflow and timing limitations in [Berry workflows](references/workflows.md#standalone-scripts-and-protected-updates).

Choose the capture format for its consumer. PNG is the default: save it to a
task-owned `.png` path when the agent will inspect the image. When the user asks
the agent to show or return a screenshot, treat that as a request to render it
in the conversation (the user is interacting through a harness): use
`screen capture --format emoji` by default, unless the user explicitly requests
another format. Emoji is an approximate text preview, not a pixel-exact image.
Use ANSI only when requested or when a true-color terminal rendering is wanted.

`apps select` acknowledges selection without guaranteeing visibility or
dismissing notifications. `apps order-get` reports inventory slot projections,
not the full persisted rotation document. Before `apps order-set`, determine
the intended complete disabled-name set: omitted `--disabled` defaults to `[]`
and can re-enable apps; omitted `--order` preserves the existing order.
For a temporary rotation test requiring restoration, proceed only when the
original order can be reliably restored; inventory slot projections are
insufficient. Otherwise, leave the order unchanged and report the limitation.

### Send a notification or maintain a pushed app

For an authorized notification or named temporary status app:

```sh
awtrix-cli --target http://awtrix.local --json notify send \
  --payload '{"text":"Build passed"}'
awtrix-cli --target http://awtrix.local --json apps create build \
  --payload '{"text":"Building...","lifetimeMs":60000}'
awtrix-cli --target http://awtrix.local --json apps update build \
  --payload '{"text":"Tests: OK","lifetimeMs":60000}'
```

`apps create` and `apps update` both create or replace the named pushed app;
use a task-owned name. Pushed apps can expire and disappear on reboot.
Prefer updating a named status app over repeatedly queueing notifications.
For complex payloads, use `--file PATH` instead of `--payload JSON`.

For settings, read relevant current values before a patch; apply the discovery
rules above only for syntax or semantics still missing.

## 5. Interpret evidence and failures

- Use `--json` for machine output. `--fields a,b` selects known top-level result
  fields; choose them from the specific operation's documented output or a complete
  result, preserving needed evidence. Generic family descriptions and nested keys
  are not field catalogs; inspect server-defined responses before projecting.
  On `describe`, projection selects the description's own top-level fields.
  `logs follow` emits JSONL and does not accept `--fields`.
- Field selection is evaluated after execution; projection errors do not undo
  device writes, saved files, profile changes, or executable updates. Inspect the
  outcome rather than retrying a write solely because projection failed. Error
  envelopes are not projected.
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

For a read-only question, return the requested fact concisely; include target or
evidence details only when needed to disambiguate the answer. For a mutation,
report the selected target, what changed, relevant observed evidence/artifact
paths, and any error or uncertain outcome.
