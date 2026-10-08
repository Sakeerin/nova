#!/usr/bin/env bash
# The Phase 3.3a gate (spec
# docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md §8):
# with the installed nova, a library and a program that each have their
# own utils.nova, the program depending on the library by path. It runs
# and prints from both, and `nova test` passes in each. CI's `install` job
# runs it after gate.sh, on all three systems.
#
# Usage: packages-gate.sh NOVA WORKDIR
#   NOVA     the nova executable to check
#   WORKDIR  where to make the packages; it must not exist yet
set -euo pipefail

if [ $# -ne 2 ]; then
  echo "usage: packages-gate.sh NOVA WORKDIR" >&2
  exit 2
fi
nova=$1
work=$2

mkdir "$work"
cd "$work"

# 1. A library whose lib.nova uses its own utils.nova. The template's
#    tests/geom_test.nova tests greeting(), which stays.
"$nova" new --lib geom
cat > geom/src/utils.nova <<'EOF'
pub fn width() -> Int {
    3
}
EOF
cat > geom/src/lib.nova <<'EOF'
import utils

pub fn greeting() -> String {
    "Hello, Nova!"
}

pub fn area() -> Int {
    width() * width()
}

@test
fn area_is_nine() {
    assert_eq(area(), 9)
}
EOF

# 2. A program with a different utils.nova.
"$nova" new app
cat > app/src/utils.nova <<'EOF'
pub fn label() -> String {
    "app utils"
}
EOF
cat > app/src/main.nova <<'EOF'
import geom
import utils

fn main() {
    println("area ${area()}")
    println(label())
}

@test
fn area_comes_from_geom() {
    assert_eq(area(), 9)
}
EOF

# 3. The dependency, added by path.
cd app
"$nova" add geom --path ../geom

# 4. Both utils modules, in one program.
ran=$("$nova" run)
expected=$'area 9\napp utils'
if [ "$ran" != "$expected" ]; then
  echo "packages-gate: nova run printed '$ran'" >&2
  exit 1
fi

# 5. The tests of each package: the app's one, and the library's own with
#    tests/geom_test.nova's.
"$nova" test | tee test.txt
if ! grep -q '1 passed; 0 failed' test.txt; then
  echo "packages-gate: nova test did not pass the app's test" >&2
  exit 1
fi
cd ../geom
"$nova" test | tee test.txt
if ! grep -q '2 passed; 0 failed' test.txt; then
  echo "packages-gate: nova test did not pass geom's two tests" >&2
  exit 1
fi
if ! grep -q 'geom_test::greeting_is_public ... ok' test.txt; then
  echo "packages-gate: tests/geom_test.nova did not run" >&2
  exit 1
fi
echo "packages-gate: passed"
