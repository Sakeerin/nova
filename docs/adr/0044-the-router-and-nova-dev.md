# ADR 0044 — The router and `nova dev`

## Status

Accepted, 2026-10-18 (Phase 4.5, branch
`claude/loving-archimedes-frtf5q-45`; spec
`docs/superpowers/specs/2026-10-18-phase-4-5-router-and-nova-dev-design.md`).
`30-FRONTEND.md` §6's names, `Router(routes)` and `Route`; `:name`
parameters and the route `*`; `Router` by the hash and `HistoryRouter` by
the history API; the URL printed and a browser opened only with `--open`;
port 3000, changed by `--port`; builds into `dist/`; each rebuild a child
process; and a browser test of the router are the user's decisions of
2026-10-18. That the generated page loads `/app.js` by its absolute path is
the user's decision 9, of the same day, made after the prototype's
fact-check; that a link's `to` is always made a path within the app, with
or without a router, is the user's decision 10, of 2026-10-19, made after
the review.

## Context

After 4.4 a program could write a page as a component and build it into
`dist/`, but had no routing and no dev server. `nova-spec/30-FRONTEND.md`
§6 sketches a router, `component Router(routes: [Route])` with `navigate`,
`use_params` and `use_query`, "hash-based routing by default; HTML5 history
API via flag". §8 sketches a dev server: a watcher, an incremental
recompile, HMR over a WebSocket, an error overlay and source maps, on
`axum`. ADR 0038 already scoped Phase 4's `nova dev` to `std::net` and a
reload over Server-Sent Events. The repository 4.5 starts from has:
- `std/ui`'s state, a private record behind three std-only builtins, and
  no re-export, so a router can neither share that state nor move into
  `std/ui`'s own module;
- handlers that see only the target's `value`, so no handler can prevent a
  link's default;
- `std/dom`'s `location`, `history`, window listeners and `Event`, reached
  by `std/ui` only through its backend, one source per target, which a
  program cannot import;
- a view in which a name with a named argument is an element taking
  `([Attr], [View])` (E0096), so `Router(routes: […])` cannot be written
  there and `Router([…])` can;
- a CLI with no HTTP server and no watcher, whose dependencies are clap,
  anyhow, tracing, tracing-subscriber, semver and toml_edit;
- and a driver that writes colour into a pipe unless `NO_COLOR` is set.

## Decision

1. **The router is `std/ui/router`,** which a program imports with `import
   ui::router`, on both targets: `record Route { path, component }`,
   `Router(routes)`, which routes by the hash (`#/posts/1`),
   `HistoryRouter(routes)`, which routes by the path (`/posts/1`) through
   the history API, `navigate`, `use_params`, `use_query` and `link`. Both
   routers are written with `component`. In hash mode the location is the
   hash without its `#`, `/` when there is none; in history mode it is the
   path and the query. Natively it is `/`, and `navigate` sets the router's
   location alone. The location never holds a `#fragment`: what follows its
   first `#`, which reaches it in hash mode as a second `#` (through
   `navigate`, a `link` or a typed hash) and natively through `navigate`,
   is dropped before the location is set, so before
   any matching and before the query is read; `/posts/6#sec` shows post `6`
   and `/q?a=1#x` has `a = 1`.
2. **Paths.** A location's path, before any `?`, and a pattern are both
   split at `/`, empty pieces dropped, and each piece percent-decoded as
   UTF-8, or kept as written when it does not decode. A pattern's piece is
   a literal, matched with its case, or `:name`, which matches any one piece
   and records it for `use_params`. A pattern whose only piece is `*`, as
   `*` and `/*` are, matches every location. The first route that matches
   is shown, and none shows nothing. The query, after the first `?`, is
   split at `&`; an empty part is skipped, a part is `key=value` or `key`
   with the value `""`, `+` is a space before percent-decoding, and a key's
   last value wins.
3. **How a router shows a route.** A router is a `memo` of the route that
   matches its location, a `Signal<String>`, feeding a `for_each` of that
   one match or none; `for_each` builds the component untracked, and builds
   it again each time the location is set, even to the same route. So the
   component's own state starts again, and `use_params`, which reads the
   parameters while the router builds the component, and `use_query` need
   not be reactive. `navigate(to)` puts one `/` before `to` (decision 5),
   sets the hash or pushes the history, then sets the location to the one
   the page reports afterwards, which `backend_go` returns (natively `to`),
   less any fragment; it panics with no router. A browser writes the
   address its own way, `#/posts/café` as `#/posts/caf%C3%A9` and a space
   as `%20`, so the location read back is the one the `hashchange` that
   `navigate`'s own hash causes a turn later finds. The `hashchange` or
   `popstate` listener sets the location only when it differs, so that
   `hashchange` does not build the route twice, however the browser
   re-encodes the hash. In history mode a `to` that is the location
   already replaces the current history entry instead of pushing one, so a
   second click on the same link adds no entry: once its fragment is
   dropped, it has the same pieces of the path and the same parts of the
   query, in order, each piece and each part's name and value
   percent-decoded on its own, so `/posts/café` is the location
   `/posts/caf%C3%A9`. A delimiter written percent-encoded is not the
   delimiter, so `/a%2Fb` is not `/a/b`, nor `?q=a%26b` `?q=a&b`, and going
   from one to the other pushes an entry. In hash mode, setting the hash it
   already has adds none, as a browser does nothing for the same hash.
4. **Handlers run through `std/ui`'s public API.** Each listener the router
   adds runs its function as `batch(|| root(|d| { f(); d() }))`: batched,
   and with no listener, since `root` runs its function with none and its
   owner, given nothing to own, is disposed at once. `std/ui` does not
   change, so `import ui` binds no new name.
5. **Links.** `link(attrs, children)` is a builder of an element's shape,
   so a view writes `link(to: "/posts/1") { … }`. Its `to`, fixed or
   reactive, becomes an `href` that is always a path within the app:
   `#/posts/1` under `Router`, and `/posts/1` under `HistoryRouter` and
   with no router, with one `/` before `to` (the user's decision 10). Every
   tab, LF and CR is dropped first, as a URL's parser deletes them before
   it reads a URL, then a leading run of `/` and `\` becomes that one `/`,
   in an `href` and in `navigate` alike, so `link(to: "//posts/8")` is
   `href="/posts/8"` or `href="#/posts/8"`, and `/\t/evil.example/x` is
   `/evil.example/x`: never a protocol-relative URL naming another host,
   which a Ctrl-click or a middle click would follow there, and which a
   plain click would hand to `pushState`, which throws. The `href` follows
   the router mounted last: it reads a signal in the router's state, set
   when a router makes itself the one mounted last and when a router's
   disposal leaves none, so a link built before any router mounts, as a
   menu beside `show(cond, || Router(routes))` or before `Router(…)` in the
   same view, takes the router's form while it is shown and loses it once
   it is hidden. A signal has no owner, so the state makes it when it is
   first asked for. The `<a>` also gets `data-nova-link`, which
   `render_to_string` writes as `data-nova-link=""`. A link with no `to`
   panics. Each router listens for `click` on the window: the nearest `<a>`
   with `data-nova-link` at or above the target, clicked with the main
   button and no Ctrl, Meta, Shift or Alt, and with no `target` and no
   `download` attribute, has its default prevented and its `href` navigated
   to, less a leading `#` in hash mode. Only the router mounted last acts on
   such a click: the backend prevents the default only when the router's
   `take` returns true, which it does when its router is the one mounted
   last, and prevents it before it calls the router's `go`, so a panic on
   the way cannot hand a taken click back to the browser; a router still on
   the page after a later one was disposed, which leaves none mounted last,
   leaves the click to the browser. Any other click is the browser's, a
   link with `download` included, whose click saves its target. `std/dom`
   gains `Event.modified()`, which tells such a click apart.
6. **The router's state is behind three std-only builtins,**
   `router_state_present`, `_load` and `_store`, made as `ui_state_*` are:
   natively a second collector root for the thread, on WASM `std/rt`'s
   state word 112, which the collector marks. It holds the router mounted
   last, with its mode and location, the parameters of the route being
   built, the next router's number, and the signal every link's `href`
   reads, set each time the router mounted last changes. A page has one
   router: a router mounted later takes over `navigate`, `use_params` and
   `use_query`. A router's listeners are removed when its owner is
   disposed, and the state is cleared then unless a later router has taken
   over. `std/ui`'s backend
   gains `backend_location`; `backend_go` and `backend_replace`, which
   return the location the page reports afterwards; `backend_on_location`;
   and `backend_on_link_click`, whose `take` returns whether it takes the
   click. Natively they give `/`, go nowhere and return `to`, and listen
   for nothing.
7. **`nova dev [FILE] [--port <PORT>] [--open] [-o <DIR>]`.** `FILE`, `-o`
   and the output directory are `nova build --target wasm`'s, and so are
   its refusals, made before a port is bound: a library, an output
   directory that holds the project or lies inside `public/`, and a package
   in nova's cache. It binds `127.0.0.1:<port>`, 3000 by default, `0`
   letting the system choose; a port in use prints `nova dev: port <port>
   is in use; choose another with --port` and exits with status 1. Before
   it binds a port other than `0`, it connects to `127.0.0.1:<port>`,
   waiting 200 ms at most, and a connection made is a port in use too: on
   macOS, `SO_REUSEADDR`, which std sets on Unix, lets `127.0.0.1:<port>`
   bind beside a listener on `0.0.0.0:<port>`. A refusal or no answer in
   time leaves the port free. It
   builds once, prints `nova dev: serving http://127.0.0.1:<port>/` on
   stdout, and with `--open` runs `xdg-open`, `open` or `cmd /c start`, a
   failure of which is a note.
8. **Builds.** Each build runs the running binary as `build --target wasm
   [FILE] -o <out>`, with `NO_COLOR=1` and `RUST_LIB_BACKTRACE=0`, so an
   inherited `RUST_BACKTRACE` puts no backtrace of an error into the
   overlay; a panic keeps its own. A good build prints the child's notes,
   then `rebuilt in <ms> ms`, the first build included, and sends `reload`;
   a failed one prints the child's stderr and sends `error` with it, and
   leaves the previous build in `dist/`. One build runs at a time, and a
   change seen during one starts one more after it, however many writes it
   was. A panic in the compiler
   cannot stop the server, and the driver does not change. The watcher
   polls times and sizes every 200 ms: for a project, every `.nova` file
   under its root, skipping directories named `target` or beginning with
   `.`, then `nova.toml`, `index.html` and everything under `public/`; for
   a single file, the file itself and the `.nova` files of its directory,
   not below it, since its program cannot import a module from a directory
   below its own (E0900), so `nova dev ~/x.nova` does not read a whole tree
   every 200 ms. The output directory is never read. A build starts after
   one more poll that sees nothing new.
9. **The server** is HTTP/1.1 on `std::net`, a thread per connection, bound
   to `127.0.0.1`. Every response has `Cache-Control: no-store`, and every
   one but the event stream's closes its connection. A request is checked
   in this order:
   - a head over 16 KiB, one that does not parse or is not UTF-8, and one
     with two `Host` headers are `400`; a head's lines may end in CRLF or in
     a bare LF, which RFC 9112 §2.2 lets a recipient take as a line end,
     and it ends at its first empty line;
   - no `Host`, or one that is not `127.0.0.1:<port>` or
     `localhost:<port>`, is `403`, against DNS rebinding, before the method
     is looked at; on port 80, which a browser leaves out of `Host`,
     `127.0.0.1` and `localhost` alone name the server too;
   - a method other than `GET` and `HEAD` is `405`;
   - a target not starting with `/`, bad percent-encoding, and a path that
     does not decode to UTF-8 are `400`;
   - a `..` piece, a `\` or a NUL, a piece the system does not read as one
     plain name (such as `C:` on Windows), and a file outside the output
     directory once links are resolved are `403`.

   A path ending in `/` names that directory's `index.html`; a file is
   served with its type by extension, `application/wasm` included, and
   `.mjs` as `text/javascript` as `.js` is; a directory is never listed. A
   path that does not end in `/` and names a directory inside the output
   directory that holds an `index.html`, itself inside once links are
   resolved, is `307 Temporary Redirect`, which a browser does not keep,
   with an empty body: its `Location` is `/`, then each piece of the path
   as written, still percent-encoded, that is neither empty nor `.`, each
   followed by `/`, then the target's query, so `/docs` and `//docs` go to
   `/docs/`, `/docs?x=1` to `/docs/?x=1`, and no `Location` begins with
   `//`. A path that names no file, whose last piece has no `.`, gets the
   output directory's `index.html`, so a `HistoryRouter` page can be
   reloaded, with the generated page or one that loads `/app.js` by its
   absolute path (decision 11); a directory without `index.html` falls back
   the same way, by the last piece of the URL. While the output directory has
   no `index.html`, the server's own page answers `/` and those paths
   alike, so the first build's errors show. Any other missing file is
   `404`. `/__nova/reload.js` and `/__nova/events` are the server's own.
10. **Reload and the overlay.** Each build's id is `<start>.<number>`: the
    server's start in milliseconds since the Unix epoch, and the build's
    number, which every build, good or failed, takes in turn. Every HTML
    response gets `<script src="/__nova/reload.js" data-build="<id>"></script>`
    before its last `</body>`, or at its end, with the id current when the
    page is served, read before the page; `Content-Length` counts it, and
    the file on disk does not change. `/__nova/events` sends `hello` with
    the current id on connecting, followed by the last build's `error` when
    it failed; `reload` with the new id after a good build; `error` with
    the child's stderr, one `data:` line per line, after a failed one; and
    `: ping` every 15 seconds. A page whose write fails is dropped. `HEAD
    /__nova/events` gets the stream's head with `Connection: close`, and its
    connection closes. The script reloads the page on `reload`, and
    on a `hello` whose id is not its own tag's, which covers a page that
    first connects after a newer build than the one it was served with, and
    every page after a restart; on `error` it shows `<div
    id="nova-dev-overlay">`, titled `nova dev: the build failed`, the text
    in a `<pre>` as `textContent`, until a good build reloads the page. A
    page served after a failed build carries that build's id, so it shows
    the error without reloading again.
11. **The generated page loads `/app.js` by its absolute path** (the
    user's decision 9), as Vite's base `/` does, so a `HistoryRouter` page
    reloads at any path under `nova dev`, or under any server that serves
    the output directory from `/`; a relative `./app.js` under `/posts/1`
    would be `/posts/app.js`. A project's own `index.html` must load
    `/app.js` the same way for that, and an app deployed under a sub-path
    writes its own page. `tests/browser/router-app` has no page of its own,
    so `router.mjs`'s reload at `/posts/2` proves it; `06-counter-spa`
    keeps its own page. `HistoryRouter`'s doc comment says so.

## What departs from `30-FRONTEND.md`

- **From §8:**
  - live reload over Server-Sent Events, not HMR over a WebSocket;
  - `std::net`, not `axum`;
  - a full build in a child process, not an incremental one;
  - no source maps;
  - a browser opened only with `--open`, where `60-EXAMPLES.md` §6's gate
    has `nova dev` open one.
- **From §6:**
  - `HistoryRouter` as the flag, a second name beside `Router`;
  - `link`, a builder with a `to` attribute, for the links;
  - `*` and `:name` in a route's path;
  - the state behind three std-only builtins;
  - one router per page.

## What departs from the spec, or fills it in

- **The reload script's tag** is `<script src="/__nova/reload.js"
  data-build="<id>"></script>`, not the spec's tag without an id, and
  `hello` and `reload` carry the id `<start>.<number>`, not the number. The
  script compares a `hello` with its own tag's id, not with the first
  `hello` it heard, a comparison that never reloads a page whose first
  `hello` already names a newer build than the one it was served with
  (decision 10).
- **The router's listeners** are wrapped in `batch` and `root` by the
  router, through `std/ui`'s public API, rather than run untracked by the
  backend; the location listener compares before it sets (decisions 3, 4).
- **A link's `href`** follows the router mounted last, through a signal
  in the router's state, where spec §3.3 read it once. It is a path within
  the app with a router and without one: one `/` is put before `to`, every
  tab, LF and CR dropped and a leading run of `/` and `\` collapsed into
  it. Spec §3.3 kept `to` as written with no router; the user's decision
  10 departs from it, since an `href` must never name another host, as
  `//evil.example/x` or `/\t/evil.example/x` as written would, for a
  Ctrl-click, a middle click or a click while no router is mounted to
  follow. `render_to_string` writes `data-nova-link=""` (decision 5).
- **The location** is read back after going: `backend_go` returns the
  location the page reports, where spec §3.6 has it return nothing, so a
  hash the browser re-encodes builds its route once; it never holds a
  `#fragment`; and in history mode a `to` that is the location already
  replaces the entry through a fifth backend function, `backend_replace`,
  comparing the pieces of the path and the parts of the query, each
  decoded on its own, so that an encoded delimiter is not the delimiter
  (decisions 1, 3, 6).
- **`backend_on_link_click`'s `take` returns whether it takes the
  click,** where spec §3.6 has `fn(String)`, and the default is prevented
  only then, and before the router's `go` runs, so a router that is not the
  one mounted last leaves the click to the browser. A link with `download`
  is the browser's, as one with `target` is (decision 5).
- **Matching:** `/*` is the wildcard as `*` is, patterns are percent-decoded
  as locations are, empty query parts are skipped, and only the router
  mounted last acts on a link's click (decisions 2, 5).
- **Builds:** the child also gets `RUST_LIB_BACKTRACE=0`; a good build
  prints the child's notes before `rebuilt in`; a single file is watched
  itself as well as the `.nova` files of its directory, but not those below
  it, where spec §4.6 applies the project's rules; and `nova dev` refuses a
  package in nova's cache, as `nova build` does (decisions 7, 8).
- **The server:** its own page answers history paths too, a directory
  without `index.html` falls back by the last piece, and one with an
  `index.html` named without its last `/` is a `307` to its path with one;
  `HEAD /__nova/events` gets the stream's head with `Connection: close`,
  `.mjs` is `text/javascript`, a head's lines may end in a bare LF, a
  `Host` without a port names the server on port 80, and the refusals of
  decision 9 beyond the spec's, with the `Host` check before the method
  check (decision 9). Before it binds a port, `nova dev` probes it with a
  connection, which finds a listener on `0.0.0.0` that the bind does not
  on macOS (decision 7).
- **The generated page** loads `/app.js`, where 4.0's loaded `./app.js`,
  so the user's decision 3, a page in history mode reloaded under
  `nova dev`, holds for it (decision 11).

## Consequences

- **A component's own state starts again** on every change of location,
  even to the same route with other parameters.
- **The generated page assumes the app is served from `/`:** it loads
  `/app.js` by the absolute path, so a page in either mode reloads at any
  path, and an app deployed under a sub-path writes its own `index.html`.
- **Each rebuild is a full build** in a fresh process, so a panic in the
  compiler shows as a failed build and the server keeps serving; an
  incremental build could later replace the child without changing what a
  page hears.
- **Killing `nova dev`** other than by Ctrl-C at a terminal, which reaches
  the whole process group, such as `kill` or a test's `child.kill()`,
  leaves a build that is running to finish on its own: the child is not
  killed with its parent, and writes the output directory and exits.
- **Watching by polling** misses nothing a time and a size can show; a file
  system with coarse times could miss an edit that keeps the size within
  one tick.
- **A page of another site** can still send requests to `127.0.0.1` without
  reading the answers; `nova dev` changes nothing on a `GET`.
- **A page-wide `<base target=…>` is not looked at:** a plain click on a
  link under one is still taken, and stays in the page.
- **No crate is added,** and `Cargo.lock` does not change.

## What is left out

- **For the router:** HMR, and state kept across a reload; nested routers
  and layouts; guards, lazy routes and a base path for history mode;
  scroll restoration; an accessor for the current location; and rendering
  a given path natively, which is 4.6's.
- **For `nova dev`:** serving a native or server-rendered program, which is
  4.6's and 4.7's; an overlay for a runtime panic; HTTPS, and access from
  other machines (`--host`); and source maps.

## References

- `docs/phase-4-plan.md` §3, decisions 6, 9, 10 and 12, §4's 4.5 entry,
  and §5's security rules.
- The spec named above.
- ADR 0038, the recorded scope; ADR 0041, `std/dom`; ADR 0042, `std/ui`;
  ADR 0043, the component syntax and the bundle.
- `nova-spec/30-FRONTEND.md` §6, §8 and §13; `nova-spec/40-TOOLING.md`
  §1.1; `nova-spec/60-EXAMPLES.md` §6.
