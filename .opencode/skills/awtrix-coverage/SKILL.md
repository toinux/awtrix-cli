---
name: awtrix-coverage
description: Check whether the official AWTRIX NG documentation gained features that awtrix-cli does not cover yet, and keep docs/feature-coverage.md current. Use when asked to re-check AWTRIX NG docs, find uncovered AWTRIX features, verify documentation drift or coverage, or refresh the feature-coverage baseline.
---

# AWTRIX NG documentation coverage

`docs/feature-coverage.md` records what the AWTRIX NG device API offers and what
`awtrix-cli` covers. The documentation source of truth is the
[Blueforcer/awtrix-ng](https://github.com/Blueforcer/awtrix-ng) repository — the
published site is a build of it — so a re-check is a git diff.

## Run the check

```sh
.opencode/skills/awtrix-coverage/scripts/awtrix-coverage.sh
```

The script clones (or fetches) the documentation repository, diffs it against the
reviewed commit recorded in `docs/feature-coverage.md`, checks the covered list
against `src/`, prints a Markdown coverage table, and exits non-zero on any drift.
Output goes to stdout; progress and fetch errors to stderr.

| Variable | Default |
| --- | --- |
| `AWTRIX_NG_REPO` | `https://github.com/Blueforcer/awtrix-ng` — point it at a local clone to work offline |
| `AWTRIX_CLI_REPO` | the repository root, four levels above this skill |
| `AWTRIX_COVERAGE_DOC` | `<repo>/docs/feature-coverage.md` |
| `AWTRIX_COVERAGE_CACHE` | `${TMPDIR:-/tmp/opencode}/awtrix-coverage` |

## Read the result

The repository is one Markdown tree plus one `docs/api/openapi.yaml`. Route and
page names are relative to `docs/`.

| Output | Meaning |
| --- | --- |
| `docs version: A -> B` | a release landed; its `docs/releases/<version>.md` is the change list |
| `NEW route:` / `NEW page:` | documented since the review — the candidate list for `docs/feature-coverage.md` |
| `changed route:` | the route gained, lost or reassigned methods or clock variants |
| `removed route:` / `M page:` / `D page:` | the device API or a page changed shape |
| `stale covered.txt entry` | a route marked covered no longer appears in `src/` |

Per clock-variant availability comes from the path-level `x-only: [...]` markers in
`docs/api/openapi.yaml`; the script reproduces the published ESP32, ESP32-S3 and
TC002 route sets exactly.

## Act on the result

1. For each `NEW`/`changed`/`removed` route or page, read the linked page or the
   route's `summary:` in `docs/api/openapi.yaml`, and decide with the user whether
   `awtrix-cli` should cover it. Update `docs/feature-coverage.md` accordingly.
2. When a command starts or stops calling a route, edit `data/covered.txt`
   (official OpenAPI path, one per line) and the matching table row.
3. Record the reviewed state once the document reflects reality:

```sh
.opencode/skills/awtrix-coverage/scripts/awtrix-coverage.sh --snapshot
```

`--snapshot` rewrites the reviewed commit row in `docs/feature-coverage.md` with
the fetched tip, its date and the `version` file. Run it only after the review.

## Limits

- Route availability uses path-level `x-only` markers only. Markers deeper inside
  a schema narrow fields, not routes; review the "Partially covered schemas"
  section of the document by hand when the schemas change.
- Covered status comes from literal route strings in `src/**/*.rs` and
  `data/covered.txt`. A partial schema gap (`settings patch` accepting 2 of 46
  keys, a payload key reachable only through `--raw`) is invisible here.
- Page drift is reported for `docs/**/*.md` and `docs/api/openapi.yaml`; assets
  and firmware bundles are ignored.
- The check needs git and network access, or a local clone through
  `AWTRIX_NG_REPO`. It never touches a physical device.
