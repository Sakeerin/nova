#!/usr/bin/env bash
# The Phase 3.0 gate (spec
# docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md §7 and
# §8): a `nova` with nothing beside it carries its runtime library, and
# makes, runs and builds a new project; with --test it also tests it. Since
# 4.8 (spec
# docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md
# §8, and the user's decision 8 in its header) it also builds the project
# for WASM: `dist/` holds the page, the shim, the module and the manifest
# (and the `package.json` that makes the shim a module to Node), the module
# begins with WASM's magic number, and, where `node` is on PATH, the shim
# run under Node prints what the native build prints. With --node that run
# is required, and a PATH without `node` fails the gate. Its callers, each
# on runners that have `node`: CI's `install` job, with --node;
# package-gate.sh, which CI's `package` job and publish-crates.yml run,
# with --node; release.yml's smoke test, with --node; and the Release gate
# (release-gate.yml), whose two jobs run the gate.sh of the tag they check
# and pass --node when that gate.sh takes it, from v0.4.0 on.
#
# Usage: gate.sh NOVA WORKDIR [--test] [--node]
#   NOVA     the nova executable to check
#   WORKDIR  where to make the project; it must not exist yet
#   --test   also run `nova test` in it
#   --node   require the WASM build's run under Node
set -euo pipefail

usage() {
  echo "usage: gate.sh NOVA WORKDIR [--test] [--node]" >&2
  exit 2
}
if [ $# -lt 2 ]; then
  usage
fi
nova=$1
work=$2
shift 2
with_test=""
with_node=""
for flag in "$@"; do
  case "$flag" in
    --test) with_test=--test ;;
    # release-gate.yml greps for the next line, as it stands, to pass --node
    # only to a gate.sh that takes it; change the two together
    # (crates/nova-cli/tests/release_manifests.rs checks that they agree).
    --node) with_node=--node ;;
    *) usage ;;
  esac
done

if env | grep -q '^NOVA_'; then
  env | grep '^NOVA_' >&2
  echo "gate: a NOVA_ variable is set, and the gate needs none" >&2
  exit 1
fi

mkdir "$work"
cd "$work"

"$nova" version | tee version.txt
if ! grep -qx 'runtime: embedded' version.txt; then
  echo "gate: this nova does not carry its runtime library" >&2
  exit 1
fi

"$nova" new demo
cd demo

ran=$("$nova" run)
if [ "$ran" != "Hello, Nova!" ]; then
  echo "gate: nova run printed '$ran'" >&2
  exit 1
fi

"$nova" build
built=$(./target/debug/demo)
if [ "$built" != "Hello, Nova!" ]; then
  echo "gate: the program nova build wrote printed '$built'" >&2
  exit 1
fi

"$nova" build --target wasm
for file in index.html app.js app.wasm manifest.json; do
  if [ ! -s "dist/$file" ]; then
    echo "gate: nova build --target wasm wrote no dist/$file" >&2
    exit 1
  fi
done
magic=$(od -A n -t x1 -N 4 dist/app.wasm | tr -d ' \n')
if [ "$magic" != "0061736d" ]; then
  echo "gate: dist/app.wasm begins with $magic, not WASM's magic number" >&2
  exit 1
fi
if command -v node > /dev/null; then
  wasm=$(node dist/app.js)
  if [ "$wasm" != "Hello, Nova!" ]; then
    echo "gate: the WASM build printed '$wasm' under Node" >&2
    exit 1
  fi
  echo "gate: the WASM build prints Hello, Nova! under Node"
elif [ "$with_node" = "--node" ]; then
  echo "gate: --node, but there is no node on PATH to run the WASM build" >&2
  exit 1
else
  echo "gate: no node on PATH, so the WASM build is built but not run"
fi

if [ "$with_test" = "--test" ]; then
  "$nova" test | tee test.txt
  if ! grep -q '1 passed; 0 failed' test.txt; then
    echo "gate: nova test did not pass the template's one test" >&2
    exit 1
  fi
fi
echo "gate: passed"
