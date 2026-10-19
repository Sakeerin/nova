# Phase 4.6, "SSR, hydration and SSG": design

- **Date:** 2026-10-19.
- **Status:** approved by the user on 2026-10-19.
- **The user's decisions, 2026-10-19:**
  1. **Two entries, one project.** `src/main.nova` is the client, built
     for WASM, which hydrates. `src/server.nova` is the server, built
     natively. The components they share live in a module of the project,
     such as `src/app.nova`. `nova build --ssr` builds both: `dist/` and
     the server's executable.
  2. **Hydration walks the nodes in order.** The server writes comment
     markers around each `show` and `for_each` region, and between two
     texts that stand side by side. Elements get no attributes for it.
  3. **On a mismatch,** the client reports the first difference with
     `console.error`, drops the server's nodes under the root, and builds
     the view afresh. The tests assert that their pages hydrate with no
     mismatch.
  4. **No seed is shared between server and client.** Sending the
     server's hash seed to every visitor would defeat its HashDoS
     protection. A view must not show a `Map`'s order: it sorts first. The
     mismatch check catches a view that forgets.
  5. **The page is the built `dist/index.html`,** the generated page or
     the project's own. The server, and SSG, put the rendered HTML inside
     its `<div id="app"></div>`, and the client calls `hydrate("#app",
     App)`. The head, the title and the scripts stay in that one file for
     both.
  6. **SSG renders the static routes:** those with no `:name` piece that
     are not the catch-all `*`. `nova build --ssg` builds the client and
     the server, then runs the server once. The std function that serves
     writes `dist/<path>/index.html` for each static route instead. Routes
     with parameters are skipped.
  7. **std serves at two levels,** in a new std/ui module that only a
     native program can import:
     - a high level, one function that serves `dist/`'s files and renders
       every other `GET` as a page, which is enough for the gate;
     - a low level, functions that render the page at one path to a
       string, for a server written by hand, as 4.7's blog will be.

     `POST`, cookies and path parameters in `std/http` stay 4.7's.
  8. **The server's data reaches the client typed, through JSON.** The
     server writes a `ToJson` value into the page, escaped, in a `<script
     type="application/json">`. The client reads it back with `FromJson`
     and hydrates with the same value.
  9. **The names:**
     - `hydrate("#app", App)` sits in `std/ui` beside `mount`, with a form
       that takes the server's data.
     - The server writes `import ui::ssr`, which is native only, as `dom`
       is WASM only.
     - `ssr::serve(…)` is the high level and `ssr::page(…)` the low.
     - This spec settles the other names.
  10. **`nova dev` does not change in 4.6;** it stays client only. In an
      SSR project, its page has an empty root, and `hydrate` builds the
      view there. 4.7, or the backlog, decides on a development mode that
      restarts the server.
  11. **The gate and the tests:**
      - A small test app, `tests/browser/ssr-app`, has a router, a `show`,
        a `for_each`, a counter, and data from the server.
      - In CI, Playwright opens one server-rendered page and one
        statically generated page. It clicks, and checks that the server's
        nodes were not made again.
      - In `cargo test`, the fake DOM parses the HTML `std/ui` writes, and
        hydration and its mismatches are tested.
      - No example changes; 07 is 4.7's.
  12. **A path no route serves gets `404`** with the rendered page, when
      it falls to `*` or matches no route. The router says which route a
      path matches.
- **Branch:** `claude/loving-archimedes-frtf5q-46`, from `main` at
  `2b7a6ed`, as 4.5 used `-45`.
- **Inputs:**
  - `docs/phase-4-plan.md`:
    - decision 10 (SSR, hydration and SSG);
    - decision 11 (renderers escape by construction);
    - §4's 4.6 entry and its gate;
    - risk 5 (`Map` order);
    - §5's security rules;
  - `nova-spec/30-FRONTEND.md` §7.4, and its notes from 4.3 to 4.5;
  - `00-MASTER-SPEC.md` §1.8: "SSR/SSG: built-in flags on `nova build`",
    one of the points it does not put up for debate;
  - `60-EXAMPLES.md` §7, the blog 4.7 builds on this;
  - ADR 0042 ("a new renderer, such as 4.6's hydration, walks the same
    `View`"), and ADR 0044, which left rendering a given path natively,
    and serving a rendered program, to 4.6.

## 1. What 4.6 delivers

- **A project with a server and a client.** `src/app.nova` holds what
  both run:

  ```nova
  import ui
  import ui::html::{button, div, h1, li, p, ul}
  import ui::router::{HistoryRouter, Route, link}

  pub record Data { names: [String] }

  impl ToJson for Data { … }
  impl FromJson for Data { … }

  component Home() {
      let count = signal(0)
      view {
          div {
              p { text("Clicked ${count.value()} times") }
              button(on_click: || count.update(|n| n + 1)) { text("+") }
              link(to: "/about") { text("About") }
          }
      }
  }

  component About() { view { p { text("About") } } }

  component NotFound() { view { p { text("Nothing here") } } }

  pub fn routes() -> [Route] {
      [
          Route { path: "/", component: Home },
          Route { path: "/about", component: About },
          Route { path: "*", component: NotFound },
      ]
  }

  pub component App(data: Data) {
      view {
          div {
              h1 { text("Names") }
              ul { for_each(signal(data.names), |name: String| view { li { text(name) } }) }
              HistoryRouter(routes())
          }
      }
  }
  ```

  `src/main.nova`, the client:

  ```nova
  import ui
  import app::{App}

  fn main() { hydrate_with("#app", App) }
  ```

  `src/server.nova`, the server:

  ```nova
  import ui
  import ui::ssr::{Page, Site, page_with, serve}
  import app::{App, Data, routes}

  fn render(location: String) -> Page {
      page_with(location, Data { names: ["Ada", "Grace"] }, App)
  }

  async fn main() {
      let site = Site { address: "127.0.0.1:3000", dist: "dist", routes: routes(), render: render }
      let _ = serve(site).await
  }
  ```

- **`nova build --ssr`** writes `dist/`, as `--target wasm` does, and the
  server's executable. Run from the project's root, the server prints
  `ssr: serving http://127.0.0.1:3000/` and answers each page with its
  HTML already in it. Then the client hydrates that HTML: it adopts the
  nodes, and starts their effects and handlers.
- **`nova build --ssg`** also writes `dist/index.html` and
  `dist/about/index.html`, already rendered, so any static server can
  serve the site.
- **The gate,** in CI's `browser` job, through Playwright, on
  `tests/browser/ssr-app`:
  - a server-rendered page hydrates in Chromium, and responds to clicks
    without making its nodes again;
  - `--ssg` writes one file per static route, and such a page hydrates the
    same way.

## 2. The starting point (read on 2026-10-19, at `2b7a6ed`)

- **`std/ui`:**
  - `View` has four variants: `Element`, `Text`, `DynText` and `Region`.
    `Attr` has five: `Static`, `Dynamic`, `Property`, `DynProperty` and
    `On`. `show` and `for_each` are regions. `for_each` builds its whole
    list again on every change.
  - **`mount(selector, f)`** builds each subtree apart, then inserts it,
    after the root's existing children. A `DynText` is a text node of its
    own with an effect. A region sits between two empty text nodes, and
    its effect builds its views between them.
  - **`render_to_string(f)`** returns the view's HTML without a page
    around it:
    - text is escaped for `&`, `<` and `>`, and attribute values for `"`
      too;
    - each `DynText`, `Dynamic` attribute and region is read once;
    - a property is written as an attribute, and `On` is dropped;
    - `on_mount` never runs;
    - **a region writes no marker,** two texts side by side print as one
      run of text, and an empty text prints nothing. So a browser's
      parse of that HTML has fewer nodes than `mount` builds.
  - Nothing in the tree mentions SSR, SSG or hydration.
- **The backend** (`$std.ui.backend`, one source per target):
  - Its WASM functions create, insert, remove and set nodes, step to the
    next sibling, and compare two nodes. None reads the first child, a
    node's kind or its text.
  - Natively every node function panics with `mount`'s message.
  - `backend_location` is `"/"` natively.
- **`std/dom`** has `first_child`, `next_sibling`, `parent`, `tag` (the
  lower-cased `nodeName`, so `#text` or `#comment` for those nodes),
  `text` (`textContent`), `element_by_id` and `create_text`. It has no
  way to write to the console.
- **The router** reads its location from the backend when it is made. The
  only way to render another path natively is to call `navigate` inside
  `render_to_string`, which builds the root route first. `link`'s `href`
  under `HistoryRouter` is the rooted path.
- **`Map`** iterates in table order. Each process draws its seed at
  random, natively from `RandomState` and on WASM from
  `crypto.getRandomValues`, so a server and its client iterate one `Map`
  in different orders.
- **`std/http`:**
  - `Server::new().get(path, handler)` routes a `GET` whose target equals
    `path`, its query included.
  - Any other request gets `Response::not_found()`. Handlers are plain
    `fn(Request) -> Response`.
  - `listen(addr)` binds, serves until SIGINT or SIGTERM, and prints
    nothing.
  - `TcpListener::local_port()` in `std/net` reads a bound port.
- **`std/json`** compiles on both targets.
  - A generic function bounded by `ToJson` or `FromJson`, calling
    `T::from_json`, compiles and runs on both. This was probed on
    2026-10-19.
  - `std/process::args()` gives a native program its arguments.
- **A project's modules:**
  - `src/main.nova` and `src/server.nova` can both import `src/app.nova`
    with `import app::{name}`. This was probed. `nova run src/server.nova`
    runs it, and `nova build --target wasm` builds the client.
  - A path written `app::name` is E0900.
  - Today a project's program is `src/main.nova` alone, and `nova.toml`
    names no entry.
- **The CLI:** `nova build` takes `--target native|wasm`, `--release` and
  `-o`.
  - A native build of a project writes `target/debug/<name>`, or
    `target/release/<name>`.
  - `bundle.rs` writes `dist/`. Its `manifest.json` lists every file the
    build wrote, `assets/` included, with its size.
  - `nova dev` serves `dist/`. It redirects a directory holding an
    `index.html` from `/dir` to `/dir/`, and falls back to the page for a
    path whose last piece has no `.`.
- **The WASM build keeps only what `main` reaches** (`nova-mir`'s `mono`
  walks from `main`). So a program that only mounts does not carry
  hydration's code.
- **The tests:**
  - `tests/runtime/ui_*.nova` run on both targets, and under GC stress.
  - `tests/dom/*` run WASM programs under Node against `fake_dom.mjs`.
    The fake DOM has no HTML parser, no comments and no console capture.
  - `tests/browser/*.mjs` run in CI only.

## 3. The HTML a server writes

`std/ui` gains a renderer for HTML that will be hydrated:

```nova
pub fn render_hydratable(f: fn() -> View) -> String
```

- It does what `render_to_string` does, with three additions:
  - **A region** is written between `<!--[-->` and `<!--]-->`, with its
    views between them.
  - **Two text views side by side** are separated by `<!--|-->`. Each of
    the two is a `Text` or a `DynText`, and they are siblings in one
    element's children or in one region's views. An empty text still
    prints nothing, but its separators are written.
  - **A carriage return** in text or in an attribute value is written
    `&#13;`. The browser's parser turns a raw CR into LF, and the text
    must reach the client as it was.
- **`render_to_string` does not change.** Its output, its tests and its
  doc comments stay as 4.3 to 4.5 left them.
- An example:

  ```nova
  render_hydratable(
      || div([], [text("count: "), dyn_text(|| "2"), ul([], [for_each(list, item)])]),
  )
  ```

  writes `<div>count: <!--|-->2<ul><!--[--><li>a</li><li>b</li><!--]--></ul></div>`.
- No marker can be forged by a program: text is escaped, so `<!--` in a
  text arrives as `&lt;!--`.

## 4. Hydration

### 4.1 `hydrate` and `hydrate_with`

```nova
pub fn hydrate(selector: String, f: fn() -> View)
pub fn hydrate_with<T: FromJson>(selector: String, f: fn(T) -> View)
```

- **`hydrate`** finds the first element `selector` matches; it panics as
  `mount` does when none matches.
  - **An empty root** builds the view as `mount` does, with no warning.
    Empty means no child nodes, or only text that is all whitespace.
    This covers `nova dev`, and a page served without SSR.
  - **Otherwise it walks** `f()`'s view against the root's child nodes,
    under a root owner it keeps, as `mount` keeps one (§4.2).
  - `on_mount` callbacks run after the walk, as they do after a mount.
- **`hydrate_with`** first reads the data the server wrote (§6.2):
  - it takes the text of the element whose id is `nova-data`, parses it as
    JSON, and makes a `T` with `T::from_json`;
  - then it does what `hydrate(selector, || f(data))` does;
  - it panics with `std/ui: hydrate_with: the page has no data from the
    server` when there is no such element, and with `std/ui:
    hydrate_with: the page's data does not parse: <error>` when the JSON
    or `from_json` fails.
- **Natively,** both panic with `mount`'s message.

### 4.2 The walk

A cursor steps through the existing nodes in order. For each view:

- **`Element(tag, attrs, children)`:**
  - The node at the cursor must be an element with that tag; the tag is
    compared lower-cased. The element is adopted.
  - `Static` attributes are neither written nor checked.
  - `Dynamic` attributes and `DynProperty`s get their effects, as under
    `mount`, so their first run writes the value again.
  - `Property` is set, since the HTML carried it as an attribute.
  - `On` is listened for.
  - Then the children are walked from the element's first child. No node
    may be left after them.
- **`Text(t)` or `DynText(f)`:**
  - When the cursor is at a text node, the node is adopted, and its text
    must equal the view's: `t`, or what the `DynText`'s effect reads on
    its first run.
  - When the cursor is at anything else (a marker, an element, or the
    end) and the view's text is empty, a new empty text node is inserted
    there.
  - Where the next view is text too, a `<!--|-->` must follow. It stays in
    the page.
- **`Region(f)`:**
  - The node at the cursor must be a `<!--[-->` comment.
  - The region's first run adopts its views' nodes. Its later runs build
    between the two markers, as under `mount`; the comments are the two
    markers `mount` would have made.
  - After its views, the cursor must be at the matching `<!--]-->`.
- **After the root's views,** no node may be left in the root.

### 4.3 A mismatch

- **The first difference stops the walk.** These are differences: a wrong
  tag; a text that differs; a missing marker; a missing node; a node left
  over.
- **It is reported** with `console.error`:

  ```text
  nova: hydration mismatch: expected <p>, found <div>; building the view again
  ```

  Each side reads `<tag>`, `text "…"` (its first 40 characters), `a
  region`, `the end of a region`, or `nothing`.
- **Then the view is built afresh:**
  1. the walk's root owner is disposed, which stops the effects and
     handlers it had begun;
  2. every child of the root element is removed;
  3. the view is built there as `mount` would build it, calling `f`
     again.

  `on_mount` callbacks queued under the disposed owner do not run.
- **The tests' pages must hydrate without one** (decision 3).
- **Known causes,** which the records list:
  - HTML the browser's parser rebuilds, such as a `div` inside a `p`, or a
    `tr` without a `tbody`;
  - a `script` or `style` element's text, which the parser does not
    decode;
  - a NUL in text, which the parser drops;
  - a value that differs between the two renders, such as the time, a
    random number, or a `Map`'s order (decision 4).

## 5. The router on a server

`ui::router` gains three functions, on both targets:

```nova
pub fn with_location<T>(location: String, f: fn() -> T) -> T
pub fn found(routes: [Route], location: String) -> Bool
pub fn static_paths(routes: [Route]) -> [String]
```

- **`with_location`** runs `f` so that a router made inside it starts at
  `location`, and not at the page's address.
  - `location` is a path and its query, as a request's target writes them.
  - The previous location is restored afterwards, so calls may nest.
  - This is how a server renders `/posts/1`.
  - On WASM it applies too, though nothing in std uses it there.
- **`found`** is true when the first route that matches `location` is not
  the catch-all (`*` or `/*`). It is false when only the catch-all
  matches, or no route does. A route with parameters that matches counts
  as found.
- **`static_paths`** gives the paths of the routes with no `:name` piece
  that are not the catch-all, in order, without repeats, each as written.
- **A hash `Router` is not for SSR.** A server never sees the hash, so it
  renders `/`, and a page opened at `#/posts/1` gets a mismatch and is
  built again. The records say so; `HistoryRouter` is the router for SSR.

## 6. `ui::ssr`

### 6.1 The module

- **The file:** `std/ui/ssr/lib.nova`. A program imports it with `import
  ui::ssr`, or `import ui::ssr::{…}`.
- **Native only.** On WASM the import is E0092, as `import dom` is
  natively.
- **It uses** `std/ui`, `std/ui/router`, `std/http`, `std/net`, `std/fs`,
  `std/json` and `std/process`.

```nova
pub record Page { body: String, data: Option<String> }

pub fn page(location: String, f: fn() -> View) -> Page
pub fn page_with<T: ToJson>(location: String, data: T, f: fn(T) -> View) -> Page
pub fn document(template: String, page: Page) -> String

pub record Site { address: String, dist: String, routes: [Route], render: fn(String) -> Page }

pub async fn serve(site: Site) -> Result<(), IoError>
```

### 6.2 The low level

- **`page(location, f)`** is `with_location(location, ||
  render_hydratable(f))`, with no data.
- **`page_with(location, data, f)`** renders `f(data)` the same way. Its
  `data` is `stringify(data.to_json())`, made safe for a `script`
  element:
  - `<`, `>` and `&` are written `\u003c`, `\u003e` and `\u0026`;
  - U+2028 and U+2029 are written `\u2028` and `\u2029`.

  These characters only occur inside JSON strings, so the escaped text
  parses back to the same value.
- **`document(template, page)`** returns the template with its first
  `<div id="app"></div>` replaced by:
  - `<div id="app">` + `page.body` + `</div>`;
  - when the page has data, followed by `<script type="application/json"
    id="nova-data">` + the data + `</script>`.

  It panics with `ssr: the page has no <div id="app"></div>` when the
  template has none.
- **A server written by hand** (4.7's) calls these from its own handlers.
  It decides the status itself, with `found`.

### 6.3 `serve`

- **On start,** `serve` reads `<dist>/index.html`, the template, and
  `<dist>/manifest.json`, whose `files` are what it may serve.
  - It returns an `Err` naming the file when either cannot be read, or
    when the template has no `<div id="app"></div>`.
  - It binds `site.address` and prints `ssr: serving
    http://<host>:<port>/` with the bound port, so a test can bind port 0.
- **Each `GET`** is answered by its path: the target before any `?`,
  percent-decoded, less its leading `/`.
  - **A path that is one of the manifest's files** gets `200` and the
    file. Its `Content-Type` follows its extension: `html`, `js`/`mjs`,
    `wasm` (`application/wasm`), `css`, `json`, `svg`, `png`, `jpg`,
    `gif`, `webp`, `ico`, `txt`, `woff2`; any other is
    `application/octet-stream`. Each file also gets
    `X-Content-Type-Options: nosniff`.
  - **Any other path whose last piece holds a `.`** gets `404 not
    found`.
  - **Any other path is a page:**
    - `document(template, render(target))`, where `target` keeps its
      query;
    - `200` when `found(routes, target)`, otherwise `404`;
    - `text/html; charset=utf-8`.
  - **No file outside the manifest is ever read.** So there is no path
    traversal to refuse, and a file added after the build is not served
    until the server starts again.
- **Other methods** get `404`, as `std/http`'s `Server` gives them.
- **SIGINT or SIGTERM** stops it as `std/http`'s `listen` does, and
  `serve` returns `Ok(())`.
- `serve` is built on `std/http`'s `Server`, with two additions (§8.3).

### 6.4 SSG: `serve` run with `--ssg <dir>`

When the program's arguments hold `--ssg` and a directory, `serve` does not
bind:

- **It reads `<dir>/index.html`** as the template.
- **For each path of `static_paths(site.routes)`,** it writes
  `document(template, render(path))`:
  - `/` goes to `<dir>/index.html`, which it overwrites, since the
    template is already read;
  - any other path goes to `<dir>/<its pieces>/index.html`, and the
    directories are made.

  It prints `ssg: wrote <file>` for each page.
- **It refuses a path** with a piece that is empty, `.` or `..`, or one
  that holds `\`, NUL, `?` or `#`. It then returns an `Err` naming the
  path, and writes nothing more.
- **Then it returns `Ok(())`.**

So a server written by hand can be pre-rendered too, if its `main` calls
`serve` when given `--ssg`.

## 7. `nova build --ssr` and `--ssg`

- **`nova build --ssr`** works in a project with `src/server.nova`. It:
  1. builds the client from `src/main.nova` as `nova build --target wasm`
     does, into `dist/`, or the directory `-o` names;
  2. builds the server natively, as the project's program with
     `src/server.nova` as its root, with the project's dependencies, into
     `target/debug/<name>-server`, or `target/release/<name>-server`
     under `--release`;
  3. prints `built <dist>` and `built <server>`.
- **`nova build --ssg`** builds both as `--ssr` does. Then it:
  1. runs the server from the project's root with `--ssg <dist>`; the
     server's output passes through, and a failure fails the build, with
     its status;
  2. adds the pages the server wrote to `manifest.json`, with their sizes.
- **`--ssr --ssg` together** is `--ssg`. The server's executable stays in
  either case.
- **`--release`** applies to both builds.
- **Refusals,** each an error before anything is built:
  - a file argument;
  - `--target` beside `--ssr` or `--ssg`;
  - a library;
  - no `src/server.nova`: "`--ssr` needs src/server.nova, the server's
    entry", or the same for `--ssg`.
- **Unchanged:** `nova bundle`, `nova dev`, `nova run` and `nova check`.
  `nova run src/server.nova` runs the server, as it does today.

## 8. What else changes

### 8.1 `std/dom`

- `pub fn console_error(message: String)` writes to `console.error`. It
  comes with its shim function.

### 8.2 The backend

- **WASM** gains:
  - `backend_first_child(node) -> Option<Node>`;
  - `backend_kind(node) -> String`, the lower-cased `nodeName`;
  - `backend_text_of(node) -> String`;
  - `backend_report(message)`, through `console_error`;
  - `backend_data() -> Option<String>`, the text of `#nova-data`.
- **Natively,** each of these panics with `mount`'s message.

### 8.3 `std/http`

- **`Server::fallback(handler)`** answers a `GET` that no route matches.
- **`Server::listen_on(listener)`** serves a listener the program has
  already bound. `listen(addr)` becomes `bind` and then `listen_on`, with
  its behaviour unchanged.
- **Nothing else:** `POST`, cookies and path parameters stay 4.7's
  (decision 7).

### 8.4 The driver and the resolver

- **The driver** can build a project's program rooted at
  `src/server.nova`.
- **The resolver** registers `ui::ssr` as an imported std module natively,
  and refuses it on WASM.
- **`nova-doc`** lists the new module.

## 9. Testing

### 9.1 Unchanged rules

- Tests never touch the internet.
- Servers bind `127.0.0.1:0`. Temp directories have fixed names.
- Runtime programs run on both targets and under GC stress, with one set
  of goldens.
- Browser tests run in CI only.

### 9.2 Natively and on both targets

- **`tests/runtime/ui_hydratable.nova`** checks `render_hydratable`'s
  goldens, on both targets:
  - nested regions;
  - an empty region;
  - empty texts between other texts;
  - texts side by side, inside elements and inside regions;
  - a CR in text and in an attribute;
  - `render_to_string` of the same views, unchanged.
- **`tests/runtime/ui_router_server.nova`** checks:
  - `with_location` rendering `/posts/1` and `/q?a=1`, nested, and
    restored;
  - `found` for a static route, a route with a parameter, the catch-all,
    and no match;
  - `static_paths` with repeats, parameters and the catch-all.
- **Native programs in `run_tests.rs`** check:
  - `page`, and `page_with` with data holding `</script>`, `&` and
    U+2028, against goldens;
  - `document`, including its panic on a template with no root;
  - `serve` end to end, on port 0:
    - `/` gives `200` and the filled page;
    - `/app.js` and `/app.wasm` give their content types;
    - `/missing.png` and `/..%2fsecret` give `404`;
    - `/nowhere` gives `404` with the rendered page;
    - a `POST` gives `404`;
  - SSG mode writing its files, and refusing a bad path.
- **E0092** for `import ui::ssr` on WASM.
- **`std/http`'s unit tests** cover `fallback` and `listen_on`.

### 9.3 Under the fake DOM

- **The fake DOM gains:**
  - an `innerHTML` setter that parses the HTML `render_hydratable` writes:
    elements, double-quoted attributes, void elements, comments, and the
    entities `&amp;`, `&lt;`, `&gt;`, `&quot;` and `&#13;`;
  - comment nodes, whose `nodeName` is `#comment`;
  - capture of `console.error`.
- **Each hydration test** takes the HTML a native render of the same view
  writes, so the pair is tested end to end. It puts that HTML into the
  root, runs the WASM program, and checks:
  - the nodes are the same objects after hydration;
  - clicks update them;
  - regions build again after a change;
  - separators and empty texts are handled;
  - `hydrate_with` reads its data;
  - an empty root mounts with no report;
  - a mismatch reports its exact text and builds the view again.
- These tests run as they are and under GC stress.

### 9.4 The CLI

`crates/nova-cli/tests/` checks:
- `--ssr` writes `dist/` and the server;
- `--ssg` writes one page per static route, skips routes with parameters,
  and lists the pages in `manifest.json`;
- the four refusals.

### 9.5 In the browser (CI)

- **`tests/browser/ssr-app/`** is a project: `nova.toml`, `src/app.nova`,
  `src/main.nova` and `src/server.nova`. It has a counter, a `show`, a
  `for_each` over the server's data, a static route, a route with a
  parameter, and the catch-all.
- **`tests/browser/ssr.mjs`** builds it with `--ssg`, starts the server on
  port 0, and checks:
  - **A server-rendered page hydrates in place:**
    1. it opens `/` while holding back `app.wasm`, and keeps handles to
       the server's nodes;
    2. it lets `app.wasm` load and waits for hydration;
    3. the handles are the page's nodes still, a click updates them, and
       the console has no error.
  - **Other pages:**
    - a link goes to `/about` without a reload;
    - `/posts/2` hydrates;
    - `/nowhere` answers `404` with the not-found page.
  - **A statically generated page hydrates the same way.** It serves
    `dist/` with a static Node server, opens `/about/`, checks that its
    HTML was already rendered, and that it hydrates in place.
- **`06-counter-spa`'s size test** stays under 50 KB, and since 06 does
  not call `hydrate`, its size should not change.

### 9.6 Checks

- `cargo fmt`, clippy with `-D warnings`, and `nova fmt --check` over the
  new directories.
- Rust 1.78.
- `Cargo.lock` unchanged.

## 10. Crates and files

- **No new crate.** `Cargo.lock` does not change.
- **`std`:**
  - `std/ui/lib.nova`: `render_hydratable`, `hydrate`, `hydrate_with`;
  - `std/ui/backend/{wasm,native}.nova`;
  - `std/ui/router/lib.nova`: `with_location`, `found`, `static_paths`;
  - `std/ui/ssr/lib.nova`, new;
  - `std/dom/lib.nova` and its shim function: `console_error`;
  - `std/http/lib.nova`: `fallback`, `listen_on`;
  - `std/lib.rs`.
- **Crates:**
  - `nova-resolver`: native-only registration;
  - `nova-doc`: the module list;
  - `nova-driver`: the server's root;
  - `nova-cli`: `cmd/run.rs` for the flags, and `bundle.rs` for the SSG
    pages in the manifest;
  - `nova-codegen-wasm`: the shim function.
- **Tests:**
  - `tests/runtime/ui_hydratable*` and `ui_router_server*`;
  - `tests/dom/hydrate_*`, and `fake_dom.mjs`;
  - `tests/browser/ssr-app/` and `ssr.mjs`;
  - the new CLI tests;
  - CI's `browser` job.

## 11. Records

- **ADR 0045,** "SSR, hydration and SSG". It records:
  - decisions 1 to 12;
  - the markers;
  - the walk and the mismatch policy;
  - the data hand-off and its escaping;
  - the manifest as the server's list of files;
  - SSG through `serve`;
  - what is left out (§13).
- **Dated notes:**
  - `30-FRONTEND.md` §7.4 (the API and the flags) and §4 (a view must not
    show a `Map`'s order);
  - `40-TOOLING.md` (`nova build --ssr` and `--ssg`);
  - `20-STDLIB.md` (`std/http`'s two additions, and `ui::ssr`);
  - `00-MASTER-SPEC.md` Phase 4, items 8 and 9.
- **The phase plan's 4.6 entry** gets its amendment, as 4.0 to 4.5 have.
- **Also:** CHANGELOG, ARCHITECTURE, and the doc comments of every new
  function, each with an example that a doc test runs where one can.

## 12. Risks

1. **The browser's parser can build a different tree** from the HTML,
   for example a `p` closed early, or a `tbody` added.
   - The page still works: the mismatch builds it again.
   - But the server's work is wasted, and the console says so.
   - The records list the causes known (§4.3).
2. **A render that is not deterministic** mismatches every time, for
   example one that shows the time, a random value, or a `Map`'s order.
   - The records tell programs to pass such values as data (§6.2), and
     to sort.
3. **`render` is synchronous.**
   - A page cannot `await` its data inside it.
   - 4.6's data comes from memory.
   - Loading per request is 4.7's question.
4. **Hydration adds code** to every client that calls `hydrate`. The size
   test watches 06, which does not call it. `ssr-app`'s size is printed,
   not limited.
5. **A server binary is a second program to build.** That takes another
   native build, which is slow under `--release`. The CLI tests build
   once and reuse it.

## 13. Not in 4.6

- A development mode for SSR in `nova dev` (decision 10).
- Partial hydration, or islands. 4.7 decides whether the blog's comments
  need more than whole-page hydration.
- Streaming HTML, and asynchronous data in `render`.
- `POST`, cookies and path parameters in `std/http` (4.7).
- Pre-rendering routes with parameters; a list of paths given by the
  program.
- Checking attributes during hydration.
- A keyed `for_each`.
- A `nova serve` command.
- `nova check` of `src/server.nova` as a second root.

## 14. Decisions made while writing this spec

1. `render_to_string` does not change. Hydratable HTML has its own
   function, `render_hydratable`, in `std/ui`, beside the walk that reads
   it.
2. The markers are `<!--[-->`, `<!--]-->` and `<!--|-->`. A CR is written
   `&#13;`.
3. Hydration checks tags and texts, not attributes. It sets properties,
   and lets dynamic attributes write themselves.
4. An empty root, or one holding only whitespace, mounts with no report.
5. The data's element is `<script type="application/json"
   id="nova-data">`, after the root, and the root's id is `app`.
6. The router's server functions are `with_location`, `found` and
   `static_paths`.
7. `ui::ssr`'s names are `Page`, `page`, `page_with`, `document`, `Site`
   and `serve`. `Site.render` maps a location to a `Page`, so each page
   can choose its own data.
8. `serve` serves only the files `manifest.json` lists, answers only
   `GET`, and applies the `.` rule `nova dev` uses.
9. `std/http` gains `Server::fallback` and `Server::listen_on`.
10. SSG is `serve` run with `--ssg <dir>`. Each page is
    `<dir>/<path>/index.html`, and the CLI adds the pages to the manifest.
11. The server's executable is `target/<profile>/<name>-server`. `-o`
    names the client's directory only.
12. Mismatches are reported through a new `std/dom::console_error`.
