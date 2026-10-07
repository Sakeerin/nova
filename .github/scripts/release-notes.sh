#!/usr/bin/env bash
# Print CHANGELOG.md's section for one version: the lines after the heading
# that starts with `## [VERSION]`, closing bracket included so that 0.2.0
# never matches 0.2.0-alpha.4, up to the next `## [` heading. Fails when the
# section is missing or empty (spec
# docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md §7).
#
# Usage: release-notes.sh VERSION CHANGELOG
set -euo pipefail

if [ $# -ne 2 ]; then
  echo "usage: release-notes.sh VERSION CHANGELOG" >&2
  exit 2
fi
notes=$(awk -v head="## [$1]" '
  index($0, "## [") == 1 { if (found) exit; found = (index($0, head) == 1); next }
  found { print }
' "$2")
# No pipe here: under pipefail, `printf … | grep -q` fails whenever grep
# stops reading at its first match while printf still has more than a pipe
# buffer to write, so a section over 64 KiB read as "no notes".
if [ -z "${notes//[[:space:]]/}" ]; then
  echo "release-notes: $2 has no notes for $1" >&2
  exit 1
fi
printf '%s\n' "$notes"
