# End-to-end workflow and compatibility evidence

This document is the reproducible acceptance record for the agent workflow. The
executable is `awtrix-cli`; replace example URLs and paths with values available
to the operator. Do not run mutating steps against a shared/physical display
without the owner's explicit approval. On 2026-10-07 the approved main workflow
was run on the recorded ESP32; ESP32-S3 and TC002 physical checks remain unrun.

## Discover, exercise and clean up

Start by discovering the command contract offline and diagnosing the selected
target before making changes:

```sh
awtrix-cli --help
awtrix-cli --json describe 'project deploy'
awtrix-cli --json describe 'test project'
awtrix-cli --target "$AWTRIX_URL" --json device diagnose
awtrix-cli --target "$AWTRIX_URL" --json apps list
awtrix-cli --target "$AWTRIX_URL" --json project validate \
  --manifest examples/project/awtrix.toml
```

After the read-only ownership preflight below passes, run the mutating workflow:

```sh
awtrix-cli --target "$AWTRIX_URL" apps create agent-progress \
  --payload '{"text":"Preparing project","lifetimeMs":600000}'
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
profile/default from silently selecting a different device. Before any physical
or shared-target mutation, obtain explicit owner approval. Then perform this
read-only ownership preflight and stop if any item is not clearly safe:

1. Run `project validate` and inspect the manifest's exact project name, script
   names/files, module names/files, resource destination paths/files, and test
   assertions. Check the local resource bytes and Berry sources are the intended
   content; record hashes (`sha256sum` or the host equivalent) for later
   comparison. Confirm `agent-progress` is absent in the pushed-app inventory;
   if it exists, choose a new unique progress name rather than replace/delete it.
2. Run `resources modules list`, `apps list`, and `resources files list
   --dir /ICONS` (plus `resources files list --dir DIR` for every other
   destination directory). For each colliding script/module, run
   `resources modules get NAME` or `script get NAME`, save its exact response,
   and record its hash. For each colliding resource path, download/read it using
   an independently verified, documented device procedure before proceeding.
   The CLI has no generic resource download route; if exact backup and restore
   cannot be established, abort rather than overwrite that resource.
3. Compare the inventory with the manifest. If any name/path exists and is not
   explicitly disposable project-owned content, either obtain approval to
   replace it and verify a tested restoration path for the saved original, or
   change the project names/paths to unique temporary values and validate again.
   If neither option is safe, abort. A local backup alone is not permission to
   overwrite.

Only after that preflight may the operator run the commands above. Deployment is
additive and not transactional; project deploy tracks successful items but does
not roll them back. On a disposable target, after reviewing the plan,
`project prune --dry-run` previews and `project prune` explicitly deletes only
stale items tracked for that project and target. On a shared target do not
prune; preserve foreign/pre-existing content and manually remove only temporary
items whose ownership is known. If an approved replacement occurred, restore the
saved original and verify its source/hash and runtime state as agreed with the
owner. The progress app is deleted explicitly; notifications are transient and
may be dismissed with `notify delete-active` only when safe to dismiss the
device's current notification.

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
the stable `error.code` contract. Never parse human diagnostics as JSON. Exact
external-CLI fixture evidence (mock HTTP server; **not** AWTRIX firmware) is:

```sh
cargo test --test cli script_get_preserves_raw_source_and_script_put_reports_berry_error
cargo test --test cli successful_conditional_response_with_setup_error_is_operational_failure
cargo test --test cli script_deploy_surfaces_conflict_without_fallback_or_overwrite
cargo test --test cli script_deploy_uses_atomic_expected_source_route_and_does_not_pre_read
```

The real Linux AWTRIX 1.2.2 headless test includes Berry-error deployment and
isolated-run cleanup, but does **not** exercise concurrent source edits (that
case is covered by the mock HTTP CLI fixture above):

```sh
AWTRIX_LINUX_BIN=/path/to/awtrix-linux AWTRIX_WEBUI=/path/to/webui/index.html \
  cargo test --test headless -- --ignored --exact \
  real_awtrix_headless_declarative_runs_cover_success_failure_berry_error_and_isolation --nocapture
```

Together these commands separate simulated correction/conflict behavior from
real headless Berry/runtime behavior; neither is physical-device evidence.

## Evidence classes and compatibility

| Evidence class | Observed here | What it establishes / does not establish |
| --- | --- | --- |
| HTTP simulation | CLI integration fixtures cover ESP32, ESP32-S3 and TC002 response/capability contracts, Berry errors, conflicts and failures. | Deterministic client contract only; not firmware execution. |
| Actual AWTRIX headless | Upstream 1.2.2 Linux binary; isolated declarative project test and its success/failure/Berry-error/isolation/cleanup scenarios passed. | Linux runtime behavior; not ESP32 memory/instruction budgets, audio, sensors, or physical rendering. |
| Physical ESP32 | On 2026-10-07, AWTRIX NG 1.2.2, boardType `awtrixng`, soc `esp32`, 32x8, `scriptUpdates:true`. Full workflow ran against `http://192.168.1.202` using disposable project `ticket20-physical-20261007`: progress pushed app accepted (visibility unknown); module `ticket20_helpers`, `/ICONS/ticket20-validation.gif`, and script `ticket20-check` deployed; bounded 5-second verification reported `start_verified:true`, active/in-loop, `runtime_error:null`, complete window, logs and 32x8 capture; final 5-second notification accepted (visibility unknown). After cleanup, the test app/script/module/resource were absent; original Update-Checker, Anothertime, Tesla, and pushed app `hello-world` remained. Brightness stayed 4 and matrix power stayed on; no setting command or settings mutation was used. | Full script/project/deploy/verify/log/capture/notification/cleanup path observed on this ESP32 and firmware. Not a minimum-version guarantee or a hardware resource-budget certification. Test-generated diagnostic log lines remain in the device's bounded log history; they were not cleared to avoid erasing user logs. Captures: `/tmp/opencode/ticket20-esp32-during.png` and `/tmp/opencode/ticket20-esp32-after.png` (32x8). |
| Physical ESP32-S3 | NOT EXECUTED; no target available. | No compatibility claim. |
| Physical TC002 | NOT EXECUTED; no target available. | No compatibility claim. |
| Host distribution | Native CI run 37663709425 passed host and package jobs for Linux x86_64 GNU, macOS arm64, and Windows x86_64 MSVC after the `awtrix-cli` rename. | CLI artifact/help/version/HTTP checks per ticket 18; not full CLI or physical firmware coverage on those hosts. |

Minimum verified firmware is **1.2.2 for the actual Linux headless runtime**.
The physical ESP32 workflow was tested on firmware 1.2.2; this single observation
does not establish a minimum supported physical firmware version. ESP32-S3 and
TC002 physical checks remain unexecuted. Official OpenAPI contracts (checked 2026-10-07) include
the script conditional-update, resource, logs, screen and core device routes
for all three variants, but contract presence is not execution evidence.
Capabilities must be read from the target: e.g. `scriptUpdates` is required
for protected deployment; unsupported/missing capabilities fail explicitly.
No variant-level claim beyond these contract definitions is inferred.

During the physical run, the first module upload was rejected with Berry error
`module must end with 'return <value>'`. The CLI reported the module as uncertain;
read-only inspection showed the exact temporary module and its error. It was
deleted after confirming the name was absent before this run, local uncertain
tracking was reconciled, and a final `return true` was added before retrying. The
corrected module, resource, and script then deployed successfully. The failed
temporary module was not left installed.

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
