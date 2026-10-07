# AWTRIX NG CLI

`awtrix` is a Rust command-line client for AWTRIX NG over HTTP. It provides
human-readable output by default and compact JSON with `--json`; device commands
can use a URL (`--target` / `AWTRIX_URL`) or a saved profile (`--profile`).
Credentials belong in personal profile configuration or environment variables,
not project manifests. Start with `awtrix --help` and `awtrix describe` (offline
command descriptions); quote multiword topics, for example
`awtrix describe "script verify" --json`.

## Build and run

```sh
cargo build --release --locked
./target/release/awtrix --help
./target/release/awtrix --json --target http://awtrix.local device diagnose
awtrix profile add --help
```

The supported client-host targets and standalone artifacts are documented in
[`docs/distribution.md`](docs/distribution.md). The Linux AWTRIX headless
executable is a separate caller-supplied prerequisite; it is not bundled or
downloaded. Read [`docs/cli-contract.md`](docs/cli-contract.md) for output,
configuration and exit-code contracts, and the domain contracts in
[`docs/apps-contract.md`](docs/apps-contract.md).

## Project workflow

Initialize and validate a local project without contacting a device:

```sh
awtrix project init ./demo
awtrix project validate --manifest ./demo/awtrix.toml
```

The manifest paths are relative to the manifest. Scripts, modules and resources
are deployed additively; pruning is explicit and restricted to successfully
tracked project-owned entries. Protected script deployment requires the
`scriptUpdates` capability unless `--force` is deliberately selected. The
example manifest is [`examples/project/awtrix.toml`](examples/project/awtrix.toml).

Deploy and run declarative checks on an explicitly selected physical target:

```sh
awtrix --profile desk --json project deploy --manifest ./demo/awtrix.toml
awtrix --profile desk --json test project --manifest ./demo/awtrix.toml
```

Local isolated headless testing (Linux only, real provided executable and web
assets required) uses this exact CLI shape:

```sh
AWTRIX_LINUX_BIN=/path/to/awtrix-linux AWTRIX_WEBUI=/path/to/webui/index.html \
  cargo test --test headless -- --ignored --exact \
  real_awtrix_headless_declarative_runs_cover_success_failure_berry_error_and_isolation
```

The test starts isolated instances, deploys the sample project, checks successful
and failing assertions including Berry errors, checks rendered pixels, and
verifies cleanup. It does not certify hardware behavior, memory/instruction
budgets, audio or sensors. Physical runs require explicitly supplied devices.

## Verification status and evidence boundary

Tickets 01–17 have been implemented in this checkout; see
`.scratch/awtrix-cli/issues/` for individual scope and contracts. Ticket 18
(native macOS/Windows distribution validation) remains pending acceptance; local
Linux checks do not substitute for native runners. Ticket 19 is not accepted:
its physical-device matrix has not been run, and its prerequisite 18 remains
pending. Do not interpret HTTP fixtures or headless results as hardware
compatibility evidence. The ticket's current validation gate and resumption
conditions are in
`.scratch/awtrix-cli/issues/19-validation-parcours-compatibilite.md` and
`docs/distribution.md`.

The checks run for this checkout are recorded with their actual commands and
results in the implementation handoff; neither native host checks nor physical
device checks are claimed here as completed validations.
