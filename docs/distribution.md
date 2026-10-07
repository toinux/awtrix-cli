# Standalone CLI distribution

## Supported client hosts

The v0.1 distribution matrix is **Linux x86_64 (GNU/glibc)**, **macOS arm64**,
and **Windows x86_64 (MSVC)**. Artifacts are respectively named
`awtrix-x86_64-unknown-linux-gnu`, `awtrix-aarch64-apple-darwin`, and
`awtrix-x86_64-pc-windows-msvc.exe`. The CLI requires no Rust installation.
Linux artifacts dynamically use the host GNU C runtime; use a compatible
glibc-based system. macOS and Windows artifacts use their platform system
libraries. HTTPS uses bundled Rustls roots and does not require OpenSSL.

The supported build targets, CI matrix and artifacts are deliberately explicit.
From a clean checkout with the stable Rust toolchain, build reproducibly with
the committed lockfile, for example:

```sh
cargo build --release --locked --target x86_64-unknown-linux-gnu
target/x86_64-unknown-linux-gnu/release/awtrix --help
target/x86_64-unknown-linux-gnu/release/awtrix --version
```

On macOS use `aarch64-apple-darwin` and on Windows use
`x86_64-pc-windows-msvc`; install the Rust target with `rustup target add`
before building on a different host. The matching native linker/SDK is
required. `.github/workflows/verify.yml` runs on push and pull request and
uploads one binary artifact per host. It does not publish a release or push to
a remote. Workflow runs use the host-native runners: their help/version and
HTTP contract test executes the built binary against a local deterministic
HTTP fixture.

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
