# awtrix-cli

**Give your AI agent a practical way to work with your AWTRIX NG display.**

`awtrix-cli` is a command-line toolkit for [AWTRIX NG](https://github.com/Blueforcer/awtrix-ng). It lets an agent or shell workflow send notifications, manage temporary apps, deploy persistent Berry scripts, inspect runtime behavior, and capture the display—all through one command-line interface.

## Install the skill

Install the `awtrix-cli` skill so your coding agent knows how to install, configure, and use the CLI:

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

## What you can do

- **Control the display:** inspect device information and capabilities, adjust settings, change brightness or power, and reboot.
- **Manage temporary content:** create, update, list, select, order, and delete apps; send, queue, or dismiss notifications.
- **Develop Berry scripts:** deploy scripts, manage their configuration and data, inspect state and logs, and verify runtime behavior.
- **Manage reusable content:** deploy Berry modules and upload icons or other resources.
- **Work with projects:** declare scripts, modules, resources, configuration, and tests in an `awtrix.toml` project; validate and deploy it, then preview obsolete project-owned entries before pruning.
- **Capture the screen:** save the display framebuffer as a PNG.
- **Automate from agents and shell scripts:** discover commands, request JSON output, select relevant fields, and consume bounded JSONL log streams.

## Configure a device

Profiles keep device addresses and optional credentials in your personal CLI configuration. The setup above makes `desk` the default, so commands use it automatically. Use `awtrix-cli profile show desk` to inspect a profile or `awtrix-cli profile update desk --target URL` to change its address. For a single command, `--target URL` selects a device directly. `AWTRIX_URL` can also supply a target. If the device uses HTTP Basic authentication, configure credentials on the profile or set `AWTRIX_USERNAME` and `AWTRIX_PASSWORD`.

## Ask your agent

The installed skill teaches your agent the CLI workflows. You can ask it to perform tasks in natural language, for example:

> Deploy `./demo` to my default display, minify the script during deployment, and verify it for 10 seconds.

For direct shell use, `awtrix-cli --help` lists command groups. `awtrix-cli describe "script verify"` explains an operation without contacting a device. Add `--json` for machine-readable results, and `--fields a,b` to select top-level fields from a result.

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

Project deployments are additive and are not transactional: they do not delete undeclared device content. Review `project prune --dry-run` before removing previously tracked project entries. Linux headless testing is also available, but requires a separately supplied AWTRIX Linux executable and web assets; they are not bundled with this CLI.

### Compatibility

This client targets the AWTRIX NG HTTP API. Available operations depend on the capabilities advertised by the connected device. The headless lifecycle and isolated headless tests are Linux-only. A successful HTTP request alone does not prove that content is visible on the physical display; use runtime verification and screen capture when you need evidence of the result.
