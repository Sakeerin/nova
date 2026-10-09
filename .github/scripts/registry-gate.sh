#!/usr/bin/env bash
# The Phase 3.3b gate (spec
# docs/superpowers/specs/2026-10-09-phase-3-3b-index-and-publishing-design.md
# §11.1): with the installed nova and a local index, a library is
# published, and a program depends on it from the index, runs, builds and
# tests; then, with the index gone, it builds again from nova.lock and the
# cache alone. NOVA_INDEX and NOVA_HOME are set in this script's process
# only. CI's `install` job runs it after packages-gate.sh, on all three
# systems.
#
# Usage: registry-gate.sh NOVA WORKDIR
#   NOVA     the nova executable to check
#   WORKDIR  where to make the index and the packages; it must not exist yet
set -euo pipefail

if [ $# -ne 2 ]; then
  echo "usage: registry-gate.sh NOVA WORKDIR" >&2
  exit 2
fi
nova=$1
work=$2

mkdir "$work"
cd "$work"
# Absolute, since step 5 renames the index from inside app/.
work=$(pwd)
# nova on Windows needs a Windows path: Git Bash's `pwd -W` gives C:/...
if root=$(pwd -W 2>/dev/null); then :; else root=$(pwd); fi

# 1. A local index whose tarballs are under it.
mkdir index
printf '%s\n' '{"dl":"dl/{name}-{version}.nova-pkg"}' > index/config.json
export NOVA_INDEX="$root/index"
export NOVA_HOME="$root/home"

# 2. A library, published to it.
"$nova" new --lib geom
(cd geom && "$nova" publish) | tee publish.txt
if ! grep -q 'published geom 0.1.0' publish.txt; then
  echo "registry-gate: nova publish did not say it published geom 0.1.0" >&2
  exit 1
fi

# 3. A program that depends on it from the index.
"$nova" new app
cd app
"$nova" add geom
cat > src/main.nova <<'EOF'
import geom

fn main() {
    println("from geom: ${greeting()}")
}

@test
fn greeting_comes_from_geom() {
    assert_eq(greeting(), "Hello, Nova!")
}
EOF

# 4. It runs and builds, and the tests of both packages pass.
expected='from geom: Hello, Nova!'
ran=$("$nova" run)
if [ "$ran" != "$expected" ]; then
  echo "registry-gate: nova run printed '$ran'" >&2
  exit 1
fi
"$nova" build
built=$(./target/debug/app)
if [ "$built" != "$expected" ]; then
  echo "registry-gate: the program nova build wrote printed '$built'" >&2
  exit 1
fi
"$nova" test | tee test.txt
if ! grep -q '1 passed; 0 failed' test.txt; then
  echo "registry-gate: nova test did not pass the app's test" >&2
  exit 1
fi
(cd ../geom && "$nova" test) | tee ../geom-test.txt
if ! grep -q '2 passed; 0 failed' ../geom-test.txt; then
  echo "registry-gate: nova test did not pass geom's two tests" >&2
  exit 1
fi

# 5. With the index gone, nova.lock and the cache are enough.
mv "$work/index" "$work/index-gone"
rm -rf target
"$nova" build
built=$(./target/debug/app)
if [ "$built" != "$expected" ]; then
  echo "registry-gate: without the index, the program printed '$built'" >&2
  exit 1
fi
echo "registry-gate: passed"
