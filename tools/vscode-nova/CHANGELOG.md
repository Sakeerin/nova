# Changelog

All notable changes to the Nova extension for VS Code are documented here.

Format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).
The extension's version is always the version of Nova it is released
with.

## [0.3.0] - 2026-10-12

The first published version.

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
