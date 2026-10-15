# Changelog

All notable changes to this project are documented in this file, in English.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and releases follow [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Entries summarize meaningful changes rather than individual commits.

Historical entries were reconstructed from Git history, tags, and GitHub release
notes. Dates use GitHub publication dates in UTC, or the tag date for unpublished
versions. There was no tagged or published `0.3.0` release.

## [Unreleased]

### Added

- AWTRIX NG feature-coverage documentation and a Git-based documentation drift
  checker to help keep CLI coverage aligned with upstream documentation.
- This changelog, covering the project from its inception, and a mandatory
  changelog update in the release procedure.

### Changed

- Centralized protected local-file persistence across existing callers while
  preserving their overwrite, backup, and failure-handling contracts.
- Isolated and consolidated the device HTTP adapter, with characterization tests
  for response decoding, request timeouts, and route-specific error handling.

## [0.4.0] - 2026-10-13

### Added

- ANSI true-color terminal rendering for `screen capture --format ansi`.
- Emoji screen previews with `screen capture --format emoji`. PNG remains the
  default; emoji colors are approximate rather than pixel-exact.
- Screen-capture format documentation, discovery contracts, and persistence
  failure tests.

### Changed

- Release workflows can retry an existing tag and use the explicitly selected
  tag when triggered manually.

### Fixed

- Reject device URLs containing query strings or fragments instead of silently
  accepting ambiguous targets.
- Serialize concurrent profile updates to prevent lost changes, and preserve
  existing profiles when writing a temporary save file fails.
- Start the reboot observation deadline after the reboot request completes.
- Update OpenCode worker and reviewer permissions for the v2 permission model,
  with regression tests for their access boundaries.

## [0.3.3] - 2026-10-13

### Changed

- Expanded the AWTRIX agent skill's operational guidance and documentation.
- Reduced redundant native CI builds and documented the release procedure.
- Removed obsolete local implementation bookkeeping.

### Fixed

- Keep the visual test fixture alive across idle gaps to avoid intermittent
  observation failures.

## [0.3.2] - 2026-10-13

### Added

- MIT license.
- Self-contained command discovery through `describe --list`,
  `describe <topic> --offline`, and topics written as separate words.
- Discovery tests for global options, payload schemas, and complete workflows.

### Changed

- Expanded command help and descriptions with inputs, outputs, schemas,
  prerequisites, examples, target and credential precedence, and limitations.
- Documented that `--fields` projects results after execution and cannot undo a
  device write, file save, profile change, or executable update.
- Clarified project deployment order, partial-failure behavior, protected script
  updates, headless ownership, and test observation boundaries.
- Documented Berry minifier limitations around fields accessed through aliases
  or receivers other than literal `self`.

### Fixed

- Align discovery with actual execution behavior, including local-only topics,
  live capability requests, output fields, and unsupported resource downloads.
- Route questions about the currently displayed app to structured device state
  rather than confusing it with app inventory or rotation.
- Stabilize CI observation fixtures and UTF-8 output handling.

## [0.3.1] - 2026-10-08

### Changed

- Skip executable downloads and replacement when the installed CLI is already
  current or newer than the latest stable release. `update --force` can reinstall
  the same version but does not downgrade a newer installation.
- Broadened agent skill selection for CLI installation, updates, device state,
  notifications, scripts, resources, and project workflows.
- Lead the README with agent skill installation and practical workflows.
- Deliver specification implementations through verified pull requests, recover
  interrupted runs idempotently, and defer issue closure until merge.
- Require complete fixed-reference reviews while allowing focused verification
  of workflow changes.

### Fixed

- Verify exact release asset requests in self-update regression tests.

## [0.2.0] - 2026-10-08

### Added

- Optional startup notices for newer stable releases; notices never install an
  update automatically. Set `AWTRIX_NO_UPDATE_CHECK=1` to disable the check.
- Explicit `update` command with checksum verification, protected executable
  replacement, and failure and backup-cleanup reporting.
- Local Berry minification into a sibling `.min.ax` file, leaving the original
  source unchanged, with controls for class and variable renaming.
- Opt-in in-memory minification during script deployment with `--minify`.
- `script re-minify` for installed scripts, with an exact-source local backup by
  default and a conditional update to protect concurrent edits.

### Changed

- **Migration required:** personal configuration now uses the `awtrix-cli`
  directory instead of `awtrix`: `~/.config/awtrix-cli/config.json` on Linux and
  macOS, or `%APPDATA%/awtrix-cli/config.json` on Windows (falling back to
  `%USERPROFILE%/AppData/Roaming/awtrix-cli/config.json`). The old directory is
  neither searched nor migrated. Move the existing `config.json` manually or
  select it explicitly with `AWTRIX_CONFIG`. The headless ownership record lives
  beside the selected configuration file.
- Prefer verified release binaries over source builds in agent installation
  guidance.

### Fixed

- Persist minified output and re-minification backups atomically, and prevent
  deployment when minification fails.
- Complete self-update failure handling and Windows replacement-fixture support.

## [0.1.2] - 2026-10-07

### Changed

- Streamlined agent skill bootstrapping and CLI discovery guidance.
- Standardized AWTRIX script examples and project scaffolding on `.ax` files;
  reusable Berry modules use `.be`.
- Moved issue and specification tracking to GitHub Issues.
- Recorded end-to-end, headless, and physical ESP32 validation evidence, with
  explicit limits on what those checks establish.

### Fixed

- Verify both `--version` and `--help` in release installers before installation.
- Require skill bootstrapping before app-specific clarification in agent flows.

## [0.1.1] - 2026-10-07

First published release. Includes the initial CLI described under `0.1.0` below.

### Fixed

- Correct release validation so native binaries can be published and installed
  successfully on Linux, macOS, and Windows.
- Synchronize log-follow fixtures and keep them alive through child-process exit
  to avoid intermittent native test failures.

## [0.1.0] - 2026-10-07

Initial implementation and release tag. CI blocked publication; no GitHub
Release or downloadable release assets were published for this version.

### Added

- Rust-based `awtrix-cli` with device identity, state, capabilities, and connection
  diagnostics for AWTRIX NG, including ESP32, ESP32-S3, and TC002 identification.
- Named device profiles, a default profile, environment and command-line target
  selection, and HTTP Basic authentication with credential-safe output.
- Structured JSON results, top-level field selection, stable error codes and exit
  statuses, bounded HTTP requests, and command descriptions.
- Protected Berry script creation and updates using exact-source conflict
  detection, explicit forced replacement, and independent start verification.
- Script lifecycle, configuration, and runtime data management; bounded runtime
  verification with log and optional screen-capture evidence.
- Cursor-based log reads and bounded JSONL log following with stream-failure and
  incomplete-observation reporting.
- PNG framebuffer capture with validated dimensions and pixels, bounded input,
  and atomic destination replacement.
- Berry module and device resource management.
- Temporary pushed apps with variant-aware limits, app inventory, active-app
  selection, and rotation ordering.
- Notifications, queue and active-notification deletion, and payload validation.
- Settings and display administration, brightness, power, overlays, reboot, and
  redacted system settings reads.
- Declarative `awtrix.toml` projects with scaffolding, preflight validation,
  ordered deployment, deployment provenance, explicit pruning, and reconciliation
  of uncertain tracking state.
- Declarative project tests for script activity, data, logs, and visual PNG
  assertions, with bounded observation and structured evidence.
- Linux-only headless runtime lifecycle and isolated project testing with
  process/listener ownership checks and cleanup.
- Native standalone distribution for Linux x86_64 GNU/glibc, macOS Apple Silicon,
  and Windows x86_64 MSVC, plus SHA-256 checksums and native CI validation.
- Agent Skills-compatible AWTRIX guidance and shell/PowerShell installers that
  select and verify release binaries without requiring a Rust toolchain.
- User documentation, example projects, domain terminology, and agent workflows
  for specification implementation and review.

[Unreleased]: https://github.com/toinux/awtrix-cli/compare/v0.4.0...HEAD
[0.4.0]: https://github.com/toinux/awtrix-cli/compare/v0.3.3...v0.4.0
[0.3.3]: https://github.com/toinux/awtrix-cli/compare/v0.3.2...v0.3.3
[0.3.2]: https://github.com/toinux/awtrix-cli/compare/v0.3.1...v0.3.2
[0.3.1]: https://github.com/toinux/awtrix-cli/compare/v0.2.0...v0.3.1
[0.2.0]: https://github.com/toinux/awtrix-cli/compare/v0.1.2...v0.2.0
[0.1.2]: https://github.com/toinux/awtrix-cli/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/toinux/awtrix-cli/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/toinux/awtrix-cli/tree/v0.1.0
