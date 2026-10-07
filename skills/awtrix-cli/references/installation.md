# Install only when needed

The skill and the executable are separate installations. First run
`awtrix-cli --version`. Use an existing executable directly when it works.
If it is missing, check whether Cargo's binary directory is simply absent from
`PATH` before installing again (`$CARGO_HOME/bin`, normally `~/.cargo/bin`, or
`%USERPROFILE%\.cargo\bin` on Windows).

## Preferred path: verified GitHub release binary

Locate the installed skill's own directory, using the path from which its
`SKILL.md` was loaded. Invoke the bundled installer by that absolute path;
it works from any working directory and does not require a source checkout.

Linux x86_64 GNU/glibc or macOS Apple Silicon, with `curl` and either `sha256sum`
or `shasum` available:

```sh
sh /path/to/awtrix-cli/scripts/install.sh
export PATH="$HOME/.local/bin:$PATH"
awtrix-cli --version
awtrix-cli --help
```

Windows x86_64, from PowerShell:

```powershell
& 'C:\path\to\awtrix-cli\scripts\install.ps1'
$env:PATH = "$env:LOCALAPPDATA\Programs\awtrix-cli\bin;$env:PATH"
awtrix-cli --version
awtrix-cli --help
```

Replace the example script path with the actual installed skill path. If local
PowerShell policy blocks execution, report it and use a caller-approved execution
method rather than changing persistent machine policy.

The installers reuse an existing working command. Otherwise, they resolve the
latest stable release to one concrete tag, choose the host asset, download it and
`SHA256SUMS`, and check its exact checksum and version/help before replacing the
destination. Downloads/install errors remain nonzero, and a failed verification
leaves an existing destination intact. Installation is per-user and needs no
administrator access. The scripts print PATH instructions; a child script cannot
change its parent's environment. Use the absolute installed binary path if PATH
cannot be changed, and preserve the original diagnostics on failure.

To choose a release or another destination **when the CLI is missing**, use
`--version v0.1.0 --install-dir DIR` for shell, or `-Version v0.1.0 -InstallDir DIR`
for PowerShell. These are bootstrap installers, not automatic upgrade commands.

Linux ARM64/musl, Intel macOS, and Windows ARM64 currently have no published
asset. The installers fail explicitly on unsupported hosts; use the source-build
fallback below if a suitable native Rust toolchain is available. An incompatible
glibc runtime also fails the downloaded binary's checks before installation.

## Fallback: Cargo with the committed lockfile

Check `cargo --version` and `rustc --version`. Building requires a stable Rust
toolchain and a native linker/SDK. If absent, use the official
[Rust installation instructions](https://rustup.rs/) for the host and make the
toolchain available to the current shell.

From an actual awtrix-cli source checkout (the directory containing its
`Cargo.toml` and `Cargo.lock`):

```sh
cargo install --path . --locked
```

When only this installed skill is available, install from the upstream repository:

```sh
cargo install --git https://github.com/toinux/awtrix-cli.git --locked awtrix-cli
```

The installed skill directory is not a Rust source checkout. The project does
not currently document a crates.io release; do not assume `cargo install awtrix-cli`.

Verify installation in the same shell:

```sh
awtrix-cli --version
awtrix-cli --help
```

If either check fails, retain the installation/check diagnostics and fix the
toolchain or `PATH` rather than proceeding as if installation succeeded.
For a local source build without installing, use `cargo build --release --locked`
and substitute `./target/release/awtrix-cli` (Windows: the `.exe` path) in commands.

## Hosts and standalone artifacts

The documented distribution matrix is Linux x86_64 GNU/glibc, macOS arm64,
and Windows x86_64 MSVC. Cargo builds for the current host; cross-compilation
also needs the matching linker/SDK. Version-tagged CI publishes verified binaries
and checksums as [GitHub releases](https://github.com/toinux/awtrix-cli/releases).
Consult the current
[distribution documentation](https://github.com/toinux/awtrix-cli/blob/HEAD/docs/distribution.md)
when selecting an artifact or another architecture; verify its version/help
after placing it on `PATH`. No compiled binaries are bundled with this skill.

An AWTRIX NG HTTP(S) endpoint is required only for device operations. Linux
headless workflows additionally require a separately supplied AWTRIX Linux
executable; installing this CLI does not install that runtime.
