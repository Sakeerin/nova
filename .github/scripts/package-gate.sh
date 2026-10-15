#!/usr/bin/env bash
# The package route (spec
# docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
# §4): the 20 crates publish-crates.sh lists package and verify as
# crates.io would check them; each packaged Cargo.lock names only what the
# workspace's names, at the same versions; `nova` installs from the unpacked
# packages as `cargo install nova-cli` would from crates.io, a release build
# that embeds the runtime; and that nova passes gate.sh --test. CI's
# `package` job runs it, from the repository's root, on all three systems.
#
# Usage: package-gate.sh WORKDIR
#   WORKDIR  where to package, unpack and install; it must not exist yet
set -euo pipefail

if [ $# -ne 1 ]; then
  echo "usage: package-gate.sh WORKDIR" >&2
  exit 2
fi
mkdir "$1"
workdir=$(cd "$1" && pwd)
# On Windows, Git Bash's /d/a/... paths mean nothing to cargo, and the
# config file below takes no backslashes: D:/a/... suits both.
if command -v cygpath > /dev/null 2>&1; then
  workdir=$(cygpath -m "$workdir")
fi

# The workspace's version: [workspace.package]'s, the only line of the
# root Cargo.toml that starts with `version = `.
version=$(tr -d '\r' < Cargo.toml | sed -n 's/^version = "\(.*\)"$/\1/p' | head -n 1)
# The crates, in publish order, from publish-crates.sh's `crates=(` list,
# so that there is one list.
crates=()
while IFS= read -r crate; do
  crates+=("$crate")
done < <(tr -d '\r' < .github/scripts/publish-crates.sh | awk '
  /^crates=\(/ { listing = 1; next }
  listing && /^\)/ { exit }
  listing { gsub(/[ \t]/, ""); if ($0 != "" && $0 !~ /^#/) print }')
if [ "${#crates[@]}" -ne 20 ] || [ -z "$version" ]; then
  echo "package-gate: expected 20 crates and a version, found ${#crates[@]} and '$version'" >&2
  exit 1
fi
echo "package-gate: ${#crates[@]} crates at $version, in $workdir"

export CARGO_TARGET_DIR="$workdir/target"
began=$SECONDS
step_began=$SECONDS
step_done() {
  echo "package-gate: $1 took $((SECONDS - step_began)) s"
  step_began=$SECONDS
}

# 0. Everything Cargo.lock names, for every platform, so that the rest runs
#    offline: the packages' verification and the install find exactly the
#    workspace's versions and nothing newer.
cargo fetch --locked
step_done "fetching the locked dependencies"

# 1. Package and verify, crates.io's own check, with cargo's local overlay
#    standing in for the not yet published nova crates. Verification builds
#    in the dev profile, so it never embeds the runtime; step 3 does.
package_args=()
for crate in "${crates[@]}"; do
  package_args+=(-p "$crate")
done
CARGO_INCREMENTAL=0 CARGO_PROFILE_DEV_DEBUG=0 \
  cargo package --locked --offline "${package_args[@]}"
packages="$workdir/target/package"
total=0
for crate in "${crates[@]}"; do
  file="$crate-$version.crate"
  # Relative names: GNU tar takes the `D:` of D:/a/... for a remote host.
  files=$(cd "$packages" && tar -tzf "$file" | wc -l | tr -d ' ')
  bytes=$(wc -c < "$packages/$file" | tr -d ' ')
  total=$((total + bytes))
  printf 'package-gate: %-36s %4s files %9s bytes\n' "$file" "$files" "$bytes"
done
echo "package-gate: ${#crates[@]} packages, $total bytes"
step_done "packaging and verifying"

# 2. Each packaged Cargo.lock names only packages the workspace's
#    Cargo.lock has, at the same versions, so `cargo install nova-cli
#    --locked` builds what CI built.
mkdir "$workdir/unpacked"
for crate in "${crates[@]}"; do
  (cd "$packages" && tar -xzf "$crate-$version.crate" -C ../../unpacked)
done
lock_entries() {
  tr -d '\r' < "$1" | awk '
    /^\[\[package\]\]/ { name = "" }
    /^name = / { name = $3; gsub(/"/, "", name) }
    /^version = / && name != "" { v = $3; gsub(/"/, "", v); print name " " v }' |
    LC_ALL=C sort -u
}
lock_entries Cargo.lock > "$workdir/workspace-lock.txt"
for crate in "${crates[@]}"; do
  lock="$workdir/unpacked/$crate-$version/Cargo.lock"
  if [ ! -f "$lock" ]; then
    echo "package-gate: $crate's package has no Cargo.lock" >&2
    exit 1
  fi
  extra=$(lock_entries "$lock" | LC_ALL=C comm -23 - "$workdir/workspace-lock.txt")
  if [ -n "$extra" ]; then
    echo "package-gate: $crate's Cargo.lock has what the workspace's has not:" >&2
    echo "$extra" >&2
    exit 1
  fi
done
echo "package-gate: every packaged Cargo.lock agrees with the workspace's"
step_done "comparing the lockfiles"

# 3. Install as crates.io would: from the unpacked nova-cli, its 18
#    libraries patched in for crates.io's copies. Cargo reads config from
#    the current directory up, and nova-cli's build script runs its nested
#    runtime build from nova-cli's unpacked directory, so that build reads
#    the same patches; it ignores CARGO_TARGET_DIR.
mkdir "$workdir/unpacked/.cargo"
{
  echo "[patch.crates-io]"
  for crate in "${crates[@]}"; do
    if [ "$crate" != nova-cli ]; then
      echo "$crate = { path = \"$workdir/unpacked/$crate-$version\" }"
    fi
  done
} > "$workdir/unpacked/.cargo/config.toml"
(cd "$workdir/unpacked" &&
  cargo install --path "nova-cli-$version" --offline --root "$workdir/root")
installed=$(ls -A "$workdir/root/bin")
case "$installed" in
  nova|nova.exe) ;;
  *)
    echo "package-gate: the install put more than nova into bin/: $installed" >&2
    exit 1
    ;;
esac
step_done "installing nova from the packages"

# 4. The installed nova carries its runtime, and makes, runs, builds and
#    tests a project.
bash .github/scripts/gate.sh "$workdir/root/bin/nova" "$workdir/gate" --test
step_done "the gate"

echo "package-gate: passed in $((SECONDS - began)) s"
