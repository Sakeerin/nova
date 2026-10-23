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
Visual Studio Build Tools on Windows. A build for WebAssembly, and
`nova dev`, need no linker.

## Start

```sh
nova new hello
cd hello
nova run
```

## Web

- `nova build --target wasm` compiles a program to WebAssembly into
  `dist/`: the page, `app.js`, `app.wasm`, `manifest.json`, a
  `package.json` for Node, and `public/` as `assets/`. It needs nothing
  else installed.
- `nova dev` builds it and serves it on `http://127.0.0.1:3000/`,
  building again and reloading the page when a source changes, and showing
  the build's errors over it; `--port` and `--open` change where and how.
- With `--release`, a build for WebAssembly also runs `wasm-opt -Oz
  --enable-bulk-memory` on `app.wasm` when binaryen's `wasm-opt` is on
  your PATH, and says so when it is not.
- `nova build --ssr` builds a project's client from `src/main.nova` and
  its server, natively, from `src/server.nova`, which renders the pages
  the client hydrates; `nova build --ssg` also writes a page for each
  route other than `*` with no parameters.

A program imports what it uses: `ui` (signals, effects, `mount`,
`hydrate`), `ui::html` (the element builders `view { … }` calls),
`ui::router` and, natively, `ui::ssr`; std's documentation site,
attached to each GitHub release, documents them. `dom`, the WASM-only
layer beneath them, is not on the site: read
[`std/dom/lib.nova`](https://github.com/Sakeerin/nova/blob/main/std/dom/lib.nova).
The repository's
[`examples/06-counter-spa`](https://github.com/Sakeerin/nova/tree/main/examples/06-counter-spa)
and
[`examples/07-fullstack-blog`](https://github.com/Sakeerin/nova/tree/main/examples/07-fullstack-blog)
say in their READMEs how to run each.

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
