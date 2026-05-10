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

---

## 7. Bundler (`nova-bundler`)

```bash
nova build --target wasm     # outputs dist/
nova bundle                  # alias
nova dev                     # dev server with HMR
```

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

---

## 8. Dev Server (`nova dev`)

- File watcher on `.nova` files
- Incremental recompile on change
- HMR via WebSocket: client receives "module updated" message, re-evaluates
- Error overlay: type errors render in browser overlay
- Source maps for debugging in browser DevTools

Built using `axum` (Rust) for the dev server backend.

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
