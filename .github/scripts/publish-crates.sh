#!/usr/bin/env bash
# Publishes the 19 crates to crates.io, in the order below (spec
# docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
# §5.2). A crate whose sparse index file already lists VERSION is skipped,
# so a run that stopped can be started again from the same tag. When
# crates.io refuses a crate for its rate limit on new crates, the script
# waits and tries that crate again, up to 30 times; any other failure stops
# the run before the next crate. It ends with one line per crate.
# publish-crates.yml runs it, on the tag, after package-gate.sh.
#
# Usage: publish-crates.sh VERSION
#   VERSION  the version to publish, without the v (the workspace's)
#
# The environment may set:
#   CARGO               the cargo to run (default: cargo)
#   NOVA_PUBLISH_INDEX  the sparse index's base URL
#                       (default: https://index.crates.io)
#   NOVA_PUBLISH_WAIT   seconds to wait after a rate-limit refusal
#                       (default: 660)
# cargo reads CARGO_REGISTRY_TOKEN from the environment. Nothing here prints
# it: no `set -x`, and no line names it but this one.
#
# release_manifests.rs and package-gate.sh read the list: keep `crates=(`,
# one crate per line, and `)` on its own line.
set -euo pipefail

crates=(
  nova-diagnostics
  nova-std
  nova-ast
  nova-lexer
  nova-runtime
  nova-parser
  nova-pm
  nova-fmt
  nova-index
  nova-resolver
  nova-hir
  nova-mir
  nova-typeck
  nova-codegen-cranelift
  nova-codegen-llvm
  nova-driver
  nova-doc
  nova-lsp
  nova-cli
)

# The attempts one crate gets while crates.io answers with its rate limit.
max_attempts=30

if [ $# -ne 1 ]; then
  echo "usage: publish-crates.sh VERSION" >&2
  exit 2
fi
version=$1
# A regex in a variable: bash 3.2 (macOS) and later read it the same way.
version_pattern='^[0-9]+\.[0-9]+\.[0-9]+(-[0-9A-Za-z.-]+)?$'
if ! [[ $version =~ $version_pattern ]]; then
  echo "publish-crates: '$version' is not a version such as 0.3.0" >&2
  exit 2
fi
cargo=${CARGO:-cargo}
index=${NOVA_PUBLISH_INDEX:-https://index.crates.io}
index=${index%/}
wait=${NOVA_PUBLISH_WAIT:-660}
case $wait in
  '' | *[!0-9]*)
    echo "publish-crates: NOVA_PUBLISH_WAIT must be a number of seconds, not '$wait'" >&2
    exit 2
    ;;
esac

scratch=$(mktemp -d "${TMPDIR:-/tmp}/publish-crates.XXXXXX")
trap 'rm -rf "$scratch"' EXIT

# What happened to each crate, by its place in the list.
results=()

summary() {
  echo "summary for $version:"
  local i=0 crate
  for crate in "${crates[@]}"; do
    printf '  %-24s %s\n' "$crate" "${results[$i]:-not tried}"
    i=$((i + 1))
  done
}

# Stop the run: the reason on stderr, then the summary.
stop() {
  echo "publish-crates: $1" >&2
  summary
  exit 1
}

# A crate's file in the sparse index: its name in lower case, under 1/, 2/,
# 3/<first character>/ or <characters 1-2>/<characters 3-4>/.
index_path() {
  local name
  name=$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')
  case ${#name} in
    1) printf '1/%s' "$name" ;;
    2) printf '2/%s' "$name" ;;
    3) printf '3/%s/%s' "${name:0:1}" "$name" ;;
    *) printf '%s/%s/%s' "${name:0:2}" "${name:2:2}" "$name" ;;
  esac
}

# Sets `listed` to yes when crates.io has the crate at $version: its index
# file has a line with "vers":"<version>", the closing quote included so
# that 0.3.0 never matches 0.3.0-alpha.1. A yanked version is listed too,
# and cannot be published again. 404 means the crate has no version yet;
# any other answer stops the run.
check_index() {
  local url status
  url="$index/$(index_path "$1")"
  if ! status=$(curl -sS --connect-timeout 30 --max-time 120 \
    -o "$scratch/index" -w '%{http_code}' "$url"); then
    stop "could not read the index at $url"
  fi
  case $status in
    200)
      if grep -qF "\"vers\":\"$version\"" "$scratch/index"; then
        listed=yes
      else
        listed=no
      fi
      ;;
    404) listed=no ;;
    *) stop "the index answered $status for $url" ;;
  esac
}

echo "publish-crates: ${#crates[@]} crates at $version, index $index"
i=0
for crate in "${crates[@]}"; do
  check_index "$crate"
  if [ "$listed" = yes ]; then
    echo "skip $crate $version (already on crates.io)"
    results[i]=skipped
    i=$((i + 1))
    continue
  fi
  attempt=1
  while :; do
    echo "publish-crates: cargo publish --locked -p $crate (attempt $attempt)"
    # cargo's output is shown as it comes, and kept to be read below.
    if "$cargo" publish --locked -p "$crate" 2>&1 | tee "$scratch/cargo.log"; then
      echo "published $crate $version"
      results[i]=published
      break
    fi
    if ! grep -qiE 'status 429|too many requests|try again after' "$scratch/cargo.log"; then
      results[i]=failed
      stop "cargo publish failed for $crate (its output is above); nothing after it was published"
    fi
    if [ "$attempt" -ge "$max_attempts" ]; then
      results[i]=failed
      stop "crates.io still refused $crate for its rate limit after $max_attempts attempts"
    fi
    echo "rate limited: waiting $wait s"
    sleep "$wait"
    attempt=$((attempt + 1))
  done
  i=$((i + 1))
done
summary
