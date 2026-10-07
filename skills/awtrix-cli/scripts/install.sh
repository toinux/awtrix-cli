#!/bin/sh
# Install the published awtrix-cli binary after verifying its release checksum.
set -eu

repo=toinux/awtrix-cli
version=
install_dir=
while [ "$#" -gt 0 ]; do
    case "$1" in
        --version|-Version)
            [ "$#" -ge 2 ] || { printf '%s\n' 'Missing value for --version' >&2; exit 2; }
            version=$2; shift 2 ;;
        --install-dir|-InstallDir)
            [ "$#" -ge 2 ] || { printf '%s\n' 'Missing value for --install-dir' >&2; exit 2; }
            install_dir=$2; shift 2 ;;
        -h|--help)
            printf '%s\n' 'Usage: install.sh [--version vX.Y.Z] [--install-dir DIR]'; exit 0 ;;
        *) printf 'Unknown argument: %s\n' "$1" >&2; exit 2 ;;
    esac
done

if command -v awtrix-cli >/dev/null 2>&1; then
    if existing_version=$(awtrix-cli --version 2>/dev/null) && case "$existing_version" in 'awtrix-cli '*) true ;; *) false ;; esac; then
        awtrix-cli --help >/dev/null 2>&1 || { status=$?; exit "$status"; }
        printf '%s\n' 'awtrix-cli is already installed; using the existing command.'
        printf '%s\n' "$existing_version"
        exit 0
    fi
fi

valid_version() { printf '%s\n' "$1" | awk '/^v[0-9]+\.[0-9]+\.[0-9]+$/ { ok=1 } END { exit !ok }'; }

case "$(uname -s):$(uname -m)" in
    Linux:x86_64|Linux:amd64)
        asset=awtrix-cli-x86_64-unknown-linux-gnu
        if [ -r /proc/self/maps ] && grep -Eqi 'musl' /proc/self/maps; then
            printf '%s\n' 'Unsupported Linux libc: this release requires GNU/glibc; musl is not supported. Use the documented Cargo build path.' >&2; exit 1
        fi ;;
    Darwin:arm64|Darwin:aarch64) asset=awtrix-cli-aarch64-apple-darwin ;;
    *) printf 'Unsupported host %s/%s. Use the documented Cargo build path.\n' "$(uname -s)" "$(uname -m)" >&2; exit 1 ;;
esac

if [ -n "$version" ] && ! valid_version "$version"; then
    printf 'Invalid version tag: %s (expected vMAJOR.MINOR.PATCH)\n' "$version" >&2; exit 2
fi

command -v curl >/dev/null 2>&1 || { printf '%s\n' 'curl is required.' >&2; exit 1; }
if command -v sha256sum >/dev/null 2>&1; then hash_cmd=sha256sum
elif command -v shasum >/dev/null 2>&1; then hash_cmd=shasum
else printf '%s\n' 'sha256sum or shasum is required.' >&2; exit 1; fi

if [ -z "$install_dir" ]; then
    install_dir="${HOME}/.local/bin"
fi
mkdir -p "$install_dir"
tmp=$(mktemp -d "${TMPDIR:-/tmp}/awtrix-cli-install.XXXXXX")
cleanup() { rm -rf "$tmp"; }
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

if [ -z "$version" ]; then
    # GitHub's stable-latest endpoint redirects to a concrete /tag/vX.Y.Z URL.
    effective=$(curl --proto '=https' --proto-redir '=https' --connect-timeout 20 --max-time 60 --fail --silent --show-error --location --output /dev/null --write-out '%{url_effective}' "https://github.com/$repo/releases/latest") || { printf '%s\n' 'Could not resolve the latest stable release.' >&2; exit 1; }
    version=${effective##*/}
    if ! valid_version "$version"; then printf 'Could not determine a concrete release tag from %s\n' "$effective" >&2; exit 1; fi
fi

# Version is now pinned once; both files and the binary use this immutable tag URL.
base="https://github.com/$repo/releases/download/$version"
curl --proto '=https' --proto-redir '=https' --connect-timeout 20 --max-time 120 --fail --silent --show-error --location "$base/$asset" -o "$tmp/$asset" || { printf 'Binary download failed for %s.\n' "$version" >&2; exit 1; }
curl --proto '=https' --proto-redir '=https' --connect-timeout 20 --max-time 60 --fail --silent --show-error --location "$base/SHA256SUMS" -o "$tmp/SHA256SUMS" || { printf 'Checksum download failed for %s.\n' "$version" >&2; exit 1; }
expected=$(awk -v name="$asset" '$2 == name {print $1}' "$tmp/SHA256SUMS")
case "$expected" in *[!0-9a-fA-F]*|'') printf 'No valid checksum entry for %s.\n' "$asset" >&2; exit 1 ;; esac
[ "${#expected}" -eq 64 ] || { printf 'Malformed checksum for %s.\n' "$asset" >&2; exit 1; }
if [ "$hash_cmd" = sha256sum ]; then actual=$(sha256sum "$tmp/$asset" | awk '{print $1}')
else actual=$(shasum -a 256 "$tmp/$asset" | awk '{print $1}'); fi
[ "$actual" = "$expected" ] || { printf 'SHA-256 verification failed for %s; existing installation was not changed.\n' "$asset" >&2; exit 1; }

dest="$install_dir/awtrix-cli"
chmod 755 "$tmp/$asset"
# Validate the staged program before touching an existing destination, including
# runtime compatibility (for example, the host's glibc version).
staged=$(mktemp "$install_dir/.awtrix-cli.new.XXXXXX")
trap 'rm -f "$staged"; cleanup' EXIT
cp "$tmp/$asset" "$staged"
chmod 755 "$staged"
reported_version=$("$staged" --version) || { printf '%s\n' 'Downloaded binary failed --version; installation was not changed.' >&2; exit 1; }
[ "$reported_version" = "awtrix-cli ${version#v}" ] || { printf 'Unexpected binary version: %s (expected awtrix-cli %s).\n' "$reported_version" "${version#v}" >&2; exit 1; }
"$staged" --help >/dev/null 2>&1 || { printf '%s\n' 'Downloaded binary failed --help; installation was not changed.' >&2; exit 1; }
mv -f "$staged" "$dest"
printf 'Installed awtrix-cli %s at %s\n' "$version" "$dest"
printf 'Add %s to PATH for this shell (for example: export PATH="%s:$PATH").\n' "$install_dir" "$install_dir"
printf '%s\n' "$reported_version"
