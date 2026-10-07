# End-to-end workflow and compatibility evidence

This document is the reproducible acceptance record for the agent workflow. The
executable is `awtrix-cli`; replace example URLs and paths with values available
to the operator. Do not run mutating steps against a shared/physical display
without the owner's explicit approval. The physical checks below were not run.

## Discover, exercise and clean up

Start by discovering the command contract offline and diagnosing the selected
target before making changes:

```sh
awtrix-cli --help
awtrix-cli --json describe 'project deploy'
awtrix-cli --json describe 'test project'
awtrix-cli --target "$AWTRIX_URL" --json device diagnose
awtrix-cli --target "$AWTRIX_URL" --json apps list
awtrix-cli --target "$AWTRIX_URL" apps create agent-progress \
  --payload '{"text":"Preparing project"}'
awtrix-cli --target "$AWTRIX_URL" --json project validate \
  --manifest examples/project/awtrix.toml
awtrix-cli --target "$AWTRIX_URL" --json project deploy \
  --manifest examples/project/awtrix.toml
awtrix-cli --target "$AWTRIX_URL" --json script verify main \
  --duration-secs 5 --capture /tmp/awtrix-main.png
awtrix-cli --target "$AWTRIX_URL" --json logs read
awtrix-cli --target "$AWTRIX_URL" notify send \
  --payload '{"text":"Project checks complete"}'
awtrix-cli --target "$AWTRIX_URL" apps delete agent-progress
```

`AWTRIX_URL` is required for this example; the explicit `--target` prevents a
profile/default from silently selecting a different device. `examples/project`
contains a script, `@module` source, GIF resource, and active/log assertions.
Deployment is additive and not transactional. Project deploy tracks successful
items but does not roll them back. On a disposable target, after reviewing the
plan, `project prune --dry-run` previews and `project prune` explicitly deletes
only stale items tracked for that project and target. On a shared target do not
prune: preserve foreign or pre-existing scripts, and manually remove only
temporary items whose ownership is known. The progress app is deleted explicitly;
notifications are transient and may be dismissed with `notify delete-active`
only when it is safe to dismiss the device's current notification.

For an isolated Linux headless run (recommended for repeatable acceptance),
provide the upstream AWTRIX executable and its UI asset:

```sh
awtrix-cli --json test project --manifest examples/project/awtrix.toml \
  --binary /path/to/awtrix-linux --webui /path/to/webui/index.html
```

This owns a fresh child, data directory and test tracking state, then cleans
them on success or failure. No physical target is selected or modified. In this
checkout the command was verified with AWTRIX NG source
`a02f3ab66cd88cbf5c5f08f3dcc47eb231ae0188` (upstream version 1.2.2), using
`/tmp/opencode/awtrix-ng-build/awtrix-linux` and
`/tmp/opencode/awtrix-ng-src/webui/index.html`.

## Correction loop and machine reports

For an HTTP-200 Berry compile/setup error, treat `BERRY_ERROR` as failure, not
successful execution. Inspect `error`/verification details and source; correct
the local file, then redeploy against the original reference. For
`CONFLICT` (stale `expected_source`), first fetch/compare the remote source and
incorporate the concurrent edit, then retry with the newly reviewed original
source reference. Do not automatically retry with `--force`; it deliberately
removes the conditional-write protection. Keep machine results on stdout and
use `jq`, for example:

```sh
awtrix-cli --target "$AWTRIX_URL" --json script deploy main \
  --file examples/project/src/main.be --expected-source "$OLD_SOURCE" \
  | jq '{source_saved, start_verified, error, verification}'
awtrix-cli --target "$AWTRIX_URL" --json project deploy \
  --manifest examples/project/awtrix.toml | jq '{succeeded, failed, not_run, cause}'
```

On nonzero exit, capture stdout and stderr separately; JSON error envelopes use
the stable `error.code` contract. Never parse human diagnostics as JSON. The
fixture suite exercises HTTP success carrying Berry errors and stale-source
conflicts at the external CLI seam.

## Evidence classes and compatibility

| Evidence class | Observed here | What it establishes / does not establish |
| --- | --- | --- |
| HTTP simulation | CLI integration fixtures cover ESP32, ESP32-S3 and TC002 response/capability contracts, Berry errors, conflicts and failures. | Deterministic client contract only; not firmware execution. |
| Actual AWTRIX headless | Upstream 1.2.2 Linux binary; isolated declarative project test and its success/failure/Berry-error/isolation/cleanup scenarios passed. | Linux runtime behavior; not ESP32 memory/instruction budgets, audio, sensors, or physical rendering. |
| Physical ESP32 | Read-only diagnosis on `http://192.168.1.202`: AWTRIX NG 1.2.2, boardType `awtrixng`, soc `esp32`, 32x8, `scriptUpdates:true`. Earlier smoke: read state and submit a five-second notification (acceptance only; visibility unknown). | Identity and limited smoke only. Full workflow NOT EXECUTED. Existing Update-Checker, Anothertime and Tesla scripts were observed and must be preserved. No persistent/visible acceptance was performed for this ticket. |
| Physical ESP32-S3 | NOT EXECUTED; no target available. | No compatibility claim. |
| Physical TC002 | NOT EXECUTED; no target available. | No compatibility claim. |
| Host distribution | Native CI run 37663709425 passed host and package jobs for Linux x86_64 GNU, macOS arm64, and Windows x86_64 MSVC after the `awtrix-cli` rename. | CLI artifact/help/version/HTTP checks per ticket 18; not full CLI or physical firmware coverage on those hosts. |

Minimum verified firmware is **1.2.2 for the actual Linux headless runtime**.
The only known physical firmware is ESP32 1.2.2, with read-only identity and
limited smoke evidence above. No minimum firmware version for any physical
variant is established. Official OpenAPI contracts (checked 2026-10-07) include
the script conditional-update, resource, logs, screen and core device routes
for all three variants, but contract presence is not execution evidence.
Capabilities must be read from the target: e.g. `scriptUpdates` is required
for protected deployment; unsupported/missing capabilities fail explicitly.
No variant-level claim beyond these contract definitions is inferred.

### Physical resumption protocol

For each newly available ESP32, ESP32-S3 or TC002: first run read-only
`device diagnose`, `device capabilities`, `apps list`, and script state; record
date, exact firmware, reported boardType/soc/dimensions and capability JSON.
Before any visible or persistent action, obtain the device owner's explicit
agreement and schedule a maintenance window. Back up/read the specific target
script source and confirm temporary names/resources do not overlap installed
content. Run the workflow in the first section, including a bounded test and
explicit removal only of the uniquely named temporary app/project content;
verify pre-existing scripts remain unchanged afterwards. Record command
outputs, exit status, physical variant and cleanup result here. If approval,
target, backup, or safe cleanup is unavailable, leave that variant marked NOT
EXECUTED. Never treat a mock or headless run as physical evidence.
