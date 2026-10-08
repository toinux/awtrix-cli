# Bootstrap details and fallbacks

Load this reference only when the executable is missing or installation needs
recovery. A successful `--version` establishes availability; retain that path.

## Choose install or upgrade

Prefer the compatible published release binary: the bundled installers select
the host artifact and verify it against `SHA256SUMS`. Supported assets are Linux
x86_64 GNU/glibc (`awtrix-cli-x86_64-unknown-linux-gnu`), macOS arm64
(`awtrix-cli-aarch64-apple-darwin`), and Windows x86_64 MSVC
(`awtrix-cli-x86_64-pc-windows-msvc.exe`). No Rust toolchain is needed.

For an existing `awtrix-cli`, use the explicit command when the user requests an
upgrade:

```sh
awtrix-cli update
```

The command installs only when explicitly run. A startup release notice is
informational and never installs anything automatically. The update command uses
the matching stable published binary and verifies its checksum; if unavailable,
report the limitation and offer the source-build fallback below. Do not silently
switch an update request into a source build.

## Bootstrap a missing executable

Installing the skill does not install the CLI. Check the normal per-user path
below before installing again. If unavailable, announce the installation and
execute the bundled installer. Set `SKILL_DIR` / `$SkillDir` to the absolute
loaded skill directory, not the workspace. Use the reported destination as
authoritative; keep home/environment paths as shell expressions.

Linux/macOS:

```sh
sh "$SKILL_DIR/scripts/install.sh"
"$HOME/.local/bin/awtrix-cli" --version
```

Windows PowerShell:

```powershell
& (Join-Path $SkillDir 'scripts\install.ps1')
& "$env:LOCALAPPDATA\Programs\awtrix-cli\bin\awtrix-cli.exe" --version
```

Done means a successful install and version check. Retain the absolute executable
path across shell calls rather than reinstalling when a PATH change disappears.
Preserve nonzero installer status and recover using the applicable branch below.
If a tool blocks installation, ask specifically to allow it; report any remaining
blocker and ask only for task requirements/targets that cannot be inferred.

Cargo's binary directory can also contain an executable absent from `PATH`
(`$CARGO_HOME/bin`, normally `~/.cargo/bin`, or `%USERPROFILE%\.cargo\bin` on Windows).
Validate it by absolute path before considering a second installation.

## Preferred path: verified GitHub release binary

The shell installer requires `curl` and either `sha256sum` or `shasum`.
The Windows installer uses PowerShell's web requests and SHA-256 support.
Invoke scripts from the loaded skill's absolute directory. If scripts are absent,
the installed skill copy is incomplete: reinstall it from `toinux/awtrix-cli` using
`npx skills add toinux/awtrix-cli --skill awtrix-cli` with the original agent/scope
options. If local PowerShell policy blocks execution,
report it and use a caller-approved execution method rather than changing
persistent machine policy.

The installers reuse an existing working command. Otherwise, they resolve the
latest stable release to one concrete tag, choose the host asset, download it and
`SHA256SUMS`, and check its exact checksum and version before replacing the
destination. Downloads/install errors remain nonzero, and a failed verification
leaves an existing destination intact. Installation is per-user and needs no
administrator access. The scripts print PATH instructions; a child script cannot
change its parent's environment. Prefer the absolute verified executable path
for agent shell calls; if adding to PATH, do so in each new shell invocation.
Preserve the original diagnostics on failure.

To choose a release or another destination **when the CLI is missing**, use
`--version v0.2.0 --install-dir DIR` for shell, or `-Version v0.2.0 -InstallDir DIR`
for PowerShell. These are bootstrap installers, not automatic upgrade commands.

Linux ARM64/musl, Intel macOS, and Windows ARM64 currently have no published
asset. The installers fail explicitly on unsupported hosts; use the source-build
fallback below only when no compatible asset exists or the user explicitly
requests source compilation. An incompatible glibc runtime also fails the
downloaded binary's checks before installation.

## Source-build fallback: Cargo with the committed lockfile

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
```

If the check fails, retain the installation/check diagnostics and fix the
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
when selecting an artifact or another architecture; verify its version
after placing it on `PATH`. No compiled binaries are bundled with this skill.

An AWTRIX NG HTTP(S) endpoint is required only for device operations. Linux
headless workflows additionally require a separately supplied AWTRIX Linux
executable; installing this CLI does not install that runtime.
