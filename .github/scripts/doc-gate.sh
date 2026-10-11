#!/usr/bin/env bash
# The Phase 3.5a gate's installed half (spec
# docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md
# §17): the installed nova documents a copy of examples/05-geometry, an app
# with a path library, with no warning, and writes the pages the gate needs.
# CI's `install` job runs it, from the repository's root.
#
# Usage: doc-gate.sh NOVA WORKDIR
#   NOVA     the nova executable to check
#   WORKDIR  where to copy the example; it must not exist yet
set -euo pipefail

if [ $# -ne 2 ]; then
  echo "usage: doc-gate.sh NOVA WORKDIR" >&2
  exit 2
fi
nova=$1
work=$2

mkdir "$work"
cp -R examples/05-geometry "$work/geometry"
cd "$work/geometry"

start=$("$nova" doc 2> doc.err)
if [ -s doc.err ]; then
  cat doc.err >&2
  echo "doc-gate: nova doc wrote to stderr" >&2
  exit 1
fi
start=${start//\\//}
if [ "$start" != "target/doc/geometry/index.html" ]; then
  echo "doc-gate: nova doc printed '$start'" >&2
  exit 1
fi
for page in index.html geometry/index.html shapes/index.html shapes/units/index.html \
    std/index.html std/core/index.html std/primitive.String.html; do
  if [ ! -f "target/doc/$page" ]; then
    echo "doc-gate: no target/doc/$page" >&2
    exit 1
  fi
done
echo "doc-gate: passed"
