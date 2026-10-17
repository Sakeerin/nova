# ADR 0042 — `std/ui`: reactivity and views

## Status

Accepted, 2026-10-17 (Phase 4.3, branch
`claude/loving-archimedes-frtf5q-43`; spec
`docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`).
Builders taking two arrays, effects run inside `set`, the runtime's
implicit context, `import ui` and `import ui::html`, an unbounded
`Signal<T>` whose `set` always notifies, `for_each` rebuilding its whole
list, the bundler's pieces moving to 4.4, and `mount(selector, f)` with
`render_to_string` are the user's decisions of 2026-10-16.

## Context

After 4.2 a WASM build could build a page and answer clicks through
`std/dom`, one call at a time. `nova-spec/30-FRONTEND.md` §2 to §4 wants
signals, effects and memos, and builder calls that keep a page up to date,
but the language 4.3 starts from has:
- no trait objects, so a signal's readers must be one record type;
- no mutable globals, so the current computation must live somewhere the
  runtime keeps;
- no way to compare closures, so a reader is found again by an id;
- one std source per module, the same on every target;
- and `import` paths of one segment.

## Decision

1. **The runtime keeps `std/ui`'s state.** Three std-only builtins,
   `ui_state_present`, `ui_state_load<T>` and `ui_state_store<T>`, keep one
   record per thread, or per page on WASM:
   - natively in a `thread_local` of `nova-runtime`, a collector root from
     its first store;
   - on WASM in `std/rt`'s state word 104, which `gc_collect` marks.
   The current listener and owner, the queue of effects, and the roots
   `mount` keeps all hang from it, so neither the program nor `std/ui`'s
   API passes a context around.
2. **Propagation is synchronous and glitch-free, and memos are lazy.**
   - `set` marks what reads the signal dirty and what lies downstream of a
     memo "check", then runs the queue of effects before it returns, unless
     inside `batch` or another flush.
   - An effect that is only "check" first brings the memos it read up to
     date, and runs only if one of them ran, so a diamond runs once.
   - Before a queued effect runs, its owners that are stale run first, from
     the top down, so a child its parent's run disposes never runs, whatever
     order a batch's `set`s queued them in.
   - A memo computes when it is made, as a signal needs a value, then again
     only when something reads it after an input changed.
   - `set` notifies every time, since most types have no `==`; a flush that
     would run effects more than 100,000 times panics.
   - Every computation, root and render is an `Owner`. Before it runs again,
     or when it is disposed, it drops what it read, so that nothing its
     cleanups set runs it again; then disposes of what it made, and runs its
     cleanups, each in reverse order. A computation is marked clean before
     that, and a disposed one is marked disposed, so a cleanup that reads it
     gets its last value; a disposed owner never runs again, and what is
     made under one is disposed at once. `root`'s dispose function, an
     effect's run, a read that computes a memo and `render_to_string` are
     each a batch.
   - A list `std/ui` empties is replaced by a new one, never cleared, since
     `Vec::clear` and `pop` keep their storage, and what it held with it.
3. **Views are data.** `View` and `Attr` are sum types the builders make.
   `render_to_string` walks a view into escaped HTML on any target; `mount`
   walks it into DOM nodes on WASM, with an effect for each part that
   changes. `show` and `for_each` are one primitive, a `Region` between two
   empty text nodes, rebuilt whole each time its signal is set; each render
   has its own owner, so a rebuild disposes of the last one's effects,
   listeners and `on_mount` callbacks not yet run; a callback runs with
   that owner, which owns what it makes.
   - Both renderers check every tag, attribute and property name before
     they write any, and refuse `innerHTML`, `outerHTML` and `srcdoc`,
     which parse their value as HTML, as attributes or properties and in
     any case: `std/ui` inserts no raw HTML.
   - `render_to_string` escapes every text and attribute value; `mount`
     hands values to the DOM as text and attribute values, never as markup.
   - What is code is the program's to place: an attribute's value is
     written as given, so what an `onclick` or a `javascript:` URL does is
     the program's, as is the text of a `script` element it makes.
4. **`std/ui`'s backend has two sources.** `$std.ui.backend` is
   `std/ui/backend/wasm.nova` on WASM, calling `std/dom`, and
   `std/ui/backend/native.nova` natively, where each function panics with
   `mount`'s message. Like `std/rt`, it is internal: only std's modules see
   its names. It is the first std module whose text differs by target. A
   region finds its nodes by walking from its first marker to its second,
   so a region inside another is removed with it; a listener's removal is a
   cleanup of the owner that built it.
5. **std's modules may have nested import paths.** `import ui::html` and
   `import ui::html::{div}` name `$std.ui.html` through `IMPORTED_STD`; a
   user's nested path is still E0900, and `import ui::nope` is E0001.
6. **The departures from `30-FRONTEND.md`:**
   - `Signal<T>` has no `Clone` bound: `value()` returns the signal's own
     value;
   - `memo` returns a `Signal<T>`, whose `set` panics;
   - `root`, `batch` and `Signal::update` are added, and `root` returns
     what its function returns, which is how a program keeps the dispose
     function, since a closure captures by value;
   - `mount(selector, f)`, not `mount(App)`;
   - builders take positional arrays until 4.4's `view { … }`;
   - reactive text is its own builder, `dyn_text`, as there is no
     overloading, and a reactive property has its own variant,
     `DynProperty`;
   - handlers get the target's `value` as a `String`, not an event;
   - `show` takes a `Signal<Bool>` and has no type parameter;
   - `Signal<T>`'s fields are reachable, as Nova has no private fields:
     writing `val` changes the value without telling anyone, and `src`
     reaches `std/ui`'s own records, which a program must leave alone.

## Consequences

- **A page's whole graph is reachable from one root**, word 104 on WASM,
  which `gc_collect` marks; 4.2's closure table needs nothing new: a
  disposed owner is dropped by its parent, its listeners removed by its
  cleanups, and the collector frees the rest and releases their handles.
- **A new renderer, such as 4.6's hydration, walks the same `View`** and
  needs a backend function only where it touches nodes.
- **`std/ui` lands in every page that uses it.** The builder counter is
  16.5 KB of WASM and 4.4 KB of `app.js` by `gzip -9`, where 4.2's counter
  is 10.1 KB and 4.1 KB; 4.4's gate measures `06-counter-spa` against
  50 KB.
- **A keyed `for_each`, `untrack`, context, resources and error
  boundaries** are not in 4.3; each can be added as new functions without
  changing these.

## References

- `docs/phase-4-plan.md` §3, decisions 3, 5, 6, 7, 10 and 11, and §4's 4.3
  entry.
- The spec named above.
- ADR 0038, the recorded scope; ADR 0040, the collector and the executor;
  ADR 0041, `std/dom` and the boundary with JS.
- `nova-spec/30-FRONTEND.md` §2 to §4 and §13; `nova-spec/13-RUNTIME.md`
  §8; `nova-spec/20-STDLIB.md`.
