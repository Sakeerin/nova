# ADR 0029 — The language server

## Status

Accepted, 2026-10-08 (Phase 3.2, branch `phase-3-2-lsp-core`; spec
`docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md`).

## Context

`nova-spec/40-TOOLING.md` §3.2 specifies the language server as
`tower-lsp` for the protocol and `salsa` for incremental queries, and ADR
0025 mapped `salsa` to Phase 3. When Phase 3 was planned on 2026-10-06:
- `tower-lsp`'s last release was 0.20.0, from 2023-08-11;
- its fork `tower-lsp-server` needs Rust 1.85, and the workspace's minimum
  is 1.78.

The phase plan therefore recommended `lsp-server` and a re-check engine
(`docs/phase-3-plan.md` §3, decision 7). It left the latency budget to
3.2's spec, and held `salsa` back unless the budget was missed.

The front end was built for finished programs:
- the driver stopped at the first stage that found an error;
- it printed its diagnostics;
- it read every module from disk.

## Decision

1. **The protocol is `lsp-server` 0.7.8, pinned `=0.7.8`, with `lsp-types`
   0.97.**
   - `lsp-server` is rust-analyzer's synchronous framing crate, which suits
     a checker thread.
   - Releases 0.7.9 and later are edition 2024, which needs Rust 1.85, so
     the user chose to keep the 1.78 minimum and pin. A caret requirement
     would let `cargo update` take 0.7.9.
   - Every crate the two bring is edition 2021 or earlier, and declares Rust
     1.71 or less, or nothing.
2. **The engine re-checks the whole program on every change**, on one
   checker thread, behind `nova_driver::analyze`, a seam `salsa` could
   replace. Before 3.2, `nova check` of `05-json-api` and its 6,116 lines
   of std took 108-127 ms in a release build. Std is over 95% of every
   check.
3. **The budget is 200 ms** for both the median and the maximum, of 20
   edits to their diagnostics and of 20 completions, on `05-json-api`, in a
   release build on the development host. CI asserts 2 s with the debug
   binary. Measured on 2026-10-08, on Windows 11:

   | Run | Edit to diagnostics, median / max | Completion, median / max | Binary |
   |---|---|---|---|
   | 1 | 17 / 18 ms | 13 / 14 ms | `target/release/nova.exe`, 14,534,144 bytes, built 2026-10-08 05:28 UTC |
   | 2 | 17 / 18 ms | 13 / 21 ms | the same binary |
   | 3 | 18 / 24 ms | 13 / 15 ms | the same binary |

   An edit that arrives during a check waits for it, so a burst of typing
   can take up to about two checks. Nothing cancels a check.
4. **The front end keeps going for the server only.** `analyze` with
   `keep_going` runs every stage whatever the earlier ones found, and
   returns its diagnostics. With `tests`, it checks `@test` bodies, as
   `nova test` does. `nova check`, `build`, `run` and `test` keep their
   staged output: they share `analyze`'s loader, reading through
   `DiskSources`, and print exactly what they printed before.
5. **What is at the cursor comes from recovery plus a probe.**
   - The parser keeps an unfinished `foo.`.
   - The type checker's probe, an offset in one file, records:
     - the receiver of a member access whose receiver ends before the
       offset and whose name ends after it, a name on the next line
       included;
     - its members;
     - the locals in scope.
   - Members follow declared visibility, which the checker does not
     enforce.
6. **Each file's diagnostics have one owning analysis.**
   - A project's analysis owns every module its `src/main.nova` reaches.
   - An open file the entry does not reach is checked as a module, and so
     is a loose file with no `fn main`.
   - An E0001 about an item the parser dropped mid-edit is removed.
7. **The extension runs the installed `nova`**, from `nova.server.path` or
   PATH, as one platform-neutral `.vsix`.
8. **Full text sync, and the client's file watcher**, registered
   dynamically, so no watching crate is added.

## Alternatives

- **`tower-lsp-server`.** It needs Rust 1.85, and its async runtime would
  bring Tokio into a compiler that has none.
- **Protocol types written by hand.** Nova would own a few hundred lines of
  structs, more with each 3.4 capability.
- **`salsa` now.** Re-checking meets the budget, so `salsa`'s cost buys
  nothing yet. If a later sub-phase misses the budget, adopting it is a
  step decided with the user.
- **Raising the MSRV to 1.85** for `lsp-server` 0.10. The user chose the
  pin, which keeps a policy change out of a feature.
- **A placeholder name written at the cursor**, or **a full position
  index**, instead of the probe. The first edits the user's text; the
  second makes every check pay for tables only the server reads.
- **A `nova` bundled in the extension.** It could disagree with the
  user's own `nova`.

## Consequences

- **The server's analyses can meet code the front end never saw before:**
  half-typed programs. `catch_unwind` keeps a panic from killing the
  server, and a test analyses 364 cut programs on every run.
- **The stack's crates are not current.** `lsp-types` has had no release
  since 2024-06, and `lsp-server` is pinned to 2024-12's. LSP 3.17 is
  stable, and both crates are small enough to vendor.
- **3.4 adds hover, definitions, references, rename, code actions and
  semantic tokens on the same engine.** The probe is where hover's "what
  is here" starts.

  **Amended 2026-10-10 (branch `phase-3-4a-navigation`):** 3.4 is two
  sub-phases. 3.4a adds hover, definitions, references and rename on the
  same engine. They read an index the checker records only when the
  server asks, which revisits the "full position index" set aside above
  (ADR 0032). Code actions and semantic tokens are 3.4b's.

  **Amended 2026-10-10 (branch `phase-3-4b-fixes-colour`):** 3.4b adds
  code actions and semantic tokens on the same engine, each request
  analysing afresh with the index on (ADR 0033).

## References

- `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md`
- `docs/superpowers/plans/2026-10-08-phase-3-2-lsp-core.md`
- `docs/phase-3-plan.md` §3, decision 7, and §4's 3.2 entry
- ADR 0025 (`salsa` mapped to Phase 3), ADR 0026 (Phase 3's scope), ADR
  0028 (the formatter's library entry points)
