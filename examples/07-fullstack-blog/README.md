# 07-fullstack-blog

A blog that a visitor reads and comments on, and that its admin writes, from
a browser: a server on `std/http` that renders every page, and a client that
hydrates it.

## What this demonstrates

- **A hand-written server beside the built site.** `src/server.nova` is
  `std/http`'s `Server` with the blog's routes, by method and with `:id`
  parameters (`req.params`), and a fallback that asks `ui::ssr::Dist` for
  the built site's files. Each page is rendered with `page_with`, its data
  after it, and answered with the status and cookie it needs.
- **Hydration.** `src/main.nova`, the client, built for WASM, adopts the
  server's HTML with `hydrate_with`: one `Data` record, its JSON written by
  hand, carries what any page needs. The same `routes()` in `src/app.nova`
  give each path its page on both sides. Links are plain anchors, so each
  page arrives from the server with its own data.
- **Forms that work without JavaScript.** Every form is a plain HTML form
  whose `POST` is answered with `303` when it succeeds (post, redirect,
  get), and with the form again, its values and messages, and `400` when a
  field fails a check. Once the page has hydrated, the client runs the same
  checks (`src/forms.nova`) as the fields change: each field's message
  shows under it, and the submit button is disabled while the form is not
  valid.
- **Sessions and CSRF tokens.** Every visitor gets a session: a random id in
  a table in memory, carried by the cookie `sid` as `<id>.<tag>`, the tag
  `hmac_sha256` of `sid:` and the id, `HttpOnly` and `SameSite=Lax`. Every
  form carries a token derived from the session, `hmac_sha256` of `csrf:`
  and the id, so no token is a tag, and a `POST` without the right one is
  refused with `403`. Logging in starts a new session, so an id set before
  it is worth nothing after; logging out ends it (`src/auth.nova`).
- **Passwords stored only as hashes,** PBKDF2-HMAC-SHA256 with a random
  salt and OWASP's 600,000 rounds, the count kept with each hash. No
  password is in the repository: the server sets the admin's from standard
  input.
- **Escaping by construction.** Every title, body, name and comment reaches
  the page as a text node or an attribute's value, which `std/ui` escapes,
  and no `href` holds a visitor's text. A post's body is plain text; a blank
  line starts a new paragraph.
- **A store in JSON files.** The posts and comments live in memory; each
  change is written whole to `data/blog.json.tmp`, then renamed over
  `data/blog.json`, so if the server stops, or its process dies, the file
  is the old one or the new one, never half of one (`src/store.nova`).
  `data/seed.json`, in the repository, is where the first start reads
  from. The users are in `data/users.json`, which only `--set-password`
  writes, the same way, and the server reads when it starts.

## Run it

```sh
cd examples/07-fullstack-blog
nova build --ssr
stty -echo; IFS= read -r PW; stty echo
printf '%s\n' "$PW" | target/debug/fullstack-blog-server --set-password admin
unset PW
target/debug/fullstack-blog-server
```

The commands are for a POSIX shell. `nova build --ssr` builds the client
into `dist/` and the server into `target/debug/fullstack-blog-server`
(`.exe` on Windows). The third line reads the admin's password as you type
it, without showing it, and since no command line holds it, neither does
the shell's history; press Enter after it. `printf`, built into the common
shells, hands it to the server on standard input, not as an argument a list
of processes would show. `--set-password` reads one line, of at least eight
characters and not blank, writes the user `admin` and its hash into
`data/users.json`, and ends.
Then the server serves the blog on `127.0.0.1:3000`; open it in a browser,
and log in at `/login`. Ctrl-C stops it, once the writes it has started are
done.

To set a password, stop the server, run `--set-password`, then start the
server. The server reads `data/users.json` only when it starts, so a
password set while it runs takes effect at its next start. Nothing is lost
meanwhile: only `--set-password` writes `data/users.json`, and only the
server writes `data/blog.json`. Run one `--set-password` at a time: each
hashes first and reads `data/users.json` only then, so one that runs while
another hashes is kept, but two that read and write the file in the same
few milliseconds can lose one's user.

The server's arguments: `--address <host:port>`, port 0 for one the system
chooses; `--data <dir>` and `--dist <dir>`, `data` and `dist` unless given;
and `--iterations <n>`, PBKDF2's rounds for `--set-password`, which most
tests lower to 1,000.

`nova test` runs `tests/*.nova`, which reach the server's modules through
`src/lib.nova`.

## Expected output

```
built dist
built target/debug/fullstack-blog-server
blog: set the password of admin
blog: serving http://127.0.0.1:3000/
```

`/` lists three posts, newest first, each with its date and first
paragraph. A post's page shows its paragraphs, its comments and a form to
add one, whose button stays disabled until both fields are filled. After
logging in, `/admin` lists the posts with a link to edit each and a button
to delete it, and `Write a new post` opens the editor.

## Notes

- **What a public deployment adds:** TLS, with the session cookie made
  `Secure` (`Cookie`'s `secure`, which `std/http` leaves off so the blog
  works over plain HTTP on `127.0.0.1`); a limit on login attempts;
  keeping `data/` private: the key in `data/secret` and the password hashes
  in `data/users.json` are written with the system's default permissions,
  since `std/fs` sets no mode; and a copy of `data/` kept elsewhere: nothing
  flushes a file to disk before it is renamed into place, which `std/fs`
  cannot do, so after a power loss or a crash of the system, a file system
  that does not flush first can leave `data/blog.json` or
  `data/users.json` empty, and the server then refuses to start. PBKDF2's
  600,000 rounds hold the
  single-threaded server for each login, about 2.6 seconds on a debug
  runtime.
- **The session table is in memory,** so a restart logs everyone out. The
  signing key is kept in `data/secret`, made at random on the first start.
  Every page asked for without a cookie starts a session. The ended ones
  are dropped once the table has grown to twice the size it had after they
  were last dropped, and to at least 64, so it holds at most twice the
  sessions live at the last scan, or 64 if that is more, and sessions that
  end after a burst stay until it doubles again. The scans cost the same
  per session on average: on a debug runtime, a page asked for without a
  cookie took 1.5 to 1.9 ms with 200 to 8,200 sessions in the table, where
  dropping them at every start took 1.6 ms at 200 and 12.8 ms at 8,200.
  The request that finds the table doubled scans all of it, in time linear
  in its size: the one that started the 16,385th session took 18.4 ms, and
  the 32,769th's 35.7 ms. A session takes about 658 bytes, so a client
  that keeps no cookie and asks for pages as fast as the server answers,
  391 a second in a probe, would grow the server's memory by about 22 GB
  in a day.
- **`std/ui` does not refuse a `javascript:` URL** in an `href`. The blog
  never puts a visitor's text in one: a post's link is built from its id.
- **`src/` is flat,** unlike `nova-spec/60-EXAMPLES.md` §7's tree: a nested
  module path is not supported yet. The store is a JSON file, not SQLite
  (ADR 0038).
- **The submit button** is two buttons, each under a `show`: `std/ui` never
  removes an attribute, and `disabled` disables a button by being there at
  all, so the server's HTML holds the enabled one, which a page without
  JavaScript keeps, and the client swaps them as the form's checks change.
- `nova dev` does not serve a server of its own yet: build with
  `nova build --ssr` and start the server.
- `cargo test` builds it and drives its server over HTTP
  (`crates/nova-cli/tests/blog_tests.rs`), and runs its `nova test`
  (`run_tests.rs`); CI's `browser` job drives it in Chromium
  (`tests/browser/blog.mjs`). The 4.7 spec is
  `docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md`.
- The same job runs Lighthouse three times, mobile, on `/`, `/posts/1` and
  `/login`, and needs a median Performance score of 95 or more for each
  (`tests/browser/lighthouse.mjs`). It also builds the blog with
  `nova build --ssr --release` and binaryen's `wasm-opt` on `PATH`, which
  takes the client from 49,472 bytes gzipped (by zlib at level 9) to
  27,510 with binaryen 132, and drives that build; the 4.8 spec is
  `docs/superpowers/specs/2026-10-22-phase-4-8-the-gate-and-v0.4.0-design.md`.
