# Berry, projects, and runtime evidence

Commands below use an example URL. Select the intended target explicitly,
check capabilities, and consult each operation's `--help` before adapting it.

## Protected script deployment

Scaffold a new local project in an unused directory, then validate offline:

```sh
awtrix-cli project init ./demo
awtrix-cli --json project validate --manifest ./demo/awtrix.toml
awtrix-cli --target http://awtrix.local --json script deploy main \
  --file ./demo/src/main.be --create --verify-secs 10
```

`--create` fails if the script already exists. For an update, read the remote
source **before editing** with `script get NAME`, retain its exact bytes, and
pass that original text as `--expected-source` when deploying the modified file.
Use `script deploy --help` for syntax. Human-mode `script get` writes exact
source to stdout; JSON mode returns a `source` string. POSIX command substitution
strips trailing newlines, so `--expected-source "$(cat original.be)"` is not a
reliable exact-byte reference. Prefer project `expected_source_file` for file-based
updates instead of manufacturing a shell-string reference.

Protected deploy requires live `scriptUpdates: true`. `CONFLICT` means the
reference is stale; inspect and resolve the concurrent edit. `--force` is an
explicit unprotected overwrite, not a retry or compatibility fallback.

For feedback on an existing script:

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
exit status. Specify finite observation windows. `script config-get` discovers
declared settings; `script config-put --values JSON` patches them and restarts
init/setup. Inspect `--help` and `describe` before changing configuration.

## Project deployment and cleanup

```sh
awtrix-cli --json project validate --manifest ./demo/awtrix.toml
awtrix-cli --target http://awtrix.local --json project deploy \
  --manifest ./demo/awtrix.toml
```

A project manifest declares scripts, modules, resource files, configuration,
and optional tests. Local paths are relative to the manifest. Scripts declare
either `create = true` or `expected_source_file = "original/main.be"` containing
the exact original remote source (a project-relative file captured before edits).
For an update replace the create-only declaration with that reference; do not
refresh the reference immediately before deployment to bypass a conflict.

Deploy runs modules, resources, scripts, then configuration. It is additive,
stops at the first failure, and is not transactional. A partial failure can
leave successful mutations on the device: inspect the deployment report before
retrying. Modules use raw replacement without script-update conflict protection;
inspect existing modules before replacing them. Discover individual operations
with `resources modules --help` and `resources files --help`. `/ICONS` uploads
require lowercase `.gif`/`.jpg` names and matching file signatures. Generic file
download is unsupported; use supported commands rather than guessing endpoints.

Only when cleanup is requested, preview tracked obsolete entries:

```sh
awtrix-cli --target http://awtrix.local --json project prune \
  --manifest ./demo/awtrix.toml --dry-run
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
awtrix-cli --json test project --manifest ./demo/awtrix.toml \
  --binary /path/to/awtrix-linux --webui /path/to/webui/index.html
```

The scaffold has no assertions: add them using `describe "test project"` and
the [example manifest](https://github.com/toinux/awtrix-cli/blob/HEAD/examples/project/awtrix.toml).
The runner starts a disposable instance, deploys, observes assertions, then
cleans up its process, data, and tracking. `AWTRIX_LINUX_BIN` may supply the binary.
Use `headless start --help`, `headless status`, and `headless stop` when a
longer-lived CLI-owned Linux instance is specifically needed. An explicit data
directory is retained on stop; temporary data is removed.

Testing on an existing device deploys content and requires an explicit URL:

```sh
awtrix-cli --target http://awtrix.local --json test project \
  --manifest ./demo/awtrix.toml
```

Profiles and `AWTRIX_URL` do not select an external test target. Explicit
`--target` and `--binary` conflict; choose one. Match create/update declarations
to existing device content. A failed assertion exits nonzero; report the failed
assertions and incomplete/unavailable observations. Headless runs do not validate
physical sensors, audio, or hardware resource budgets.

For contract details beyond the operation's help/description, consult the current
[CLI contract](https://github.com/toinux/awtrix-cli/blob/HEAD/docs/cli-contract.md).
