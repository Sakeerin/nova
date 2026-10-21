# Phase 4.7, `examples/07-fullstack-blog`: design

- **Date:** 2026-10-21.
- **Status:** approved by the user on 2026-10-21.
- **The user's decisions, 2026-10-21:**
  1. **std grows what the blog needs.** `std/http` gains routes by
     method, path parameters, readers for a form and for cookies, several
     `Set-Cookie` headers on one response, and responses for HTML and for a
     redirect. `ui::ssr` gives a program the parts `serve` is made of, so a
     server can put its own routes beside the built site, and each of its
     pages sees the whole `Request`. ADR 0045 left these to 4.7.
  2. **Forms are plain HTML forms, and every `POST` that succeeds is
     answered with `303`** (post, redirect, get). Every page works without
     JavaScript. Hydration adds live checks: a blank or too long field is
     reported as it is typed, and the submit button is disabled until the
     form is valid. The whole page hydrates; there are no islands, and
     `std/ui` gains nothing.
  3. **Passwords are hashed with PBKDF2-HMAC-SHA256,** which `std/crypto`
     gains natively through `ring`, already in `Cargo.lock`. It runs
     OWASP's 600,000 iterations unless told otherwise, and stores the count
     with each hash. The prototype measures its time.
  4. **The admin's account is made by the server itself:**
     `fullstack-blog-server --set-password <name>` reads the password from
     standard input and writes its hash into the data file. No password is
     known from the repository; the tests and the README run the command.
  5. **Anyone may comment,** with a name and a text. Every visitor gets a
     signed session cookie, logged in or not, which the CSRF token is bound
     to. The admin can delete a comment.
  6. **A session is a random id, signed, in a table in memory.** The
     cookie carries the id and its `hmac_sha256` tag, `HttpOnly` and
     `SameSite=Lax`. The server keeps who each id belongs to and when it
     ends. Logging out ends it; a restart ends every session. The signing
     key is made at random the first time the server starts, and kept in
     the data directory.
  7. **The JSON file is written whole, then renamed into place,** so a
     crash never leaves half a file: `std/fs` gains `rename`. The seed data
     is in the repository; the file the server writes is ignored by git.
  8. **`src/` is flat.** A nested module path is E0900 today, so the
     modules of `60-EXAMPLES.md` §7's tree sit side by side in `src/`. An
     ADR records the departure.
  9. **The tests:**
     - a new `crates/nova-cli/tests/blog_tests.rs` builds 07 with `--ssr`
       on all three systems and drives its server over HTTP: logging in,
       the admin's create, edit and delete, a form without its CSRF token
       refused, and escaping;
     - `nova test` runs the server's own modules' tests, through the
       package's `src/lib.nova`;
     - the gate runs in CI's `browser` job through Playwright, in
       Chromium (`tests/browser/blog.mjs`).
  10. **A post's body is plain text.** A blank line separates paragraphs,
      and each paragraph is a `<p>` whose text is escaped.
  11. **`nova dev` does not change in 4.7;** a development mode for SSR
      goes to the backlog. The README builds with `nova build --ssr` and
      starts the server.
  12. **The package is `fullstack-blog`,** so its server is
      `target/<profile>/fullstack-blog-server`.
- **Branch:** `claude/loving-archimedes-frtf5q-47`, from `main` at
  `9ad78cd`, as 4.6 used `-46`.
- **Inputs:**
  - `docs/phase-4-plan.md`: decision 11 (the blog), decision 12 (the
    gate's measurement), §4's 4.7 entry and its gate, §5's security rules;
  - `nova-spec/60-EXAMPLES.md` §7 (the blog), §9 (an example's README) and
    §10 (an example's tests);
  - ADR 0038 (the blog stores its posts in a JSON file, through `std/fs`
    and `std/json`);
  - ADR 0045 and the 4.6 spec, which leave to 4.7 `POST`, cookies, path
    parameters, loading data per request, and whether comments need more
    than whole-page hydration;
  - `nova-spec/20-STDLIB.md` §6, whose `Server` has `post`, `put`,
    `delete`, `route` and `req.params`.

## 1. What 4.7 delivers

- **A blog,** `examples/07-fullstack-blog`, that a visitor reads and
  comments on, and that its admin writes, from a browser:
  - `/` lists the posts, newest first, each with its title, date and first
    paragraph;
  - `/posts/:id` shows a post, its comments and a form to add one;
  - `/login` takes the admin's name and password;
  - `/admin` lists the posts with a link to edit each and a button to
    delete it; `/admin/new` and `/admin/posts/:id/edit` hold the editor;
    the post page shows the admin a button to delete each comment.
- **Its server is a Nova program on `std/http`.** It renders every page
  on the server with `ui::ssr`, sends its data with it, and the client,
  built for WASM, hydrates the page. Its state lives in memory, and each
  change is written to `data/blog.json`.
- **The rules the phase plan sets, kept by construction:**
  - sessions are cookies signed with `hmac_sha256`, `HttpOnly` and
    `SameSite=Lax`;
  - every form carries a CSRF token, and a `POST` without the right one is
    refused with `403`;
  - text from a visitor reaches the page only through `std/ui`, whose two
    renderers escape it; there is no raw HTML;
  - passwords are stored only as PBKDF2 hashes.
- **What std gains for it** (§3 to §5), each tested on its own:
  - `std/http`: `post`, `put`, `delete` and `route`; patterns with
    `:name` pieces and `req.params`; `405`; `query`, `form` and
    `cookies`; `Cookie` and `with_cookie`; `Response::html` and
    `Response::redirect`;
  - `ui::ssr`: `Dist`, the built site a hand-written server serves;
  - `std/fs::rename`, `std/crypto::pbkdf2_sha256` and its `_verify`, and
    hex in `std/bytes`.
- **The gate,** in CI's `browser` job (§7.5): every page works; the
  admin's create, edit and delete work through the UI; a script put into a
  title or a comment stays inert text; a form without its CSRF token is
  refused.

## 2. The starting point

Read on 2026-10-21, at `9ad78cd`.

- **`std/http`'s `Server` serves `GET` only** (`std/http/lib.nova`
  1004-1129). `get(path, h)` matches the target exactly, query included,
  so `/add?x=1` misses `/add`. `fallback` answers an unmatched `GET`, and
  any other method gets `404`. A handler is `fn(Request) -> Response`, so
  it cannot await.
- **A `Request`** holds its method, its raw target, its headers by
  lower-cased name, the last of two kept, and its body as `Bytes`. Nothing
  is decoded: there is no query, form, cookie or parameter reader.
- **A `Response`'s headers are a `Map`,** so a second `set-cookie`
  replaces the first. There is no `html` or `redirect` builder, and `303`
  is written `303 Redirection`.
- **`ui::ssr::serve` builds its own `Server`,** with one fallback and no
  way to add routes. `Site.render` gets only the location, so a page
  cannot see a cookie, and its status is `200` or `404`. The functions
  that read the template and serve the manifest's files are private.
  `page`, `page_with` and `document` are public, for a hand-written
  server.
- **`std/fs` has no `rename`.** Its functions are `async fn`s that never
  suspend; a handler cannot await them, but it can `spawn` them, and the
  spawned writes run after the response, in order.
- **`std/crypto`** (native only) has `hmac_sha256`, a constant-time
  `hmac_sha256_verify`, and `random_bytes` from the system. A PBKDF2
  written in Nova matches RFC 7914's vectors but takes about 0.84 s per
  100,000 iterations, during which the single-threaded server answers
  nothing. There is no hex or base64.
- **`std/process` has no environment.** A secret comes from a file or an
  argument.
- **Handlers run one at a time on one thread.** A record, `Map` or `Vec`
  a handler captures is shared between calls; a captured `let mut Int` is
  copied, so changing it is lost.
- **`std/json` has no derive.** Every record's `ToJson` and `FromJson`
  is written by hand, as in `tests/browser/ssr-app`.
- **`hydrate_with` reads one value of one type** for the whole app, once.
  A client-side `link` to another route would render it with the first
  page's data, so the blog's links are plain anchors that load the next
  page from the server.
- **`std/ui` escapes** every text and attribute value, and refuses
  `innerHTML`, `outerHTML` and `srcdoc`. A handler gets only the target's
  value, and a submit is never prevented. It does not refuse a
  `javascript:` URL in `href`.
- **A nested module path, `import routes::home`, is E0900.**
- **`public/`** is copied to `dist/assets/` and listed in the manifest.
- **A blog-shaped probe client,** with forms, inputs and closure routes,
  is 41,384 bytes gzipped and hydrates in Chromium with no error.

## 3. `std/http`

### 3.1 Routes by method, with parameters

```nova
impl Server {
    pub fn get(self, pattern: String, handler: fn(Request) -> Response) -> Server
    pub fn post(self, pattern: String, handler: fn(Request) -> Response) -> Server
    pub fn put(self, pattern: String, handler: fn(Request) -> Response) -> Server
    pub fn delete(self, pattern: String, handler: fn(Request) -> Response) -> Server
    pub fn route(self, method: Method, pattern: String, handler: fn(Request) -> Response) -> Server
}
```

- **A pattern** is a path whose pieces are literal, or `:name`, which
  matches any one non-empty piece and binds it to `name`. It is matched
  against the request's path: its target before any `?`, split on `/`,
  empty pieces dropped and each piece percent-decoded, as `std/ui/router`
  splits a route's path, so a server's route and a client's agree on what
  a path is: `/admin/` matches `/admin`.
- **`get` changes:** it takes a pattern, and the query no longer takes
  part, so `/add?x=1` now matches `/add`. Its doc and its tests change
  with it.
- **Dispatch,** for each request, in the order the routes were added:
  1. the first route whose method and pattern match answers, with
     `req.params` holding what its pattern bound;
  2. otherwise, when a route's pattern matches but its method does not,
     `405 Method Not Allowed`, with an `allow` header naming the methods
     of the routes that match, in the order they were added;
  3. otherwise a `GET` goes to the `fallback`, when there is one;
  4. otherwise `404`.
- **`Request` gains `params: Map<String, String>`,** empty in a request
  that `parse_request_head` or `read_request` returns. As
  `20-STDLIB.md` §6 has it: `req.params.get("id")`. A `Request` literal
  must now name it.
- `HEAD` gets no special treatment; `fallback` still answers `GET` only.

### 3.2 Reading a request

```nova
impl Request {
    pub fn query(self) -> Map<String, String>
    pub fn form(self) -> Map<String, String>
    pub fn cookies(self) -> Map<String, String>
}
```

- **`query`** reads the target after `?`, and **`form`** reads the body,
  both as `application/x-www-form-urlencoded`:
  - pairs are split on `&`, and an empty one is skipped;
  - a pair is split at its first `=`; one with none has the value `""`;
  - `+` is a space, and `%XX` sequences are decoded as UTF-8; a `%` that
    does not start a valid sequence stays as it is;
  - of two pairs with the same name, the last wins.
- **`form`** reads the body only when the `content-type` header's media
  type, before any `;`, trimmed and lower-cased, is
  `application/x-www-form-urlencoded`, and the body is UTF-8. Otherwise
  it returns an empty map.
- **`cookies`** reads the `cookie` header: pairs split on `;`, each
  trimmed and split at its first `=`. A pair with no `=` is skipped, and
  the first of two with the same name wins, as browsers send the most
  specific first. Values are not decoded.

### 3.3 Answering

```nova
pub record Cookie {
    pub name: String,
    pub value: String,
    pub path: String,
    pub max_age: Option<Int>,
    pub http_only: Bool,
    pub same_site: String,
    pub secure: Bool,
}

impl Cookie {
    pub fn new(name: String, value: String) -> Cookie
    pub fn expired(name: String) -> Cookie
}

impl Response {
    pub fn html(status: Int, s: String) -> Response
    pub fn redirect(location: String) -> Response
    pub fn with_cookie(self, c: Cookie) -> Response
}
```

- **`Cookie::new`** has the safe defaults: path `/`, no `Max-Age` (a
  cookie for the browser's session), `HttpOnly`, `SameSite=Lax`, and not
  `Secure`. `Cookie::expired(name)` has the value `""` and `Max-Age=0`,
  which removes the cookie.
- **`Response` gains `cookies`,** which `with_cookie` adds to and
  `to_bytes` writes, one `set-cookie` line each, after the other headers:
  `name=value; Path=/; Max-Age=N; HttpOnly; SameSite=Lax; Secure`, each
  attribute only when it applies. A cookie whose name is not an HTTP
  token, or whose value or path holds a character a cookie cannot carry,
  is left out, as a header holding CR or LF is today. A `Response`
  literal must now name `cookies`.
- **`Response::html`** is `text`'s twin with
  `text/html; charset=utf-8`. **`Response::redirect`** is
  `303 See Other`, with `location` set and an empty body.
- **Reason phrases** gain 301 `Moved Permanently`, 302 `Found`, 303
  `See Other`, 401 `Unauthorized` and 403 `Forbidden`.

## 4. `ui::ssr`: the built site, for a hand-written server

```nova
pub record Dist { … }

impl Dist {
    pub async fn open(dir: String) -> Result<Dist, IoError>
    pub fn file(self, req: Request) -> Option<Response>
    pub fn respond(self, status: Int, page: Page) -> Response
}
```

- **`Dist::open`** reads `<dir>/index.html` and `<dir>/manifest.json`,
  with every check and every error `serve` makes on them today: the
  template's root is emptied, and a manifest's path that could leave
  `dir` is never served.
- **`file`** answers a request for a file the manifest lists, by the
  rules `serve` uses: `200` and the file with its `Content-Type` and
  `nosniff`, or `404` when it cannot be read. For a path the manifest
  does not list it returns `None`, and the program decides.
- **`respond`** returns `document(template, page)` as
  `text/html; charset=utf-8`, with `status`.
- **`serve` is rebuilt on `Dist`,** with no change in what it does; its
  tests are unchanged.

A page that needs a cookie, a status of its own or a redirect is a route
of the program's own `Server`, whose fallback asks `Dist::file` first:

```nova
let app = Server::new()
    .get("/posts/:id", |req: Request| post_page(state, dist, req))
    .post("/posts/:id/comments", |req: Request| add_comment(state, req))
    .fallback(|req: Request| match dist.file(req) {
        Some(file) => file
        None => not_found(state, dist, req)
    })
```

## 5. `std/fs`, `std/crypto` and `std/bytes`

- **`std/fs::rename(from: String, to: String) -> Result<(), IoError>`,**
  an `async fn` like its neighbours, renames a file, replacing `to` when
  it exists, on Windows too.
- **`std/crypto`** (native only):

  ```nova
  pub fn pbkdf2_sha256(password: Bytes, salt: Bytes, iterations: Int, length: Int) -> Bytes
  pub fn pbkdf2_sha256_verify(password: Bytes, salt: Bytes, iterations: Int, expected: Bytes) -> Bool
  ```

  Both are `ring::pbkdf2` with HMAC-SHA256. `_verify` compares in constant
  time, as `ring` does. Each panics when `iterations` is below 1 or above
  4,294,967,295, and `pbkdf2_sha256` when `length` is below 1.
- **`std/bytes`:** `Bytes::to_hex(self) -> String`, in lower case, and
  `bytes_from_hex(s: String) -> Option<Bytes>`, which takes either case
  and returns `None` for an odd length or any other character. Both
  targets have them.

## 6. The blog

### 6.1 The project

```
examples/07-fullstack-blog/
├── nova.toml            # name = "fullstack-blog"
├── index.html           # the page: title, stylesheet, root, app.js
├── public/styles.css    # served as /assets/styles.css
├── data/seed.json       # three posts, two comments, no users
├── src/
│   ├── main.nova        # the client: hydrate_with("#app", …, App)
│   ├── app.nova         # shared: Data and its JSON, routes(), App
│   ├── home.nova        # shared: the pages
│   ├── post.nova
│   ├── login.nova
│   ├── admin.nova
│   ├── header.nova      # shared: components
│   ├── post_card.nova
│   ├── forms.nova       # shared: the checks both sides run
│   ├── store.nova       # server: the store and its file
│   ├── auth.nova        # server: sessions, CSRF tokens, passwords
│   ├── server.nova      # server: arguments, routes, handlers
│   └── lib.nova         # the package's library, for its tests
├── tests/               # nova test
└── README.md            # 60-EXAMPLES.md §9's sections
```

- The client reaches only shared modules. `store`, `auth` and `server`
  use `std/fs`, `std/crypto` and `ui::ssr`, which WASM does not have, so
  only `server.nova` and `lib.nova` import them.
- `.gitignore` gains the example's `dist/`, `data/blog.json` and
  `data/secret`.

### 6.2 Pages and routes

| Method | Path | Who | Answer |
|---|---|---|---|
| GET | `/` | anyone | the list |
| GET | `/posts/:id` | anyone | the post, its comments and the form; `404` for no such post |
| POST | `/posts/:id/comments` | anyone | `303` to `/posts/:id#comments` |
| GET | `/login` | anyone | the form |
| POST | `/login` | anyone | `303` to `/admin`, or the form again with `401` |
| POST | `/logout` | a session | `303` to `/` |
| GET | `/admin` | the admin | the posts, to edit or delete |
| GET | `/admin/new` | the admin | the editor, empty |
| POST | `/admin/posts` | the admin | `303` to `/admin` |
| GET | `/admin/posts/:id/edit` | the admin | the editor, filled |
| POST | `/admin/posts/:id` | the admin | `303` to `/admin` |
| POST | `/admin/posts/:id/delete` | the admin | `303` to `/admin` |
| POST | `/admin/comments/:id/delete` | the admin | `303` to the post |

- **Any other `GET`** is a file of `dist/`, or the not-found page with
  `404`.
- **Without a logged-in session,** a `GET` of an admin page is `303` to
  `/login`, and a `POST` to one is `403`.
- **A form that fails a check** is rendered again, with its values and
  its messages, and `400`; only a `POST` that succeeds redirects.
- **A `POST` whose CSRF token is missing or wrong** is `403`, with a page
  that says the form has expired, and changes nothing.
- The same `routes()` in `app.nova` give each path its page on both
  sides; the server's `Server` routes give it its data and its status.

### 6.3 The data

- **`Data`,** in `app.nova`, is what a page is rendered from on both
  sides, one record for every page, as `hydrate_with` needs: who is
  logged in, the CSRF token, the posts or the post, its comments, a
  form's values and messages. Its `ToJson` and `FromJson` are written by
  hand.
- **The store,** in `store.nova`, holds the posts, the comments, the
  users and the next ids. Posts keep the order they were written in, and
  views show them newest first; no view iterates a `Map`.
- **The server loads the store at start,** from `<data>/blog.json`, or
  from `<data>/seed.json` when that does not exist yet. It keeps it in
  memory, and a handler changes it there, then spawns the write: the
  whole store to `<data>/blog.json.tmp`, then `rename` over
  `<data>/blog.json`. Spawned writes run after the response, in order.
- **A post** has an id, a title, a body, and when it was written and last
  changed, as `SystemTime::to_iso8601` writes them. **A comment** has an
  id, its post's id, a name, a text and when it was written. Deleting a
  post deletes its comments.

### 6.4 Sessions and CSRF

- **The key** is 32 bytes from `random_bytes`, written in hex to
  `<data>/secret` the first time the server starts, and read from there
  afterwards.
- **A session** is `id`, 32 random bytes in hex, in a table from id to
  who is logged in, if anyone, and when it ends. The cookie is `sid`,
  `<id>.<tag>`, the tag `hmac_sha256(key, id)` in hex, checked with
  `hmac_sha256_verify`. A cookie whose tag is wrong, or whose id the table
  does not hold, or holds as ended, is no session.
- **Every page `GET` without a session starts one,** a visitor's, and
  sets its cookie, so every form can carry a token. A visitor's session
  ends after a day, a logged-in one after eight hours. Starting a session
  first drops the ended ones from the table.
- **Logging in** checks the password, then starts a new session for the
  admin, and the old one ends, so an id set before the login is worth
  nothing after it. **Logging out** ends the session and expires the
  cookie.
- **The CSRF token** is `hmac_sha256(key, "csrf:" + id)` in hex, in a
  hidden `csrf` field of every form. Each `POST` handler checks it
  against the session with `hmac_sha256_verify` before it reads anything
  else.

### 6.5 Passwords and the admin

- **A user** is a name, a salt of 16 random bytes, an iteration count and
  a hash of 32 bytes, the last three from `pbkdf2_sha256` in hex.
- **`fullstack-blog-server --set-password <name>`** reads one line from
  standard input, less its line end, and refuses one shorter than eight
  characters. It hashes it with a new salt, writes the user into
  `<data>/blog.json`, made from the seed if it does not exist, replacing a
  user of that name, prints `blog: set the password of <name>`, and ends.
  `--iterations <n>` changes the count from 600,000, for the tests.
- **Logging in** runs `pbkdf2_sha256_verify` with the user's salt and
  count. For a name no user has, it runs the same count against a fixed
  salt, so the answer takes as long, and says the same: wrong name or
  password.

### 6.6 Checks, the client and escaping

- **`forms.nova`** holds the checks both sides run: a title of 1 to 200
  characters after trimming, a body of 1 to 20,000, a comment's name of 1
  to 60, its text of 1 to 2,000, and a login's name and password not
  blank. The server runs them on every `POST`; the client runs them as
  the fields change.
- **The client** hydrates every page. In the comment form, the login
  form and the editor, each field's value is a signal set by `on_input`;
  a field's message shows under it, and the submit button is disabled
  while the form is not valid. The server renders the button enabled and
  no messages, and the client applies its checks once it has hydrated
  (`on_mount`), so a form still works without JavaScript and hydration
  finds what it expects.
- **Links are plain anchors,** so each page arrives from the server with
  its own data.
- **Escaping:** every title, body, name and comment reaches the page as a
  text node or an attribute value, which `std/ui` escapes on the server
  and sets as text on the client. No `href` holds text from a visitor; a
  post's link is built from its id.

### 6.7 The server's arguments

- `--address <host:port>`, `127.0.0.1:3000` unless given; port 0 lets the
  system choose. Once bound, it prints
  `blog: serving http://<host>:<port>/`.
- `--data <dir>`, `data` unless given, and `--dist <dir>`, `dist` unless
  given, both from the directory it runs in.
- `--set-password <name>` and `--iterations <n>` (§6.5).
- It stops on SIGTERM or SIGINT, as `Server` does.

## 7. Testing

### 7.1 Unchanged rules

- Tests never touch the internet. A test that needs `$NOVA_HOME` sets it
  to a fresh directory; temporary directories have fixed names.
- Servers bind `127.0.0.1:0`.
- MSRV is Rust 1.78. No new crate, and `Cargo.lock` does not change.

### 7.2 std, natively

- **Runtime programs** with goldens under `tests/runtime/`, run by
  `run_tests.rs`:
  - routes: each method, patterns and `req.params`, the order of
    routes, `405` and its `allow`, the fallback, and `get` with a query;
  - `query`, `form` (and its `content-type` rule) and `cookies`;
  - `Cookie`, `with_cookie`, several `set-cookie` lines on the wire, a
    cookie left out, `html`, `redirect` and the new reason phrases;
  - `rename` over an existing file;
  - `pbkdf2_sha256` against RFC 7914's two vectors, and `_verify`;
  - hex both ways, and the refused inputs.
- **`ui::ssr`:** `Dist::open`'s errors, `file` and `respond`, and
  `serve`'s tests unchanged.
- **Doc tests** for each new public function.

### 7.3 The blog's own tests

- **`nova test`** in the example runs `tests/*.nova` through
  `src/lib.nova`: the store's JSON both ways, sessions and their tags,
  CSRF tokens, a password hashed and checked with a low count, the form
  checks, and paragraphs.
- `run_tests.rs` runs it, on all three systems.

### 7.4 `blog_tests.rs`, on all three systems

It copies the example to a fixed temporary directory, builds it with
`nova build --ssr`, sets the admin's password through standard input with
a low count, and starts the server on port 0. Over HTTP it checks:

- every page's status, and a page's data and markup;
- a comment without a CSRF token, or with another session's, is `403`
  and changes nothing; with the right one it is `303`, and the comment is
  on the page;
- a title and a comment holding `<script>` come back escaped;
- a wrong password is `401`; the right one is `303` with a new `sid`;
- the admin's create, edit and delete, and deleting a comment;
- an admin page without a session is `303` to `/login`, and an admin
  `POST` is `403`;
- logging out ends the session;
- after the server stops and starts again, the posts are still there.

### 7.5 The gate, in CI's `browser` job

`node tests/browser/blog.mjs target/debug/nova "$RUNNER_TEMP/blog"
"$RUNNER_TEMP/pw"`, after `ssr.mjs`, builds the example as
`blog_tests.rs` does, and in Chromium:

- opens every page, with no console error and no hydration mismatch, and
  checks that a page's nodes were kept;
- types into the comment form: the button is disabled while a field is
  blank, then posts, and the comment appears;
- logs in through the form, then creates, edits and deletes a post
  through the admin's pages;
- posts a title and a comment holding a script that would set a global
  and open a dialog, and checks that neither happens and the text shows;
- removes a form's token, submits it, and gets `403`;
- prints one line with what it did and the client's gzipped size.

### 7.6 Checks

`cargo fmt`, `cargo clippy -D warnings`, `nova fmt --check` over `std`,
`examples` and the new tests, std's doc tests, the MSRV check, and
`git diff` on `Cargo.lock` empty.

## 8. Crates and files

- **`std/http/lib.nova`:** §3.
- **`std/ui/ssr/lib.nova`:** `Dist`; `serve` rebuilt on it.
- **`std/fs/lib.nova`,** with a `fs_rename` builtin in `nova-runtime`
  and its declaration where `fs_read`'s is.
- **`std/crypto/lib.nova`,** with `crypto_pbkdf2_sha256` and its verify
  in `nova-runtime`, on `ring`.
- **`std/bytes/lib.nova`:** hex.
- **`examples/07-fullstack-blog/`:** §6.
- **Tests:** the runtime programs, `blog_tests.rs`, `blog.mjs`, and a
  step in `.github/workflows/ci.yml`.
- **Updated for the two new fields:** each `Request` and `Response`
  literal in std, `examples/05-json-api` and `tests/runtime`.

## 9. Records

- **ADR 0046,** "The blog, and what std gains for it": the decisions
  above, the departures from `60-EXAMPLES.md` §7 (a JSON file, a flat
  `src/`), the two new fields and `get`'s change, and what stands (§10).
- **Dated notes:** `20-STDLIB.md` §5 (`std/fs`), §6 (`std/http`), §8
  (`std/crypto`) and §19 (`std/bytes`); `60-EXAMPLES.md` §7 and §10;
  `30-FRONTEND.md`, on hand-written SSR servers.
- **The phase plan's 4.7 amendment,** CHANGELOG, and ARCHITECTURE where
  it names std's modules.

## 10. Risks

1. **PBKDF2 blocks the server.** 600,000 iterations run on the one
   thread, so a login holds every other request for its length. The
   prototype measures it, natively, in debug and release builds.
2. **A crash between a response and its write** loses that change. The
   rename means the file is always whole, but not always the newest.
3. **The two new fields break `Request` and `Response` literals** in
   programs outside the repository. No release has them, and ADR 0046
   records it.
4. **`get` no longer sees the query.** A program that routed on it
   changes behaviour. None in the repository does.
5. **Visitor sessions grow the table.** Ended sessions are dropped
   whenever one starts, so the table holds at most a day of visitors.
6. **The blog's tests build a native server on three systems,** which
   is slow, and Windows locks a running executable. They share one build
   per test file where they can, and kill the server before cleaning up.
7. **Hydration can disagree with the server** if a view depends on
   something only one side knows. The forms' checks start after
   `on_mount` for that reason, and the gate checks for mismatches.

## 11. Not in 4.7

- Handlers that await; async file access from a handler.
- Nested module directories.
- A development mode for SSR in `nova dev` (the backlog).
- `Secure` cookies by default, HTTPS, and a limit on login attempts. The
  README says what a public deployment adds.
- More than one admin through the UI; Markdown; images; search; pages of
  posts.
- Lighthouse and the size checks, which are 4.8's.
- Chunked bodies, `HEAD`, compression and environment variables.
- Refusing `javascript:` URLs, preventing a submit, or `fetch` in shared
  components, in `std/ui`.

## 12. Decisions made while writing this spec

1. **A route's pattern is matched without the query, piece by piece,
   decoded, by the router's rules,** and `get` takes patterns too. The
   alternative, a second kind of route, would leave `get` the only one
   that sees the query.
2. **`405` with `allow`** when a path's routes have other methods, before
   the fallback.
3. **`Request.params`,** as `20-STDLIB.md` §6 shows, rather than a
   handler with a second argument.
4. **`query` and `form` share one decoder;** the last of two names wins,
   as `std/ui/router`'s `use_query` does. `cookies` keeps the first.
5. **`Response.cookies`,** a field of its own, rather than a header map
   that holds several values.
6. **`Cookie::new`'s defaults are the safe ones:** `HttpOnly` and
   `SameSite=Lax`.
7. **`ui::ssr::Dist`,** the parts of `serve`, rather than a `Site` whose
   `render` takes a `Request`; `serve` keeps its shape for programs that
   need no more.
8. **Hex in `std/bytes`,** for both targets; no base64.
9. **The CSRF token is derived from the session,** so there is nothing
   more to store.
10. **A failed form is rendered again with `400`;** only success
    redirects.
11. **A visitor's session lasts a day and the admin's eight hours.**
12. **A login with an unknown name costs as much as one with a known
    name.**
13. **The forms' checks start after `on_mount`,** so the server's markup
    is the same with or without JavaScript.
