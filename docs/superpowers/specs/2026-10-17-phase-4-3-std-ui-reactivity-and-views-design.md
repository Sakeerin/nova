# Phase 4.3, "`std/ui`: reactivity and element builders": design

- **Date:** 2026-10-17.
- **Status:** for the user's review.
- **The user's decisions, 2026-10-16:**
  1. Builders take their attributes and their children as two arrays:
     `div([class("counter")], [text("+")])`.
  2. `set()` runs the effects that depend on the signal before it returns,
     as SolidJS does. Memos recompute when they are read, and nothing sees a
     half-updated graph.
  3. The current effect and owner are the runtime's, implicitly. std-only
     builtins hold them: natively a `thread_local` of `nova-runtime`, a
     collector root; on WASM `std/rt`'s state word 104, which the collector
     marks.
  4. A program writes `import ui` for reactivity and `import ui::html` for
     the builders. Only std's modules get nested imports.
  5. `Signal<T>` has no bound on `T`, and `set()` notifies every time.
  6. `for_each` re-renders its whole list when the list changes.
  7. The bundler's pieces that `30-FRONTEND.md` §7.1 placed in 4.3 move to
     4.4: a user's own `index.html`, `assets/` from `public/`,
     `manifest.json`, and `wasm-opt`.
  8. `mount(selector, f)` puts a view into the page on WASM.
     `render_to_string(f)` renders one to HTML, on both targets.
- **Branch:** `claude/loving-archimedes-frtf5q`.
- **Inputs:**
  - `docs/phase-4-plan.md` §3 (decisions 3, 5, 6, 7, 10 and 11) and §4's
    4.3 entry;
  - `nova-spec/30-FRONTEND.md` §2 to §4, §7.1 and §13;
  - the 4.2 spec,
    `docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md`;
  - ADR 0038 (the recorded scope), ADR 0040 (the collector and the
    executor), ADR 0041 (`std/dom` and the boundary with JS).

## 1. What 4.3 delivers

After 4.3, a program builds its page from signals and builder calls. The
page updates itself when a signal changes. The same view also renders to an
HTML string, natively and on WASM, which is how the unit tests check it.

1. **Reactivity** (§4, §5):
   - `signal`, `effect`, `memo`, `on_mount` and `on_cleanup`, as
     `30-FRONTEND.md` §2 writes them;
   - and three small additions: `root`, `batch` and `Signal::update`.
   - An owner tree disposes of effects and runs their cleanups.
2. **Views** (§6): a `View` value, `std/ui/html`'s builders and attribute
   helpers, and `show` and `for_each`.
3. **Two renderers** (§7):
   - `mount(selector, f)` builds DOM nodes on WASM and keeps them up to
     date;
   - `render_to_string(f)` writes HTML on either target, escaping all text.
4. **`import ui` and `import ui::html`** (§3), on both targets.
5. **ADR 0042** records the runtime's context, views as data, and the
   per-target backend (§11).

**The gate:**
- `tests/dom/ui_counter.nova` is a counter written with builder calls. It
  works under the fake DOM in `cargo test` on all three systems, and in
  Chromium in the `browser` job.
- `30-FRONTEND.md` §13's unit tests pass on both targets: signal
  reactivity, effect cleanup, and memo memoization. They are programs in
  `tests/runtime/` that `run_tests.rs` runs natively and `wasm_tests.rs`'s
  gate runs on WASM.

## 2. The starting point (read on 2026-10-16, at `c6d02f8`)

- **Generics work** for records, functions and methods, with trait bounds
  (`Mutex<T>` in `std/sync`, `Map<K: Hash + Eq, V>`). Each generic function
  is instantiated per type list from `main` (`nova-mir`'s `mono.rs`).
  - A closure is lifted with its enclosing function's generics, so a
    closure that captures a `T` inside a generic function should work.
  - No test or `.nova` file covers that case yet, or a `fn() -> T`
    parameter.
- **`Clone`** is implemented for `Int`, `Float`, `Bool`, `Char` and
  `String` only. Records are shared, never copied.
- **There are no trait objects.** A list must hold one type, so a signal's
  subscribers must be one record type, not a mix.
- **Closures** (`fn()`, `fn(T) -> U`) capture by value, but a captured
  record is shared, so its fields can be mutated. Closures can be stored in
  records and arrays. They cannot be compared: `==` covers only `Int`,
  `Float`, `Bool`, `Char`, `String` and unit.
  - `| |` is the empty-parameter closure, since `||` lexes as one token;
    4.4 adds `||`.
- **There are no mutable globals.** A `const` is a function evaluated at
  each use.
  - Native state lives in `nova-runtime`'s `thread_local!`s. A heap pointer
    kept there must be a collector root (`gc::add_root`).
  - On WASM, `std/rt` keeps state words at fixed addresses below 128.
    4.2 used 88 and 96, so **104, 112 and 120 are free**.
  - The collector's roots are named in `gc_collect`.
- **Collections:**
  - `[T]` is fixed-length: index, `len`, `[x; n]`.
  - `Vec<T>` has `push`, `pop`, `get`, `set`, `clear` and `iter`, but no
    remove at an index.
  - `Map<K, V>` and `Set<T>` exist.
- **Sum types** (`type T = | A(X) | B`) may be recursive through an array,
  as `std/json`'s `JsonValue` is.
- **Imports:**
  - `import dom` and `import dom::{Element}` work, through 4.2's
    `IMPORTED_STD`.
  - A path of two or more segments (`import ui::html`) parses, but the
    resolver refuses it with E0900, "qualified or nested import paths
    (`a::b`) are not supported yet".
  - `ui` is reserved (E0093; `nova-pm` refuses it).
- **No std module has a different source per target.** Today's per-target
  differences lie below the source: a builtin's `RtFunc` is a
  `nova-runtime` function natively and a `std/rt` function or a JS import
  on WASM.
- **Calls:**
  - There are no named arguments, no trailing blocks, no default arguments,
    no variadics and no overloading: a second binding of a name is E0002.
  - Module-qualified calls (`ui::element(...)`) are E0900, so
    `30-FRONTEND.md` §3.1's `ui.element` must be a bare imported name.
- **`type` and `for` are keywords,** so neither can name an attribute
  helper.
- **`std/dom`** can do everything a renderer needs:
  - create elements and text nodes;
  - set attributes and properties;
  - append, insert before and remove nodes;
  - listen for events, and read an event target's `value` through
    `property("value")`;
  - find an element with `query_selector`.
- **`nova test` runs natively only;** `nova run` has no `--target`. So a
  test "on both targets" is a golden program in `tests/runtime/`:
  - `run_tests.rs` runs it natively;
  - `wasm_tests.rs`'s gate list runs it on WASM under Node.

## 3. Modules and imports

- **`std/ui`** (`$std.ui`) and **`std/ui/html`** (`$std.ui.html`) are in
  both targets' std lists, and neither is in the prelude.
  - `import ui` binds every public name of `std/ui`, and
    `import ui::{signal, effect}` binds some.
  - `import ui::html` and `import ui::html::{div, text}` do the same for
    the builders.
- **Nested paths, for std only.** `IMPORTED_STD` gains `ui` and `ui::html`:

  ```
  ("ui", "$std.ui"), ("ui::html", "$std.ui.html")
  ```

  The resolver looks up a path of two or more segments there before its
  E0900 check, and the driver no longer skips such a path when its first
  segment is a std module's name.
  - A user's nested path is still E0900.
  - `import ui::nope` is E0001, "cannot find module `ui::nope`".
- **`std/ui/html` uses `std/ui`'s types.** It imports `ui` as a program
  would. A std module importing std is new, and the resolver's fallback
  already allows it.
- **Both modules are documented**: they join `std/lib.rs`'s `ALL`, so
  `nova doc --std` lists them.

## 4. The runtime's context

`signal`, `effect` and `memo` need to know which computation is running and
which owner is current, with no argument naming either (decision 3).

- **One record holds it all.** `UiState` is private to `std/ui`. It holds
  the current listener, the current owner, the queue of effects to run, the
  batch depth, and the roots that `mount` keeps.
- **Three std-only builtins** keep a pointer to it:

  ```
  ui_state_present() -> Bool
  ui_state_load<T>() -> T
  ui_state_store<T>(state: T)
  ```

  `std/ui` reaches its state through one function:

  ```nova
  fn ui() -> UiState {
      if !ui_state_present() { ui_state_store(UiState::new()) }
      ui_state_load()
  }
  ```

  The generic builtins take their type from the caller, as `TaskOutput`
  does. Only std may name them.
- **Natively** they are `nova-runtime` functions over a `thread_local` cell.
  The first store makes the record a collector root (`gc::add_root`), and
  it stays one: one UI state per thread, for the thread's life.
- **On WASM** they are `std/rt` functions over state word 104.
  `gc_collect` marks the word's record, so everything reachable from it is
  kept: the mounted roots, and the graph they own.
- `UiState` is the same record on both targets. Only the cell holding it
  differs.

## 5. Reactivity

### 5.1 The API (`std/ui`)

```nova
pub record Signal<T> { /* private fields; none is named `value` */ }
impl<T> Signal<T> {
    pub fn value(self) -> T           // tracked read
    pub fn set(self, new_value: T)    // a memo's: panics
    pub fn update(self, f: fn(T) -> T)
}
pub fn signal<T>(initial: T) -> Signal<T>
pub fn effect(f: fn())
pub fn memo<T>(f: fn() -> T) -> Signal<T>
pub fn on_mount(f: fn())
pub fn on_cleanup(f: fn())
pub fn root(f: fn(fn()))   // runs f with a new root owner; f's argument disposes it
pub fn batch(f: fn())      // effects run once f returns
```

- **`Signal<T>` has no bound** (decision 5).
  - `value()` returns the stored value itself. A record read from a signal
    is the signal's own, so mutating it changes the signal without telling
    anyone; `set()` or `update()` is how to tell.
  - `set()` notifies every time, since most types have no `==`.
- **`memo` returns a `Signal<T>`,** so it can go anywhere a signal can, such
  as `show`'s condition. Calling `set` on it panics: `std/ui: set() on a
  memo`.
- **`value()` is a method.** 4.4 decides whether `count.value` (no
  parentheses) becomes sugar for it (decision 7 of the phase plan).

### 5.2 Semantics

1. **Tracking.** A computation is an effect or a memo, and it runs with
   itself as the current listener. Each `value()` it calls subscribes it to
   that signal. Before each run, it drops the subscriptions of its last run.
2. **Ownership.**
   - A computation, a root, a `mount`, or one of `show`'s or `for_each`'s
     renders is an owner.
   - `effect` and `memo` register with the current owner, and
     `on_cleanup` adds a cleanup to it.
   - Re-running a computation first disposes the children of its last run
     and runs its cleanups. Disposing an owner disposes its children, then
     runs its own cleanups, each in reverse order of registration.
   - A disposed computation never runs again, and its signals drop it.
3. **`set()`** (decision 2):
   - It stores the value, then marks each subscriber dirty and each
     computation downstream of them as needing a check.
   - The marked effects go on the queue. Unless a `batch` or another flush
     is in progress, `set` runs the queue before it returns.
   - A queued effect that is only "check" first brings its memos up to
     date, and re-runs only if one of them re-ran.
   - So each computation runs at most once for one `set`, and only after
     everything it reads is current. A diamond (`a` feeding memos `b` and
     `c`, both read by `d`) runs `d` once.
4. **Memos are lazy.** A memo re-runs when something reads it after an
   input changed, not when the input changes. Since `set()` notifies every
   time, a memo that re-ran counts as changed.
5. **Effects run at once when created,** in their owner. Signals they set
   while running queue other effects, which run in the same flush.
6. **A loop is an error.** A flush that runs effects 100,000 times panics:
   `std/ui: effects did not settle`.
7. **`on_mount(f)`** runs `f` once, untracked:
   - after the nearest `mount` (or `show`/`for_each` render) has put its
     nodes in the page, in registration order;
   - at once when no mount or render is building;
   - never under `render_to_string`.
8. **Event handlers** run untracked, inside a `batch`, so a handler that
   sets three signals runs each effect once.

## 6. Views

### 6.1 `View` and `Attr` (`std/ui`)

A view is data: a tree the builders make and a renderer walks. Both
renderers share it, and 4.6's hydration will walk the same tree.

```nova
pub type View =
    | Element(String, [Attr], [View])  // tag, attributes, children
    | Text(String)
    | DynText(fn() -> String)          // re-read when its signals change
    | Region(fn() -> [View])           // show's and for_each's dynamic part

pub type Attr =
    | Static(String, String)           // name, value
    | Dynamic(String, fn() -> String)
    | Property(String, String)         // input's value, and the like
    | On(String, fn(String))           // event kind; gets the target's value
```

- A `Region`'s closure runs as a computation. Each run returns the views to
  show now. The renderer disposes the last run's owner and nodes, and
  builds the new ones in a fresh owner.
- Names are checked when a renderer meets them. A tag or attribute name
  must be ASCII letters, digits and `-`, starting with a letter; anything
  else panics: `std/ui: "<name>" is not a tag name`. Text and attribute
  values are always escaped (§7.2).

### 6.2 Builders (`std/ui/html`)

Decision 1's shape, which 4.4's `view { … }` will desugar to:

```nova
pub fn element(tag: String, attrs: [Attr], children: [View]) -> View
pub fn div(attrs: [Attr], children: [View]) -> View
pub fn button(attrs: [Attr], children: [View]) -> View
pub fn input(attrs: [Attr]) -> View          // void elements take no children
pub fn text(content: String) -> View
pub fn dyn_text(content: fn() -> String) -> View
```

- **The elements:** `div`, `span`, `p`, `a`, `button`, `label`, `form`,
  `ul`, `ol`, `li`, `h1` to `h6`, `section`, `header`, `footer`, `nav`,
  `article`, `strong`, `em`, `code`, `pre`, `small`, `table`, `thead`,
  `tbody`, `tr`, `th`, `td`, `select`, `option` and `textarea`; and the void
  ones, `input`, `img`, `br` and `hr`.
  - `main` is left out: a glob import would clash with a program's
    `fn main`. `element("main", …)` writes one.
- **The attribute helpers:**
  - `attr(name, value)` and `dyn_attr(name, f)`;
  - `class`, `id`, `style`, `href`, `src`, `alt`, `name` and `placeholder`;
  - `value(v)` and `dyn_value(f)`, which set the property;
  - `on(kind, f: fn())`, `on_click(f: fn())`, and
    `on_input(f: fn(String))`, which gets the input's value.
  - `type` and `for` are keywords, so `attr("type", "checkbox")` writes
    those.
- **Reactive text is its own builder,** `dyn_text`, since there is no
  overloading.
- A glob `import ui::html` binds about 60 names. A program that defines one
  of them (`text`, `input`, `label`) gets E0002, and should import a list
  instead. The functions' docs say so.

### 6.3 `show` and `for_each` (`std/ui`)

```nova
pub fn show(when: Signal<Bool>, then: fn() -> View) -> View
pub fn for_each<T>(items: Signal<[T]>, render: fn(T) -> View) -> View
```

- Each is a `Region`:
  - `show` returns `then()`'s view while `when` holds, and nothing
    otherwise;
  - `for_each` maps `render` over the list.
- When its signal changes, the whole region is rebuilt (decision 6): its
  rows' owners are disposed, their nodes removed, and new ones built. A
  keyed version can come later as a new function, leaving this one as it
  is.

## 7. The renderers

### 7.1 `mount(selector, f)`, on WASM

- **Mounting:**
  1. `mount` makes a root owner, which `UiState` keeps, then calls `f`
     inside it.
  2. It builds DOM nodes from the view and appends them to the element the
     selector finds.
  3. It runs the `on_mount` callbacks.
- **No match** is a panic: `std/ui: mount: no element matches "#app"`.
- **The parts:**
  - an `Element` becomes an element;
  - a `Text` becomes a text node;
  - a `DynText` becomes a text node with an effect that sets its text;
  - a `Dynamic` attribute gets an effect that sets it;
  - an `On` gets a listener that runs the handler (§5.2 item 8);
  - a `Region` sits between two empty text nodes, which mark where its
    nodes go.
- **Natively, `mount` panics:** `std/ui: mount needs a build for WASM;
  render_to_string renders on any target`.
- **The DOM work is in a per-target backend** (§7.3), so `std/ui` stays one
  source.

### 7.2 `render_to_string(f) -> String`, on both targets

- `render_to_string` calls `f` in a root owner and writes the view's HTML.
  It reads each `DynText`, `Dynamic` attribute and `Region` once, drops the
  event handlers, then disposes the root, which runs its cleanups.
- **Escaping:**
  - text escapes `&`, `<` and `>`;
  - attribute values escape those three and `"`.
  - There is no way to insert raw HTML (phase plan decision 11).
- Void elements are written without a closing tag. A `Property` is written
  as the attribute of the same name, so `value("x")` gives `value="x"`.
- It is pure Nova, so it behaves the same on both targets. This is what the
  unit tests use (§9), and it is the start of 4.6's server rendering.

### 7.3 The backend: one name, two sources

- **`std/ui` builds DOM nodes through `$std.ui.backend`,** a std-internal
  module. Only std's modules see it, as only they see `std/rt`, so
  `STD_INTERNAL` becomes a list. It has the same names on both targets:

  ```
  record Node { … }
  fn backend_query(selector: String) -> Option<Node>
  fn backend_element(tag: String) -> Node
  fn backend_text(content: String) -> Node
  fn backend_set_text(node: Node, content: String)
  fn backend_set_attr(node: Node, name: String, value: String)
  fn backend_set_prop(node: Node, name: String, value: String)
  fn backend_append(parent: Node, child: Node)
  fn backend_insert_before(parent: Node, child: Node, before: Node)
  fn backend_remove(node: Node)
  fn backend_parent(node: Node) -> Option<Node>
  fn backend_listen(node: Node, kind: String, f: fn(String))
  ```

- **Two sources, one module name.** The two texts are
  `std/ui/backend/wasm.nova` and `std/ui/backend/native.nova`.
  - **WASM:** `Node` wraps a `dom::Element`, and each function calls
    `std/dom`, which this module imports.
  - **Native:** `Node` is empty, and each function panics with `mount`'s
    message. Only `mount` reaches them.
  - `std_source()` looks a module's text up per target, so a diagnostic in
    the backend shows the target's own text.
- **This is how `std/ui` selects its backend** (phase plan decision 5).
  It is the first std module with a different source per target. ADR 0042
  records why: one view type, and a renderer that a target lacks failing
  in one place.

## 8. The collector

- **WASM:** state word 104 holds `UiState`, and `gc_collect` marks it.
  - Everything a mounted page uses is reachable from there, or from 4.2's
    closure table: the roots `mount` keeps, their owners, effects and
    signals, and the handlers' closures.
  - An owner that is disposed is dropped from its parent, so the collector
    frees it, with its effects, its nodes' handles and its closures. Its
    listeners are removed first.
- **Native:** `UiState` is a collector root from its first store.
- **No new rule for when the collector runs.** On WASM it still runs at the
  end of a turn, and a call from JS (a click) is a turn. A `set` inside a
  handler finishes its effects before the turn ends.

## 9. Testing

### 9.1 Unchanged rules

- Tests never touch the internet.
- A test that needs `$NOVA_HOME` sets it to a fresh directory, and test
  directories have fixed names.
- Under `NOVA_REQUIRE_NODE=1`, as in CI, a test that needs Node fails
  without it.

### 9.2 On both targets: `tests/runtime/ui_*.nova`

Each program prints what it checks, has a `.stdout`, and imports `ui`
without touching the DOM:
- `run_tests.rs` runs it natively;
- `wasm_tests.rs`'s gate list runs it on WASM;
- the gate's stress module runs it under `NOVA_GC_STRESS`.

- **`ui_signal`** (signal reactivity):
  - an effect re-runs on `set` and `update`;
  - it drops the subscriptions of a branch it no longer reads;
  - `batch` runs it once;
  - a diamond runs once per `set`.
- **`ui_cleanup`** (effect cleanup):
  - `on_cleanup` runs before an effect re-runs and when its owner is
    disposed, in reverse order;
  - a disposed effect never runs again;
  - `root`'s dispose function.
- **`ui_memo`** (memo memoization):
  - a memo computes once however many readers it has, and only when it is
    read after a change;
  - `set` on a memo panics (a separate program, exit 134).
- **`ui_render`:** `render_to_string` of nested elements, static and
  dynamic attributes, `show` and `for_each` both ways, escaping, void
  elements, and a bad tag name's panic.
- **`ui_loop`:** an effect that sets the signal it reads panics with
  `std/ui: effects did not settle`.

### 9.3 Under the fake DOM: `tests/dom/ui_*.nova`

Each runs as is and under stress, as 4.2's do.

- **`ui_counter`** (the gate): `dyn_text`, `on_click`, and three clicks
  make the count read 3.
- **`ui_show_for_each`:**
  - `show` toggles a branch, and its cleanups run when it hides;
  - `for_each` rebuilds its rows when the list changes, and only between
    its markers;
  - under stress, the old rows' handles are released.
- **`ui_attrs`:** a dynamic class, a `value` property, and `on_input`
  receiving the typed value. The harness sets `value` and dispatches
  `input`.
- **`ui_mount`:**
  - `on_mount` runs after the nodes are in the page;
  - mounting into a missing selector panics;
  - natively `mount` panics with its message (a `tests/runtime` program).

### 9.4 In Chromium

The `browser` job also builds `ui_counter` and runs
`tests/browser/counter.mjs` against it. The builder counter uses the same
ids as 4.2's counter (`#add`, `#count`), so one script checks both.

### 9.5 Checks

- `import ui::html::{div}` checks on both targets.
- A user's `import a::b` is still E0900.
- `import ui::nope` is E0001.
- `signal(0)` without an import is E0001.
- `std/ui`'s builtins are std-only.
- The state builtins' lowering has `lower_tests.rs` cases, and the
  resolver's new paths have unit tests.

## 10. Crates and files

| Where | Change |
|---|---|
| `nova-resolver` | `$std.ui`, `$std.ui.html`, `$std.ui.backend` per target; nested `IMPORTED_STD` paths; `STD_INTERNAL` a list; three std-only builtins |
| `nova-driver` | nested std imports not skipped; `std_source()` per target |
| `nova-typeck` | the builtins' signatures |
| `nova-mir` | their `RtFunc`s: `nova-runtime` natively, `std/rt` on WASM |
| `nova-runtime` | the UI state's `thread_local` cell and its root |
| `nova-std` | embeds the four new files |
| `std/ui/lib.nova` | new: reactivity, `View`, `Attr`, `show`, `for_each`, `mount`, `render_to_string` |
| `std/ui/html/lib.nova` | new: the builders and attribute helpers |
| `std/ui/backend/{wasm,native}.nova` | new: the DOM backend, and its native stub |
| `std/rt/lib.nova` | state word 104, its functions, and its mark in `gc_collect` |
| `tests/runtime/ui_*`, `tests/dom/ui_*` | the programs, harnesses and outputs |
| `crates/nova-cli/tests/{run_tests,wasm_tests}.rs` | the tests of §9 |
| `.github/workflows/ci.yml` | the `browser` job builds and clicks `ui_counter` too |

**No new crate, and no change to `Cargo.lock`.**

## 11. Records

- **ADR 0042, `std/ui`: reactivity and views:**
  - the runtime's context in builtins, and why not an argument;
  - synchronous, glitch-free propagation with lazy memos;
  - views as data, walked by two renderers;
  - the per-target backend module, the first std module with two sources;
  - nested imports for std;
  - and the departures from `30-FRONTEND.md`:
    - no `Clone` bound;
    - `memo` panics on `set`;
    - `root`, `batch` and `update` added;
    - `mount(selector, f)`;
    - builders with positional arrays until 4.4;
    - `dyn_text`.
- **Dated notes:**
  - `30-FRONTEND.md` §2 and §4 (the API as built), §7.1 (the bundler's
    pieces move to 4.4), §13 (the tests on both targets);
  - `13-RUNTIME.md` §8 (state word 104, and the root);
  - `20-STDLIB.md` (`std/ui`, `std/ui/html`);
  - `docs/phase-4-plan.md`'s 4.3 and 4.4 entries.
- **`CHANGELOG.md`**, under `[Unreleased]`.
- **`ARCHITECTURE.md`**, the `nova-std` row.

## 12. Risks

1. **The generic and closure paths `std/ui` leans on are untested:**
   closures capturing a `T` in a generic function, `fn() -> T` parameters,
   recursive sum types holding closures, and generic builtins over a
   record. The plan's prototype builds the core first, so a gap shows
   before anything rests on it.
2. **A value only the graph holds is freed,** on WASM through a missed
   mark, natively through a missed root. The stress runs collect after
   every turn, under the fake DOM, and on the gate's programs.
3. **Propagation runs a computation twice, or with stale inputs.** The
   diamond and branch tests check counts, not just final values.
4. **An effect loop hangs a page.** The settle limit panics instead.
5. **Glob imports of `ui::html` clash with programs' names.** E0002 says
   which, and the docs recommend listed imports.
6. **Size:** `std/ui` lands in every page that uses it. 4.4's gate measures
   06 against 50 KB; 4.3 records the builder counter's size so that
   measure starts known.

## 13. Not in 4.3

- Component syntax, `view { … }`, named arguments, trailing blocks and `||`
  (4.4), and `examples/06-counter-spa` (4.4).
- The bundler's pieces moved to 4.4 (decision 7).
- A keyed `for_each`; `untrack` and `peek`; context (provide and inject);
  resources and async signals; error boundaries.
- A view's access to its own DOM element (refs), event objects beyond the
  target's value, `classList` and style objects.
- The router (4.5), server rendering of whole apps, and hydration (4.6).

## 14. Decisions made while writing this spec

1. **Views are data,** a sum type that both renderers walk, not DOM calls
   inside the builders. It costs an allocation per node, and buys one
   source for both targets plus a tree for 4.6's hydration.
2. **`show` and `for_each` are one primitive,** `Region(fn() -> [View])`,
   since both rebuild what they hold.
3. **`root`, `batch` and `Signal::update` are added** to §2's list:
   - `root` is how a program outside `mount` disposes of what it made;
   - `batch` is how handlers avoid running effects once per `set`;
   - §2 already lists `update`.
4. **`memo` returns a `Signal<T>`,** whose `set` panics, so a memo works
   where §4 wants a signal.
5. **`on_mount` never runs under `render_to_string`,** as a server renderer
   has nothing mounted.
6. **Handlers get the target's value as a `String`,** since `std/ui`'s API
   cannot name `std/dom`'s `Event` on both targets.
7. **Names are checked, and values escaped, by both renderers,** so a
   dynamic tag or attribute name cannot break out of its markup.
8. **The settle limit is 100,000 effect runs** per flush.
9. **The UI state lives for the thread or the page.** A program that
   mounts twice keeps both roots.
