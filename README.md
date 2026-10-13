# awtrix-cli

**Give your AI agent a practical way to work with your AWTRIX NG display.**

`awtrix-cli` is a command-line toolkit for [AWTRIX NG](https://github.com/Blueforcer/awtrix-ng). It lets an agent or shell workflow send notifications, manage temporary apps, deploy persistent Berry scripts, inspect runtime behavior, and capture the display—all through one command-line interface.

## Install the skill

Optionally install the `awtrix-cli` skill for installation guidance and efficient agent workflows. The executable also supports autonomous discovery through `--help` and `describe`:

```sh
npx skills add toinux/awtrix-cli --skill awtrix-cli
```

Then ask your agent to use `awtrix-cli`, for example:

> Set up awtrix-cli, connect to my display at http://awtrix.local, and show its capabilities.

The skill guides the agent through checking for the executable and installing it if needed. It also teaches the agent how to select a device, use structured output, and verify changes. You do not need to run the skill's bundled installer scripts yourself.

## Get started

Ask your agent to connect to your AWTRIX NG device and configure a profile. A profile saves the device address so you do not need to repeat it for each command:

```sh
awtrix-cli profile add desk --target http://awtrix.local
awtrix-cli profile set-default desk
awtrix-cli device diagnose
```

Once a default profile is set, commands use it automatically. For a one-off command without a configured default, pass a target explicitly:

```sh
awtrix-cli --target http://awtrix.local notify send --payload '{"text":"Build passed"}'
```

You can also ask your agent to do something directly:

> Show “Tests passed” on my display, then remove the temporary status app.

## Common workflows

### Show progress and announce completion

```sh
awtrix-cli apps create build --payload '{"text":"Building..."}'
awtrix-cli apps update build --payload '{"text":"Tests: OK"}'
awtrix-cli notify send --payload '{"text":"Ready to ship"}' --stack --wakeup
awtrix-cli apps delete build
```

Pushed apps are temporary: they can expire and are lost on reboot. For logic that should live on the device, use a persistent Berry script.

### Deploy a Berry script with on-the-fly minification

Create a project scaffold, then deploy its readable source with `--minify`. The CLI minifies the deployment payload in memory; it does not rewrite your source file.

```sh
awtrix-cli project init ./demo
awtrix-cli script deploy main --file ./demo/src/main.ax --create --minify --verify-secs 10
```

For an existing script, use `--expected-source` with the original remote source to protect against overwriting a concurrent change. The source file remains unchanged when minification is enabled.

You can also minify a local file without deploying it. This writes a `.min.ax` sibling and leaves the original untouched:

```sh
awtrix-cli minify ./demo/src/main.ax
```

### Adjust the display

```sh
awtrix-cli settings brightness 80 --auto false
awtrix-cli settings power off
awtrix-cli settings power on
```

### Inspect runtime behavior

```sh
awtrix-cli --json script data main
awtrix-cli logs follow --duration-secs 15
awtrix-cli script verify main --duration-secs 10 --capture main.png
```

### Capture the screen

PNG is the default and is best when the agent needs to inspect the image. To
show a text rendering in the conversation, request emoji explicitly (or ask
your agent to show the screen); ANSI is also available for true-color terminals:

```sh
awtrix-cli screen capture --output capture.png
awtrix-cli screen capture --format emoji
awtrix-cli screen capture --format ansi
```

Emoji approximates each pixel with a representative color, so it is a readable
preview rather than an exact image.

## What you can do

- **Control the display:** inspect device information and capabilities, adjust settings, change brightness or power, and reboot.
- **Manage temporary content:** create, update, list, select, order, and delete apps; send, queue, or dismiss notifications.
- **Develop Berry scripts:** deploy scripts, manage their configuration and data, inspect state and logs, and verify runtime behavior.
- **Manage reusable content:** deploy Berry modules and upload icons or other resources.
- **Work with projects:** declare scripts, modules, resources, configuration, and tests in an `awtrix.toml` project; validate and deploy it, then preview obsolete project-owned entries before pruning.
- **Capture the screen:** save the display framebuffer as a PNG.
- **Automate from agents and shell scripts:** discover commands, request JSON output, select relevant fields, and consume bounded JSONL log streams.

## Configure a device

Profiles keep device addresses and optional credentials in your personal CLI configuration. Use `awtrix-cli profile show desk` to inspect a profile without revealing credentials. `profile update desk --target URL` fully replaces the entry and clears omitted credentials; `profile add` also replaces an existing entry. Supply credentials again when retaining them.

For ordinary device commands, target precedence is `--target` → `AWTRIX_URL` → `--profile`/`AWTRIX_PROFILE` → project profile (project commands only) → default profile. Thus an environment URL overrides a named profile. Basic authentication uses `--username`/`--password` → `AWTRIX_USERNAME`/`AWTRIX_PASSWORD` → credentials of the selected profile only; a URL override does not inherit profile credentials.

## Ask your agent

The installed skill teaches your agent the CLI workflows. You can ask it to perform tasks in natural language, for example:

> Deploy `./demo` to my default display, minify the script during deployment, and verify it for 10 seconds.

### Discover the CLI without a skill

The executable provides its own discovery path for humans and agents:

```sh
awtrix-cli --help                                # command groups and discovery hints
awtrix-cli --json describe --list                # supported topics and usage guidance
awtrix-cli --json describe apps create --offline  # semantics, payload schema, outputs, examples
awtrix-cli apps create --help                    # exact execution syntax and flags
```

`describe` reads documentation; it does not execute the documented operation or validate a payload. Topics can be separate words (`describe script verify`) or a quoted string (`describe "script verify"`). With no topic it describes `device`. Some operations share a family-level description, notably `script get`, `script deploy`, and `script re-minify`; use their execution help for exact flags.

Descriptions use a local ESP32 reference by default. Supplying `--target` **or setting `AWTRIX_URL`** requests live capabilities from `/api/v1/capabilities`; failures propagate, and connected capability names are authoritative. Use `--offline` to read local documentation regardless of those target settings. Named/default profiles alone do not trigger a connection; `--list`, profile descriptions, and `describe update` are always local. Offline documentation does not prove that a particular device supports an operation.

Descriptions are JSON objects (pretty by default, compact with `--json`). `--fields a,b` selects top-level fields of the description itself. For execution, choose fields from that operation's documented output or a complete result, especially for server-defined responses. Projection happens **after execution**: a field-selection error does not undo a device write, saved file, profile change, or executable update. Inspect the outcome before retrying. Check the exit status and both stdout and stderr on failure.

## Technical notes

### Install or build the executable yourself

The skill normally handles CLI installation for the agent. If you prefer to install or build it yourself, published binaries are available from [GitHub Releases](https://github.com/toinux/awtrix-cli/releases) for Linux x86_64 (GNU/glibc), macOS Apple Silicon, and Windows x86_64. The CLI also supports building from source with Rust:

```sh
cargo install --git https://github.com/toinux/awtrix-cli --locked
```

Or, from a local checkout:

```sh
cargo install --path . --locked
```

Run `awtrix-cli update` to explicitly update an installed release. The optional startup update check only prints a notice; it never installs automatically. Set `AWTRIX_NO_UPDATE_CHECK=1` to disable that check.

### Projects and testing

An `awtrix.toml` project can describe scripts, Berry modules, resources, configuration patches, and optional tests. Start with `awtrix-cli project init ./demo`, then validate and deploy the generated manifest. See the [example project](examples/project/awtrix.toml) for a complete project with a module, script, resource, and test assertions.

Project deployment validates local inputs, then writes **modules → resources → scripts → config**, in manifest order within each group. It is additive and nontransactional: prior successes remain after a later failure, with no global rollback. Modules are unconditional raw replacements. Scripts require exactly one of `create = true` or `expected_source_file`; `project deploy --force` changes script writes to unconditional replacements but still requires a valid manifest. Review `project prune --dry-run` before removing previously tracked project entries.

A minimal manifest for `src/main.ax` is:

```toml
[project]
name = "demo"
version = "0.1.0"

[[scripts]]
name = "main"
file = "src/main.ax"
create = true
```

For an existing script, first save its exact remote source with `awtrix-cli script get main > original.ax` beside the manifest. Keep that reference unchanged, edit a separate source file, and replace `create = true` with `expected_source_file = "original.ax"`. A stale reference produces a conflict rather than overwriting a concurrent edit.

Discover and run project tests with:

```sh
awtrix-cli --json describe test project --offline
awtrix-cli test project --help
awtrix-cli --json test project --manifest demo/awtrix.toml --binary ./awtrix-linux
awtrix-cli --json --target http://127.0.0.1:8080 test project --manifest demo/awtrix.toml
```

Tests deploy first, then observe `[tests]` assertions for `window_secs` (default 10 seconds; deployment/readiness are outside this bound). Supported forms are `script_active`, `script`/`path`/`equals`, `log_contains`, and visual PNG comparisons. JSON reports include assertion evidence, errors, deployment, target mode and observation completeness. Isolated Linux runs require a caller-supplied AWTRIX executable (optional Web UI assets), and clean up their child, temporary data and tracking on completion, failure or Ctrl-C. External reuse requires command-line `--target`; environment/default targets do not select it. Explicit `--target` and `--binary` conflict. External tests write to that endpoint and never stop it.

For the independent local lifecycle, describe `headless start`, `headless status` or `headless stop` with `--offline`, then use the matching command help. Ownership is recorded in `headless.json` beside the selected personal config. `status` checks recorded process identity, not HTTP health. `stop` sends SIGTERM and waits up to five seconds; temporary data is removed on successful stop, while explicit `--data` directories are retained. Device target settings do not select the local process.

### Compatibility

This client targets the AWTRIX NG HTTP API. Available operations depend on the capabilities advertised by the connected device. The headless lifecycle and isolated headless tests are Linux-only. A successful HTTP request alone does not prove that content is visible on the physical display; use runtime verification and screen capture when you need evidence of the result.
