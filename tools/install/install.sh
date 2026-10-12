#!/bin/sh
# Install nova, the Nova toolchain, on Linux or macOS from a GitHub
# release: this system's archive, checked against the release's
# SHA256SUMS, into $NOVA_HOME/bin (by default ~/.nova/bin). Running it
# again updates nova; deleting ~/.nova uninstalls it. Spec:
# docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md
# section 6.
#
#   curl -sSfL https://github.com/Sakeerin/nova/releases/latest/download/install.sh | sh
#
# Environment:
#   NOVA_VERSION       the version to install, such as 0.3.0
#                      (default: the latest release)
#   NOVA_HOME          nova's home; nova goes into its bin directory
#                      (default: ~/.nova)
#   NOVA_INSTALL_BASE  where the releases are
#                      (default: https://github.com/Sakeerin/nova/releases)
#
# It edits no file: when the directory is not on PATH, it prints the line
# to add. It needs curl, tar, and sha256sum or shasum. POSIX sh: it runs
# under dash. Everything happens in main, called on the last line, so a
# download cut short runs nothing.
set -eu

say() {
  printf 'nova: %s\n' "$*"
}

die() {
  printf 'nova: error: %s\n' "$*" >&2
  exit 1
}

need() {
  command -v "$1" > /dev/null 2>&1 || die "this script needs $1, which is not installed"
}

# This system's release target (spec section 6.1, step 1).
detect_target() {
  os=$(uname -s)
  arch=$(uname -m)
  case "$os $arch" in
    "Linux x86_64" | "Linux amd64") target=x86_64-unknown-linux-gnu ;;
    "Darwin arm64") target=aarch64-apple-darwin ;;
    "Darwin x86_64") target=x86_64-apple-darwin ;;
    *) die "no ready-built nova for $os $arch; install it with: cargo install nova-cli --locked" ;;
  esac
}

# NOVA_VERSION, or the latest release's version, read from where
# <base>/latest redirects: .../releases/tag/v<version> (step 2).
pick_version() {
  if [ -n "${NOVA_VERSION:-}" ]; then
    version=$NOVA_VERSION
  else
    say "finding the latest release at $base/latest"
    if ! effective=$(curl -sSfL -o "$tmp/latest" -w '%{url_effective}' "$base/latest"); then
      die "could not reach $base/latest"
    fi
    case "$effective" in
      */tag/*) ;;
      *) die "could not tell the latest version: $base/latest led to $effective" ;;
    esac
    version=${effective##*/tag/}
    version=${version%%[/?#]*}
  fi
  version=${version#v}
  case "$version" in
    "" | *[!0-9A-Za-z.+-]*) die "'$version' is not a version" ;;
  esac
}

fetch() {
  say "downloading $1"
  curl -sSfL -o "$2" "$1" || die "could not download $1"
}

sha256_of() {
  if [ "$sha_tool" = sha256sum ]; then
    sha256sum "$1"
  else
    shasum -a 256 "$1"
  fi
}

main() {
  need curl
  need tar
  if command -v sha256sum > /dev/null 2>&1; then
    sha_tool=sha256sum
  elif command -v shasum > /dev/null 2>&1; then
    sha_tool=shasum
  else
    die "this script needs sha256sum or shasum, and neither is installed"
  fi

  base=${NOVA_INSTALL_BASE:-https://github.com/Sakeerin/nova/releases}
  base=${base%/}
  nova_home=${NOVA_HOME:-}
  if [ -z "$nova_home" ]; then
    [ -n "${HOME:-}" ] || die "neither NOVA_HOME nor HOME is set"
    nova_home=$HOME/.nova
  fi
  bin=$nova_home/bin

  detect_target
  tmp_root=${TMPDIR:-/tmp}
  tmp=$(mktemp -d "${tmp_root%/}/nova-install.XXXXXX") || die "could not make a temporary directory"
  pick_version
  name=nova-$version-$target
  archive=$name.tar.gz
  say "installing nova $version for $target"

  # Step 3: the archive and SHA256SUMS, into the temporary directory.
  fetch "$base/download/v$version/$archive" "$tmp/$archive"
  fetch "$base/download/v$version/SHA256SUMS" "$tmp/SHA256SUMS"

  # Step 4: the archive's SHA-256 against its line in SHA256SUMS.
  expected=$(awk -v name="$archive" '{ sub(/\r$/, "") } $2 == name || $2 == "*" name { print tolower($1); exit }' "$tmp/SHA256SUMS")
  [ -n "$expected" ] || die "SHA256SUMS has no line for $archive, so it cannot be checked; nothing was installed"
  actual=$(sha256_of "$tmp/$archive" | awk '{ print tolower($1) }')
  [ "$actual" = "$expected" ] || die "$archive does not match its SHA-256 in SHA256SUMS (expected $expected, got $actual); nothing was installed"
  say "checked $archive against SHA256SUMS"

  mkdir "$tmp/unpacked"
  tar -xzf "$tmp/$archive" -C "$tmp/unpacked" || die "could not unpack $archive"
  [ -f "$tmp/unpacked/$name/nova" ] || die "$archive holds no $name/nova"

  # Step 5: written beside the old nova, run there (step 6), and only then
  # renamed over it, so a failed install, or a nova that cannot run on
  # this system, leaves the old one.
  mkdir -p "$bin" || die "could not make $bin"
  bin=$(CDPATH='' cd -- "$bin" && pwd)
  staged=$bin/.nova-install.$$
  cp "$tmp/unpacked/$name/nova" "$staged" || die "could not write to $bin"
  chmod +x "$staged"

  # Step 6: the new nova says what it is, before it replaces anything.
  if ! printed=$("$staged" version); then
    rm -f "$staged"
    staged=
    die "the new nova does not run on this system (\`nova version\` failed), so nothing was replaced; install it with: cargo install nova-cli --locked"
  fi
  mv -f "$staged" "$bin/nova" || die "could not replace $bin/nova"
  staged=
  say "installed $bin/nova"
  printf '%s\n' "$printed"

  # Step 7: no file is edited; the line to add, if the directory is not on
  # PATH, with $HOME as it is when the directory is under it.
  case ":$PATH:" in
    *":$bin:"* | *":$bin/:"*) ;;
    *)
      shown=$bin
      home_dir=${HOME:-}
      home_dir=${home_dir%/}
      if [ -n "$home_dir" ]; then
        case "$bin" in
          "$home_dir"/*) shown="\$HOME${bin#"$home_dir"}" ;;
        esac
      fi
      say "add nova to your PATH: export PATH=\"$shown:\$PATH\""
      ;;
  esac
}

tmp=
staged=
cleanup() {
  if [ -n "$staged" ]; then
    rm -f "$staged"
  fi
  if [ -n "$tmp" ]; then
    rm -rf "$tmp"
  fi
}
trap cleanup EXIT
trap 'exit 1' HUP INT TERM

main "$@"
