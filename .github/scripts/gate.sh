#!/usr/bin/env bash
# The Phase 3.0 gate (spec
# docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md §7 and
# §8): a `nova` with nothing beside it carries its runtime library, and
# makes, runs and builds a new project; with --test it also tests it. CI's
# `install` job and release.yml's smoke test both run this script.
#
# Usage: gate.sh NOVA WORKDIR [--test]
#   NOVA     the nova executable to check
#   WORKDIR  where to make the project; it must not exist yet
set -euo pipefail

if [ $# -lt 2 ] || [ $# -gt 3 ]; then
  echo "usage: gate.sh NOVA WORKDIR [--test]" >&2
  exit 2
fi
nova=$1
work=$2
with_test=${3:-}

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

if [ "$with_test" = "--test" ]; then
  "$nova" test | tee test.txt
  if ! grep -q '1 passed; 0 failed' test.txt; then
    echo "gate: nova test did not pass the template's one test" >&2
    exit 1
  fi
fi
echo "gate: passed"
