# 30 — Frontend / WASM Specification

> Phase: 4
> Crates: `nova-codegen-wasm`, `nova-bundler`, `std/dom`, `std/ui`

---

## 1. Strategy

Nova compiles to WASM + auto-generated JS shim. Reactivity uses **signals** (SolidJS-style), not virtual DOM.

Why signals:
- No reconciliation overhead
- Smaller runtime (vs Vue/React reactivity systems)
- Better mapping to compiled language (mutability tracked at type level)
- Phase 4 priority: minimal runtime, fastest possible bundle size

---

## 2. Reactivity Primitives

```nova
module std.ui

// Signal: reactive cell
pub record Signal<T> { /* opaque */ }
impl<T: Clone> Signal<T> {
    pub fn value(self) -> T
    pub fn set(self, new_value: T)
    pub fn update(self, f: fn(T) -> T)
}

pub fn signal<T>(initial: T) -> Signal<T>

// Effect: runs when its signal dependencies change
pub fn effect(f: fn())

// Memo: derived signal
pub fn memo<T>(f: fn() -> T) -> Signal<T>

// On-mount / on-cleanup
pub fn on_mount(f: fn())
pub fn on_cleanup(f: fn())
```

Implementation: each signal owns a list of subscriber effects. `value()` registers current effect (tracked via thread-local "current effect" pointer). `set()` runs all subscribers.

**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-43`):** 4.3
builds `std/ui`, which a program imports with `import ui`, on both targets.
`Signal<T>` has no `Clone` bound, and `set` notifies every time; `memo`
returns a `Signal<T>` whose `set` panics; and `root`, `batch` and `update`
are added, `root` returning what its function returns. `set` runs the
effects that read the signal before it returns, each once and after the
memos it reads are current; memos are lazy. The current effect and owner
are in a record the runtime keeps for the thread or the page
(`docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
§4, §5; ADR 0042).

---

## 3. Component Model

```nova
import nova/ui
import nova/ui/html { div, button, text, input }

component Counter() {
    let count = signal(0)

    view {
        div(class: "counter") {
            text("Count: ${count.value}")
            button(on_click: || count.set(count.value + 1)) {
                text("+")
            }
            button(on_click: || count.set(count.value - 1)) {
                text("-")
            }
        }
    }
}
```

`component` is sugar for a function returning a `View` value. The `view { ... }` block is sugar for tree-builder calls.

### 3.1 Compilation
```nova
component Counter() {
    let count = signal(0)
    view {
        div { text("${count.value}") }
    }
}
```
desugars to:
```nova
fn Counter() -> View {
    let count = signal(0)
    let __root = ui.element("div", [], [
        ui.text_node(|| "${count.value}")
    ])
    __root
}
```

The `|| ...` closure subscribes to `count` because `count.value` access within an effect tracks dependencies.

**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-44`):** 4.4 builds
this syntax (RFC 0001). A signal is read with `count.value()`, as 4.3
built it. `component` and `view` are contextual keywords: `component` where
an item starts, a name and `(` follow and a block (or a refused `->`)
follows the parameters, `view` where an expression starts
and `{` follows, outside an `if`, `while`, `for` or `match` head, even
inside parentheses there, as a record literal. A
component has no type parameters and no written return type. The type
checker, not a pass before it, desugars a view into 4.3's builders: an
element becomes a call of the function of its name with `[Attr]` and
`[View]`, or `[Attr]` alone for a void element; `text("literal")` is
`Text`, and any other `text(e)` is `DynText(|| e)`, so text that is not a
literal follows its signals. A view holds one root (E0094); a void element
takes no block (E0095), and one with no attributes is written `br([])`,
since `br()` is a call with no arguments; an element names a builder
(E0096). `if` and `for`
are not in a view: `show` with `|| view { … }` and `for_each` with
`|item| view { … }` do that
(`docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md` §3, §4; ADR 0043).

---

## 4. Element Builder API

```nova
module std.ui.html

pub fn div(attrs: AttrList, children: ChildList) -> View
pub fn span(attrs: AttrList, children: ChildList) -> View
pub fn button(attrs: AttrList, children: ChildList) -> View
pub fn input(attrs: AttrList) -> View
pub fn text(content: String) -> View
// ... all HTML elements

// Attribute helpers
pub fn class(value: String) -> Attr
pub fn id(value: String) -> Attr
pub fn on_click(handler: fn()) -> Attr
pub fn on_input(handler: fn(String)) -> Attr
pub fn style(value: String) -> Attr
// ... etc

// Conditional
pub fn show<T>(when: Signal<Bool>, then: fn() -> View) -> View
pub fn for_each<T>(items: Signal<[T]>, render: fn(T) -> View) -> View
```

The `view { div { ... } }` macro-like syntax is parser-supported sugar that maps to these builders.

**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-43`):** 4.3
builds `std/ui/html`, imported with `import ui::html` or a list from it.
An element takes its attributes and its children as two arrays, `[Attr]`
and `[View]`, until 4.4's `view { … }`; `dyn_text` is reactive text, and
`dyn_attr` and `dyn_value` reactive attributes and properties. `View` is
data, which `mount(selector, f)` builds into the page on WASM and
`render_to_string(f)` writes as escaped HTML on either target; neither
inserts raw HTML, and both refuse `innerHTML`, `outerHTML` and `srcdoc`.
`show` and `for_each` are in `std/ui`, and each rebuilds its whole part
each time its signal is set; `show` has no type parameter
(`docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
§6, §7; ADR 0042).

**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-44`):** inside a
view, an element takes attributes by name, `div(class: "a") { … }`, with
no helper needed: `_` becomes `-`, `value` is the property, and `on_kind` is
the handler of the DOM event `kind`, a `fn()` or a `fn(String)` given the
target's value. A string literal with no interpolation is fixed; any other
value becomes `Dynamic`, `DynProperty` or `DynText`, read again when its
signals change. A value that is not a `String`, or a handler of another
type, is E0097. A positional argument is an `Attr`, so 4.3's helpers still
work. Named arguments exist only there (`docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md` §4; ADR 0043).

**Amended 2026-10-19 (branch `claude/loving-archimedes-frtf5q-46`):** a
view must not show a `Map`'s order. Each process seeds its `Map`s at
random, natively and on WASM, and no seed passes from a server to its
client, since that would hand every visitor the server's HashDoS seed; so
a server's render and the client's hydration can iterate one `Map` in
two orders. A view sorts a `Map`'s `keys()` before it shows them, and
takes a value that differs between two renders, such as the time or a
random number, as the server's data; hydration reports a view that forgets
as a mismatch, and builds it again
(`docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
decision 4; ADR 0045).

---

## 5. DOM Bindings (`std/dom`)

Low-level FFI to browser DOM via JS imports:

```nova
module std.dom

@js_import("__nova_dom__")
extern fn create_element(tag: String) -> Element

@js_import("__nova_dom__")
extern fn set_attribute(el: Element, name: String, value: String)

@js_import("__nova_dom__")
extern fn append_child(parent: Element, child: Element)

@js_import("__nova_dom__")
extern fn add_event_listener(el: Element, event: String, handler: fn(Event))

pub record Element { /* opaque, wraps JS handle */ }
pub record Event { /* opaque */ }
```

**Amended 2026-10-16 (branch `claude/loving-archimedes-frtf5q`):** 4.2
builds `std/dom` as a written subset, which a program imports with `import
dom`: elements and text, attributes and properties, inserting and removing,
listeners, selectors, `location` and `history`, timers, and an async
`fetch`. It declares its JS functions with `extern "js"`, not
`@js_import`, and an `Element` holds a handle the collector releases
(`docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md`
§3, §4, §6; ADR 0041).

Generated JS shim provides `__nova_dom__`:

```javascript
// auto-generated app.js
const dom = {
    create_element: (tag) => document.createElement(tag),
    set_attribute: (el, name, val) => el.setAttribute(name, val),
    append_child: (p, c) => p.appendChild(c),
    add_event_listener: (el, evt, cb) => el.addEventListener(evt, cb),
    // ...
}
```

---

## 6. Router

```nova
module std.ui.router

pub component Router(routes: [Route]) { ... }

pub record Route {
    pub path: String
    pub component: fn() -> View
}

pub fn navigate(to: String)
pub fn use_params() -> Map<String, String>
pub fn use_query() -> Map<String, String>
```

Hash-based routing by default; HTML5 history API via flag.

**Amended 2026-10-18 (branch `claude/loving-archimedes-frtf5q-45`):** 4.5
builds `std/ui/router`, which a program imports with `import ui::router`,
on both targets. The flag is a second name: `Router(routes)` routes by the
hash, `#/posts/1`, and `HistoryRouter(routes)` by the path, `/posts/1`,
through the history API; natively the location is `/`. A route's path is
split at `/` into literals, matched with their case, and `:name`
parameters, which `use_params()` reads; a path whose only piece is `*`
matches every location; patterns and locations are percent-decoded alike,
and the first route that matches is shown. A router is a `memo` of the
matched route feeding a `for_each`, which builds the component untracked,
again on every change of location, so `use_params` and `use_query` are not
reactive. `navigate(to)` sets the hash or pushes the history, then the
location to the one the page reports, which never holds a `#fragment`, and
panics with no router; a link to the current location adds no history
entry. Links are `link(to: "/posts/1") { … }`, a builder whose `to` becomes
an `href` (`#/posts/1` under `Router`, `/posts/1` under `HistoryRouter` and
with no router, a leading run of `/` and `\` as one `/`) beside
`data-nova-link`; each
router listens for clicks on the window, and the one mounted last
navigates on a plain one, which `std/dom`'s new `Event.modified()` tells
apart, leaving any other to the browser. The router's state is behind
three std-only builtins, `router_state_present`, `_load` and `_store`, and
a page has one router, the one mounted last. The router's listeners run
batched and untracked through `std/ui`'s public API, and `std/ui` does not
change
(`docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
§3; ADR 0044).

---

## 7. Bundler (`nova-bundler`)

```bash
nova build --target wasm     # outputs dist/
nova bundle                  # alias
nova dev                     # dev server with HMR
```

**Amended 2026-10-14 (branch `claude/loving-archimedes-frtf5q`):** 4.0 builds
`nova build --target wasm` and `nova bundle`. `dist/` holds `index.html`,
`app.js` and `app.wasm`, and a `package.json` marking `app.js` a module for
Node, at a project's root or in the current directory, or where `-o`
names. A program may use the language and std's core, bytes,
collections, strings, fmt and json; `manifest.json`, `assets/`, a user's own
`index.html` and `wasm-opt` come with 4.3
(`docs/superpowers/specs/2026-10-14-phase-4-0-foundations-design.md` §7; ADRs
0038 and 0039).

**Amended 2026-10-15 (branch `claude/loving-archimedes-frtf5q`):** 4.1 adds
std's task, sync, time and log, and async on the event loop: an `async fn
main` returns to JS, which calls the module again on each timer, and `init`
resolves when the program ends. A name from a module WASM lacks is E0092,
naming the module
(`docs/superpowers/specs/2026-10-15-phase-4-1-the-language-on-wasm-design.md`
§3, §5, §7; ADR 0040).

**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-43`):** by the
user's decision of 2026-10-16, a user's own `index.html`, `assets/` from
`public/`, `manifest.json` and `wasm-opt` move from 4.3 to 4.4, with
`examples/06-counter-spa`
(`docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md`
decision 7).

**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-44`):** 4.4 builds
them, without `chunks/`. A project's own `index.html`, beside `nova.toml`,
is copied as it is, and must load `app.js` and call its `init()`;
otherwise the generated page, which has `<div id="app"></div>`, is written.
`public/` is copied into `assets/` afresh on each build, a link only when
it leads to a file inside `public/`, and nothing when `public/` itself
leads outside the project; without a `public/` to copy, an `assets/`
already there is left as it is. `manifest.json` holds the build's name and version, from
`nova.toml` or the file's stem and `null`, whether it was `--release` and
whether `wasm-opt` ran, and each file written with its size, sorted, and
nothing that changes between two builds. `--release` runs `wasm-opt -Oz` on
`app.wasm` when it is on `PATH`, says so when it is not, and fails when it
fails. An output directory that holds the project or lies inside `public/`
is refused, and a link at a name the build writes, symbolic or, on Unix,
hard, is removed first (`docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md` §7; ADR 0043).

**Amended 2026-10-22 (branch `claude/loving-archimedes-frtf5q-48`):** step 4
of §7.2 runs `wasm-opt -Oz --enable-bulk-memory`, since the backend writes
`memory.copy` and `memory.fill`, which binaryen's `wasm-opt` refuses
without bulk memory: 4.4's `-Oz` alone failed every build with binaryen on
`PATH`. With binaryen 132, `06-counter-spa`'s `app.wasm` and `app.js` come
to 11,974 bytes gzipped by zlib at level 9, from 20,997, and
`07-fullstack-blog`'s client to 27,510, from 49,472. §11's Lighthouse
target, as ADR 0047 reads it, a median Performance score of 95 or more
over three mobile runs, was met where 4.8 was built, every median 100;
CI's `browser` job measures it, and the release pull request records that
job's run in ADR 0047
(`docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`
§3, §4; ADR 0047).

### 7.1 Output Layout
```
dist/
├── index.html              # auto-generated or user-provided
├── app.wasm                # main module
├── app.js                  # JS shim
├── chunks/
│   └── lazy-loaded.wasm    # code-split modules
├── assets/                 # copied static files
└── manifest.json           # build metadata
```

### 7.2 Pipeline
1. Compile all Nova files to MIR
2. Tree-shake at MIR level (only emit used functions)
3. Codegen to WASM
4. Run `wasm-opt` if available (size optimization)
5. Generate JS shim (imports, init, exports)
6. Generate `index.html` if missing
7. Process static assets
8. Output to `dist/`

### 7.3 Code Splitting
Triggered by `lazy()` in user code:
```nova
let admin_module = lazy(|| import("./admin/dashboard"))
```
Bundler emits separate `.wasm` chunk loaded on demand.

### 7.4 SSR/SSG

```bash
nova build --ssr     # server-side rendering: emit native binary that renders HTML
nova build --ssg     # static site gen: pre-render at build time
```

Same component code runs on:
- Server (native binary, renders to string)
- Client (WASM, hydrates)

Hydration: client downloads WASM, attaches to existing DOM, replays effects.

**Amended 2026-10-19 (branch `claude/loving-archimedes-frtf5q-46`):** 4.6
builds both flags for a project with two entries: `src/main.nova`, the
client, and `src/server.nova`, the server, which share components through
a module such as `src/app.nova`. `nova build --ssr` builds the client into
`dist/` as `--target wasm` does, and the server natively into
`target/debug/<name>-server`; `--ssg` then runs the server once with
`--ssg <dist>`, which writes `dist/<path>/index.html` for each route with no
`:name` piece that is not `*`, and lists those pages in `manifest.json`.
The server is the project's own program: it imports `ui::ssr`, native
only, and calls `serve(Site { address, dist, routes, render })`, which
serves `dist/`'s listed files and renders any other `GET` into the built
`index.html`'s `<div id="app"></div>`, with `404` where no route but `*`
serves the path;
`page`, `page_with` and `document` render a page to a string for a server
written by hand. The server writes `render_hydratable`'s HTML, which marks
each `show` and `for_each` with `<!--[-->` and `<!--]-->` and two texts side
by side with `<!--|-->`, and the client calls `hydrate("#app", App)`, or
`hydrate_with("#app", fallback, App)` for data the server wrote as JSON
in a `<script type="application/json" id="nova-data">` just after the root,
or the `fallback` the program gives when the page holds none. Hydration
walks the view against the page's nodes in order and adopts them, checking
tags and texts but not attributes, and starts their effects and handlers
rather than replaying them; at the first difference it reports `nova:
hydration mismatch: …` with `console.error`, empties the root and builds
the view afresh. An empty root is built into with no report, as in the
page `nova dev`'s build writes, unless the project's own page holds
something in its root; there `hydrate_with` builds the page from its
fallback (the user's decision 13, of 2026-10-20). A hash `Router` is not
for SSR; `HistoryRouter` is
(`docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
§3 to §7; ADR 0045).

**Amended 2026-10-21 (branch `claude/loving-archimedes-frtf5q-47`):** a
server written by hand, as `examples/07-fullstack-blog`'s is, builds on
`ui::ssr`'s parts rather than on `serve`. `Dist::open(dir)` reads the built
`dist/`'s `index.html` and `manifest.json` once, with `serve`'s checks. The
program's own `std/http` `Server` routes each page by method and pattern,
renders it with `page_with(location, data, App)`, and answers with
`dist.respond(status, page)`, which puts the page into the template as
`document` does, and to which a route can add a cookie with `with_cookie`;
it answers a form's `POST` as well, as the blog's does with a `303` redirect
once the form succeeds. Its `fallback` asks `dist.file(req)` for a file the
manifest lists before it answers with its own not-found page. So each page
can choose its status, read the request's cookies and form, and carry the
data it needs, which the client reads with `hydrate_with`. Since
`hydrate_with` reads one value for the whole app, once, such a server's
pages link to each other with plain anchors, each page arriving from the
server with its own data, rather than with a router's `link`. `serve` is
rebuilt on `Dist`, and does what it did
(`docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md` §4
and §6; ADR 0046).

---

## 8. Dev Server (`nova dev`)

- File watcher on `.nova` files
- Incremental recompile on change
- HMR via WebSocket: client receives "module updated" message, re-evaluates
- Error overlay: type errors render in browser overlay
- Source maps for debugging in browser DevTools

Built using `axum` (Rust) for the dev server backend.

**Amended 2026-10-18 (branch `claude/loving-archimedes-frtf5q-45`):** 4.5
builds `nova dev [FILE] [--port <PORT>] [--open] [-o <DIR>]`, as ADR 0038
scoped it, and it is the `nova dev` of §7's list. It builds into `dist/` as
`nova build --target wasm` does, each build a full one in a child process
with `NO_COLOR=1`, whose stderr is the overlay's text; it polls the
sources' times and sizes every 200 ms; and it serves on `127.0.0.1`, port
3000 by default, from an HTTP/1.1 server on `std::net` with a thread per
connection, not `axum`. A good build reloads the page through Server-Sent
Events, `/__nova/events`, which a script the server adds to each page
listens to; a failed one shows `<div id="nova-dev-overlay">` over the page
until a good build. There is no HMR, so the page's state starts again on
each reload; there are no source maps; and a browser opens only with
`--open`. A path whose last piece has no `.` and that names no file, nor a
directory with an `index.html`, is answered with the page, so a
`HistoryRouter` page can be reloaded, with the generated
page, which loads `/app.js` by its absolute path, or a page of the
project's own that does the same; a path that leaves
`dist/` is `403`, and so is a request whose `Host` is not the server's own
(`docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`
§4; ADR 0044).

---

## 9. JS Interop

### 9.1 Calling JS from Nova
```nova
@js_import("lodash")
extern module lodash {
    fn debounce<F: Fn>(f: F, ms: Int) -> F
    fn throttle<F: Fn>(f: F, ms: Int) -> F
}

let debounced = lodash.debounce(my_handler, 300)
```

Bundler resolves `lodash` from `package.json` (npm). Yes, Nova frontend uses npm for JS deps — this is intentional for ecosystem leverage.

### 9.2 Calling Nova from JS
```nova
@js_export
fn calculate(x: Int, y: Int) -> Int { x + y }
```

In JS:
```javascript
import init, { calculate } from './app.js'
await init()
console.log(calculate(2, 3))   // 5
```

### 9.3 Marshalling
- Primitives pass directly
- Strings: copy across boundary (UTF-8 ↔ UTF-16)
- Objects: pass by handle (ID into a JS-side table)
- Functions: trampoline via stable function pointer

**Amended 2026-10-16 (branch `claude/loving-archimedes-frtf5q`):** in 4.2
only std crosses the boundary, through `extern "js"` (`Int`, `Float`, `Bool`
and `String`); user-level `@js_import` and `@js_export` stay in ADR 0038's
backlog. A handle is released when the collector frees the box that holds
it. A function JS holds is an entry of `std/rt`'s table of closures, a
root, and each outermost call from JS is a turn
(`docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md`
§3 to §5; ADR 0041).

---

## 10. WASM Runtime Decisions

- **Memory:** 16 MB initial, max 1 GB (configurable)
- **Stack size:** 1 MB
- **GC:** Reference counting in Phase 4 (simpler), migrate to WASM GC proposal when stable
- **No threads** in Phase 4 (revisit when WASM threads + SharedArrayBuffer ubiquitous)
- **No SIMD** unless target supports it; auto-detect at runtime

---

## 11. Performance Targets

- Hello world WASM (gzipped): < 30 KB
- Counter component: < 50 KB
- Todo MVC: < 100 KB
- TTI on Counter app: < 100ms on cable connection
- Lighthouse Performance score: > 95

---

## 12. Examples (Phase 4)

```
examples/
├── 06-counter-spa/         # signals, simple component
├── 07-fullstack-blog/      # SSR + client hydration + DB
├── 08-todomvc/             # canonical TodoMVC
├── 09-realworld-app/       # RealWorld spec demo (auth, CRUD, routing)
└── 10-game-2048/           # WASM perf showcase
```

---

## 13. Tests

- Unit: signal reactivity, effect cleanup, memo memoization
- Integration: render component, simulate events (use `wasm-bindgen-test`)
- E2E: Playwright tests against `nova dev` server
- Visual regression: snapshot DOM state

**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-43`):** the
unit tests are `tests/runtime/ui_*.nova`, which run natively and on WASM,
those that do not panic under each target's collector stress too;
`tests/dom/ui_*.nova` render and click under the fake DOM, and CI's
`browser` job clicks the builder counter in Chromium
(`docs/superpowers/specs/2026-10-17-phase-4-3-std-ui-reactivity-and-views-design.md` §9).

**Amended 2026-10-17 (branch `claude/loving-archimedes-frtf5q-44`):**
`tests/runtime/ui_view.nova` renders components and views on both targets;
`examples/06-counter-spa` is clicked under the fake DOM in `cargo test`, as
it is and under stress, and in Chromium by the `browser` job
(`tests/browser/spa.mjs`); its `app.wasm` and `app.js` are under 50 KB
gzipped (`docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md` §9).

**Amended 2026-10-18 (branch `claude/loving-archimedes-frtf5q-45`):** the
E2E tests run against `nova dev`. CI's `browser` job drives
`06-counter-spa` under it in Chromium (`tests/browser/dev.mjs`): it counts,
an edit reloads it, a page that missed a build reloads when it connects, a
type error shows the overlay, and the fix takes it away. The same job
drives `tests/browser/router-app`, a `HistoryRouter` app
(`tests/browser/router.mjs`), through links, the back button, a reload at
`/posts/2`, a path only `*` matches, and a query.
`tests/runtime/ui_router.nova` matches paths and queries on both targets;
`tests/dom/router_hash.nova` and `router_history.nova` route in each mode
under the fake DOM, as they are and under stress, and `router_two.nova`
leaves a click to the browser once its router is not the one mounted last;
and
`crates/nova-cli/tests/dev_tests.rs` speaks HTTP to `nova dev`
(`docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md` §5).
