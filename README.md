# awtrix-cli

**Bring your AWTRIX display into your terminal — and your AI agent's workflow.**

`awtrix-cli` is a command-line toolkit for [AWTRIX NG](https://github.com/Blueforcer/awtrix-ng).
Send notifications, display live status, manage apps, and develop persistent Berry
scripts from the same tool. Go from a local project to deployment, runtime logs,
and a PNG screen capture without assembling your own HTTP client.

The executable is `awtrix-cli`. It is built in Rust, with readable output for people
and structured JSON for scripts and AI agents that can run shell commands.

```sh
# Tell your display that a build has finished
awtrix-cli --target http://awtrix.local notify send --payload '{"text":"Build passed"}'

# Add a temporary status app to the rotation
awtrix-cli --target http://awtrix.local apps create build \
  --payload '{"text":"Tests: OK","lifetimeMs":60000}'

# Save the current framebuffer as a PNG
awtrix-cli --target http://awtrix.local screen capture --output screen.png
```

[Quick start](#quick-start) · [Features](#features) · [Examples](#everyday-examples) ·
[AI agents](#built-for-ai-agent-workflows) · [Script projects](#develop-deploy-and-test-script-projects) ·
[Documentation](#documentation)

## Why awtrix-cli?

- **Make your desk display useful.** Turn build results, job progress, or a service
  status into notifications and rotating apps from shell scripts or CI jobs that
  can reach your device.
- **Keep the development loop in one place.** Deploy Berry source, inspect its
  configuration and stored data, follow logs, observe runtime errors, and capture
  the rendered pixels.
- **Give agents a discoverable interface.** `describe` exposes command parameters,
  inputs, outputs, and examples; `--json` produces machine-readable results and
  errors; `--fields` keeps responses focused.
- **Manage projects, not a pile of HTTP calls.** Declare scripts, modules,
  resources, configuration, and tests in `awtrix.toml`. Validate locally, deploy
  additively, and explicitly prune obsolete project-owned entries.
- **Test without occupying your display.** On Linux, run declarative project tests
  against a caller-provided AWTRIX headless executable in a disposable instance.

## Quick start

### Install from this checkout

With a stable [Rust toolchain](https://rustup.rs/) installed:

```sh
cargo install --path . --locked
awtrix-cli --help
```

To build without installing, use `cargo build --release --locked` and run
`./target/release/awtrix-cli` instead. See [distribution](docs/distribution.md) for
standalone artifact names and the Linux, macOS, and Windows build matrix.

### Connect your display

Use an AWTRIX NG HTTP(S) endpoint reachable from your machine. Replace the example
URL with your device's address.

```sh
# Save a personal device profile
awtrix-cli profile add desk --target http://awtrix.local

# Inspect the device and its advertised capabilities
awtrix-cli --profile desk device diagnose

# Send your first notification
awtrix-cli --profile desk notify send --payload '{"text":"Hello from the terminal"}'
```

Use `--profile desk` to select this device, or make it the default with
`awtrix-cli profile set-default desk`. For one-off commands, use `--target URL`;
`AWTRIX_URL` is also supported.

If HTTP Basic authentication is enabled, supply `AWTRIX_USERNAME` and
`AWTRIX_PASSWORD`, or credentials in your personal profile. Project manifests
reference profiles by name and do not store credentials. Examples use POSIX-shell
quoting; adapt quotes and environment-variable syntax for your shell.

## Features

| Area | What you can do | Commands |
| --- | --- | --- |
| Device inspection | Read identity, firmware version, state, capabilities, and diagnostics | `device identity`, `state`, `capabilities`, `diagnose` |
| Device profiles | Save multiple targets and choose a default; keep credentials in personal configuration | `profile add`, `update`, `list`, `show`, `set-default`, `delete` |
| Temporary apps | Create or replace JSON-driven content, inspect the app inventory, select an app, and set rotation order | `apps create`, `update`, `delete`, `list`, `select`, `active-get`, `order-get`, `order-set` |
| Notifications | Send, queue, hold, wake the display, and dismiss active or named notifications | `notify send`, `delete-active`, `delete` |
| Display settings | Read settings, adjust brightness and power, configure supported overlays, and request reboot | `settings get`, `patch`, `brightness`, `display-get`, `display-patch`, `power`, `system-get`, `reboot` |
| Persistent Berry scripts | Read and deploy source, enable or disable scripts, inspect runtime state, edit declared configuration, and read stored data | `script get`, `deploy`, `enable`, `disable`, `delete`, `state`, `config-get`, `config-put`, `data` |
| Runtime feedback | Observe script health over a bounded window; read or follow incremental logs | `script verify`, `logs read`, `logs follow` |
| Screen capture | Export the device framebuffer to a PNG using its reported dimensions | `screen capture` |
| Modules and resources | Deploy reusable Berry modules and upload icon/resource files | `resources modules` (`list`, `get`, `deploy`, `delete`), `resources files` (`list`, `upload`, `delete`) |
| TOML projects | Scaffold, validate, deploy, preview pruning, and reconcile uncertain tracking state | `project init`, `validate`, `deploy`, `prune`, `reconcile` |
| Declarative tests | Assert script activity, stored values, log messages, and rendered pixels against PNG references with explicit tolerances | `test project` |
| Linux headless lifecycle | Start, inspect, and stop a locally owned AWTRIX Linux instance | `headless start`, `status`, `stop` |
| Agent-friendly interface | Discover operations, select output fields, consume JSON results and JSONL log streams | `describe`, `--json`, `--fields` |

## Everyday examples

The examples below use the `desk` profile created in the quick start.

### Display progress, then announce completion

```sh
awtrix-cli --profile desk apps create build --payload '{"text":"Building..."}'
awtrix-cli --profile desk apps update build --payload '{"text":"Tests: OK"}'
awtrix-cli --profile desk notify send --payload '{"text":"Ready to ship"}' --stack --wakeup
awtrix-cli --profile desk apps delete build
```

Pushed apps are temporary: they can expire and are lost on reboot. Both `create`
and `update` create or replace the named app. For logic that lives on the device,
use a persistent Berry script.

### Adjust the display

```sh
awtrix-cli --profile desk settings brightness 80 --auto false
awtrix-cli --profile desk settings power off
awtrix-cli --profile desk settings power on
```

### Deploy a new script and inspect it

```sh
# Create a local project with a starter Berry script
awtrix-cli project init ./demo

# Create the script only if its name is absent, then observe it for 10 seconds
awtrix-cli --profile desk script deploy main --file ./demo/src/main.be \
  --create --verify-secs 10

# Inspect stored data and collect runtime feedback
awtrix-cli --profile desk --json script data main
awtrix-cli --profile desk logs follow --duration-secs 15
awtrix-cli --profile desk script verify main --duration-secs 10 --capture main.png
```

Protected deployment requires the device's `scriptUpdates` capability. For an
existing script, supply the original source with `--expected-source`, or use the
project manifest's `expected_source_file`. A stale reference produces a conflict
instead of overwriting a concurrent edit. `--force` explicitly opts into an
unprotected overwrite.

## Built for AI agent workflows

An agent with shell access can use the same executable as a person. Discover an
operation, inspect the target, perform it, and collect runtime evidence:

```sh
# Discover an operation without contacting a device
awtrix-cli --json describe "script verify"

# Inspect live capabilities and a focused app inventory
awtrix-cli --profile desk --json device capabilities
awtrix-cli --profile desk --json --fields apps apps list

# Observe a script and return a structured report plus a PNG
awtrix-cli --profile desk --json script verify main \
  --duration-secs 10 --capture main.png

# Stream bounded runtime logs as JSONL
awtrix-cli --profile desk --json logs follow --duration-secs 15
```

- **Discoverable commands:** `awtrix-cli --help` lists command families;
  `awtrix-cli describe "<topic>"` describes an operation offline. Pass an explicit
  `--target` to refine the description with the connected device's capabilities.
- **Predictable output:** `--json` requests compact JSON independently of terminal
  detection. `logs follow --json` streams typed JSONL records. `--fields a,b`
  selects known top-level result fields for non-streaming output.
- **Actionable failures:** machine-readable error codes and nonzero exit codes
  distinguish argument, authentication, timeout, HTTP, and compatibility errors.
- **Bounded observation:** log following and script verification have explicit
  time windows, so an agent can collect feedback and continue its workflow.
- **Evidence-aware results:** reports distinguish HTTP acceptance, saved source,
  observed runtime state, and unavailable evidence.

For example, ask your shell-capable coding agent:

> Use `awtrix` with the `desk` profile. Inspect its capabilities and the description
> of script verification, verify `main` for 10 seconds, save a screen capture, and
> summarize any reported Berry errors.

See the [CLI contract](docs/cli-contract.md) for exact output, error, target
selection, and exit-code semantics.

## Develop, deploy, and test script projects

Keep your display code and its dependencies together in a versionable project:

```sh
awtrix-cli project init ./demo
awtrix-cli project validate --manifest ./demo/awtrix.toml
awtrix-cli --profile desk --json project deploy --manifest ./demo/awtrix.toml
```

An `awtrix.toml` manifest declares scripts, Berry modules, resources, configuration
patches, and optional tests. Paths resolve relative to the manifest. Local
validation checks the manifest and dependencies before deployment contacts a
device. See the [complete example project](examples/project/awtrix.toml), including
a module, script, GIF resource, and test assertions.

Deployments are additive and run modules, resources, scripts, then configuration.
They do not delete undeclared content or provide a transaction across operations.
The generated starter uses create-only deployment; to update an existing script,
replace `create = true` with an `expected_source_file` reference to its original
remote source.

Preview obsolete, previously tracked project entries before pruning them:

```sh
awtrix-cli --profile desk project prune --manifest ./demo/awtrix.toml --dry-run
```

### Test in an isolated Linux instance

Provide the AWTRIX Linux executable yourself; it is separate from this CLI and is
not bundled or downloaded. For a project with test assertions:

```sh
awtrix-cli --json test project --manifest examples/project/awtrix.toml \
  --binary /path/to/awtrix-linux --webui /path/to/webui/index.html
```

The runner starts an instance on an available loopback port, deploys the project,
checks its assertions, then stops the instance and removes temporary data and
tracking files. Tests can check that a script remains active, that a stored value
matches, or that a log message appears during the observation window. Visual
assertions compare the framebuffer with a reference PNG using explicit color and
pixel-count tolerances; captures are not synchronized to an exact animation frame.

To deploy and run those tests on an existing device, select its URL explicitly:

```sh
awtrix-cli --target http://awtrix.local --json test project \
  --manifest examples/project/awtrix.toml
```

External test runs require `--target`; profiles and environment target defaults
do not select the test device. Use a project whose create/update declarations
match that device's existing scripts.

## Compatibility and validation status

The CLI targets the **AWTRIX NG HTTP API**, with contracts checked against the
ESP32, ESP32-S3, and TC002 definitions. Available features depend on the connected
device's advertised capabilities. The client-host distribution matrix is Linux
x86_64 (GNU/glibc), macOS arm64, and Windows x86_64 (MSVC); headless lifecycle and
isolated headless testing are Linux-only.

Core implementation tickets 01–17 are complete in this checkout. Native
macOS/Windows distribution acceptance (ticket 18) and the physical-device
compatibility matrix (ticket 19) remain pending. Linux checks, HTTP fixtures, and
headless runs are not physical-device validation. Headless tests do not validate
sensors, audio, or hardware memory/instruction budgets.

HTTP acceptance alone does not prove that content is visible, and a bounded
verification window does not prove future script correctness. Device logs retain
only the latest 34 lines, so log collection is not exhaustive.

The [distribution documentation](docs/distribution.md) and
[physical validation ticket](.scratch/awtrix-cli/issues/19-validation-parcours-compatibilite.md)
record the remaining acceptance gates.

## Documentation

- [CLI contracts](docs/cli-contract.md) — command behavior, authentication,
  profiles, structured output, script protection, projects, and tests.
- [App contracts](docs/apps-contract.md) — pushed payloads, rotation, lifetime,
  request limits, and device acceptance semantics.
- [Distribution](docs/distribution.md) — build targets, standalone artifacts,
  native validation, and headless prerequisites.
- [Example project](examples/project/awtrix.toml) — scripts, module, resource,
  and declarative assertions ready to inspect and test.
- [AWTRIX NG](https://github.com/Blueforcer/awtrix-ng) — the upstream firmware
  and Linux runtime this CLI talks to.

For the real headless integration suite, supply the executable and web assets:

```sh
AWTRIX_LINUX_BIN=/path/to/awtrix-linux AWTRIX_WEBUI=/path/to/webui/index.html \
  cargo test --test headless -- --ignored --exact \
  real_awtrix_headless_declarative_runs_cover_success_failure_berry_error_and_isolation
```
