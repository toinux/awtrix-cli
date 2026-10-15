#!/usr/bin/env bash
# Compare the official AWTRIX NG documentation with the commit this project last
# reviewed, and report what awtrix-cli covers.
#
#   awtrix-coverage.sh              fetch the docs repo, diff since the reviewed commit
#   awtrix-coverage.sh --snapshot   record the fetched commit as the reviewed one
#
# The repository is the source of the published site: docs/ holds one Markdown
# tree plus one docs/api/openapi.yaml whose path-level `x-only: [...]` markers
# select the clock variants. tools/docs/build_site.py renders the four sites.
#
# Data:
#   ../data/covered.txt   routes awtrix-cli calls; hand-maintained, checked
#                         against src/ on every run
#   docs/feature-coverage.md
#                         holds the reviewed commit, the single baseline
#
# Exit 0 when nothing changed since the reviewed commit, 1 otherwise.
set -euo pipefail

command -v git >/dev/null || { echo "git is required" >&2; exit 1; }

UPSTREAM=${AWTRIX_NG_REPO:-https://github.com/Blueforcer/awtrix-ng}
HERE=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
DATA=$HERE/../data
REPO=${AWTRIX_CLI_REPO:-$(cd "$HERE/../../../.." && pwd)}
DOC=${AWTRIX_COVERAGE_DOC:-$REPO/docs/feature-coverage.md}
CACHE=${AWTRIX_COVERAGE_CACHE:-${TMPDIR:-/tmp/opencode}/awtrix-coverage}
CLONE=$CACHE/awtrix-ng
ROW='| Last reviewed AWTRIX NG commit'

snapshot=0
[ "${1:-}" = "--snapshot" ] && snapshot=1

mkdir -p "$CACHE"
if [ -d "$CLONE" ]; then
  printf 'fetching %s ...\n' "$UPSTREAM" >&2
  git -C "$CLONE" fetch --prune --no-tags --quiet origin main
else
  printf 'cloning %s ...\n' "$UPSTREAM" >&2
  git clone --no-checkout --no-tags --quiet "$UPSTREAM" "$CLONE"
fi
head=$(git -C "$CLONE" rev-parse origin/main)

if [ "$snapshot" = 1 ]; then
  version=$(git -C "$CLONE" show "$head:version")
  date=$(git -C "$CLONE" show -s --format=%cs "$head")
  tmp=$DOC.tmp
  awk -v row="$ROW" -v new="$ROW | \`$(git -C "$CLONE" rev-parse --short=12 "$head")\` ($date, version $version) |" '
    index($0, row) == 1 { $0 = new; hit = 1 }
    { print }
    END { exit(hit ? 0 : 1) }
  ' "$DOC" > "$tmp" || { echo "could not update '$ROW' in $DOC" >&2; rm -f "$tmp"; exit 1; }
  mv "$tmp" "$DOC"
  printf 'reviewed commit recorded: %s (%s, version %s)\n' "${head:0:12}" "$date" "$version" >&2
  exit 0
fi

base=$(grep "^$ROW" "$DOC" 2>/dev/null | grep -oE '[0-9a-f]{7,40}' | head -1 || true)
if [ -z "$base" ]; then
  echo "no reviewed commit recorded in $DOC; run with --snapshot after a review" >&2
  base=$head
elif ! git -C "$CLONE" cat-file -e "$base^{commit}" 2>/dev/null; then
  echo "recorded commit $base is not in $UPSTREAM (history rewritten?); review and --snapshot again" >&2
  base=$head
fi

# "path<TAB>method<TAB>targets", targets from the path-level x-only marker.
routes_at() {
  git -C "$CLONE" show "$1:docs/api/openapi.yaml" | awk '
    /^  \/[^ ]+:$/ { p = $1; sub(/:$/, "", p); only = ""; next }
    /^    x-only: \[/ { only = $0; sub(/^    x-only: \[/, "", only); sub(/\].*/, "", only); gsub(/ /, "", only); next }
    /^    (get|post|put|patch|delete):$/ {
      m = $1; sub(/:$/, "", m)
      print p "\t" m "\t" (only == "" ? "esp32,esp32-s3,tc002" : only)
    }
  '
}

# path<TAB>esp32<TAB>esp32-s3<TAB>tc002, methods sorted, "-" when absent.
matrix() {
  routes_at "$1" | sort -k1,1 -k2,2 | awk -F'\t' '
    { seen[$1] = 1
      if ($3 ~ /(^|,)esp32(,|$)/)    E[$1] = E[$1] "," $2
      if ($3 ~ /(^|,)esp32-s3(,|$)/) S[$1] = S[$1] "," $2
      if ($3 ~ /(^|,)tc002(,|$)/)    C[$1] = C[$1] "," $2 }
    END { for (p in seen) printf "%s\t%s\t%s\t%s\n", p, cut(E[p]), cut(S[p]), cut(C[p]) }
    function cut(x) { return (x == "" ? "-" : substr(x, 2)) }
  ' | sort
}
matrix "$base" > "$CACHE/base.tsv"
matrix "$head" > "$CACHE/head.tsv"

drift=0
note() { printf '%s\n' "$1"; drift=1; }

base_version=$(git -C "$CLONE" show "$base:version")
head_version=$(git -C "$CLONE" show "$head:version")
[ "$base_version" != "$head_version" ] && note "docs version: $base_version -> $head_version"

while IFS=$'\t' read -r p e s t; do
  live=$(awk -F'\t' -v p="$p" '$1 == p { print $2 "\t" $3 "\t" $4 }' "$CACHE/head.tsv")
  if [ -z "$live" ]; then
    note "removed route: $p"
  elif [ "$live" != "$e	$s	$t" ]; then
    note "changed route: $p ($e|$s|$t -> $live)"
  fi
done < "$CACHE/base.tsv"
while IFS=$'\t' read -r p e s t; do
  grep -q "^$p	" "$CACHE/base.tsv" || note "NEW route: $p ($e|$s|$t)"
done < "$CACHE/head.tsv"

while IFS=$'\t' read -r status path; do
  case "$path" in docs/assets/*|docs/firmware/*) continue ;; esac
  case "$path" in *.md|docs/api/openapi.yaml) note "$status page: $path" ;; esac
done < <(git -C "$CLONE" diff --name-status "$base" "$head" -- docs)

while IFS= read -r p; do
  case "$p" in ''|'#'*) continue ;; esac
  grep -rhoE '/api/v1/[A-Za-z0-9/_{}.-]+' "$REPO/src" --include='*.rs' | grep -qxF "$p" \
    || note "stale covered.txt entry (not found in src/): $p"
done < "$DATA/covered.txt"

printf '\nLegend: methods per target, "-" = route absent on that clock, CLI = covered by awtrix-cli.\n\n'
printf '| Route | ESP32 | ESP32-S3 | TC002 | CLI |\n| --- | --- | --- | --- | --- |\n'
while IFS=$'\t' read -r p e s t; do
  if grep -qxF "$p" "$DATA/covered.txt"; then mark=yes; else mark='**no**'; fi
  printf '| `%s` | %s | %s | %s | %s |\n' "$p" "$e" "$s" "$t" "$mark"
done < "$CACHE/head.tsv"

total=$(wc -l < "$CACHE/head.tsv" | tr -d ' ')
gaps=$(awk -F'\t' 'NR == FNR { c[$1] = 1; next } !($1 in c)' "$DATA/covered.txt" "$CACHE/head.tsv" | wc -l | tr -d ' ')
printf '\n%s routes: %s covered, %s not covered.\n' "$total" "$((total - gaps))" "$gaps"
printf 'Reviewed commit %s -> %s (%s).\n' "${base:0:12}" "${head:0:12}" "${head_version}"
if [ "$drift" = 0 ]; then
  printf 'No documentation drift.\n'
else
  printf 'Documentation drift detected; review the lines above, then --snapshot.\n'
fi
exit "$drift"
