# Agent release procedure

Use this procedure only when the user explicitly asks to cut a release. Pushing
the tag publishes a stable GitHub Release, so first prepare a proposal and wait
for the user's explicit confirmation before making release commits or pushing
the tag.

## Prepare and propose

1. Verify the working tree is clean, fetch `origin`, and confirm the checkout is
   on `main` and up to date with `origin/main`. Do not release from a feature
   branch or silently discard unrelated work.
2. Inspect commits since the latest `v*` tag and reconcile all notable changes
   with `CHANGELOG.md`'s `Unreleased` section. Keep entries in English, grouped
   under the applicable Keep a Changelog categories (`Added`, `Changed`,
   `Deprecated`, `Removed`, `Fixed`, `Security`), and explicitly describe breaking
   changes and migration steps. Summarize these entries in concise release notes.
3. Choose the next stable Semantic Versioning number: patch for compatible fixes,
   minor for backward-compatible features, and major for breaking changes. Do
   not create prerelease or build-metadata versions.
4. Present the proposed version, rationale, concise release notes and exact tag
   (`v<version>`). Wait for explicit confirmation before proceeding.

## Prepare and publish after confirmation

1. Update the package version in `Cargo.toml`. Update the `awtrix-cli` package
   entry in `Cargo.lock` without changing unrelated dependency versions. Update
   `docs/distribution.md`'s current-version reference if needed. For every release,
   move the approved `Unreleased` entries into a new `## [<version>] - YYYY-MM-DD`
   section in `CHANGELOG.md`, using the UTC publication date. Keep an empty
   `Unreleased` section at the top and update the version comparison links.
2. Run `cargo check` to refresh the lockfile, then `cargo check --locked` and
   relevant local checks. Inspect the diff to ensure only intended version and
   documentation changes are included.
3. Create a commit on `main` for the version bump and changelog. Create an **annotated** tag
   named `v<version>` on that commit; its message must contain the concise
   release notes proposed to the user.
4. Push the commit to `origin/main`, then push the tag. Never force-push, move,
   or replace an existing tag. If the branch or tag has diverged or the working
   tree is no longer clean, stop and report the issue rather than overwriting
   state.
5. Follow the tag-triggered GitHub Actions run through release publication and
   the native post-release installation checks. Report the run and release URL
   and any failures; do not claim success until those checks pass.

The workflow validates the tag against `Cargo.toml`, rejects non-stable
versions, builds and tests the three native targets, publishes the binaries and
`SHA256SUMS`, and verifies installation on Linux, macOS and Windows. It reads
the release description from the annotated tag message. Do not bypass these
checks or manually mutate an already-published release.
