# Phase 4.5, "The router and `nova dev`": design

- **Date:** 2026-10-18.
- **Status:** approved by the user on 2026-10-18.
- **The user's decisions, 2026-10-18:**
  1. **The router keeps `30-FRONTEND.md` §6's names:** `component
     Router(routes: [Route])` and `record Route { path, component }`. A
     program writes `Router([Route { path: "/", component: Home }, …])`.
     A `link` makes the links.
  2. **A route's path holds `:name` parameters,** which `use_params()`
     reads. The route `"*"` matches every path that no route before it
     matches, such as a "not found" page.
  3. **Both modes, by two names:** `Router(routes)` routes by the URL's
     hash, and `HistoryRouter(routes)` by the history API. A `link`'s click
     navigates without reloading the page. `nova dev` serves `index.html`
     for a path with no file, so a page in history mode can be reloaded.
  4. **`nova dev` prints its URL, and opens a browser only with
     `--open`.**
  5. **The port is 3000 by default,** and `--port` changes it. When the
     port is taken, `nova dev` stops and says to use `--port`. Tests use
     `--port 0` and read the URL `nova dev` prints.
  6. **`nova dev` builds into `dist/`,** as `nova build --target wasm`
     does (phase plan decision 9). It adds its reload script to a page
     when it serves it, and never writes the script into a file.
  7. **Each rebuild runs `nova build --target wasm` as a child process.**
     Its stderr, without colour, is the overlay's text. A panic in the
     compiler cannot stop the server, and the driver does not change.
  8. **The router gets a browser test as well as the gate's:** a small
     router program under `nova dev`, in history mode, driven by
     Playwright through links, the back button and a reload.
- **Branch:** `claude/loving-archimedes-frtf5q-45`, from `main` at
  `75aa2d4`, as 4.4 used `-44`.
- **Inputs:**
  - `docs/phase-4-plan.md`:
    - the user's decisions of 2026-10-14;
    - §3, decisions 6, 9, 10 and 12;
    - §4's 4.5 entry;
    - §5's security rules;
  - `nova-spec/30-FRONTEND.md` §6 (the router), §8 (the dev server) and
    §13; `40-TOOLING.md`; `60-EXAMPLES.md` §6 and §7;
  - the 4.3 spec (`std/ui`) and the 4.4 spec (views, the bundle);
  - ADR 0038, which records `std::net`, live reload over Server-Sent
    Events, the overlay, and what Phase 4 leaves out.

## 1. What 4.5 delivers

- **`std/ui/router`,** imported as `import ui::router` (decision 6 of the
  phase plan: not in the prelude), on both targets:

  ```nova
  import ui
  import ui::html::{div, h1, p, text}
  import ui::router::{HistoryRouter, Route, link, use_params}

  component Home() {
      view {
          div {
              h1 { text("Posts") }
              link(to: "/posts/1") { text("The first post") }
          }
      }
  }

  component Post() {
      let id = match use_params().get("id") {
          Some(id) => id
          None => "?"
      }
      view { p { text("Post ${id}") } }
  }

  component NotFound() { view { p { text("Nothing here") } } }

  fn main() {
      mount(
          "#app",
          || HistoryRouter(
              [
                  Route { path: "/", component: Home },
                  Route { path: "/posts/:id", component: Post },
                  Route { path: "*", component: NotFound },
              ],
          ),
      )
  }
  ```

- **`nova dev`:**
  - builds the program into `dist/` and serves it on `127.0.0.1`;
  - watches its sources, and rebuilds when one changes;
  - then reloads the page, or, when the build fails, shows its errors over
    the page.
- **The gate,** in CI's `browser` job, through Playwright:
  - `06-counter-spa` under `nova dev` responds to clicks;
  - an edit of its source reloads the page with the change;
  - a type error shows the overlay, and its fix takes the overlay away.
- **The router's browser test,** in the same job (the user's decision 8).

## 2. The starting point (read on 2026-10-18, at `75aa2d4`)

- **`std/ui`:**
  - `View` and `Attr` are public enums.
  - A `Region(fn() -> [View])` renders its views again, alone, each time a
    signal they read changes. `show` and `for_each` are written as
    regions, and a router can be one too.
  - Its state, `UiState`, is a private record, kept for the thread or the
    page through three std-only builtins, `ui_state_present`, `_load` and
    `_store`.
  - A handler sees only the target's `value`, so no handler can call
    `preventDefault`.
- **`std/dom` already has what a router reads and writes:**
  - `location_pathname`, `location_search`, `location_hash` and
    `set_location_hash`;
  - `history_push` and `history_replace`;
  - `window_add_event_listener`, and `Event`'s `target` and
    `prevent_default`.

  `tests/dom/history.nova` drives them all under the fake DOM, which has
  `FakeLocation` and `FakeHistory`.
- **`std/ui`'s backend** is the internal module `$std.ui.backend`, with
  one file per target. A program cannot import it.
  - `import dom` is E0092 natively, so `std/ui` reaches the page only
    through the backend.
  - The native backend panics where a page is needed.
- **A module can import a module that imports it back.**
- **Nova has no re-export.**
- **The CLI:**
  - has no `dev` command, and the workspace has no HTTP server, watcher or
    crate for either;
  - its dependencies are clap, anyhow, tracing, semver and toml_edit;
  - `build_wasm` and `bundle.rs` write `dist/`;
  - the driver renders a build's errors to stderr with
    `ColorChoice::Auto`, which writes colour even into a pipe unless
    `NO_COLOR` is set.
- **The tests:**
  - `wasm_tests.rs` runs DOM programs under Node with the fake DOM, as is
    and under GC stress.
  - CI's `browser` job builds into `$RUNNER_TEMP`, installs Playwright
    1.56.1 there, and runs `tests/browser/*.mjs`, which serve `dist/`
    from a Node server of their own.
  - `run_tests.rs` starts servers on `127.0.0.1:0` and reads the port they
    print.
- **The component syntax limits a router's shape:**
  - In a view, a name with a named argument is an element, which must take
    `([Attr], [View])` (E0096).
  - So `Router(routes: […])` cannot be written there, and `Router([…])`
    can.

## 3. `std/ui/router`

### 3.1 The API

```nova
/// A route: a path pattern and the component it shows.
pub record Route {
    pub path: String
    pub component: fn() -> View
}

/// Routes by the URL's hash: `#/posts/1`.
pub component Router(routes: [Route])

/// Routes by the URL's path, through the history API: `/posts/1`.
pub component HistoryRouter(routes: [Route])

/// Goes to `to`, a path within the app such as `/posts/1?tab=2`.
pub fn navigate(to: String)

/// The parameters of the route being shown: `{"id": "1"}` for `/posts/:id`.
pub fn use_params() -> Map<String, String>

/// The query of the current location: `{"tab": "2"}` for `?tab=2`.
pub fn use_query() -> Map<String, String>

/// An `<a>` that navigates without reloading the page; `to` is its path.
pub fn link(attrs: [Attr], children: [View]) -> View
```

- **`Router` and `HistoryRouter`** return a `View`: a region that shows
  the component of the route matching the current location, and shows it
  again each time the location changes. They are written with
  `component`, so they are documented as functions (4.4, decision 12).
- **One router per page.**
  - A router mounted later takes over `navigate`, `use_params` and
    `use_query`.
  - Nested routers are not in 4.5.
- **The location** is a path within the app, with its query:
  - `/posts/1?tab=2`.
  - In hash mode, it is the hash without its `#`, and `/` when the hash
    is empty.
  - In history mode, it is `location.pathname` followed by
    `location.search`.
  - The app is served from `/`; a base path is not in 4.5.

### 3.2 Paths and matching

- **Splitting:** a location's path, before any `?`, is split at `/`.
  - Empty pieces are dropped, so `/`, `//` and `` are all the root, and
    `/posts/1/` is `/posts/1`.
  - Each piece is percent-decoded as UTF-8. A piece that does not decode
    is kept as written.
- **A pattern** is split the same way. Each piece is:
  - **a literal,** which matches the same piece exactly, case included;
  - **or a parameter,** `:name`, which matches any one piece and records it
    under `name`.

  The pattern `*`, alone, matches every location.
- **Matching:**
  - A route matches when its pattern has as many pieces as the path, and
    each piece matches.
  - The routes are tried in order, and the first that matches is shown.
  - When none matches, the router shows nothing.
- **The query** is the text after the first `?`. It is split at `&`.
  - Each part is `key=value`, or `key` with the value `""`.
  - Both halves are decoded: `+` is a space, then percent-decoding.
  - For a key given twice, the last value wins.

### 3.3 Navigation

- **`navigate(to)`:**
  - A `to` that does not begin with `/` gets one in front.
  - In hash mode, it sets the hash to `#` and `to`.
  - In history mode, it pushes `to` onto the history.
  - In both modes, it then sets the router's location, so the new route
    shows at once.
  - With no router mounted, it panics: `navigate: no router is mounted`.
- **The back and forward buttons:**
  - Each router listens for `hashchange` (hash mode) or `popstate` (history
    mode), and sets its location from the URL.
  - A hash typed in the address bar works the same way.
- **`link(attrs, children)`** is a builder of the shape an element needs,
  so a view writes `link(to: "/posts/1") { text("…") }`.
  - Its `to` attribute, fixed or reactive, becomes an `href`:
    - `#/posts/1` in hash mode;
    - `/posts/1` in history mode;
    - `to` as written when no router is mounted.
  - It also gets `data-nova-link`. Its other attributes pass to the `<a>`
    unchanged.
  - A link with no `to` panics: `link: no \`to\` attribute`.
  - Outside a view, `link([attr("to", "/posts/1")], [text("…")])`.
- **How a link's click navigates.** A `std/ui` handler cannot prevent the
  browser's default, so each router listens for `click` on the window.
  - The listener looks for the nearest `<a>` with `data-nova-link`,
    starting at the click's target and walking up.
  - It intercepts the click only when all of these hold:
    - such an `<a>` is found;
    - the click is a plain one: the main button, with no Ctrl, Meta, Shift
      or Alt;
    - the `<a>` has no `target` attribute.
  - Then it prevents the default and calls `navigate` with the `href`,
    less a leading `#` in hash mode.
  - Any other click is the browser's, so a link still opens in a new tab.
  - Hash mode intercepts too:
    - both modes then behave alike;
    - a hash link works under the fake DOM, which follows no link.
- **Cleanup:**
  - A router's listeners are removed when its owner is disposed.
  - The router's state is cleared then, unless a later router has taken
    over.

### 3.4 `use_params` and `use_query`

- **`use_params()`:**
  - returns the parameters of the matched route while the router renders
    that route's component;
  - returns an empty map at any other time, or with no router.
- **`use_query()`** returns the current location's query, or an empty map
  with no router.
- **They are not reactive,** and need not be: each change of location
  renders the matched component again, even when the route is the same,
  so a component reads both afresh.
  - The component's own state starts again on each change.
  - Keeping it across a change of parameters is not in 4.5.

### 3.5 The router's state

- **Why a slot of its own.** The router keeps three things for the page:
  - the location, a `Signal<String>`;
  - its mode;
  - the parameters of the route being rendered.

  `navigate`, `use_params` and `use_query` reach them. They cannot live in
  `std/ui`'s `UiState`, which is private to `std/ui`, and Nova has no
  re-export to move the router into `std/ui`'s own module.
- **So the router gets three std-only builtins of its own:**
  `router_state_present`, `router_state_load` and `router_state_store`.
  - They are made as `ui_state_*` are.
  - Natively, the state is a collector root for the thread.
  - On WASM, it is `std/rt`'s state word 112, which the collector marks.
    It is free, below `DATA_BASE` at 128.
  - A program cannot name them.

### 3.6 The backend

`$std.ui.backend` gains four functions on both targets:
- `backend_location(history: Bool) -> String`:
  - on WASM, from the hash or from the pathname and search;
  - natively, `/`.
- `backend_go(history: Bool, to: String)`:
  - on WASM, sets the hash or pushes the history;
  - natively, nothing.
- `backend_on_location(history: Bool, f: fn()) -> fn()`:
  - listens for `hashchange` or `popstate`, and returns the function that
    stops listening;
  - natively, it listens for nothing.
- `backend_on_link_click(f: fn(String)) -> fn()`:
  - the window's `click` listener of §3.3;
  - natively, nothing.

Each handler runs untracked and batched, as `std/ui`'s handlers do. `std/dom` gains
`Event.modified() -> Bool`, true when a modifier key is down or the button
is not the main one. It comes with:
- an `extern "js"` function;
- its shim function;
- the same in the fake DOM.

### 3.7 Natively

- **The router compiles natively,** so a doc test can render it, and 4.6
  can render it on a server.
- **Natively, the location is `/`,** and `navigate` sets the router's
  location without any history.
  - `render_to_string(|| Router(routes))` writes the root route's HTML.
  - A `link` writes `<a href="#/…" data-nova-link>`.
- **Rendering a given path natively** is 4.6's.

## 4. `nova dev`

### 4.1 The command line

```
nova dev [FILE] [--port <PORT>] [--open] [-o <DIR>]
```

- **`FILE`, `-o` and the output directory** follow `nova build --target
  wasm`:
  - a project's `dist/` at its root, or `./dist` for a single file;
  - the same refusals of an output directory that holds the project or
    lies inside `public/`;
  - the same refusal of a library.
- **`--port`:**
  - 3000 by default;
  - `0` lets the system choose.
- **`--open`** opens the URL in the system's browser, after the first
  build:
  - `xdg-open` on Linux, `open` on macOS, `cmd /c start` on Windows;
  - a failure to start it is a note on stderr, not an error.

### 4.2 What it does, in order

1. **It resolves the program and the output directory,** and refuses
   what `nova build --target wasm` refuses.
2. **It binds `127.0.0.1:<port>`.**
   - When the port is taken, it prints `nova dev: port 3000 is in use;
     choose another with --port` and exits with status 1.
3. **It builds once** (§4.3), whether or not that build succeeds.
4. **It prints one line on stdout,** `nova dev: serving
   http://127.0.0.1:<port>/`, with the port it bound. Tests read this
   line.
5. **It serves** (§4.4), and watches (§4.6), until it is stopped with
   Ctrl-C.

### 4.3 Builds

- **Each build is a child process:** the running `nova` binary,
  `std::env::current_exe()`, with `build --target wasm`, the same `FILE`,
  and `-o` the output directory.
  - It runs with `NO_COLOR=1`, its stdout and stderr captured.
  - The child writes `dist/` exactly as `nova build --target wasm` does,
    `manifest.json` included.
- **A build that succeeds:**
  - prints `rebuilt in <ms> ms` on stderr;
  - sends `reload` to every page (§4.5).
- **A build that fails:**
  - prints the child's stderr on `nova dev`'s stderr;
  - sends `error`, with that text, to every page;
  - leaves the previous build in `dist/`, as a failed `nova build`
    already does.
- **One build at a time.** A change seen during a build starts one more
  build after it.

### 4.4 The server

- **The basics:**
  - HTTP/1.1, written on Rust's `std::net` with a thread per connection,
    with no crate (ADR 0038).
  - It binds `127.0.0.1` only.
  - Every response but the event stream closes its connection.
- **Requests:**
  - `GET` and `HEAD` are served; any other method is `405`.
  - A request whose head is over 16 KiB, or does not parse, is `400`.
- **The `Host` header** must be `127.0.0.1:<port>` or `localhost:<port>`.
  Any other is `403`, so a page on another site cannot read the
  dev server through a name that resolves to `127.0.0.1` (DNS
  rebinding).
- **A request's path:**
  - is taken before any `?`, and percent-decoded;
  - is `403` when a piece is `..`, or the path holds a `\` or a NUL;
  - is `403` when the file it names lies outside the output directory once
    links are resolved.
  - A path ending in `/` names that directory's `index.html`.
- **Serving files:**
  - A file is served with its type by extension: `html`, `js`, `wasm`
    (`application/wasm`, which `instantiateStreaming` needs), `json`,
    `css`, `svg`, `png`, `jpg`, `gif`, `ico`, `txt`, `woff2`. Any other
    extension is `application/octet-stream`.
  - Every response has `Cache-Control: no-store`.
  - A directory is never listed.
- **The fallback for history mode:**
  - It applies to a `GET` or `HEAD` for a path that names no file, whose
    last piece has no `.`.
  - Such a request gets the output directory's `index.html`, `200`, so
    `/posts/2` can be reloaded.
  - Any other missing file is `404`.
- **The reload script:**
  - Every HTML response gets `<script src="/__nova/reload.js"></script>`
    before its last `</body>`, or at its end when it has none.
  - `Content-Length` counts it. The file on disk is not changed.
- **Paths under `/__nova/`** are the server's own, and never read from
  disk:
  - `/__nova/reload.js`, the script of §4.5;
  - `/__nova/events`, the event stream of §4.5.
- **Before the first good build:** when the output directory has no
  `index.html`, `/` gets a page of the server's own, with the script, so
  the first build's errors show.

### 4.5 Reload and the overlay

- **`/__nova/events`** is a stream of Server-Sent Events,
  `text/event-stream`. It sends three events:
  - **`hello`,** on connecting. Its data is the current build's number,
    which counts the builds from 1. When the last build failed, its
    `error` follows at once.
  - **`reload`,** after a build that succeeds, with the new number.
  - **`error`,** after a build that fails, with the child's stderr, one
    `data:` line per line.
- **Keeping the stream alive:**
  - A comment, `: ping`, goes every 15 seconds.
  - A page whose write fails is dropped.
- **`/__nova/reload.js`:**
  - opens an `EventSource` on `/__nova/events`;
  - reloads the page on `reload`;
  - on `error`, shows the overlay.
  - It also reloads on a `hello` whose number differs from the first one
    it saw. That covers a missed event, and a server that was restarted.
- **The overlay** is a fixed `<div id="nova-dev-overlay">` over the whole
  page:
  - its title is `nova dev: the build failed`;
  - the text is in a `<pre>`, set as `textContent`, so no error text is
    read as HTML;
  - it stays until the next good build reloads the page.
- **The page's state starts again on each reload.** HMR is not in Phase 4
  (the user's decision of 2026-10-14).

### 4.6 Watching

- **Polling:** every 200 ms, `nova dev` reads the modification time and
  size of the program's files.
- **For a project, those files are:**
  - every `.nova` file under its root, skipping `target/`, the output
    directory and directories whose names begin with `.`;
  - `nova.toml` and `index.html`;
  - everything under `public/`.
- **For a single file,** they are the `.nova` files of its directory, by
  the same rules.
- **When a file is added, removed or changed:**
  - `nova dev` waits for one more poll that sees nothing new, so an editor
    that writes twice builds once;
  - then it builds.
- **The output directory is never watched,** so a build does not trigger
  itself.

## 5. Testing

### 5.1 Unchanged rules

- Tests never touch the internet. Every server binds `127.0.0.1:0`.
- A test that needs `NOVA_HOME` sets it to a fresh directory.
- Temporary directories have fixed names.
- Browser tests run in CI's `browser` job, never in `cargo test`.
- A test that needs Node fails without it under `NOVA_REQUIRE_NODE=1`.

### 5.2 The router

- **Under the fake DOM,** run as is and under GC stress:
  - **`tests/dom/router_hash.nova`,** driven by `router_hash.mjs`, prints
    what shows:
    - at `/`;
    - after a link's click;
    - after `navigate` with a query;
    - after the back button;
    - at a path no route matches but `*`;
    - with a percent-encoded parameter.
  - **`tests/dom/router_history.nova`** does the same with `HistoryRouter`,
    through `pushState` and `popstate`. It also checks that clicks are
    left to the browser for:
    - a click with a modifier;
    - a link with `target`;
    - a plain `<a>` without `data-nova-link`.
- **Natively:** doc tests in `std/ui/router` render the root route, a
  `link`, and a router with no match, through `render_to_string`.
- **The builtins:** the resolver's tests of `STD_ONLY` cover the three new
  ones, and `nova-runtime` tests the native slot as it tests `ui_state`.

### 5.3 `nova dev`

**`crates/nova-cli/tests/dev_tests.rs`** starts `nova dev --port 0` in a
fresh copy of a small project, reads the URL it prints, and speaks HTTP
over `TcpStream`. The child is killed when the test ends, pass or fail.
It checks:
- **Pages and files:**
  - `/` is the project's page, with the reload script added;
  - `/app.wasm` is `application/wasm`, with `dist/`'s bytes;
  - `HEAD` has no body, and `POST` is `405`.
- **What is refused:**
  - `/../nova.toml`, `/%2e%2e/nova.toml` and a link out of `dist/` are
    `403`;
  - a foreign `Host` is `403`.
- **The fallback:** `/posts/2` is the page, and `/missing.js` is `404`.
- **The event stream:**
  - it says `hello`;
  - an edited source sends `reload`;
  - a type error sends `error` with the checker's message;
  - the fix sends `reload`.
- **The port:** a second `nova dev` on the first one's port fails with the
  `--port` message.

Unit tests in the server's module cover:
- the request path's resolution and refusals;
- the content types;
- where the script goes into a page;
- the `Host` check;
- an error's lines as events.

### 5.4 In the browser (CI)

- **`tests/browser/dev.mjs`** copies `examples/06-counter-spa` into
  `$RUNNER_TEMP` and starts `nova dev --port 0` there. Through Playwright:
  1. It clicks `+` and `-` and reads the count.
  2. It edits `src/main.nova`, from `Count:` to `Clicks:`, and waits for
     the page to reload showing `Clicks: 0`.
  3. It writes a type error, and waits for `#nova-dev-overlay` with the
     checker's message.
  4. It restores the file, and waits for the overlay to go and the page
     to count again.
- **`tests/browser/router.mjs`** starts `nova dev` on
  `tests/browser/router-app`, a small project with `HistoryRouter`. It
  sets a marker on `window` that a full reload would lose. Then:
  1. It clicks a link: the URL is `/posts/2`, the post shows, and the
     marker is still there.
  2. It goes back: the home page shows.
  3. It reloads at `/posts/2`: the fallback serves the page, and the post
     shows.
  4. It opens `/nowhere`: the "not found" route shows.
  5. It follows a link with a query: `use_query` reads it.
- **The `browser` job** runs both, beside the 4.2 to 4.4 tests it runs
  now.

### 5.5 Checks

- fmt, clippy with `-D warnings`, and `nova fmt --check` over `std`,
  `examples`, `tests/wasm`, `tests/dom` and the new programs;
- the full suite, and Rust 1.78's check;
- `Cargo.lock` unchanged;
- mutants for the router's matching and clicks, the server's refusals,
  the fallback, and the events.

## 6. Crates and files

- **New:**
  - `std/ui/router/lib.nova`;
  - `crates/nova-cli/src/cmd/dev.rs` (the command, the watcher and the
    builds), and a module for the server and the event stream;
  - `crates/nova-cli/src/dev/reload.js`, embedded with `include_str!`;
  - `crates/nova-cli/tests/dev_tests.rs`;
  - `tests/dom/router_{hash,history}.{nova,mjs,stdout}`;
  - `tests/browser/dev.mjs`, `router.mjs` and `router-app/`.
- **Changed:**
  - `std/lib.rs`, the resolver's lists of std modules (`STD_MODULES`,
    `WASM_STD_MODULES`, `IMPORTED_STD`) and `nova-doc`'s;
  - the three builtins:
    - the resolver's `Builtin` and `STD_ONLY`, and the checker's
      signatures;
    - native codegen and `nova-runtime`;
    - WASM codegen and `std/rt`, with its word 112 and its marking;
  - `std/ui/backend/{wasm,native}.nova`;
  - `std/dom/lib.nova`, the shim and `tests/dom/fake_dom.mjs`
    (`Event.modified`);
  - `crates/nova-cli/src/main.rs`;
  - `.github/workflows/ci.yml`'s `browser` job;
  - `crates/nova-driver/tests/index_complete.rs`'s counts.
- **No new crate. `Cargo.lock` does not change.**

## 7. Records

- **ADR 0044, "The router and `nova dev`",** records the departures:
  - **from `30-FRONTEND.md` §8:**
    - live reload over Server-Sent Events, not HMR over a WebSocket;
    - `std::net`, not `axum`;
    - a full build in a child process, not an incremental one;
    - no source maps;
    - `--open`, not a browser opened by default;
  - **from §6:**
    - `HistoryRouter` as the flag;
    - `link` as a builder with a `to` attribute;
    - `*` and `:name`;
    - the state behind three std-only builtins;
    - one router per page.
- **Dated notes:**
  - `30-FRONTEND.md` §6, §8 and §13;
  - `40-TOOLING.md`'s `nova dev`;
  - `60-EXAMPLES.md` §6: the gate is now `nova dev`.
- **The phase plan's 4.5 entry,** the CHANGELOG and ARCHITECTURE.
- **`examples/06-counter-spa/README.md`** runs it with `nova dev`.

## 8. Risks

- **Watching by polling** misses nothing a time and size can show.
  - A file system whose times are coarse could miss an edit that keeps the
    size and lands in the same tick.
  - The tests change sizes.
  - It costs a scan every 200 ms, small for a project of this size.
- **`dev_tests.rs` runs on all three systems,** with a child process, a
  port and a watcher.
  - Its waits poll with generous limits, 30 seconds, and fail with what
    was read.
  - Windows spawns `current_exe()` like the others, and its paths go
    through the same refusals; the CI run is what proves them.
- **The three builtins** touch both back ends.
  - They copy `ui_state_*`, which are already tested on both.
  - The collector must mark word 112, or a router's state would be freed.
    The GC stress runs catch that.
- **A record field named `component`** (`Route.component`):
  - `component` is a keyword only where an item starts.
  - The prototype checks a field and a field initialiser of that name.
- **Port 3000** is the default the user chose, and `examples/03` uses it
  too. No test uses it.
- **A page of another site** can still send requests to `127.0.0.1` without
  reading their answers. `nova dev` changes nothing on a `GET`, so this
  is harmless.

## 9. Not in 4.5

- **For the router:**
  - HMR, and state kept across a reload;
  - nested routers and layouts;
  - guards, lazy routes and a base path for history mode;
  - scroll restoration;
  - an accessor for the current location;
  - rendering a given path natively, which is 4.6's.
- **For `nova dev`:**
  - serving a native or server-rendered program, which is 4.6 and 4.7's;
  - an overlay for a runtime panic;
  - HTTPS, and access from other machines (`--host`);
  - source maps.

## 10. Decisions made while writing this spec

1. **The router's state is behind three std-only builtins** (§3.5). It
   cannot live in `UiState`, which is private to `std/ui`, and Nova has no
   re-export. `ui_state_*` are the precedent.
2. **`link` is a builder, `([Attr], [View]) -> View`, with `to` as an
   attribute,** so it is an element in a view. A reactive `to` gives a
   reactive `href`.
3. **Both routers intercept a plain click on a `data-nova-link` anchor**
   through one listener on the window (§3.3). A `std/ui` handler cannot
   prevent the default. Hash mode then behaves as history mode does, and
   a hash link works under the fake DOM.
4. **Natively, the location is `/`,** and `navigate` changes the router's
   location only (§3.7).
5. **`navigate` with no router panics,** as `mount` does with no element.
6. **The server checks `Host`,** against DNS rebinding (§4.4).
7. **`hello` carries the build's number,** so a page reloads after a
   missed event or a restarted server (§4.5).
8. **The fallback applies only to a path whose last piece has no `.`,** so
   a missing script stays a `404` and is not answered with HTML.
9. **Polling every 200 ms, by time and size,** waiting one quiet poll
   before a build (§4.6).
10. **`Event.modified()` joins `std/dom`,** so a click with a modifier is
    left to the browser.
