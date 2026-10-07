# Standalone CLI distribution

## Supported client hosts

The v0.1 distribution matrix is **Linux x86_64 (GNU/glibc)**, **macOS arm64**,
and **Windows x86_64 (MSVC)**. Artifacts are respectively named
`awtrix-cli-x86_64-unknown-linux-gnu`, `awtrix-cli-aarch64-apple-darwin`, and
`awtrix-cli-x86_64-pc-windows-msvc.exe`. The executable is named `awtrix-cli` on
all hosts. The CLI requires no Rust installation.
Linux artifacts dynamically use the host GNU C runtime; use a compatible
glibc-based system. macOS and Windows artifacts use their platform system
libraries. HTTPS uses bundled Rustls roots and does not require OpenSSL.

The supported build targets, CI matrix and artifacts are deliberately explicit.
From a clean checkout with the stable Rust toolchain, build reproducibly with
the committed lockfile, for example:

```sh
cargo build --release --locked --target x86_64-unknown-linux-gnu
target/x86_64-unknown-linux-gnu/release/awtrix-cli --help
target/x86_64-unknown-linux-gnu/release/awtrix-cli --version
```

On macOS use `aarch64-apple-darwin` and on Windows use
`x86_64-pc-windows-msvc`; install the Rust target with `rustup target add`
before building on a different host. The matching native linker/SDK is
required. `.github/workflows/verify.yml` runs on branch pushes, pull requests,
and version tags. Every native check uploads its binary artifact, retaining the
existing CI artifact behavior on ordinary branches and pull requests. Those
ordinary verification runs do not publish a release. Workflow runs use the
host-native runners: their help/version and
HTTP contract test executes the just-built release artifact (selected by
`AWTRIX_DISTRIBUTION_BINARY`) against a local deterministic HTTP fixture; it
does not substitute Cargo's debug test executable. Headless process control is
Linux-only: on other hosts `headless stop` returns `UNSUPPORTED_HOST` before
reading ownership state, while `headless status` continues to report the
recorded process as not running when Linux process identity cannot be checked.

## Tagged releases

Pushing a stable tag whose name is exactly `v<package version>` (currently
`v0.1.2`; prerelease/build-metadata versions are rejected) starts the same three
native verification jobs. Each job builds once with
`--locked`, exercises the release executable, and uploads that verified binary;
the release job reuses those artifacts rather than rebuilding. A tag that does
not exactly match the `awtrix-cli` version in `Cargo.toml` fails before release
publication. Wait for all native jobs to pass before considering the release
available.

Native jobs also exercise the skill's installers with offline fixtures, including
the freshly built Windows executable. After publication, a second native matrix
downloads and installs the actual published assets with the bundled installers
and runs `--version`/`--help` on Linux, macOS, and Windows.

On a green run, the workflow assembles exactly these assets:

* `awtrix-cli-x86_64-unknown-linux-gnu`
* `awtrix-cli-aarch64-apple-darwin`
* `awtrix-cli-x86_64-pc-windows-msvc.exe`
* `SHA256SUMS`

`SHA256SUMS` is conventional `sha256sum` text, one line per binary in the form
`<hex>  <asset basename>`. Verify downloads with `sha256sum -c SHA256SUMS` from
the directory containing all three binaries. The workflow creates the release
as a draft, attaches/replaces all assets and checksum first, then publishes it
as the stable latest release. A rerun of an already-published tag compares the
asset set and checksums and leaves a matching release unchanged; mismatched
published releases fail rather than being modified. Incomplete draft releases
may be safely completed by rerunning the workflow.

To cut a release, update the package version and lockfile as appropriate, tag
the matching commit (for example `git tag v0.1.2`), and push that tag. The
workflow needs repository `contents: write` permission for its release job.

The crate disables reqwest's default TLS backend and selects `rustls-tls`;
blocking, JSON and multipart support remain enabled. Personal profile
configuration follows the platform conventions documented in
`cli-contract.md` (Windows `%APPDATA%`, macOS/Linux `$HOME/.config`), with
`AWTRIX_CONFIG` override. Unix configuration permissions remain 0600.

## Headless support is separate

The `headless` lifecycle manages an explicitly supplied **Linux AWTRIX**
executable only. It is not bundled or downloaded, and is not claimed to run on
macOS or Windows. The HTTP client and other CLI commands are supported on all
three listed hosts. Headless commands return the existing explicit unsupported
platform error outside Linux.

## Verification boundary

CI is the acceptance gate for each announced host. Local Linux checks do not
establish macOS or Windows execution. A host without an available native runner
must remain unverified until the corresponding CI job actually runs; merely
cross-compiling is not a runtime test. No physical AWTRIX is needed by the
HTTP fixture, and the fixture is not a physical-device validation.
