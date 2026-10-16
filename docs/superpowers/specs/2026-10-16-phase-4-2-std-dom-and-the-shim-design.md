# Phase 4.2, "`std/dom` and the shim": design

- **Date:** 2026-10-16.
- **Status:** for the user's review.
- **The user's decisions, 2026-10-16:**
  - std reaches JS through `extern "js"` declarations, which only std may
    write; user code keeps E0090;
  - the collector releases a JS object's handle once no Nova value holds
    it, and a listener's closure stays a root until the listener is
    removed;
  - `fetch` is in 4.2, as an `async fn`: the executor gains a wait that JS
    ends;
  - a program imports `std/dom` with `import dom`, from 4.2: it is not in
    the prelude, and the names `dom` and `ui` are std's.
- **Branch:** `claude/loving-archimedes-frtf5q`.
- **Inputs:**
  - `docs/phase-4-plan.md` §3 (decisions 5, 6, 12 and 13) and §4's 4.2
    entry;
  - the 4.1 spec,
    `docs/superpowers/specs/2026-10-15-phase-4-1-the-language-on-wasm-design.md`;
  - ADR 0038 (the recorded scope), ADR 0039 (the backend), ADR 0040 (the
    collector and the executor);
  - `nova-spec/30-FRONTEND.md` §5 and §9.

## 1. What 4.2 delivers

After 4.2, a program built for WASM can build a page, change it, and
answer clicks, timers and network replies. Tests drive it under Node
against a small fake DOM kept in the repository, and CI drives it in
Chromium.

1. **`extern "js"`** (§3.1). A std module declares a JS function as an
   `extern "js"` function. The backend imports it from the shim's `env`
   by name, and the shim holds its JS. Its parameters and result are
   `Int`, `Float`, `Bool` or `String`.
2. **`import dom`** (§3.2). `std/dom` is std's first module outside the
   prelude. A program imports it by name, as it imports its own modules.
3. **Handles** (§4). JS objects are held in a table in the shim, and Nova
   holds their indices in small boxes the collector knows. When the
   collector frees a box, it tells the shim to drop the object.
4. **Calls from JS** (§5). A closure that JS may call, such as an event
   listener, goes into a table in `std/rt` whose entries are roots. Each
   call from JS is a turn: tasks it wakes run, and the collector runs at
   its end.
5. **`std/dom`** (§6): elements and text, attributes and properties,
   inserting and removing, listeners, selectors, `location` and
   `history`, timers, and `fetch`.
6. **Waits that JS ends** (§7). A task can wait on a token that JS
   resolves, which is how `fetch` awaits its reply.
7. **The fake DOM** (§9) for `cargo test`, and a CI job that drives a
   program in Chromium through Playwright (§10.3).
8. **ADR 0041** records `extern "js"`, the handles, the calls from JS and
   the importable std module (§12).

**The gate:**
- `tests/dom/counter.nova`, a program that builds a button and a count
  and adds one to the count on each click, behaves as its expected output
  says under the fake DOM in `cargo test`, on all three operating
  systems;
- the same program, served from its `dist/`, responds to clicks in
  Chromium, in the new CI job.

## 2. The starting point (read on 2026-10-16, at `1bcede5`)

- **Imports from JS.** The backend imports a host function for each
  runtime function whose `on_wasm()` is `WasmRuntime::Host`: printing,
  float text, case, the hash seed, the clocks, and `abort`. The shim's
  `env` holds exactly those, and a name the shim lacks fails the build.
  Every new host function so far has meant a `Builtin`, an `RtFunc`, a
  type in the checker, and a shim entry.
- **`extern` blocks.** `extern "C"` declares C functions with `Int`,
  `Float` and `Bool` values. The checker refuses any other ABI with
  E0900, and the resolver refuses any `extern` block on WASM with E0090.
  MIR records each called extern as an `ExternDecl`.
- **Values across the boundary.** `Int` is an `i64` (a `BigInt` in JS),
  `Float` an `f64`, `Bool` an `i32`, and a `String` an `i32` address. The
  shim reads a string's text with `text(s)` and makes a string with
  `string(value)`, through `nova_alloc_leaf`.
- **Closures** are 16-byte objects: a table index, then an environment
  address. `CallIndirect` passes the environment first. A call routed to
  `std/rt` converts a pointer to the `Int` address the function takes.
- **std modules** are all glob-imported into every module, under names
  (`$std.core`, …) that no `import` can spell. `import x` binds a
  project's or a dependency's module `x`; `import x::{a, b}` binds some of
  its names. Nested paths (`a::b`) are E0900.
- **The executor** parks a task on a deadline or on another task. A turn
  that ends with tasks parked but none with a deadline reports a
  deadlock. Only a module with an `async fn main` exports `nova_turn` and
  `nova_gc_switches`. A program with a synchronous `main` never collects.
- **Record fields are not private.** Nova keeps no field visibility past
  the parser, so a program can build any std record by hand, as
  `forged_join_handle` shows.
- **Node** 22 is CI's version and this machine's (22.22.0). It has
  `EventTarget`, `Event`, `fetch` and `FinalizationRegistry`.

## 3. The language

### 3.1 `extern "js"`

```nova
extern "js" {
    fn dom_create_element(tag: String) -> Int
    fn dom_set_text(node: Int, text: String)
}
```

- **Where.** Only a std module may declare one, and only std modules
  compiled for WASM declare any. In a user's module an `extern` block is
  still E0090 on WASM. Natively, `extern "js"` is still E0900 ("only the C
  ABI is supported"), and no native std module declares one.
- **Types.** Each parameter is `Int`, `Float`, `Bool` or `String`; the
  result is one of those or none. A JS function never sees a Nova object
  other than a string; handles (§4) are `Int`s.
- **Names.** The symbol is the function's name, imported from `env`
  under it. The `nova_` prefix and `main` stay reserved, as for
  `extern "C"`. A name the shim has no JS for fails the build, as an
  unknown host function does today.
- **Lowering.** The checker records the ABI on `hir::ExternFn` and MIR on
  `ExternDecl`. The backend imports each called `extern "js"` function
  after the runtime's host functions and before `abort`, in the order of
  first use, and calls it like a host function.
- **The shim** keeps each function's JS in `shim/dom.js` and adds only
  the ones the module imports, as it does for host functions. Each one
  turns a JS exception into a Nova panic whose message names the function
  and quotes the exception: `nova: panic: std/dom: dom_append_child:
  HierarchyRequestError: …`.

This replaces ADR 0038's "compiler-known functions" for `std/dom`. User
code still has no JS interop: `@js_import` and `@js_export` stay in the
backlog.

### 3.2 `import dom`

- **`std/dom` is in WASM's std** (`$std.dom`), after `std/log` and before
  `std/rt`, but it is not glob-imported. A module that wants it writes
  `import dom` (every public name) or `import dom::{Element, create_element}`.
- **Natively,** `import dom` is E0092: "`dom` is a std module, which
  target native does not have", labelled "not on target native". Its
  names are not in the prelude on either target, so a name from it
  without the import is E0001, as for any unknown name.
- **The names `dom` and `ui` are std's.** 4.3's `std/ui` will take `ui`.
  - A project's own module named `dom` or `ui` is a new error, E0093:
    "`dom` is the name of a std module; rename this module". It names
    the file.
  - A dependency named either is refused when the manifest is read: the
    package-name check in `nova-pm` gains the two names, so `nova new dom`
    and `nova publish` refuse them too.
  - Before this lands, the plan checks the index (`Sakeerin/nova-index`)
    and the repository's own `.nova` files for either name.
- **What `import dom` binds** follows the rules for importing a
  project's module. std's other modules still see each other's names as
  before; `std/dom` itself sees theirs, as every std module does.
- **`check --target wasm`** checks a program that imports `dom`;
  `nova check` without a target checks it natively and reports E0092.

## 4. Handles

### 4.1 The shim's table

- `handles` is an array, and slot 0 is never used, so 0 can mean "none".
  A free list reuses slots. `handle(obj)` stores an object and returns its
  slot; `object(id)` reads one.
- **A slot that holds nothing is a panic:** `nova: panic: std/dom: handle
  7 is not live`. A forged `Element` (§6.1), or one used after the shim
  released it, reaches this and nothing worse.
- `handle_release(id)` empties a slot. Only the collector calls it (§4.2).

### 4.2 Boxes, and the collector

- **A box** is a 16-byte leaf object from `std/rt`: the slot's index,
  then 0. `std/dom`'s records hold the box's address in an `Int` field.
  The collector is conservative, so that word keeps the box alive while
  the record is reachable.
- **The registry** is a leaf table in `std/rt`, at a new state word (88),
  holding each live box's address and slot. It does not mark the boxes:
  it is weak.
- **After each sweep,** `std/rt` walks the registry, as `gc_prune` walks
  the task index. A box that no object starts at any more was freed:
  `std/rt` calls `handle_release` with its slot and drops the entry.
- **Under `NOVA_GC_DEBUG`,** a collection that released handles writes
  one more line: `nova-dom: collection 3 released 120 handles, 4 live`.
  The native collector has no such line; it appears only on WASM, only in
  a program that uses handles.
- **Each fetch of a node makes a new box** and a new slot. Two boxes may
  name the same node; `Element::is_same` compares the objects.

## 5. Calls from JS

### 5.1 The closure table

- `std/rt` keeps a table of closures that JS may call, at a new state
  word (96), with a free list. Each entry is a closure's address, and the
  collector marks every entry: the table is a root.
- Every closure in it has the type `fn(Int)`. `std/dom` wraps each
  handler in one, which boxes the argument and calls the handler: an
  event listener's argument is the event's slot, and a timer's is 0.
- An entry leaves the table when its listener is removed, its interval
  cleared, or its timeout fired or cleared.
- **The call** is a new WASM-only builtin, `closure_call(f, arg)`, which
  `std/rt` alone may use. It lowers to a `call_indirect` of type
  `(i32 env, i64) -> ()`, as `future_poll` lowers to one of type
  `(i32 state, i32 ctx) -> i64`.

### 5.2 Every call from JS is a turn

- **`nova_call(id, arg)`** is a new export. It calls entry `id`; an entry
  that is gone is skipped. Then it runs the queue for up to the turn
  budget, collects if due, and returns what `nova_turn` returns (§7.2).
  So a handler that spawns a task, or wakes one, sees it run.
- **A call from JS inside a call from JS** happens when a handler does
  something that dispatches an event at once, such as `Element::click`,
  or removing a focused element. A depth word in `std/rt` counts the
  calls in progress. An inner call runs its handler only. The outermost
  call runs the queue and collects. The collector never runs while a
  Nova frame is on the WASM stack.
- **Which modules export it:** a module that registers a closure, which
  is any module that reaches `std/rt`'s closure table. Such a module also
  exports `nova_turn` and `nova_gc_switches`, and the collector then runs
  in a program whose `main` is synchronous, at the end of each call from
  JS. 4.1's decision 19 still holds for a program that registers none.
- **`main` itself** is not a call from JS in this sense: a synchronous
  `main` that only builds the page returns without collecting, as today.

### 5.3 A panic in a call from JS

A panic ends the program, as it does natively. The shim writes the
message as today. It then marks the program ended:
- later events, timers and replies call nothing;
- under Node, the exit status is 134, or 132 for a trap;
- in a browser the page stays as it was, with the message in the
  console.

## 6. `std/dom`

### 6.1 Types

- `pub record Element { box: Int }`: a DOM node, an element or a text
  node.
- `pub record Event { box: Int }`: the event a listener receives.
- `pub record Listener { target: Element, kind: String, entry: Int, js: Int }`:
  an added listener, which `remove` removes.
- `pub record Timer { entry: Int, js: Int, repeats: Bool }`: a timeout or
  an interval, which `clear` stops.
- `pub record Header { pub name: String, pub value: String }`.
- `pub record FetchInit { pub method: String, pub headers: [Header], pub body: String }`,
  and `FetchInit::new()` for `GET`, with no headers and no body.
- `pub record Response { pub status: Int, pub ok: Bool, pub body: String }`.
- `pub record FetchError { pub message: String }`.

The fields without `pub` are `std/dom`'s. A program can still build such
a record by hand; using it is the panic of §4.1.

### 6.2 Functions

| Area | Nova | JS it calls |
|---|---|---|
| The document | `body() -> Element` | `document.body` |
| | `create_element(tag: String) -> Element` | `document.createElement` |
| | `create_text(text: String) -> Element` | `document.createTextNode` |
| | `query_selector(selector: String) -> Option<Element>` | `document.querySelector` |
| | `query_selector_all(selector: String) -> [Element]` | `document.querySelectorAll` |
| | `element_by_id(id: String) -> Option<Element>` | `document.getElementById` |
| An element | `tag(self) -> String` | `nodeName`, lowercased |
| | `set_attribute(self, name: String, value: String)` | `setAttribute` |
| | `attribute(self, name: String) -> Option<String>` | `getAttribute` |
| | `remove_attribute(self, name: String)` | `removeAttribute` |
| | `set_property(self, name: String, value: String)` | `el[name] = value` |
| | `property(self, name: String) -> String` | `String(el[name] ?? "")` |
| | `set_property_bool(self, name: String, value: Bool)` | `el[name] = value` |
| | `property_bool(self, name: String) -> Bool` | `Boolean(el[name])` |
| | `set_text(self, text: String)` | `textContent = text` |
| | `text(self) -> String` | `textContent` |
| | `append_child(self, child: Element)` | `appendChild` |
| | `insert_before(self, child: Element, before: Option<Element>)` | `insertBefore` |
| | `remove_child(self, child: Element)` | `removeChild` |
| | `remove(self)` | `remove` |
| | `parent(self) -> Option<Element>` | `parentNode` |
| | `first_child(self) -> Option<Element>` | `firstChild` |
| | `next_sibling(self) -> Option<Element>` | `nextSibling` |
| | `query_selector(self, selector: String) -> Option<Element>` | `querySelector` |
| | `is_same(self, other: Element) -> Bool` | `===` |
| | `click(self)` | `click` |
| | `add_event_listener(self, kind: String, handler: fn(Event)) -> Listener` | `addEventListener` |
| Window events | `window_add_event_listener(kind: String, handler: fn(Event)) -> Listener` | `window.addEventListener` |
| A listener | `remove(self)` | `removeEventListener` |
| An event | `kind(self) -> String` | `type` |
| | `target(self) -> Option<Element>` | `target` |
| | `key(self) -> String` | `key`, or `""` |
| | `prevent_default(self)` | `preventDefault` |
| | `stop_propagation(self)` | `stopPropagation` |
| Location | `location_href() -> String`, `location_pathname() -> String`, `location_search() -> String`, `location_hash() -> String` | `location.*` |
| | `set_location_hash(hash: String)` | `location.hash = hash` |
| History | `history_push(url: String)`, `history_replace(url: String)`, `history_back()` | `history.pushState(null, "", url)`, `replaceState`, `back` |
| Timers | `set_timeout(ms: Int, f: fn()) -> Timer`, `set_interval(ms: Int, f: fn()) -> Timer` | `setTimeout`, `setInterval` |
| A timer | `clear(self)` | `clearTimeout`, `clearInterval` |
| Fetch | `async fn fetch(url: String) -> Result<Response, FetchError>` | `fetch(url)` |
| | `async fn fetch_with(url: String, init: FetchInit) -> Result<Response, FetchError>` | `fetch(url, init)` |

- Every function has a `///` doc, as std's do.
- **A window listener's target** is `window`, which is not a node, so
  `Event::target` on its events is `None`.
- **`fetch` reads the whole body as text** before it completes. `ok` is
  JS's (a status from 200 to 299). A network failure, or a refused
  request, is `Err` with JS's message.
- **Each JS function is in `shim/dom.js`,** named after the Nova one with
  a `dom_` prefix: `dom_create_element`, `dom_set_attribute`, and so on.

## 7. Waits that JS ends

### 7.1 Tokens

- The shim keeps a table of pending waits, as it keeps handles. `fetch`
  starts the request, stores a token's slot, and returns the slot to
  Nova.
- A task waits with a new leaf future in `std/rt`,
  `rt_task_host_future(token)`, which `std/dom` reaches through a routed
  builtin, as `std/time` reaches the sleep future. It stages a wait on the
  token and awaits the yield future until the shim says the token is
  done; the task parks meanwhile. The park record's target holds the
  token, with a flag that tells it from a task.
- **When the request settles,** the shim stores the status and body, or
  the error, under the token and calls a new export, `nova_wake(token)`.
  It queues the tasks parked on the token, then runs a turn, as
  `nova_call` does.
- `fetch` then reads the result with `extern "js"` functions and frees
  the token.

### 7.2 What a turn returns

- **0:** tasks are still queued (the budget of ADR 0040).
- **A positive number:** the milliseconds to the earliest deadline.
- **-1:** no task is queued or parked.
- **-2 (new):** tasks are parked, none has a deadline, and at least one
  waits on a token. JS sets no timer; the token's `nova_wake` brings the
  next turn.
- **A deadlock** is now tasks parked with no deadline and none waiting
  on a token. The report's lines do not change.

### 7.3 The shim's scheduler

- One function, `schedule(delay)`, takes every turn's result. It keeps
  at most one timer: a sooner delay replaces a later one.
- `init` resolves once `main` has returned and a turn has returned -1.
  Listeners, timers and replies may still call in later; each call is a
  turn, and schedules its own follow-up.
- A program with neither an `async fn main` nor a registered closure
  keeps today's shape: `main`, then nothing.

## 8. The shim

- **`shim/dom.js`:** the handle table, the closure dispatch, the token
  table, and each `dom_*` function. Like `float.js`, only the parts the
  module imports are included.
- **The exports it calls:** `nova_call` and `nova_wake` (new),
  `nova_turn` and `nova_gc_switches` (4.1's), and `nova_alloc_leaf`.
- **Strings** cross as today: `text(s)` reads one, and `string(value)`
  makes one. An `Option<String>` result is two calls: whether there is a
  value, then the value.
- **The page** is 4.0's `index.html`: a program finds `body()` and builds
  inside it. A user's own `index.html` comes with 4.3.
- **Hello world** imports none of it, so its size does not change.

## 9. The fake DOM

`tests/dom/fake_dom.mjs`, plain JS with no dependencies, which a test's
harness installs as `globalThis.document`, `window`, `location`,
`history` and `fetch` before it imports `app.js`. It implements what
`std/dom` calls and what the tests need to look at:
- `Node`, `Element` and `Text`, with `parentNode`, `firstChild`,
  `nextSibling`, `appendChild`, `insertBefore`, `removeChild` and
  `remove`;
- attributes; the properties `id`, `className`, `value`, `checked`,
  `disabled` and `textContent`;
- `querySelector`, `querySelectorAll` and `getElementById` for tag, `#id`,
  `.class` and `[attr]` selectors, compound, with the descendant
  combinator;
- events on Node's own `EventTarget`, bubbling to `document` and
  `window`, `click()`, and a blur when a focused element is removed, so a
  test can call in from inside a call (§5.2);
- `location` and `history`, with `popstate` and `hashchange`;
- `fetch` answering from routes the harness sets, after a timer, never
  over the network;
- `outerHTML`, escaped, for a harness to print.

**It holds what a browser holds and no more.** A listener it keeps is
the shim's JS function, which holds only the closure's index. So if
`std/rt`'s table were not a root, a collection under stress would free
the closure, and the next click would fail. The stress tests rely on
that.

## 10. Testing

### 10.1 Unchanged rules

Tests never touch the internet. A test that needs `$NOVA_HOME` sets it to
a fresh directory, and test directories have fixed names. Under
`NOVA_REQUIRE_NODE=1`, as in CI, a test that needs Node fails without it.

### 10.2 `cargo test`

Each DOM test builds a program from `tests/dom/`, runs it under Node with
a harness that installs the fake DOM, drives it, and prints the page,
and compares stdout with a `.stdout` file. Each runs twice, once as is
and once under `NOVA_GC_STRESS` and `NOVA_GC_DEBUG`, where it must also
show collections.
- **counter** (the gate): three clicks, and the count reads 3.
- **build:** elements, text, attributes, properties, `insert_before`,
  `remove`, and the walk with `first_child` and `next_sibling`.
- **listeners:** a removed listener no longer runs, and its entry
  leaves the table; `stop_propagation` and `prevent_default`; `key`.
- **handles:** 50 clicks that each make and drop 200 nodes, under
  stress: the `nova-dom:` lines show handles released, and the live count
  stays under 400.
- **timers:** a timeout, an interval cleared after three ticks, and a
  cleared timeout that never runs; the order of their lines.
- **nested:** a handler that calls `click()` on another element, under
  stress: the inner handler runs, and nothing is collected until the
  outer call ends.
- **history:** push, replace, `location_hash`, and a window listener for
  `popstate`.
- **selectors:** `query_selector`, `query_selector_all` and
  `element_by_id`, present and absent.
- **fetch:** from an `async fn main`, a 200, a 404, and a network error;
  and from a click handler, through a spawned task.
- **panics:** a handler that panics writes the message and exits 134,
  and a later click calls nothing; a forged `Element` is the panic of
  §4.1.
- **checks:** `extern "js"` in a user's module is E0090 on WASM and E0900
  natively; an `extern "js"` with a record parameter is refused; `import
  dom` natively is E0092; a project module named `dom` is E0093;
  `nova new ui` is refused.
- **The shim's unit tests:** only the imported `dom_*` functions are in
  the shim; a JS exception becomes a panic naming its function.

### 10.3 Chromium, in CI

A new job, `browser`, on Ubuntu only:
1. builds `nova` and `tests/dom/counter.nova` for WASM;
2. installs `playwright`, pinned, into a scratch directory with
   `npm install --no-save`, and Chromium with `npx playwright install
   --with-deps chromium`;
3. runs `tests/browser/counter.mjs`, which serves `dist/` from a small
   Node server in the same file (with `application/wasm` for the module),
   opens the page, clicks the button three times, and checks the count
   reads 3 and the console holds no error.

Nothing in `cargo test` or `nova` installs a package. The job is the
first of decision 12's; Lighthouse joins it in 4.8.

### 10.4 Checks that stay

The gate's 94 programs and the collector's programs still pass on both
targets; hello world stays under 30 KB gzipped, and its size does not
change.

## 11. Crates and files

| Where | Change |
|---|---|
| `nova-resolver` | `$std.dom` on WASM, not glob-imported; `import dom`; E0092 for it natively; E0093; `extern "js"` allowed in std |
| `nova-typeck` | the `"js"` ABI and its types; the ABI on `hir::ExternFn` |
| `nova-hir` | `ExternFn::abi` |
| `nova-mir` | `ExternDecl::abi`; `closure_call`; the routed builtins of §5 and §7; exports `nova_call` and `nova_wake` |
| `nova-codegen-wasm` | imports for `extern "js"`; `closure_call`; `shim/dom.js`; the scheduler in `run.js` |
| `nova-pm` | `dom` and `ui` refused as package names |
| `nova-std` | embeds `std/dom` |
| `std/dom/lib.nova` | new |
| `std/rt/lib.nova` | boxes and the registry, the closure table, tokens, `rt_call`, `rt_wake`, the depth word |
| `tests/dom/` | the fake DOM, the programs, their harnesses and outputs |
| `tests/browser/counter.mjs` | the Playwright script |
| `.github/workflows/ci.yml` | the `browser` job |
| `crates/nova-cli/tests/wasm_tests.rs` | the tests of §10.2 |

**No new crate, and no change to `Cargo.lock`.** Playwright is installed
by the CI job only.

## 12. Records

- **ADR 0041, `std/dom` and the boundary with JS:**
  - `extern "js"` in std only, and why not a builtin per function;
  - handles released by the collector through a weak registry;
  - the closure table as a root, and every call from JS as a turn, with
    the collector held back while a call is nested;
  - waits that JS ends;
  - the importable std module and the reserved names.
- **ADR 0038** gains a note: `std/dom` reaches the shim through
  `extern "js"`, not compiler-known functions.
- **Dated notes:**
  - `nova-spec/30-FRONTEND.md` §5 (the written subset) and §9 (std-only
    `extern "js"`; user interop still in the backlog);
  - `nova-spec/14-CODEGEN.md` §8: imports from `extern "js"`,
    `closure_call`, and the new exports;
  - `nova-spec/13-RUNTIME.md` §8: handles, the closure table, and token
    waits;
  - `nova-spec/20-STDLIB.md`: `std/dom`, and importable std modules;
  - `docs/phase-4-plan.md`'s 4.2 entry.
- **`CHANGELOG.md`**, under `[Unreleased]`.
- **`ARCHITECTURE.md`**, the `nova-std` row.

## 13. Risks

1. **A value only JS holds is freed.** Listeners and timers are in a
   root table; handles are boxes a record holds. The fake DOM holds no
   more than a browser does, and the stress runs collect at the end of
   every call (§9).
2. **The collector runs with a Nova frame on the stack,** through a call
   from JS inside a call from JS. The depth word stops it, and the nested
   test checks it under stress.
3. **Handles leak** if the registry misses a box, or release one still
   held. The handles test bounds the live count; a released handle still
   in use panics rather than reaching another object.
4. **The fake DOM differs from a browser.** The Chromium job runs the
   gate's program in a real one. Later sub-phases add their own browser
   tests.
5. **Reserving `dom` and `ui` breaks a project that uses them.** The
   plan checks the index and the repository first; the error names the
   rename.
6. **CI time grows** by the browser job, on Ubuntu only, with Chromium
   cached by Playwright's own install step.

## 14. Not in 4.2

- `std/ui`, its reactivity and builders, and `mount` (4.3); a user's own
  `index.html` and `assets/` (4.3).
- User-level `@js_import` and `@js_export`, and npm (ADR 0038's backlog).
- Styles beyond `set_attribute("style", …)`, `classList`, forms beyond
  `value` and `checked`, the canvas, storage, WebSockets, and streaming
  a response's body.
- `FinalizationRegistry`: a listener on a node the page dropped stays in
  the table until it is removed.
- Lighthouse and the size checks of decision 12 (4.8).

## 15. Decisions made while writing this spec

1. **One closure type, `fn(Int)`,** in the table, so `closure_call` has
   one indirect type; `std/dom` wraps each handler.
2. **Boxes in a weak registry**, pruned after each sweep as the task
   index is, rather than a size class of their own: no change to the
   allocator's pages.
3. **Nested calls from JS** run their handler only; the outermost call
   runs the queue and collects (§5.2).
4. **A turn returns -2** when only token waits remain, so `init` keeps
   its meaning: it resolves when the program has nothing left to run.
5. **E0093** is a new error code for a module that takes a std name.
6. **`Element` is any node,** text included, as `30-FRONTEND.md` §5
   writes it; a text node's attribute call is the JS exception's panic.
7. **Window listeners are a function of their own,** since `window` is
   not a node.
8. **`click()` is in the subset,** for programs and for the nested test.
9. **The `nova-dom:` debug line** reports releases, so the handles test
   can bound the live count without a hook only tests would use.
10. **The Playwright script serves `dist/` itself,** so the job needs no
    server package and the module gets `application/wasm`.
