# Changelog

All notable changes to the Nova extension for VS Code are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
The extension's version is always the version of Nova it is released
with.

## [0.4.0] - 2026-10-23

Released with Nova 0.4.0, whose language gained components, written
with `component` and `view { … }`.

### Added

- **`component` and `view` are coloured as keywords where they are
  ones:** `component` before a component's name and its `(`, the name
  coloured as a function's, and `view` before its `{`. Neither word is
  reserved, so elsewhere both stay names.
- **With `nova` 0.4.0, `nova lsp` tells them from names:** semantic
  highlighting marks `component` and `view` as keywords only where the
  parser reads them as keywords, so a local named `view` stays a local,
  even in `if view { … }`; and hover shows a component as it is
  written, `component Card(title: String)`, with its `///` docs.

Nothing else in the extension changed.

## [0.3.2] - 2026-10-14

The first version in the Marketplace and Open VSX.

### Changed

- **The display name is Nova Language Tools.** The Marketplace refused
  `Nova`, which other extensions hold as `nova` and `NOVA`. The ID stays
  `sakeerin.vscode-nova`, and the README takes the new name as its
  title; nothing else changed.

## [0.3.1] - 2026-10-13

Released with Nova 0.3.1 as the GitHub release's `.vsix` only: the
Marketplace refused its display name, `Nova`.

### Changed

- **The extension's ID is `sakeerin.vscode-nova`.** The Marketplace
  refused the name `nova`, which another extension has. The display
  name stays Nova, and nothing else changed. If you installed `0.3.0`
  from its `.vsix`, uninstall it first
  (`code --uninstall-extension sakeerin.nova`): VS Code counts the two
  IDs as two extensions that claim the same settings and command, so
  whichever starts second fails, and the one left running may be the old
  one.

## [0.3.0] - 2026-10-12

Released with Nova 0.3.0 as the GitHub release's `.vsix` only, as
`sakeerin.nova`: the Marketplace refused its name.

### Added

- **The Nova language.** `.nova` files get the language `nova`: a
  grammar that colours comments, strings, numbers, attributes, keywords,
  function names and types, and comment toggling, bracket matching,
  auto-closing pairs and indentation.
- **A client for `nova lsp`.** The extension runs the `nova` you have
  installed, as its language server, and gives VS Code what it answers:
  - **diagnostics,** for the whole project on every edit, unsaved
    buffers included, and for `nova.toml`;
  - **completion:** after `.`, a value's fields and methods, std's
    included; elsewhere, the locals, the names in scope, the primitive
    types and the keywords;
  - **formatting,** as `nova fmt` formats, following `.editorconfig`;
  - **hover:** a name's declaration, a local's inferred type, and its
    `///` docs;
  - **go to definition,** into other modules, dependencies and std;
  - **find references** and **rename**, which refuses, saying why, a
    rename that would change what a name means;
  - **quick fixes:** make a local mutable, import a name, "did you mean",
    make an item public, remove an unreachable arm;
  - **organize imports;**
  - **semantic highlighting,** every name coloured by what it means. A
    local or parameter declared `mut` carries the `mutable` modifier,
    which a theme can style.
- **Settings.** `nova.server.path` names the `nova` to run; empty, the
  default, means `nova` from your PATH. `nova.trace.server` (`off`,
  `messages`, `verbose`) traces the messages between VS Code and
  `nova lsp`.
- **Nova: Restart Language Server,** a command that restarts it, for
  example after installing a newer `nova`.
- If `nova` cannot be started, an error names the setting and how to
  install `nova`.

The extension needs VS Code 1.91 or later and an installed `nova`.
