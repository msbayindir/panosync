#!/bin/sh
# Renders the Homebrew formula for a release from its SHA256SUMS file.
#
#   packaging/homebrew/render-formula.sh <version without v> <SHA256SUMS> [base url] > panosync.rb
#
# The base URL defaults to the GitHub release download URL for that version.
set -eu

version="$1"
sums="$2"
base="${3:-https://github.com/msbayindir/panosync/releases/download/v$version}"
template="$(dirname "$0")/panosync.rb.template"

case "$version" in
    v*) echo "render-formula.sh: pass the version without the leading v" >&2; exit 1 ;;
esac

script="s|@VERSION@|$version|g; s|@BASE_URL@|$base|g"
for target in aarch64-apple-darwin x86_64-apple-darwin aarch64-unknown-linux-gnu x86_64-unknown-linux-gnu; do
    sha="$(awk -v f="panosync-$target.tar.gz" '$2 == f || $2 == "*" f { print $1 }' "$sums")"
    case "$sha" in
        [0-9a-f]*) [ "${#sha}" -eq 64 ] || { echo "render-formula.sh: bad checksum for $target" >&2; exit 1; } ;;
        *) echo "render-formula.sh: panosync-$target.tar.gz is not in $sums" >&2; exit 1 ;;
    esac
    script="$script; s|@SHA_$target@|$sha|g"
done

formula="$(sed "$script" "$template")"
case "$formula" in
    *@VERSION@*|*@BASE_URL@*|*@SHA_*) echo "render-formula.sh: unfilled placeholder in the formula" >&2; exit 1 ;;
esac
printf '%s\n' "$formula"
