# ADR 0045 — SSR, hydration and SSG

## Status

Accepted, 2026-10-19 (Phase 4.6, branch
`claude/loving-archimedes-frtf5q-46`; spec
`docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`).
Two entries in one project; a walk in order over comment markers; a
mismatch reported and the view built again; no hash seed shared; the built
`dist/index.html` as the page; SSG of the static routes by the serving
function; two levels of serving in a native-only std module; typed data
through JSON; the names `hydrate`, `ui::ssr`, `serve` and `page`; `nova
dev` unchanged; the gate and its tests; and `404` with the rendered page
are the user's decisions of 2026-10-19 (decisions 1 to 12 below).
`hydrate_with`'s fallback is the user's decision of 2026-10-20, after the
prototype's fact-check (decision 13), and adds a parameter to the spec's
API. The spec settled the rest (its §14).

## Context

After 4.5 a program could route in the page and run under `nova dev`, but
it rendered only in the browser. `nova-spec/30-FRONTEND.md` §7.4 sketches
`nova build --ssr`, "emit native binary that renders HTML", and `nova
build --ssg`, "pre-render at build time", with the same component code on
a server that renders to a string and a client that "attaches to existing
DOM, replays effects". `00-MASTER-SPEC.md` §1.8 puts "SSR/SSG: built-in
flags on `nova build`" among the points not up for debate. ADR 0042 said a
new renderer "walks the same `View`", and ADR 0044 left rendering a given
path natively, and serving a rendered program, to 4.6. The repository 4.6
starts from (spec §2, read at `2b7a6ed`) has:
- `render_to_string`, which writes no marker for a `show` or a `for_each`
  and runs two texts side by side together, so a browser's parse of its
  HTML has fewer nodes than `mount` builds;
- a WASM backend that cannot read a node's first child, kind or text, and
  a `std/dom` with no way to write to the console;
- a router that reads its location from the backend when it is made,
  natively always `/`;
- a `Map` that iterates in table order, seeded at random in each process,
  natively and on WASM alike;
- `std/http`'s `Server`, which routes a `GET` by its exact target, answers
  anything else `not_found()`, and binds inside `listen`, printing nothing;
- a project whose program is `src/main.nova` alone, and a `nova build`
  that takes `--target`, `--release` and `-o`;
- a WASM build that keeps only what `main` reaches.

## Decision

### The user's decisions

1. **Two entries, one project.** `src/main.nova` is the client, built for
   WASM; `src/server.nova` is the server, built natively; what both run
   lives in a module of the project, such as `src/app.nova`, which each
   imports with `import app::{…}`. `nova build --ssr` builds both.
2. **Hydration walks the nodes in order.** The server writes comment
   markers around each `show` and `for_each` region, and between two texts
   side by side; elements get no attribute for it.
3. **A mismatch** is reported with `console.error`, naming the first
   difference; the server's nodes under the root are dropped, and the view
   is built afresh. The tests assert that their pages hydrate with no
   mismatch.
4. **No seed is shared** between server and client, since sending the
   server's hash seed to every visitor would defeat its HashDoS
   protection. A view must not show a `Map`'s order: it sorts first. The
   mismatch check catches a view that forgets.
5. **The page is the built `dist/index.html`,** generated or the
   project's own. The server and SSG put the rendered HTML inside its
   `<div id="app"></div>`; the client calls `hydrate("#app", App)`. The
   head, the title and the scripts stay in that one file for both.
6. **SSG renders the static routes,** those with no `:name` piece that are
   not the catch-all. `nova build --ssg` builds the client and the server,
   then runs the server once, and the std function that serves writes
   `dist/<path>/index.html` for each static route instead. Routes with
   parameters are skipped.
7. **std serves at two levels,** in a std module only a native program can
   import: one function, `serve`, that serves `dist/`'s files and renders
   every other `GET` as a page; and functions that render the page at one
   path to a string, for a server written by hand, as 4.7's blog will be.
   `POST`, cookies and path parameters stay 4.7's.
8. **The server's data reaches the client typed, through JSON:** a
   `ToJson` value, escaped, in a `<script type="application/json">`, which
   the client reads back with `FromJson` and hydrates with.
9. **The names:** `hydrate` in `std/ui` beside `mount`, with
   `hydrate_with` for the server's data; `import ui::ssr`, native only, as
   `dom` is WASM only; `ssr::serve` the high level and `ssr::page` the low.
10. **`nova dev` does not change;** it stays client only. In an SSR
    project its page has an empty root, where `hydrate` builds the view.
    4.7, or the backlog, decides on a development mode for SSR.
11. **The gate and the tests:** `tests/browser/ssr-app`, with a router, a
    `show`, a `for_each`, a counter and the server's data; in CI,
    Playwright opens a server-rendered page and a statically generated
    one, clicks, and checks that the server's nodes were not made again;
    in `cargo test`, the fake DOM parses the HTML `std/ui` writes, and
    hydration and its mismatches are tested. No example changes; 07 is
    4.7's. `tests/browser/ssr.mjs` builds the app with `--ssg`, opens `/`,
    `/posts/2` and `/nowhere` from its server and SSG's `/about/` from a
    static server, each with `app.wasm` held back, keeps a handle to every
    node under `#app`, and checks that each is `===` the page's node once
    the app's `on_mount`, which runs after the walk, has set its root's
    `data-state` from `server` to `hydrated`.
12. **A path no route serves gets `404`** with the rendered page, when it
    falls to `*` or matches no route; the router says which route a path
    matches.
13. **`hydrate_with` takes a fallback** (2026-10-20):
    `hydrate_with("#app", Data { names: [] }, App)`. When the page holds no
    data from the server, it builds or hydrates with the fallback the
    program gives, so `nova dev` shows the page (decision 10). The spec's
    `hydrate_with(selector, f)` becomes `hydrate_with(selector, fallback,
    f)`.

### Decided in the spec (§14)

1. **`render_to_string` does not change.** Hydratable HTML has its own
   renderer, `render_hydratable(f)` in `std/ui`, beside the walk that
   reads it. It writes what `render_to_string` writes, with three
   additions.
2. **The markers** are `<!--[-->` and `<!--]-->` around a region's views,
   and `<!--|-->` between two text views side by side, siblings among an
   element's children or a region's views; an empty text writes nothing,
   but keeps its separators. A carriage return, in text or in an
   attribute's value, is written `&#13;`, since the browser's parser turns
   a raw one into a line feed. No marker can come from a program's text,
   which is escaped.
3. **The walk checks tags and texts, not attributes.** `hydrate(selector,
   f)` walks `f()`'s view against the root's child nodes under a root
   owner it keeps, as `mount` keeps one. An element must have the view's
   tag, compared in lower case, and is adopted: `Static` attributes are
   neither written nor checked, `Property` is set, `Dynamic` and
   `DynProperty` get their effects, whose first run writes the value
   again, and `On` is listened for; then its children are walked, and no
   node may be left after them. A text node must hold the view's text, a
   `DynText`'s as its effect's first run reads it; where the view's text
   is empty and no text node is there, an empty one is inserted. A region
   must start at `<!--[-->`; its first run adopts its views' nodes, and
   its later runs build between the two markers as under `mount`, which
   would have made the same two nodes; it must end at `<!--]-->`.
   `on_mount` callbacks run after the walk. At the first difference (a
   wrong tag, a text that differs, a missing marker or node, a node left
   over), the walk stops, `nova: hydration mismatch: expected <p>, found
   <div>; building the view again` goes to `console.error`, each side
   read as `<tag>`, `text "…"` (its first 40 characters), `a region`, `the
   end of a region` or `nothing`, the walk's owner is disposed, the root's
   children are removed, and the view is built there as `mount` builds
   it, calling `f` again; `on_mount` callbacks queued under the disposed
   owner never run.
4. **An empty root mounts with no report:** no child, or only whitespace
   text, as `nova dev`'s page and a page served without SSR have. Its
   whitespace is removed first, so the view is not built after it.
5. **The data's element** is `<script type="application/json"
   id="nova-data">`, just after the root, and the root's id is `app`.
   `hydrate_with(selector, fallback, f)` reads the data from the root's
   next element sibling alone, past any text or comment between them, when
   it is a `script` whose `id` is `nova-data` and whose `type` is
   `application/json`, as `document` writes it: an element of the view with
   the id `nova-data`, inside the root, is never the data, nor is any
   element after the first. It parses that element's text, makes a `T` with
   `T::from_json`, and hydrates `|| f(data)`. With no such element, it
   hydrates `|| f(fallback)` (decision 13): an empty root gets the view
   built, and a filled one is walked with it. Data that is there but does
   not parse still panics, `std/ui: hydrate_with: the page's data does not
   parse: <error>`. `page_with` writes
   `stringify(data.to_json())` with `<`, `>` and `&` as `\u003c`, `\u003e`
   and `\u0026`, and U+2028 and U+2029 as `\u2028` and `\u2029`: those
   characters occur only inside JSON's strings, so the text parses back to
   the same value, and no `</script>` or `<!--` in the data can end or
   change its element.
6. **The router's server functions** are `with_location(location, f)`,
   under which a router made starts at `location` and which nests and
   restores; `found(routes, location)`, true when the first route that
   matches is not the catch-all; and `static_paths(routes)`, the static
   routes' paths in order, each once, as written.
7. **`ui::ssr`'s names** are `Page { body, data }`, `page(location, f)`,
   `page_with(location, data, f)`, `document(template, page)`, `Site {
   address, dist, routes, render }` and `serve(site)`. `Site.render` maps a
   location to a `Page`, so each page can choose its own data. `document`
   empties the template's first `<div id="app">`, as `serve` and SSG empty
   it, puts the body into it and the data's element just after it, and
   panics with `ssr: the page has no <div id="app">` when there is none.
8. **`serve` serves only the files `manifest.json` lists,** answers only
   `GET`, and applies the `.` rule `nova dev` uses. It reads the template
   and the manifest when it starts, returning an `Err` naming a file it
   cannot read; binds `site.address`; and prints `ssr: serving
   http://<host>:<port>/` with the port it bound, so a test binds port 0. A
   request's path, before any `?`, percent-decoded, less its leading `/`,
   is a listed file (`200`, a `Content-Type` by its extension and
   `X-Content-Type-Options: nosniff`), or else a `404` when its last piece
   holds a `.`, or else a page: `document(template, render(target))`, the
   target with its query, `200` when `found` says a route serves it and
   `404` otherwise. No file outside the manifest is ever read; and a
   manifest's path with a piece that is empty, `.` or `..`, or that holds
   `\` or NUL, as SSG refuses a route's path, is dropped when the manifest
   is read, so a hand-written manifest listing `../secret` serves nothing
   outside `dist`. SIGINT or SIGTERM stops it, and it returns `Ok(())`.
9. **`std/http` gains `Server::fallback(handler)`,** which answers a `GET`
   no route matches, and `Server::listen_on(listener)`, which serves a
   listener the program bound; `listen` is `bind`, then `listen_on`, as
   before.
10. **SSG is `serve` run with `--ssg <dir>`:** it binds nothing, reads
    `<dir>/index.html` as the template, writes `document(template,
    render(path))` for each of `static_paths`, `/` over `<dir>/index.html`
    and any other path to `<dir>/<its pieces>/index.html`, printing `ssg:
    wrote <file>` for each, and refuses a path with a piece that is empty,
    `.` or `..`, or holds `\`, NUL, `?` or `#`, returning an `Err` naming it
    and writing nothing more. The CLI adds the pages to the manifest. A
    server written by hand is pre-rendered too if its `main` calls `serve`
    when given `--ssg`.
11. **The server's executable** is `target/<profile>/<name>-server`. `-o`
    names the client's directory only. `nova build --ssr` refuses a file
    argument, `--target`, a library, and a project with no
    `src/server.nova`, before anything is built; `--ssr --ssg` is `--ssg`;
    `--release` applies to both builds. `nova build --ssg` runs the server
    from the project's root with `--ssg <dist>`, its output passing
    through, and a failure fails the build.
12. **Mismatches are reported through `std/dom::console_error`,** new,
    with its shim function. The WASM backend gains `backend_first_child`,
    `backend_kind`, `backend_text_of`, `backend_report` and
    `backend_data(root)`, which reads the root's next element sibling;
    natively each panics with `mount`'s message, and so do
    `hydrate` and `hydrate_with`. The resolver registers `ui::ssr` as an
    imported std module natively and refuses its import on WASM with
    E0092; `nova-doc` lists it; the driver builds a project's program
    rooted at `src/server.nova`, with its modules and dependencies.

## What departs from the spec, or fills it in

- **`hydrate`'s panic for a selector that matches nothing names the
  function,** `std/ui: hydrate: no element matches "…"`, or
  `hydrate_with:`, where spec §4.1 has `mount`'s words.
- **`hydrate_with` panics only for data that does not parse.** Spec §4.1's
  panic for a page with no data, `std/ui: hydrate_with: the page has no
  data from the server`, is gone: such a page uses the fallback (decision
  13). The spec's `backend_data()`, "the text of `#nova-data`", the first
  element with that id in the page, is `backend_data(root)`, the root's
  next element sibling, so that a view's own element with that id cannot
  stand for the data.
- **The mismatch's words:** a `<!--|-->` found is read as the text after
  it, which the server wrote there; a comment that is no marker is `a
  comment`, a reading spec §4.3 does not list; and a missing separator
  reads `expected text "<the next text>", found …`.
- **The whitespace of an empty root** is HTML's: space, tab, LF, CR and
  FF, and it is removed before the view is built. The `<error>` of
  `hydrate_with`'s panic for data that does not parse is the `JsonError`'s
  message, whether `parse` or `from_json` refused the text.
- **The disposed walk's owner** is removed from the roots `std/ui` keeps,
  so nothing holds it after the view is built again.
- **The fake DOM's parser** turns CR and CRLF into LF, as a browser does,
  so `&#13;` is tested under it; it throws on an entity it does not know,
  and on HTML it cannot parse, such as an end tag it does not expect.
- **A filled root is a template too.** After `--ssg`, `dist/index.html`
  is the rendered `/`, and a project's own page may hold whitespace or a
  placeholder in its root. So `document`, which `serve` and SSG fill each
  page with, empties the root, and drops a data element after it, just
  after or past whitespace, where the client would read it, before filling
  it, where spec §6.2's `document` replaced the first
  `<div id="app"></div>`, and panicked on a template with none, one whose
  root holds anything included. `serve` and SSG
  empty the template's root once when they read it, and refuse a template
  with no `<div id="app">`, `ssr: <dist>/index.html has no <div
  id="app">`, or `ssg: …`. The root's end is the `</div>` that balances
  it; a root no `</div>` balances keeps what follows it, after an empty
  root. A data element with no `</script>` stays where it is, since
  nothing tells where its text ends: such a template is broken HTML, whose
  rest a browser reads as that script's text. So SSG can run again over
  its own output, and a server started after `--ssg` renders each page
  from the page it was built with.
- **`serve`'s own errors:** an address it cannot bind, `ssr: cannot bind
  <address>: …`; a manifest with no `files` object; and `--ssg` with no
  directory after it, `ssg: --ssg needs a directory`. A listed file that
  cannot be read when asked for gets `404`. It reads a file through the
  builtins `std/fs` is written on, since a handler cannot await.
- **The `nosniff` header is written `x-content-type-options`,** in lower
  case, as `std/http` writes every header name.
- **A fifth refusal:** `--ssr` or `--ssg` outside a project, "builds a
  project, and there is no nova.toml here or above". The spec gave only
  the text for a missing `src/server.nova`; the CLI words the others
  itself: "builds the project's src/main.nova and src/server.nova; it
  takes no file", "builds the client for wasm and the server natively; it
  takes no `--target`", and the library refusal `nova build` already
  makes.
- **The pages `--ssg` lists** are the files the server's `ssg: wrote
  <file>` lines name, which the CLI reads from the server's stdout as it
  passes it through; then every file the manifest lists is measured again,
  since the root's page replaced `index.html`. A line's file, read from
  the server's working directory, the project's root, if relative, must
  lie in the output directory, made absolute the same way, so the answer
  does not hang on where `nova` runs; and each name below it must be a
  plain one by Rust's path components, with no `..`, root or prefix, while
  a `.` piece and a trailing `/` are normalised away, and nothing is asked
  of the file system. Otherwise, once the server has ended well, the build
  fails with ``the server's line `…` names no file inside
  <dist>``, and the manifest lists none of its pages. When the server
  fails, `nova` prints `Error: the server failed to generate the pages,
  with exit code <code>: <server>` and exits with that code; when a signal
  stops it, which leaves it no code, `…, stopped by signal <number>: …`,
  and exits with 1. These words are the same on every system, where
  `ExitStatus`'s own are not.
- **On WASM, a name of `ui::ssr` with no import** is E0001, as natively:
  the resolver leaves an imported module's names out of the list of names
  WASM lacks, since no module sees them without the import, which is
  E0092 itself.
- **`with_location` and `found` drop a `#fragment`,** as a router's
  location never holds one, so the page a location renders and its status
  agree; `found` takes any pattern whose only piece is `*` for the
  catch-all, `/*` included.

## What departs from `30-FRONTEND.md` §7.4 and the master spec

- **`--ssr` does not emit a renderer:** it builds the project's own
  server, `src/server.nova`, which calls `ui::ssr::serve` or renders with
  `page` in handlers of its own, beside the client.
- **`--ssg` runs that server** once after the build, and pre-renders only
  the routes with no parameter; the master spec's Phase 4 item 9,
  "compile-time route enumeration", becomes `static_paths` over the
  program's own routes when the built server runs, and no list of paths
  can be given.
- **The server renders with `render_hydratable`,** not `render_to_string`
  (master spec item 8), so that the browser's parse has each node `mount`
  would make.
- **Hydration adopts the server's nodes and starts their effects;** it
  does not replay them over the page. A `DynText`'s first run is compared
  with the text it adopts, a dynamic attribute's first run writes it
  again, and a mismatch builds the view anew.
- **The server's data travels in the page,** a step §7.4 does not name.
- **SSR and SSG stay flags on `nova build`,** as §1.8 has them, and `nova
  dev` does not change.

## Consequences

- **The known causes of a mismatch,** each of which builds the page again
  and says so in the console: HTML the browser's parser rebuilds, such as
  a `div` inside a `p` or a `tr` without a `tbody`; a `script` or `style`
  element's text, which the parser does not decode; a NUL in text, which
  the parser drops; a line feed that begins a `pre`'s or a `textarea`'s
  text, which the parser drops too; and a value that differs between the
  two renders, such as the time, a random number, or a `Map`'s order
  (decision 4).
- **The risks:**
  - the browser's parser can build another tree from the HTML; the page
    still works, since the mismatch builds it again, but the server's work
    is wasted;
  - a render that is not deterministic mismatches every time: programs
    pass such values as data, and sort;
  - `render` is synchronous, so a page cannot await its data; 4.6's comes
    from memory, and loading per request is 4.7's question;
  - hydration adds code to every client that calls `hydrate`;
    `06-counter-spa` does not, and its size test reads 21,070 bytes
    gzipped, the 21 KB 4.4 recorded. `ssr-app`'s client, built without
    `--release` and so without `wasm-opt`, measured 41,260 bytes gzipped
    when `ssr.mjs` printed it; its size is printed, not limited;
  - a server is a second program to build natively, slow under
    `--release`.
- **Under `nova dev`, `hydrate_with` builds the page from its fallback**
  (decision 13): the page holds no `nova-data`, its root is empty, and the
  client builds `f(fallback)` there, as `hydrate` builds its view, so an
  SSR project with data still runs under `nova dev`, without the server's
  data, until 4.7, or the backlog, gives `nova dev` a mode for SSR. A page
  a server filled with no data after the root is walked with the fallback,
  and mismatches where the server rendered other data. A project's own
  page whose root holds a placeholder is not empty either: under `nova
  dev` the client walks the placeholder, reports a mismatch with
  `console.error` on every load, and builds the view, which works.
- **A hash `Router` is not for SSR:** a server never sees the hash, so it
  renders `/`, and a page opened at `#/posts/1` mismatches and is built
  again. `HistoryRouter` is the router for SSR.
- **The data is public:** it is written into the page for every visitor.
- **An object's keys in the data** come out in its `Map`'s order, which
  each process seeds anew, so two `--ssg` builds, or two servers, can
  write the same value with its keys in another order. The client reads
  the same value; the pages are not byte for byte the same.
- **After `--ssg`, `/index.html` and `/about/index.html` are files** the
  manifest lists, served as SSG wrote them, while `/` and `/about` are
  rendered on each request. A file added to `dist/` after the build is not
  served until the server starts again.
- **A static route written with an empty piece,** such as `/about/`, is
  refused by SSG, and fails `nova build --ssg`.
- **SSG does not refuse a name Windows reserves or alters,** such as
  `CON`, a piece that ends in a dot, or one that holds `:`: on Windows such
  a route's page may fail to be written, or land under another name. It
  is still written inside `<dir>`, since a piece that is empty, `.` or
  `..`, or holds `\`, is refused.
- **A comment in the template can fool the root's emptying,** which counts
  `<div` and `</div>` and reads no comment: one inside the root holding
  `</div>` ends the root there, and the rest of the root stays in the
  page. A view cannot, since its text and attribute values are escaped,
  and `render_hydratable` writes no comment but its markers.
- **A server must report `serve`'s `Err`,** print it and exit with a
  failure, or an SSG refusal passes silently; `tests/browser/ssr-app`'s
  server does.
- **No crate is added,** and `Cargo.lock` does not change.

## What is left out

- A development mode for SSR in `nova dev` (decision 10).
- Partial hydration, or islands: 4.7 decides whether the blog's comments
  need more than whole-page hydration.
- Streaming HTML, and asynchronous data in `render`.
- `POST`, cookies and path parameters in `std/http`, which are 4.7's.
- Pre-rendering routes with parameters, and a list of paths given by the
  program.
- Checking attributes during hydration.
- A keyed `for_each`.
- A `nova serve` command.
- `nova check` of `src/server.nova` as a second root.

## References

- `docs/phase-4-plan.md` §3, decisions 10 and 11, §4's 4.6 entry, risk 5,
  and §5's security rules.
- The spec named above.
- ADR 0042, `std/ui`; ADR 0043, the bundle; ADR 0044, the router and
  `nova dev`; ADR 0022, the shutdown signals `serve` inherits.
- `nova-spec/30-FRONTEND.md` §4 and §7.4; `nova-spec/40-TOOLING.md` §1.1;
  `nova-spec/20-STDLIB.md` §1 and §6; `nova-spec/00-MASTER-SPEC.md` §1.8
  and Phase 4.
