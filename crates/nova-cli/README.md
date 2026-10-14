# nova

`nova` is the toolchain of Nova, a statically typed programming language
with type inference, pattern matching and async functions. One binary
makes, runs, builds and tests projects, and it is also Nova's formatter
(`nova fmt`), language server (`nova lsp`), package manager (`nova add`,
`nova publish`) and documentation generator (`nova doc`).

## Install

From crates.io:

```sh
cargo install nova-cli --locked
```

`--locked` builds with the dependency versions the release was tested
with. The install also builds Nova's runtime library and embeds it in
`nova`, so the installed binary needs nothing beside it.

Or a ready-built `nova` from the latest GitHub release, checked against
the release's `SHA256SUMS`. On Linux (x86_64) and macOS, into `~/.nova/bin`:

```sh
curl -sSfL https://github.com/Sakeerin/nova/releases/latest/download/install.sh | sh
```

On Windows, into `%USERPROFILE%\.nova\bin`, which the script adds to your
PATH:

```powershell
irm https://github.com/Sakeerin/nova/releases/latest/download/install.ps1 | iex
```

## What it needs

`nova run` needs nothing else. `nova build` and `nova test` write native
executables, so they need a C linker: `cc` on Linux and macOS, and the
Visual Studio Build Tools on Windows.

## Start

```sh
nova new hello
cd hello
nova run
```

## Links

- [The repository](https://github.com/Sakeerin/nova): the language's
  documentation, the changelog and the releases.
- The VS Code extension is **Nova Language Tools** by `sakeerin`, on the
  Marketplace and Open VSX
  (`code --install-extension sakeerin.vscode-nova`), and each release
  attaches it as `nova-vscode-<version>.vsix`.

The other `nova-*` crates on crates.io are `nova`'s own libraries. Their
API is internal and changes with every release.

Licensed under either of the MIT licence or the Apache License, Version
2.0, at your option.
