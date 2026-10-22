# ADR 0046 — The blog, and what std gains for it

## Status

Accepted, 2026-10-21 (Phase 4.7, branch
`claude/loving-archimedes-frtf5q-47`; spec
`docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md`).
std grown for the blog; plain HTML forms answered with `303`, and the whole
page hydrated; PBKDF2 natively through `ring`; the admin's account made by
the server; comments from anyone; sessions as signed random ids in a table
in memory; the JSON file written whole, then renamed; a flat `src/`; the
tests and the gate; plain text bodies; `nova dev` unchanged; and the name
`fullstack-blog` are the user's decisions of 2026-10-21 (decisions 1 to 12
below). The spec settled the rest (its §12). What 4.7 built beyond the
spec, or otherwise than it, is under "What departs from the spec, or fills
it in".

## Context

After 4.6 a program could render its pages natively and hydrate them in the
browser, but its server answered `GET`s alone. The phase plan's decision 11
has 4.7 build `examples/07-fullstack-blog` as `60-EXAMPLES.md` §7 describes
it, with a JSON file in place of SQLite (ADR 0038): its server a Nova
program on `std/http`; sessions in cookies signed with `hmac_sha256`,
`HttpOnly` and `SameSite`; a CSRF token in every form; text from users
escaped by both renderers by construction; and passwords hashed as 4.7's
spec decides, "for example PBKDF2 over `hmac_sha256`, written in Nova". Its
4.7 gate, in the CI job through Playwright: every page works; the admin's
full CRUD works; a script put into a title or a comment stays inert text;
and a form without its CSRF token is refused. ADR 0045 left to 4.7 `POST`,
cookies and path parameters in `std/http`; loading a page's data per
request, since `render` is synchronous; whether the comments need more than
whole-page hydration; and, with the backlog, a development mode for SSR.
The repository 4.7 starts from (spec §2, read at `9ad78cd`) has:
- a `Server` that serves `GET` alone: `get(path, h)` matches the whole
  target, query included, so `/add?x=1` misses `/add`; `fallback` answers
  any other `GET`, and every other method gets `404`; and a handler is
  `fn(Request) -> Response`, which cannot await;
- a `Request` with nothing decoded: no reader for a query, a form, a
  cookie or a parameter;
- a `Response` whose headers are a `Map`, so a second `set-cookie`
  replaces the first; no builder for HTML or a redirect; and `303` written
  `303 Redirection`;
- `ui::ssr::serve`, which builds its own `Server` with one fallback and no
  way to add a route, whose `Site.render` sees the location alone, and
  whose reading of the template and serving of the manifest's files are
  private; `page`, `page_with` and `document` are public;
- no `std/fs::rename`, and `std/fs` functions that are `async fn`s which
  never suspend: a handler cannot await them, but can `spawn` them, and the
  spawned writes run after the response, in order;
- `hmac_sha256`, its constant-time verify and `random_bytes` in
  `std/crypto`, but no PBKDF2: one written in Nova matched RFC 7914's
  vectors but took about 0.84 s per 100,000 rounds, during which the
  single-threaded server answers nothing; and no hex or base64;
- `hydrate_with`, which reads one value of one type for the whole app,
  once, so a client-side link to another route would render it with the
  first page's data;
- `std/ui`, which escapes every text and attribute value and refuses
  `innerHTML`, `outerHTML` and `srcdoc`, but not a `javascript:` URL in
  `href`;
- a nested module path, `import routes::home`, which is E0900.

## Decision

### The user's decisions

1. **std grows what the blog needs.** `std/http` gains routes by method,
   path parameters, readers for a form and for cookies, several
   `Set-Cookie` headers on one response, and responses for HTML and for a
   redirect. `ui::ssr` gives a program the parts `serve` is made of, so a
   server can put its own routes beside the built site, and each of its
   pages sees the whole `Request`.
2. **Forms are plain HTML forms, and every `POST` that succeeds is
   answered with `303`** (post, redirect, get). Every page works without
   JavaScript. Hydration adds live checks: a blank or too long field is
   reported as it is typed, and the submit button is disabled until the
   form is valid. The whole page hydrates; there are no islands, and
   `std/ui` gains nothing.
3. **Passwords are hashed with PBKDF2-HMAC-SHA256,** which `std/crypto`
   gains natively through `ring`, already in `Cargo.lock`. It runs OWASP's
   600,000 rounds unless told otherwise, and stores the count with each
   hash.
4. **The admin's account is made by the server itself:**
   `fullstack-blog-server --set-password <name>` reads the password from
   standard input and writes its hash into the data directory, in
   `users.json` (below). No password is known from the repository; the
   tests and the README run the command.
5. **Anyone may comment,** with a name and a text. Every visitor gets a
   signed session cookie, logged in or not, which the CSRF token is bound
   to. The admin can delete a comment.
6. **A session is a random id, signed, in a table in memory.** The cookie
   carries the id and its `hmac_sha256` tag, `HttpOnly` and
   `SameSite=Lax`. The server keeps who each id belongs to and when it
   ends. Logging out ends it; a restart ends every session. The signing
   key is made at random the first time the server starts, and kept in the
   data directory.
7. **The JSON file is written whole, then renamed into place,** so a crash
   never leaves half a file: `std/fs` gains `rename`. The seed data is in
   the repository; the file the server writes is ignored by git.
8. **`src/` is flat.** A nested module path is E0900, so the modules of
   `60-EXAMPLES.md` §7's tree sit side by side in `src/`.
9. **The tests:** `crates/nova-cli/tests/blog_tests.rs` builds 07 with
   `--ssr` on all three systems and drives its server over HTTP;
   `nova test` runs the server's own modules' tests, through the package's
   `src/lib.nova`; and the gate runs in CI's `browser` job through
   Playwright, in Chromium (`tests/browser/blog.mjs`).
10. **A post's body is plain text.** A blank line separates paragraphs, and
    each paragraph is a `<p>` whose text is escaped.
11. **`nova dev` does not change in 4.7;** a development mode for SSR goes
    to the backlog. The README builds with `nova build --ssr` and starts
    the server.
12. **The package is `fullstack-blog`,** so its server is
    `target/<profile>/fullstack-blog-server`.

### Decided in the spec (§12)

1. **A route's pattern is matched without the query, piece by piece,
   decoded, by the router's rules,** and `get` takes patterns too. A
   pattern's piece is literal, or `:name`, which matches any one piece and
   binds it; the request's path, its target before any `?`, is split at
   `/`, its empty pieces dropped and each percent-decoded, as
   `std/ui/router` splits a route's path, so `/admin/` matches `/admin`.
   The alternative, a second kind of route, would have left `get` the only
   one that sees the query.
2. **`405` with `allow`** when a path's routes have other methods, before
   the fallback. `dispatch` tries the routes in the order they were added;
   the first whose method and pattern match answers; otherwise a route
   whose pattern matches gives `405 Method Not Allowed`, its `allow`
   header naming the methods of the routes that match, each once, in the
   order they were added; otherwise a `GET` goes to the `fallback`, when
   there is one; otherwise `404`. `HEAD` gets no special treatment.
3. **`Request.params`,** as `20-STDLIB.md` §6 shows it,
   `req.params.get("id")`, rather than a handler with a second argument.
   It is empty in a request `parse_request_head` or `read_request` returns.
4. **`query` and `form` share one decoder,**
   `application/x-www-form-urlencoded`'s: pairs split at `&`, an empty one
   skipped, each split at its first `=`, `+` a space, `%XX` decoded as
   UTF-8, and the last of two names winning, as `std/ui/router`'s
   `use_query` does. `form` reads the body only when the `content-type`
   header's media type is that one and the body is UTF-8. `cookies` reads
   the `cookie` header, and keeps the first of two names, as a browser
   sends the most specific first; its values are not decoded.
5. **`Response.cookies`,** a field of its own, which `with_cookie` adds to
   and `to_bytes` writes, one `set-cookie` line each, after the other
   headers, rather than a header map that holds several values.
6. **`Cookie::new`'s defaults are the safe ones:** path `/`, no `Max-Age`,
   `HttpOnly` and `SameSite=Lax`, and not `Secure`. `Cookie::expired(name)`
   has the value `""` and `Max-Age=0`.
7. **`ui::ssr::Dist`,** the parts of `serve`, rather than a `Site` whose
   `render` takes a `Request`; `serve` keeps its shape for programs that
   need no more. `Dist::open(dir)` reads the template and the manifest with
   every check and error `serve` made; `file(req)` answers a request for a
   file the manifest lists, or returns `None`; `respond(status, page)`
   returns `document(template, page)` as HTML with `status`. `serve` is
   rebuilt on `Dist`, and its tests are unchanged.
8. **Hex in `std/bytes`,** for both targets: `Bytes::to_hex`, lower case,
   and `bytes_from_hex`, which takes either case. No base64.
9. **The CSRF token is derived from the session,**
   `hmac_sha256(key, "csrf:" + id)` in hex, so there is nothing more to
   store. Each `POST` handler checks it with `hmac_sha256_verify` before it
   reads anything else, and answers `403` when it is missing or wrong.
10. **A form that fails a check is rendered again with `400`,** and a
    login with a wrong name or password with `401`; only success
    redirects.
11. **A visitor's session lasts a day and the admin's eight hours.**
    Ended sessions are dropped from the table (when, below); logging in
    ends the old session and starts a new one, so an id set before the
    login is worth nothing after it.
12. **A login with an unknown name costs as much as one with a known
    name:** it runs the same verify against a fixed salt, and says the
    same, wrong name or password.
13. **The forms' checks start after `on_mount`,** so the server's markup is
    the same with or without JavaScript, and hydration finds what it
    expects.

## What departs from the spec, or fills it in

- **Percent-decoding is the router's, whole or not at all.** A path's
  piece, or a query's or form's name or value, is decoded as
  `std/ui/router`'s `decoded` decodes it, and kept as written when it
  holds a `%` not followed by two hex digits or its bytes are not UTF-8,
  but that a query's or form's `+` is still a space: `%zz%41` stays
  `%zz%41`, where spec §3.2 had only the bad `%` stay as it is. A server's
  route and a client's so agree on every path.
- **`pbkdf2_sha256` panics for a length above 65,536** too, the cap
  `random_bytes` has, where spec §5 had it panic below 1 only. The length
  is checked before the key is allocated, so no length a program passes
  can ask for an allocation the process cannot make, or reach the length
  past which `ring::pbkdf2::derive` unwraps an error. The panics read
  `<function>: iterations must be from 1 to 4294967295, not <n>` and
  `pbkdf2_sha256: length must be from 1 to 65536, not <n>`.
- **`pbkdf2_sha256_verify` takes the start of the right key as the key.**
  It derives a key of `expected`'s own length, as `ring::pbkdf2::verify`
  does, and PBKDF2's shorter output is the start of its longer one; an
  empty `expected` gives `false`. Its doc tells a caller to keep and check
  the whole key and its length, and the blog refuses a stored hash that is
  not 32 bytes.
- **Two cookie rules beyond the spec's:** a `same_site` must be empty or
  an HTTP token, so it cannot carry a `;` and an attribute of its own; and
  a value in double quotes, which RFC 6265 allows, is refused, keeping one
  rule for a value: RFC 6265's cookie-octets, so no space, `"`, `,`, `;`,
  `\`, control character or anything outside ASCII. A name must be a token,
  and a path any ASCII character but a control character or `;`. A cookie
  that breaks a rule is left out of the response, without an error.
- **The `405`'s body is `method not allowed`,** as `text/plain`, which the
  spec did not name; and **`Dist::file` does not look at the method,**
  since `Server` asks its fallback only for a `GET`.
- **The blog's explicit `GET` routes answer `404` for its paths that have
  only a `POST` route** (`/posts/:id/comments`, `/logout`, `/admin/posts`,
  `/admin/posts/:id`, `/admin/posts/:id/delete` and
  `/admin/comments/:id/delete`), with the not-found page, as spec §6.2's
  table asks of any other `GET`; without them `std/http` would answer
  `405`.
- **The example's `target/` is in `.gitignore`,** which a build in place
  makes, beside the spec's `dist/`, `data/blog.json` and `data/secret`;
  and so are `data/blog.json.tmp`, which a write leaves should it stop
  before the rename, and `data/users.json` and `data/users.json.tmp`
  (below).
- **`Data` gains `missing` and `refused`,** and `routes()` maps the paths
  forms are posted to. `missing` renders the not-found page, for a path
  with no page and a post that does not exist; `refused` renders a refused
  form's reason in place of the route's page, with `403`. A form that fails
  a check is rendered again at the path it was posted to, which is then the
  browser's location, so `routes()` gives `/posts/:id/comments`,
  `/admin/posts` and `/admin/posts/:id` their pages, the post's and the
  editor's, on both sides. A form's values and messages are `Data`'s
  `draft`, whose `failed` shows the messages from the start and whose
  `error` holds a message for the whole form, such as a wrong password.
- **The submit button is two buttons, each under a `show`.** `std/ui`
  writes every attribute it is given and never removes one, and `disabled`
  disables a button by being there at all, whatever its value, so one
  button cannot be both. The server's HTML holds the enabled one, which a
  page without JavaScript keeps; the client swaps them as the form's
  checks change.
- **Stored text is trimmed, holds no NUL, and has its carriage return and
  line feed pairs made line feeds** (`forms.nova`'s `cleaned`). A
  browser's parser drops a line feed that begins a `textarea`'s text, so a
  body stored with one would come back to the editor as another text, and
  mismatch on every hydration (ADR 0045's consequences); it drops a NUL
  from a page's text, and makes it U+FFFD in a `textarea` or an attribute,
  so one comment holding a NUL broke the hydration of its post's page for
  every visitor, and `std/ui` cannot write a NUL that the parser keeps; and
  a browser sends a `textarea`'s lines ending in CRLF, where the field's
  value holds LF. The checks of a post's and a comment's fields measure a
  value as `cleaned` leaves it, on both sides, so both measure the same
  text. In Chromium, no other character
  below U+0020, nor U+007F, broke hydration: not in a post's title and
  body and a comment's name and text on the post's page, nor in a failed
  comment's name and text, where they are an attribute's value and a
  `textarea`'s text; the parser keeps them, form feed included, and
  `std/ui` writes a lone carriage return as `&#13;`. The login's name is
  trimmed, and its password is not.
- **An unknown login name uses the largest iteration count of the users'
  hashes** that PBKDF2 takes, or, when there is none, `--iterations`,
  600,000 unless given, which the spec's "the same count" left open, so a
  name no user has takes as long as the name of the user whose hash has
  the most rounds. A user whose hash has fewer rounds than another's, such
  as one set with a lower `--iterations`, still answers sooner, so the time
  tells that name from one no user has; passwords all set with the same
  count close it.
- **`--set-password` refuses, and writes nothing:** with status 2, a name
  that is blank or has spaces around it, before it reads the password; and
  with status 1, a password that is not UTF-8, one shorter than eight
  characters, and one that is blank once trimmed, which the login's check
  refuses, so the user could never log in with it.
- **The users have a file of their own,** `<data>/users.json`, where spec
  §6.3 has the store hold them and §6.5 has `--set-password` write them
  into `blog.json`. Each file has one writer. `--set-password` hashes the
  password first, then reads `users.json`, no users when it does not
  exist, sets the user, writes `users.json.tmp` and renames it over
  `users.json`; it never reads or writes `blog.json`. The server reads
  `users.json` once, when it starts, and refuses to start when it cannot
  read users from it, as `--set-password` refuses then to write it; it
  never writes it. The server writes `blog.json`, the posts, the comments
  and the next ids, from its own copy, reading no file. `data/seed.json`
  has no `users` key, and the store ignores one. Both checks found the one
  file's two writers at fault. In the first, the server's next write put
  its own copy of the users over a password `--set-password` had set while
  it ran, so the old password stayed good, before a restart and after. The
  fix, each server write keeping the users the file held then, which this
  replaces, left two faults that the second check found: `--set-password`
  wrote back the copy it had read before its hash, about two seconds at
  600,000 rounds, erasing every change the server saved meanwhile, and
  under steady writes its own user too while it exited 0 (26 of 40 runs at
  1,000 rounds lost theirs in a probe); and each server write read the
  file, so a missing `blog.json` wrote the seed's users, none, erasing the
  admin, and an unreadable one stopped every write, after which a restart
  refused to start. With one writer per file, neither writes over the
  other's change: the second check's probes, run again on the new files,
  lost no comment and no user, and the server wrote a missing `blog.json`,
  or one holding bad JSON, afresh from its copy. The server still checks
  logins against the users it read when it started, so the README says to
  stop the server, run `--set-password`, then start it; a password set
  while the server runs takes effect at its next start.
- **Ended sessions are dropped once the table has doubled,** where spec
  §6.4 drops them each time a session starts: when the table holds at
  least 64 sessions, and at least twice as many as it kept after the ended
  ones were last dropped (`Sessions.kept`), starting one drops them first.
  The table so holds at most twice the sessions live at the last scan, or
  64 if that is more. At least half the sessions each scan reads have
  started since the last scan, so the scans cost the same per session on
  average, however many the table holds; but the request that finds the
  table doubled scans all of it, in time linear in its size: on a debug
  runtime, the one that started the 16,385th session took 18.4 ms, and the
  32,769th's 35.7 ms. Scanning at every start made the cost grow with the
  table: a `GET /login` without a cookie, on a debug runtime, took 1.59 ms
  with 200 sessions in the table, 4.91 with 2,200, 7.30 with 4,200, 10.08
  with 6,200 and 12.78 with 8,200; dropping them once the table has
  doubled, it took 1.52, 1.60, 1.66, 1.87 and 1.76 ms. A request with a
  cookie took 1.72 ms in the first run and 1.87 in the second.
- **`Dist::open`'s doc example is `ignore`,** where spec §7.2 asks a doc
  test of each new public function: it reads a site `nova build --ssr`
  wrote into `dist` and serves it, which a doc test has not. The doc tests
  of `file` and `respond` call `Dist::open` on a site they write, so it is
  exercised there.
- **`auth.nova` imports `store`,** for `User`, where spec §6.1 names only
  `server.nova` and `lib.nova` as importers of the server's own modules.
  Both are server modules, and the client, `main.nova`, reaches neither.
- **`lib.nova` is thin wrappers,** such as `store_from`, `sessions`,
  `user_with`, `logs_in` and `message_for`, since Nova has no re-exports:
  `tests/*.nova` import the library as `fullstack_blog`, can name only what
  it declares, and use what those functions return through its methods and
  fields.
- **The session cookie lasts as long as its session,** `Max-Age` 86,400 for
  a visitor and 28,800 for the admin, so a browser drops it when the server
  would; logging out sends `Cookie::expired("sid")`.
- **A refusal's details:** a `POST` with no session, to an admin's path or
  not, gets `403` with the expired-form page; one with a visitor's session
  and its right token, to an admin's path, gets `403` with "Only the admin
  can do that. Log in first."; a `403` page starts no session; and a
  `POST` that names a post or a comment that does not exist, once its
  token is checked, gets the not-found page with `404`.
- **The tests share one build:** `blog_tests.rs` builds the example once
  per run of the file, and each of its thirteen tests has a data
  directory of its own. Nine set the admin's password at 1,000 rounds. The
  test that times logins sets another user's at 1,000 first, then the
  admin's at 100,000, so that the hashing is most of a login's time and a
  name no user has must cost the admin's count, not the first user's; it
  needs the median of five logins with a name no user has and of five
  with a wrong password each to be at least half the other. Eleven start a
  server of their own; one of them posts comments while `--set-password`
  hashes at 600,000 rounds, and another first runs a `--set-password` at
  1,000 rounds to its end while one at 600,000 hashes, and needs both
  users in `users.json`. The server's refusal to start is checked on a key
  file that does not hold 32 bytes in hex, and on a `users.json` that
  does not hold users. The example's `nova test` runs 21 tests.

## What departs from `60-EXAMPLES.md` §7 and `20-STDLIB.md` §6

- **JSON files in place of SQLite,** by ADR 0038: `data/blog.json`,
  written by the server, made from `data/seed.json`, which is in the
  repository, the first time, and `data/users.json`, written by
  `--set-password`; there is no `std/db`.
- **A flat `src/`** (decision 8): the tree's `routes/` and `components/`
  modules, `home`, `post`, `login`, `admin`, `header` and `post_card`, sit
  in `src/`; `db.nova` is `store.nova`; `main.nova` is the client, and the
  server's entry is `server.nova`; and `app.nova`, which both share,
  `forms.nova`, the checks both run, and `lib.nova`, the tests' library,
  are added. The project also has `index.html` and `data/seed.json`.
- **The whole post page hydrates,** not only its comments (decision 2),
  and so does every other page.
- **Lighthouse is 4.8's,** with 06's; 4.7's gate is the rest of §7's.
- **`20-STDLIB.md` §6's handlers stay synchronous:** a handler is
  `fn(Request) -> Response`, since §6's `Handler`, an alias of
  `async fn(Request) -> Response`, still does not parse (P0001), and the
  routes return `Server`, not `Self`.
- **`get` takes a pattern, as every route does, and ignores the query**
  (§12, decision 1); `req.params.get("id")` returns an `Option<String>`.
  `use_middleware`, the `Handler` and `Middleware` aliases, and the client
  are still not shipped; `405` and `fallback` are not in §6.

## Consequences

- **The two new fields break `Request` and `Response` literals:** a
  program that writes one must name `params` or `cookies`. No release has
  either; in the repository, std, `examples/05-json-api` and
  `tests/runtime` were changed for them.
- **`get` no longer sees the query,** so a program that routed on it
  changes behaviour; none in the repository did. `/add/` now matches `/add`
  too.
- **A request whose path a route matches with another method is `405`,**
  where it was `404`, `HEAD` included, and a `GET` of a path that only a
  `POST` route matches no longer reaches the fallback.
  `examples/03-http-server` now answers `POST /` with `405` and
  `allow: GET`.
- **PBKDF2 holds the single-threaded server** for each login, with a
  known name or not: every other request waits meanwhile. 600,000 rounds
  took about 2.6 s with the runtime built for debugging (2,564 to 2,638 ms,
  three runs), and a program built with `nova build --release`, which
  links the same runtime, took as long; a runtime built for release is
  expected to take about 0.3 s, as `ring` built optimized took 303 to
  340 ms in a probe.
- **A crash between a response and the write it spawned loses that
  change.** The file is always whole, the old one or the new, but not
  always the newest.
- **A restart ends every session,** the admin's included, since the table
  is in memory; the key in `data/secret` stays.
- **Ended sessions stay in the table until it has doubled,** so it holds
  at most twice the sessions live at the last scan, or 64 if that is
  more: after a burst, the sessions that end stay until the table doubles
  again. A client that keeps no cookie starts a session with each page it
  asks for, and a session takes about 658 bytes, so one asking for pages
  as fast as the server answers, 391 a second in a probe, would grow it by
  about 22 GB in a day; and a steady stream of them keeps up to two days'
  worth, the live day and as many ended, where spec risk 5 expected at
  most a day of visitors.
- **`--set-password` takes effect when the server next starts,** since the
  server reads `users.json` only then; the README says to stop the server,
  run it, then start the server. A server running meanwhile loses nothing
  to it. Nor does a `--set-password` run that starts and ends while
  another hashes: each hashes first and reads `users.json` only then, so
  the slower one reads the faster one's user, as a test checks. Two runs
  can still lose one's user when their files meet after both hashes: when
  one reads `users.json` after the other has read it and before the other
  has renamed its own over it, a window of milliseconds. The README says
  to run one at a time.
- **The key and the password hashes are written with the system's default
  permissions,** since `std/fs` sets no mode; the README says to keep
  `data/` private.
- **The login's name is trimmed, not cleaned,** so a NUL in it reaches the
  page of the failed login; only the visitor who sent it sees that page,
  and Chromium kept its 26 nodes, with no mismatch.
- **A field of only U+FEFF, U+2028 or U+2029 is not blank,** since `trim`
  keeps them; both sides check it alike.
- **The client is 49,472 bytes gzipped** when `blog.mjs` printed it, built
  without `--release` and so without `wasm-opt`: just under the 50 KB the
  phase plan's decision 12 sets for 06, and 07 has no limit. 4.8's
  Lighthouse run has still to measure the page it loads.
- **`std/ui` does not refuse a `javascript:` URL** in an `href`. The blog
  never puts a visitor's text in one: a post's link is built from its id.
- **A cookie that breaks a rule is dropped silently,** such as a value
  with a space or a character outside ASCII; hex is safe.
- **No crate is added,** and `Cargo.lock` does not change.

## What is left out

- Handlers that await, and async file access from a handler.
- Nested module directories.
- A development mode for SSR in `nova dev`, which goes to the backlog.
- `Secure` cookies by default, HTTPS, and a limit on login attempts; the
  README says what a public deployment adds.
- More than one admin through the UI; Markdown; images; search; pages of
  posts.
- Lighthouse and the size checks, which are 4.8's.
- Chunked bodies, `HEAD`, compression and environment variables.
- Refusing `javascript:` URLs, preventing a submit, or `fetch` in shared
  components, in `std/ui`.

## References

- `docs/phase-4-plan.md` §3, decisions 11 and 12, §4's 4.7 entry, and §5's
  security rules.
- The spec named above.
- ADR 0038, the recorded scope; ADR 0044, the router whose rules a route
  follows; ADR 0045, SSR and hydration; ADR 0022, the shutdown signals the
  server inherits.
- `nova-spec/60-EXAMPLES.md` §7, §9 and §10; `nova-spec/20-STDLIB.md` §5,
  §6, §8 and §19; `nova-spec/30-FRONTEND.md` §7.4.
