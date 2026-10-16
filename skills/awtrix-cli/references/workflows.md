# Berry, projects, and runtime evidence

Use only the section needed, following the discovery rules in [the skill](../SKILL.md).
Examples can be executed directly; replace the example URL with the intended
target. The inspection commands below are alternatives for different questions,
not a checklist to execute in sequence.

## Standalone scripts and protected updates

Standalone scripts do not require a project, even when persistent. For a quick
experiment with an existing source file:

```sh
awtrix-cli --target http://awtrix.local --json script deploy main \
  --file ./main.ax --create --verify-secs 10
```

AWTRIX scripts use the `.ax` filename extension. Berry modules remain `.be`
files and are declared separately under `[[modules]]`.

`--create` fails if the script already exists. For an update, read the remote
source **before editing** with `script get NAME`, retain its exact bytes, and
pass that original text as `--expected-source` when deploying the modified file.
Human-mode `script get` writes exact source to stdout; JSON mode returns a
`source` string. Shell command substitution strips trailing newlines and is not
a reliable exact-byte reference. Prefer project `expected_source_file` for file-based
updates instead of manufacturing a shell-string reference.

Protected deploy requires live `scriptUpdates: true`. `CONFLICT` means the
reference is stale; inspect and resolve the concurrent edit. `--force` is an
explicit unprotected overwrite, not a retry or compatibility fallback.

When visual evidence is needed from a newly deployed script, use the opt-in
single-capture flow:

```sh
awtrix-cli --target http://awtrix.local --json script deploy main \
  --file ./main.ax --create --capture-dir ./captures
```

The script should draw normally, allow its first render to settle, then emit
one `log('[CAPTURE] ready')` from its post-paint callback. Return normally from
the callback and keep state nonblocking while the device capture is acquired.
Use `--capture-count N` to request multiple states (default 1); repeated labels get numbered distinct files, and the ordered report lists exact paths. A count names no missing marker. A batch is processed sequentially and cannot reconstruct past frames; markers do not guarantee synchronization. 500 ms of script-side stabilization is only an empirical starting point, not a CLI delay or guarantee. Review the attempt's `capture_collection` report and PNGs; the CLI does not
promise exact log/frame synchronization, visual identity or runtime correctness.
On ESP32 1.2.2 immediate captures were initially black; waiting 500 ms before
the marker succeeded 2/2 in the targeted experiment, not as a general guarantee.
Logs, canvas and framebuffer are unsynchronized; rotation, notifications and
latency remain. No direct framebuffer binding was found in examined Berry APIs.
Reserve logs for short markers, not image data. There is no implicit CLI delay.

Choose evidence for the unresolved question: `script state` gives the script
inventory and runtime state; `script data NAME` reads persisted data; `script
verify NAME` collects bounded runtime/log evidence and optional capture; `logs
follow` observes logs. Reuse deploy verification if it already answers the need.

Enabled/in-loop/present state and `start_verified` concern runtime health, not
which app is currently displayed. Use `apps active-get` for current app identity
and `screen capture` for rendered pixels, as explained in the skill.

```sh
awtrix-cli --target http://awtrix.local --json script state
awtrix-cli --target http://awtrix.local --json script data main
awtrix-cli --target http://awtrix.local --json script verify main \
  --duration-secs 10 --capture main.png
awtrix-cli --target http://awtrix.local --json logs follow --duration-secs 15
```

Read `start_verified`, `observed_window`, `runtime_error`, `not_available`, and
capture metadata rather than equating saved source with running code. Follow
produces typed JSONL log/error/end records; retain the final resume cursor and
exit status. Specify finite observation windows. Read declared settings with
`script config-get NAME` before a patch. `script config-put NAME --values JSON`
patches them and restarts init/setup; discover syntax/schema only if still unknown.

## Manual store preparation and shared publications

With CLI 0.5.0 or newer and firmware supporting these endpoints, use a store
patch for authorized manual test/debug preparation without redeploying source:

```sh
awtrix-cli --target http://awtrix.local --json script data-patch main \
  --values '{"count":9,"remove":null}'
awtrix-cli --target http://awtrix.local --json script data main
awtrix-cli --target http://awtrix.local --json --fields publications script shared
```

The patch accepts a JSON object, including `{}`: omitted keys are retained and
`null` deletes a key. Firmware rejects declared `@config` keys; use `config-put`
for those. Saving restarts the script and its initialization may immediately
change injected values. `BERRY_ERROR` means data was saved but restart failed;
retain `error.device_result`, with no rollback or automatic retry. A lost response
leaves the write outcome unknown; inspect before deciding whether to retry.

`script shared` is a read-only snapshot, not another script's private store.
Publications are temporary and may disappear on device restart or owner
removal/re-registration. Preserve their values and reported `ageMs`; an empty
list is valid. These manual commands do not add preparation hooks or shared
assertions to `test project`, and neither proves runtime health.

## Project scaffolding and deployment

Check briefly for a relevant existing `awtrix.toml` before initializing. Reuse it
when appropriate. For a new project, use the generated scaffold as the minimal
script/manifest structure; edit its `.ax` source and manifest rather than fetching
an unrelated device script as an example:

```sh
awtrix-cli project init ./hello-world
# Edit ./hello-world/src/main.ax and ./hello-world/awtrix.toml as needed.
awtrix-cli --json project validate --manifest ./hello-world/awtrix.toml
awtrix-cli --target http://awtrix.local --json project deploy \
  --manifest ./hello-world/awtrix.toml
```

A project manifest declares scripts, modules, resource files, configuration,
and optional tests. Local paths are relative to the manifest. Scripts declare
either `create = true` or `expected_source_file = "original/main.ax"` containing
the exact original remote source (a project-relative file captured before edits).
For an update replace the create-only declaration with that reference; do not
refresh the reference immediately before deployment to bypass a conflict.
For manifest fields not present in the scaffold and genuinely needed, use
`awtrix-cli --json describe "project deploy" --offline`. Add runtime verification
only when the request needs evidence beyond the deployment report.

Deploy runs modules, resources, scripts, then configuration. It is additive,
stops at the first failure, and is not transactional. A partial failure can
leave successful mutations on the device: inspect the deployment report before
retrying. Modules use raw replacement without script-update conflict protection;
inspect existing modules before replacing them. For an unknown resource operation,
inspect the relevant `resources modules` or `resources files` family help. `/ICONS` uploads
require lowercase `.gif`/`.jpg` names and matching file signatures. Generic file
download is unsupported; use supported commands rather than guessing endpoints.

## Project cleanup

Only when cleanup is requested, preview tracked obsolete entries:

```sh
awtrix-cli --target http://awtrix.local --json project prune \
  --manifest ./hello-world/awtrix.toml --dry-run
```

Review the planned names before running the same command without `--dry-run`.
Prune depends on valid per-project/per-target `.awtrix-tracking-*.json` files
beside the manifest; retain them. Uncertain tracking blocks deletion.
`project reconcile --forget-uncertain` releases local ownership without resolving
the remote outcome; inspect those entries before choosing that recovery path.

## Declarative tests and Linux headless

For a manifest with test assertions, prefer an isolated Linux run when the
caller provides the separate AWTRIX executable and any required web assets:

```sh
awtrix-cli --json test project --manifest ./hello-world/awtrix.toml \
  --binary /path/to/awtrix-linux --webui /path/to/webui/index.html
```

The scaffold has no assertions. If the assertion schema is unknown, consult
`awtrix-cli --json describe "test project" --offline`; consult the
[example manifest](https://github.com/toinux/awtrix-cli/blob/HEAD/examples/project/awtrix.toml)
only for details still missing.
The runner starts a disposable instance, deploys, observes assertions, then
cleans up its process, data, and tracking. `AWTRIX_LINUX_BIN` may supply the binary.
Use the `headless` family only when a longer-lived CLI-owned Linux instance is
specifically needed: `headless start`, `headless status`, `headless stop`.
Discover start options only if needed and unknown. An explicit data
directory is retained on stop; temporary data is removed.

Testing on an existing device deploys content and requires an explicit URL:

```sh
awtrix-cli --target http://awtrix.local --json test project \
  --manifest ./hello-world/awtrix.toml
```

Profiles and `AWTRIX_URL` do not select an external test target. Explicit
`--target` and `--binary` conflict; choose one. Match create/update declarations
to existing device content. A failed assertion exits nonzero; report the failed
assertions and incomplete/unavailable observations. Headless runs do not validate
physical sensors, audio, or hardware resource budgets.

For contract details beyond the operation's help/description, consult the current
[CLI contract](https://github.com/toinux/awtrix-cli/blob/HEAD/docs/cli-contract.md).
