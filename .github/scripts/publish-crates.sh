#!/usr/bin/env bash
# Publishes the 19 crates to crates.io, in the order below (spec
# docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
# §5.2). It runs from the repository's root, and refuses unless VERSION is
# the workspace's version there: `cargo publish` uploads the version the
# manifests hold, whatever VERSION says.
#
# Before the first upload it reads every crate's sparse index file, and
# for each crate crates.io already has, at any version, its owners from
# crates.io's API. Unless NOVA_PUBLISH_OWNER is among them, the run stops
# there and nothing is published: under one of these names, someone
# else's crate would be skipped below as if it were ours, and `cargo
# publish`'s check of the next crate would build it, build script and all,
# in the process that holds the token. Then a crate whose index file
# already lists VERSION is skipped, so a run that stopped can be started
# again from the same tag. When crates.io refuses a crate for its rate
# limit on new crates, the script waits and tries that crate again, up to
# 30 times; any other failure stops the run before the next crate. It ends
# with one line per crate. publish-crates.yml runs it, on the tag, after
# package-gate.sh.
#
# Usage: publish-crates.sh VERSION
#   VERSION  the version to publish, without the v: the workspace's
#
# The environment must set:
#   NOVA_PUBLISH_OWNER  the crates.io login that owns the crates (a GitHub
#                       login, in any case)
# and may set:
#   CARGO               the cargo to run (default: cargo)
#   NOVA_PUBLISH_INDEX  the sparse index's base URL
#                       (default: https://index.crates.io)
#   NOVA_PUBLISH_API    crates.io's API base URL
#                       (default: https://crates.io/api/v1)
#   NOVA_PUBLISH_WAIT   seconds to wait after a rate-limit refusal
#                       (default: 660)
# cargo reads CARGO_REGISTRY_TOKEN from the environment. Nothing here prints
# it or sends it: no `set -x`, no request carries it, and no line names it
# but this one.
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
# The workspace's version: [workspace.package]'s, the only line of the
# root Cargo.toml that starts with `version = `, read as package-gate.sh
# reads it.
if [ ! -f Cargo.toml ]; then
  echo "publish-crates: there is no Cargo.toml here; run it from the repository's root" >&2
  exit 2
fi
workspace=$(tr -d '\r' < Cargo.toml | sed -n 's/^version = "\(.*\)"$/\1/p' | head -n 1)
if [ "$workspace" != "$version" ]; then
  echo "publish-crates: the workspace's version is '$workspace', not '$version'; cargo publish would upload '$workspace'" >&2
  exit 2
fi
# A GitHub login: letters, digits and hyphens.
owner=${NOVA_PUBLISH_OWNER:-}
case $owner in
  '' | *[!A-Za-z0-9-]*)
    echo "publish-crates: NOVA_PUBLISH_OWNER must be the crates.io login that owns the crates, not '$owner'" >&2
    exit 2
    ;;
esac
owner_lower=$(printf '%s' "$owner" | tr '[:upper:]' '[:lower:]')
cargo=${CARGO:-cargo}
index=${NOVA_PUBLISH_INDEX:-https://index.crates.io}
index=${index%/}
api=${NOVA_PUBLISH_API:-https://crates.io/api/v1}
api=${api%/}
# crates.io's API refuses a request without a User-Agent.
user_agent='nova-publish-crates (https://github.com/Sakeerin/nova)'
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

# Reads the crate's sparse index file into $scratch/index and sets
# `status` to the index's answer: 200, or 404 when crates.io does not have
# the crate. Any other answer stops the run.
read_index() {
  local url
  url="$index/$(index_path "$1")"
  if ! status=$(curl -sS --connect-timeout 30 --max-time 120 \
    -o "$scratch/index" -w '%{http_code}' "$url"); then
    stop "could not read the index at $url; nothing was published"
  fi
  case $status in
    200 | 404) ;;
    *) stop "the index answered $status for $url; nothing was published" ;;
  esac
}

# Sets `owners` to the crate's owners on crates.io, one login per line,
# from the API's compact JSON, {"users":[{"kind":...,"login":"<login>",
# ...}, ...]}. A team's login is github:<org>:<team>. Any answer but 200
# stops the run.
read_owners() {
  local url answer
  url="$api/crates/$1/owners"
  if ! answer=$(curl -sS --connect-timeout 30 --max-time 120 -A "$user_agent" \
    -o "$scratch/owners" -w '%{http_code}' "$url"); then
    stop "could not read $1's owners at $url; nothing was published"
  fi
  if [ "$answer" != 200 ]; then
    stop "crates.io's API answered $answer for $url; nothing was published"
  fi
  owners=$(grep -oE '"login"[[:space:]]*:[[:space:]]*"[^"]*"' "$scratch/owners" |
    sed -E 's/^"login"[[:space:]]*:[[:space:]]*"//; s/"$//' || true)
}

echo "publish-crates: ${#crates[@]} crates at $version, index $index"

# Before any upload: every crate crates.io has must be the owner's. What
# the index says of each is kept, by its place in the list: `yes` when it
# lists $version, a yanked version included, since crates.io never takes
# the same version twice; the closing quote is part of the match, so that
# 0.3.0 never matches 0.3.0-alpha.1. Publishing below never asks again,
# so a name someone takes meanwhile makes its `cargo publish` fail rather
# than be skipped.
echo "publish-crates: checking that crates.io has none of them under another owner than $owner"
listed=()
i=0
for crate in "${crates[@]}"; do
  read_index "$crate"
  listed[i]=no
  if [ "$status" = 404 ]; then
    echo "  $crate: not on crates.io"
  else
    read_owners "$crate"
    shown=$(printf '%s\n' "$owners" | awk 'NF { printf "%s%s", (n++ ? ", " : ""), $0 }')
    owners_lower=$(printf '%s\n' "$owners" | tr '[:upper:]' '[:lower:]')
    if ! grep -qxF -- "$owner_lower" <<< "$owners_lower"; then
      stop "$crate is on crates.io, owned by ${shown:-no one the API names}, not by $owner; nothing was published"
    fi
    echo "  $crate: on crates.io, owned by $shown"
    if grep -qF "\"vers\":\"$version\"" "$scratch/index"; then
      listed[i]=yes
    fi
  fi
  i=$((i + 1))
done
echo "publish-crates: no crate is on crates.io under another owner; publishing"

i=0
for crate in "${crates[@]}"; do
  if [ "${listed[i]}" = yes ]; then
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
