# Phase 4.7, `examples/07-fullstack-blog`, Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** `examples/07-fullstack-blog`, a blog that a visitor reads and
comments on and its admin writes, from a browser. Its server, a Nova program
on `std/http`, renders every page with `ui::ssr` and sends the page's data
with it; its client, built for WASM, hydrates the page. Every form is a plain
HTML form: a `POST` that succeeds is answered with `303`, one whose fields
fail a check gets the form again with its values, its messages and `400`, and
one whose CSRF token is missing or wrong gets `403` and changes nothing.
Every visitor has a session, a random id signed with `hmac_sha256` in an
`HttpOnly`, `SameSite=Lax` cookie, kept in a table in memory, and the CSRF
token is derived from it. The admin's password is stored only as a PBKDF2
hash, which the server writes itself with `--set-password`. The store lives
in memory, and each change is written whole to `data/blog.json.tmp`, then
renamed over `data/blog.json`. std gains what the blog needs: `std/http`
routes by method, with `:name` patterns and `req.params`, `405` with
`allow`, `Request::query`, `form` and `cookies`, `Cookie`,
`Response.cookies` and `with_cookie`, `Response::html` and `redirect`, and
five reason phrases; `ui::ssr::Dist`, the built site a hand-written server
serves beside its own routes, on which `serve` is rebuilt; `std/fs::rename`;
`std/crypto::pbkdf2_sha256` and `pbkdf2_sha256_verify`, over `ring`; and hex
in `std/bytes`. The gate, in CI's `browser` job through Playwright: in
Chromium, every page of the blog hydrates in place, the admin creates, edits
and deletes a post through its pages, a script put into a title or a comment
stays inert text, and a form without its CSRF token is refused.

**Architecture:**
- **`std/http/lib.nova`:** `Request.params`; `Request::query`, `form` and
  `cookies`, with `form_fields`, `form_decoded`, `percent_decoded`,
  `hex_value` and `path_pieces`; `Route`, now a method and a pattern;
  `Server::get`, `post`, `put`, `delete` and `route`; `dispatch`, with
  `params_of`, `method_name`, `holds` and the `405`; `Cookie`,
  `Cookie::new` and `Cookie::expired`; `Response.cookies: [Cookie]`,
  `Response::html`, `Response::redirect` and `with_cookie`; `to_bytes`'
  `set-cookie` lines, through `cookie_line`, `is_token`, `is_tchar`,
  `is_cookie_octet`, `is_path_char` and `all_chars`; the reason phrases of
  301, 302, 303, 401 and 403.
- **`std/ui/ssr/lib.nova`:** `Dist`, with `open`, `file` and `respond`, and
  `site_path`; `Served` holds a `Dist`, and `serve` and `answer` use it.
- **`std/fs/lib.nova`:** `rename`. **`std/crypto/lib.nova`:**
  `pbkdf2_sha256`, `pbkdf2_sha256_verify` and `pbkdf2_refused`.
  **`std/bytes/lib.nova`:** `Bytes::to_hex`, `bytes_from_hex` and
  `hex_digit_value`, in Nova.
- **Three std-only builtins,** `fs_rename`, `crypto_pbkdf2_sha256` and
  `crypto_pbkdf2_sha256_verify`: `nova-resolver` (`STD_ONLY`, 90),
  `nova-typeck`'s signatures, `nova-mir`'s `RtFunc`s and lowering, and
  `nova-runtime`'s `nova_rt_fs_rename`, `nova_rt_crypto_pbkdf2_sha256` and
  `nova_rt_crypto_pbkdf2_sha256_verify`, on `ring::pbkdf2`, with
  `iterations_of`, `derive_key` and `verify_key`, in its symbol table.
- **`examples/07-fullstack-blog/`:** `nova.toml` (`fullstack-blog`),
  `index.html`, `public/styles.css`, `data/seed.json` and `README.md`; a
  flat `src/`: `main.nova`, the client; `app.nova`, `home.nova`,
  `post.nova`, `login.nova`, `admin.nova`, `header.nova`, `post_card.nova`
  and `forms.nova`, which both sides share; `store.nova`, `auth.nova` and
  `server.nova`, the server's alone; and `lib.nova`, the library its
  `tests/*.nova` reach the server's modules through.
- **Tests:** the runtime programs `bytes_hex`, `fs_rename`,
  `crypto_pbkdf2`, `crypto_pbkdf2_refused`, `http_routes`,
  `http_request_readers`, `http_responses` and `http_server_wire`, and
  `http_server_dispatch` and `http_serialise` changed; `nova-runtime`'s and
  `nova-mir`'s unit tests; `tests/ssr/site/src/dist.nova`; the blog's 20
  `nova test` tests; `crates/nova-cli/tests/blog_tests.rs`; and
  `tests/browser/blog.mjs` in the `browser` job.
- **Records:** ADR 0046, and dated notes.

**Tech Stack:**
- Rust: MSRV 1.78, edition 2021.
- **No new crate; `Cargo.lock` does not change** (spec §7.1). PBKDF2 is
  `ring`'s, already in the lock. The blog's server is `std/http`'s `Server`
  on `std/net`, in Nova.
- Nova: the blog, std and the runtime programs.
- JS: `tests/browser/blog.mjs`, an ES module run by Node 22, with
  Playwright 1.56.1, installed by the CI job only.
- Tests: unit tests in `nova-runtime`'s `crypto.rs`, `nova-typeck`'s
  `check.rs` and `nova-mir`'s `lower_tests.rs`;
  `crates/nova-cli/tests/run_tests.rs`, `wasm_tests.rs`, `doc_test.rs`,
  `ssr_tests.rs` and the new `blog_tests.rs`;
  `crates/nova-driver/tests/index_complete.rs`; programs in
  `tests/runtime/`; the project `tests/ssr/site/`; the blog's own `nova
  test`; doc tests in `std/bytes`, `std/fs`, `std/crypto`, `std/http` and
  `std/ui/ssr`.

**Spec:** `docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md`,
approved by the user on 2026-10-21 (`c8d6196`).
- Read it before Task 1: it is the authority this plan argues from.
- Its header holds the user's twelve decisions, its §12 the thirteen
  decisions made while writing it, and its §2 the code as it stood at
  `9ad78cd`. This plan's decisions say where the code departs from the
  spec, or fills in what it left open.

**Checked on 2026-10-21 and 2026-10-22, before execution:**
- **A prototype built 4.7 whole** on a local branch from `c8d6196`, the
  approved spec, in this tree, in three parts, then a fix for two test gaps,
  and the records brought up to date with it, then the fact-check's fixes,
  a test for the gap their mutants found, the second check's fixes, and a
  test for the gap theirs found:
  - part A, std (`75d1484`): `std/http`'s routes, readers and answers,
    `ui::ssr::Dist`, `rename`, PBKDF2 and hex, the builtins, and their
    tests. Its code broken on purpose, each breakage failed its tests: the
    pieces of a path not decoded, `allow` naming a method twice, the
    fallback for every method, `form` with no `content-type` check, the
    last of two cookies kept, empty parts kept, one cookie written, a value
    not checked, `303`'s phrase missing, `rename` copying, PBKDF2's salt
    and password swapped, `verify` always true, the length's cap one short,
    hex in upper case, an odd length taken, an unlisted file served, a
    template with no root opened, and `respond` ignoring its status. One
    passed: `Dist::open` no longer emptying the template's root, since
    `document` empties it again. It timed PBKDF2 for a 32-byte key: 600,000
    rounds took 2,564, 2,638 and 2,630 ms on the runtime built for
    debugging, as long in a program built with `--release`, which links the
    same runtime, and 303 to 340 ms with `ring` built optimized; 1,000
    rounds took 4 ms;
  - part B, the blog (`ca9db40`): the example, its `nova test`,
    `blog_tests.rs`, `blog.mjs` and CI's step. std needed nothing more.
    Each breakage failed its tests: the CSRF check skipped, a page
    unescaped, the session's id kept at login, the cookie's tag not
    checked, the token not bound to its session, the store not renamed into
    place, the hash's length, the admin's guard, the logout, the password's
    length, `HttpOnly` and a deleted post's comments, the live checks off, a
    disabled button rendered by the server, and the client hydrating other
    data;
  - part C, the records (`5114979`): ADR 0046 and the notes. It read every
    departure of A and B against the code, and found
    `examples/05-json-api/README.md` saying `Server` routes exact paths and
    `GET` only, which `ece48d7` corrects;
  - the gap fix (`499e51b`), after the mutants' first run (below), and
    `b7a12ac 4.7 prototype: the records after the gap fix`, which gave
    ADR 0046's "The tests share one build" the tests as they then were;
  - the fact-check's fixes (`5bae2a7`, below): the server's writes keep
    the file's users, ended sessions are dropped once the table has
    doubled, stored text holds no NUL, `--set-password` refuses a blank
    password, and a name no user has costs the largest count; two new
    blog tests and three new `nova test` tests, the refusals' statuses
    checked, a failed form's page and a NUL in `blog.mjs`; and the
    records;
  - `3e49a31 4.7 prototype: a test for the pruning floor`, after the
    mutants' run on the fixes (below);
  - the second check's fixes (`2cc807b`, below): the users in a file of
    their own, `data/users.json`, which only `--set-password` writes and
    the server reads when it starts; the timing test with two users; and
    the session table's records corrected;
  - `64064e8 4.7 prototype: a test for overlapping --set-password runs`,
    after the mutants' run on those fixes (below), with ADR 0046's and the
    README's words on two runs at once made exact.

  The parts' departures from the spec, and what they found, are this
  plan's decisions 14 to 31; the split's are 32 to 34, the gap fix's 35
  and 36, and the fact-check's fixes' 37 to 44.
- **A fact-check** then read this plan's draft against the prototype, the
  spec and the code, and probed the running blog. It found no hole in the
  CSRF check, the sessions, the admin's guard or the escaping, and the
  figures, the cited lines and the patches held. It found nine things:
  1. a `--set-password` run while the server ran was undone by the
     server's next write, and the old password kept working: fixed, the
     server's writes keep the file's users (decision 37);
  2. each new session scanned the whole table, and every page asked for
     without a cookie starts one, so a request cost about 1.3 ms more for
     every 1,000 sessions: fixed, the ended ones are dropped once the
     table has doubled (decision 38);
  3. a NUL in one comment made its post's page mismatch, and keep none of
     the server's nodes, for every visitor: fixed, `cleaned` drops it
     (decision 39);
  4. a password of eight spaces was taken, and its user could never log
     in: fixed, it is refused (decision 40);
  5. the gate never opened a failed form's page at its `POST` path, though
     the code was right: `blog.mjs` now posts a blank comment and checks
     that its `400` page at `/posts/1/comments` hydrates in place;
  6. decision 27's refusals and the key file's had no test: now
     `set_password_writes_the_admin` checks each refusal's status and
     message, and `a_key_file_without_32_bytes_in_hex_is_refused` is new;
  7. the key and the password hashes are written with the system's
     default permissions, since `std/fs` sets no mode: the README now says
     to keep `data/` private (it stands, below);
  8. a login's time could tell a name no user has from a user's when the
     users' counts differ: a name no user has now costs the largest count
     (decision 41);
  9. records and nits: ADR 0046 left out the refusal of a password that is
     not UTF-8; `Dist::open`'s doc example is `ignore`, and `auth.nova`
     imports `store`, with no decision for either; and the plan's report
     named `b4cd43e` for Task 0's figures without saying it is not an
     ancestor of `c8d6196`: the ADR now names the refusal, decisions 42
     and 43 record the two, and Task 0 says why its figures hold.

  Each fix's test failed with its fix broken. The fixes changed
  `examples/07-fullstack-blog`'s `auth.nova`, `forms.nova`, `store.nova`,
  `lib.nova` and `server.nova`, its README and three `tests/*.nova`,
  `blog_tests.rs`, `run_tests.rs`'s test of the blog's `nova test`,
  `blog.mjs`, ADR 0046 and `60-EXAMPLES.md`: Tasks 5 to 8.
- **A second check** read the plan again against the prototype at
  `3e49a31`, and probed the blog. It found five things:
  1. `--set-password` run while the server ran still lost data: it wrote
     back the store it had read before its hash, erasing the comments the
     server saved during it, and under steady writes its own user too (26
     of 40 runs at 1,000 rounds), while it exited 0: fixed, the users have
     a file of their own, which only `--set-password` writes (decision
     37);
  2. each server write read `blog.json`: a missing one made it write the
     seed's users, none, erasing the admin, and an unreadable one stopped
     every write, and then the restart: fixed, the server writes from its
     own copy and reads no file;
  3. no test covered those branches, nor `log_in`'s use of the largest
     count, since the timing test had one user: now
     `a_password_set_while_the_server_runs_loses_nothing`,
     `set_password_writes_the_admin`'s new checks, and the timing test's
     second user (decision 41);
  4. the records overstated the session table: it holds at most twice the
     sessions live at the last scan, not a day's, the start at each
     doubling scans it all (18.4 ms at 16,385 sessions, 35.7 at 32,769),
     and a session takes about 658 bytes: ADR 0046, the README,
     `auth.nova` and decision 38 now say so;
  5. nits: `stored`'s doc left out NULs; the login's name is not cleaned,
     so a NUL reaches the failed login's own page, which Chromium hydrates
     whole; a field of only U+FEFF, U+2028 or U+2029 is not blank, on both
     sides alike: the doc is fixed, and ADR 0046 records the other two.

  `2cc807b` ("4.7 prototype: the second check's fixes") holds the fixes,
  `store::the_users_have_a_file_of_their_own`, the blog test that
  replaces `a_password_set_while_the_server_runs_is_kept`, and the
  records; each fix's test failed with its fix broken. It changed the
  blog's `store.nova`, `auth.nova` (its doc), `lib.nova`, `server.nova`,
  `data/seed.json`, README, `tests/store.nova` and `tests/passwords.nova`,
  `.gitignore`, `blog_tests.rs`, `run_tests.rs`'s test of the blog's
  `nova test`, `blog.mjs`, ADR 0046 and `60-EXAMPLES.md`: Tasks 5 to 8.
- **A replay applied it** to `c8d6196` on a scratch branch, task by task,
  as the steps say, each task's tests before its code; each test-first
  step's failure is that run's.
  - A script split the prototype's final tree into one commit per task,
    each task's tests apart from its code. Files that change in more than
    one task carry their intermediate versions: `index_complete.rs` counts
    160 programs and 186 files after Task 1, 162 and 188 after Task 2, 164
    and 190 after Task 3, and 165 and 199 after Task 6, when the blog's
    `src/main.nova` makes it a program; `run_tests.rs` and `wasm_tests.rs`
    take each task's hunks in its tests, and `run_with_and_without_stress`,
    which Task 2's `http_routes_run` sits beside, comes in Task 1's, since
    `bytes_hex_run` calls it; `crypto.rs`'s three unit tests are in Task
    1's tests, its code in Task 1's code; `std/http/lib.nova` after Task 2
    holds 15 of 4.7's 23 hunks, with three texts that would name Task 3's
    builders rewritten on `Response::text` (the module doc's sentence on
    `html`, `redirect` and `with_cookie`, dropped, and `post`'s and
    `dispatch`'s examples), which Task 3 puts back; `std/ui/ssr/lib.nova`
    after Task 3 is the base with `cookies: []` in its two `Response`
    literals; and the blog's `app.nova` after Task 5 is its records, their
    JSON and `data_for`, without its pages, routes or `App`, which come in
    Task 6.
  - Where tests and code cannot part, they go together: `nova-typeck`'s
    unit test is an exhaustive `match` over `Builtin`, which compiles
    neither before the new variants nor without their arms after, so it
    rides in Task 1's code; and the resolver changes in Task 1's code
    alone, with no test of its own. `post_card.nova` comes whole in Task
    5's code, and the blog's `nova.toml` in Task 5's tests (decisions 32
    and 33).
  - Every tests step fails cleanly before its code, with no stub: Nova
    compile errors in the child `nova` in Tasks 1 to 4, Rust compile errors
    in `nova-runtime`'s and `nova-mir`'s unit tests in Task 1, example 03's
    `POST /` still `404` in Task 2, a missing `src/` in Task 5, and `nova
    build --ssr` refusing a library in Task 6.
  - The patches below are those commits. Each applies with `git apply` in
    order on `c8d6196`, and the last tree is the prototype's head,
    `2cc807b`. `git apply` warns once, for Task 3's tests:
    `tests/runtime/http_server_wire.stdout` holds `  body: ` with a
    trailing space, the golden of a `303`'s empty body, so 4.6's
    `--whitespace=error-all` would refuse that patch. No added line holds a
    raw CR; the only non-ASCII characters added are `§`, `—`, `…`, `é` and
    `✓`.
  - Every "Expected:" below is the walk's, from a fresh `NOVA_HOME`, of
    the split of `5bae2a7`, the fact-check's fixes. Each earlier prototype
    commit was split and walked the same way. That split's first eight
    trees, Tasks 1 to 4, are the previous split's, and every step of all
    fourteen was replayed; Tasks 1 to 4 printed what they printed before,
    and Tasks 5 to 8 print what the fixes add: 20 `nova test` tests, 12
    `blog_tests`, and `blog.mjs`'s failed form and NUL. `3e49a31` changed
    `tests/sessions.nova` alone, in Task 5's tests: the split was made
    again, only that patch changed, and Task 5's two steps, replayed,
    printed the same. `2cc807b`, the second check's fixes, changed Tasks
    5 to 8: the split was made again, its first eight patches as before,
    and every step from Task 5's tests on was replayed, the browser
    scripts and the final checks among them: 21 `nova test` tests, the 12
    `blog_tests` failing at `blog_tests.rs:107` and then passing, and
    `blog.mjs`'s line as before. `64064e8` changed `blog_tests.rs`, the
    README and ADR 0046: the split was made again, three patches changed,
    and Task 6's steps, Task 8's and the final checks were replayed: 13
    `blog_tests`, and the full suite one test more.
- **The mutants.** 116 breakages of what 4.7 adds, each a text replaced in
  one file, run one at a time (Task 9, Step 4): 96 from the first runs, 15
  of the fact-check's fixes, 97 to 111, and 5 of the second check's, 112
  to 116; 97, first the server's write keeping the file's users, is now
  `--set-password` writing `blog.json`, and 86 renames through the store's
  new `replace`. A first run, on the first split, caught 94 of the 96. Two passed their tests, and the prototype's `499e51b
  4.7 prototype: close two test gaps` closed both, in `blog_tests.rs` and
  no code:
  - an admin's form taken in the admin's own session without its token
    passed (mutant 73), since no test posted one: `blog.mjs` strips only
    the comment form's token. Now `the_admin_pages_need_the_admin` goes on
    to log in as the admin and post each admin form, and the logout,
    without its `csrf` field and then with the visitor's token: each is
    `403` with the expired-form page and no cookie, and `/`, `/posts/1` and
    `/admin` are the same after as before (decision 35);
  - a login with a name no user has, answered at once, passed (mutant 84),
    since only its time changed. Now
    `a_name_no_user_has_takes_as_long_as_a_wrong_password` times five
    logins of each kind over 100,000 rounds and asks each median to be at
    least half the other (decision 36). Under the mutant its medians were
    419 ms and 2.1 ms; as the code is, five runs in a row passed, the
    medians 401 to 454 ms, and two more with eight busy loops on four cores.

  On the fixes' code, `5bae2a7`'s, 110 of the 111 failed exactly their
  named tests; mutants 77, 90 and 91 were rewritten for that code, with the
  same breakage. One passed:
  - mutant 100, the floor of 64 taken out of `Sessions.start`, so that the
    ended sessions are dropped whenever the table has doubled from its
    first session: the blog's `nova test` passed all 20, its
    `sessions::ended_sessions_are_dropped_once_the_table_has_doubled`
    among them. That test started forty of the admin's sessions, which do
    not end, before any visitor's, so the scans without the floor, at 1, 2,
    4, 8, 16 and 32 sessions, dropped nothing, and the doubling alone gave
    the counts it checks. `3e49a31` closes it, in that test and no code:
    it now starts with a fresh table, which has kept none, whose 63 ended
    sessions stay, and a 64th, until the next start drops all 64 (decision
    38). Under the mutant it fails, `1 != 63`.

  On `3e49a31`'s code all 111 then failed. On `2cc807b`'s, 115 of the
  116 failed; one passed:
  - mutant 113, `--set-password` reading `users.json` before it hashes, so
    that it holds the file's users for the whole hash:
    `a_password_set_while_the_server_runs_loses_nothing` passed, since no
    test ran two `--set-password` at once. `64064e8` closes it, in
    `blog_tests.rs` and no code: `overlapping_set_password_runs_keep_both_users`
    runs one at 1,000 rounds to its end while one at 600,000 hashes, and
    needs both users in `users.json` (decision 37). Under the mutant it
    fails, `ada's run wrote over bea, who was set while it hashed`; as the
    code is, five runs in a row passed, the fast run ending 521 to 524 ms
    and the slow one 2.47 to 2.76 s after the slow one started.

  On the final code, `64064e8`'s, all 116 failed exactly their named
  tests, each through a wrong output, a panic, a wrong status or a wrong
  exit code, and none through a build that broke. Sixteen mutants of the
  blog's modules that its `nova test` reaches fail
  `the_blog_example_passes_its_own_tests`, whose output names the blog's
  own cases that failed.
- **The gate passed in the replay,** locally: every script of the `browser`
  job through Playwright 1.56.1 in Chromium, `blog.mjs` among them, on Task
  7's tree, with the runtime library beside `nova` from `cargo build
  --locked -p nova-cli -p nova-runtime` alone, as CI builds it, and no
  `NOVA_RUNTIME_LIB`.
- **The final checks ran on the split's last commit:**
  - `cargo fmt`, clippy with `-D warnings` and `nova fmt` were clean after
    every task;
  - the full suite gave 2419 passed, 1 failed (the IPv6 test) and 10
    ignored, against the base's `2392 1 10`: 27 new tests;
  - `06-counter-spa`'s `app.wasm` and `app.js` are 21,070 bytes by `flate2`
    at level 9, as at the base, against 51,200;
  - the blog's client is 49,472 bytes gzipped, as `blog.mjs` prints it,
    built without `--release` (49,423 before the forms' checks called
    `cleaned`, decision 44); it is printed, not limited, and `ssr-app`'s is
    41,688 bytes, as at the base;
  - Rust 1.78's check was clean, and `Cargo.lock` did not change.
- **What the checks left standing,** each recorded or handed on:
  - `std/ui` does not refuse a `javascript:` URL in an `href`; the blog
    never puts a visitor's text in one, since a post's link is built from
    its id (ADR 0046, spec §11);
  - a cookie that breaks a rule is left out of the response without an
    error, such as a value holding a space or a character outside ASCII;
    the blog's are hex (decision 17);
  - PBKDF2 holds the single-threaded server for each login, a known name or
    not: about 2.6 s at 600,000 rounds with the runtime built for
    debugging, and about 0.3 s expected with one built for release (ADR
    0046);
  - a crash between a response and the write it spawned loses that change,
    though the file is always whole (spec §10, risk 2);
  - two `--set-password` runs can lose one's user when one reads
    `users.json` after the other has read it and before the other has
    renamed its own over it, a window of milliseconds after both hashes;
    one that runs while another hashes is kept, as a test checks. The
    README says to run one at a time (decision 37, ADR 0046);
  - an old data folder's `blog.json` users are ignored, since the users
    are now in `users.json`: its passwords have to be set again (decision
    37);
  - a flood of clients that keep no cookie keeps up to two days of
    sessions, the live day and as many ended, about 658 bytes each
    (decision 38, ADR 0046);
  - a user whose hash has fewer rounds than the largest still answers
    sooner than a name no user has, so a login's time can tell that user's
    name from one no user has; passwords all set with the same count close
    it (decision 41, ADR 0046);
  - the key in `data/secret` and the hashes in `data/blog.json` are written
    with the system's default permissions, since `std/fs` sets no mode; the
    README says to keep `data/` private (ADR 0046);
  - the timing test compares medians at half; a runner far busier than the
    replay's could still fail it (decision 36);
  - `nova fmt --check` would reformat `tests/runtime/http_serialise.nova`,
    four places the base already had and the `Response` literal Task 3
    gives `cookies: []`, and `tests/runtime/http_server_dispatch.nova`,
    three places the base already had; `tests/runtime` is in no check's
    paths, so the plan checks the eight new programs by name, and neither of
    those;
  - Rust 1.78 checks the libraries and binaries, not the tests:
    `assert_cmd` 2.2.1's manifest does not parse on 1.78. Task 9's check is
    4.6's exactly;
  - `nova test` run in the example leaves `target/` there, which
    `.gitignore` names only from Task 6, so the steps run it in a copy
    (decision 34).

## Global Constraints

- **The workspace's minimum Rust stays 1.78,** edition 2021. CI's MSRV job
  runs `cargo check --locked --workspace` with `RUSTFLAGS=-D warnings`. No
  `std::sync::LazyLock` or `LazyCell` (1.80), no `Option::is_none_or`
  (1.82), no `std::path::absolute` (1.79).
- **`Cargo.lock` does not change.** Any change to it stops the task: ask
  the user. So does anything that needs a new crate (spec §7.1). PBKDF2 is
  `ring::pbkdf2`, and `ring` is in the lock already; the blog's server is
  Nova's own `std/http` on `std/net`.
- **No `unwrap()` outside `#[cfg(test)]` and test files:** `expect("why")`,
  `unwrap_or…` or `?`.
- **Tests never touch the internet.** Every server a test starts binds
  `127.0.0.1:0`: `http_server_wire.nova` binds port 0, reads its own
  listener's port and connects to it, and `blog_tests.rs` and `blog.mjs`
  start the blog's server with `--address 127.0.0.1:0` and read the port
  from its `blog: serving http://127.0.0.1:<port>/` line. **No test uses
  port 3000,** the blog's default address and `nova dev`'s. A test that
  needs `NOVA_HOME` sets it to a fresh directory. Every test directory has a
  fixed name under the system's temporary directory, such as
  `nova-fs-rename-run`, `nova-blog-nova-test` and `nova-blog-tests`, removed
  and made again at the test's start. Node runs only what a test built.
- **A test that needs Node** passes without running when `node` is not on
  `PATH`, and fails instead under `NOVA_REQUIRE_NODE=1`, which CI sets.
  Run the WASM tests here with `NOVA_REQUIRE_NODE=1`: Node 22 is installed.
- **Nothing in `cargo test` or `nova` installs a package,** and nothing
  installs `wasm-opt`. Playwright is the `browser` job's alone; Task 7's
  local run uses the copy already on this machine, and is skipped where
  there is none.
- **`nova-cli`'s tests need two files beside `target/debug/nova`:**
  `libnova_runtime.a` and the `nova-bench-http` binary. A build of the
  workspace writes both; Task 0's build puts them there. **Task 1 changes
  the runtime,** and `cargo test -p nova-cli` rebuilds `nova` but not the
  library beside it: cargo leaves `libnova_runtime.a` there only when a
  build names `nova-runtime`. So every run step starts with `cargo build
  --locked -p nova-cli -p nova-runtime` (Conventions), and a mutant of the
  runtime builds it before its tests and again once its file is restored
  (Task 9, Step 4). After a `cargo clean`, build the workspace again.
- **std is embedded at build time.** After a task changes a file under
  `std/`, a run of `target/debug/nova` by hand, such as Task 5's `nova test`
  or Task 7's browser scripts, needs that build first; `cargo test`
  rebuilds `nova` itself.
- **Never clean `/tmp/nova-*`:** `nova test` keeps built binaries in
  `/tmp/nova-test-bin`, and a test whose binary goes fails.
- **Work in one tree.** Two worktrees of this repository sharing one
  `target/` share artifact names, so cargo can take one tree's build as
  fresh for the other (4.0's ledger). Never build in another worktree.
- **Trailing spaces are expected output.** `tests/runtime/http_server_wire.stdout`,
  which Task 3's tests add, holds the line `  body: `, with a trailing space,
  the golden of a `303`'s empty body; `tests/dom/router_history.stdout` and
  `tests/runtime/ui_router.stdout` end lines with a space, as 4.5 left them.
  So apply the patches with plain `git apply`, as `patch_block` does: never
  with `--whitespace=fix`, which would strip that space, nor with
  `--whitespace=error-all`, which refuses Task 3's tests; and never let an
  editor trim this plan or a `.stdout` file.
- **Never use the Write or Edit tool for text that holds a `\u` escape,**
  a backslash and `u` followed by four hex digits or by a code in braces:
  they decode the escape into the character. The patches hold such escapes
  (`std/http`'s `form` doc, `tests/runtime/http_responses.nova` and
  `blog_tests.rs`), and so does this plan, in mutant 96, which edits
  `std/ui/ssr`'s own. Apply the patches with `patch_block`, from the
  committed plan; write anything else that holds one with a script, and
  check it with `grep -c`.
- **Nothing writes into the repository but the patches.** The runtime
  programs write under the system's temporary directory; the blog's `nova
  test` runs in a copy (Task 5), and `run_tests.rs`'s test of it copies the
  example; `blog_tests.rs` and `blog.mjs` copy the example, less what a
  build or a run in place leaves, into a directory outside the tree and
  build it there. Tasks 6 and 7 check it.
- **Nothing is published, and no tag is pushed.** Merge by rebase, on the
  user's word only. Never enable auto-merge.
- **This machine is Linux.** Windows and macOS results come from CI, which
  runs all three systems, Node included: `blog_tests.rs` there, with the
  server's `.exe`, a checkout that may give the copied files CRLF line
  ends, the server killed before its files are removed, and `rename` over
  an existing file, which Windows locks while it is open, are CI's to run.

## Conventions

- **Scratch directory:** `$P` is the session's scratch directory, outside
  the repository. Set it in every shell before a command that names `$P`.
- **The ledger:** `$P/ledger-47x.md`. Every ruling made while executing goes
  there, with its cost if wrong; so do each task's figures.
- **Commits.** Each task's message is written to `$P/msg-47-<task>.txt`:
  the subject the task names, a blank line, and the session's attribution
  lines. Then `git add -A`; `git status --short` lists only the task's
  files; and `git commit -F "$P/msg-47-<task>.txt"`.
- **The baseline's full output** is `$P/base-47.txt` (Task 0).
- **Runs.** The replay ran every command with these variables set. Each
  shell sets them too, with three shorthands, and runs every command from
  the repository's root:

  ```sh
  export CARGO_INCREMENTAL=0 NO_COLOR=1 RUST_BACKTRACE=0 NOVA_HOME="$P/home-47"
  unset NOVA_RUNTIME_LIB
  mkdir -p "$NOVA_HOME"
  W() { NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --test wasm_tests -- "$@"; }
  R() { cargo test -p nova-cli --test run_tests -- "$@"; }
  X() { cargo test -p nova-driver --test index_complete "$@"; }
  ```

  So `W bytes_hex` runs `wasm_tests`'s tests whose names hold `bytes_hex`,
  `R http_` `run_tests`'s, and `X` the corpus test. `NOVA_HOME` keeps a run
  by hand off `~/.nova`; every test that needs one sets its own.
- **The build.** Each "Run" step below starts with

  ```sh
  cargo build --locked -p nova-cli -p nova-runtime
  ```

  which ends with `Finished`, in about 8 seconds after a change to `std/`
  or the runtime and a second otherwise, so that the `nova` the steps run
  by hand, and the runtime library beside it, are the tree's (Global
  Constraints). The replay began every step with it.
- **The patches.** Each task's code is in patch blocks below, each marked
  `<!-- patch NAME -->` and fenced with four backticks. They are the
  replay's commits, so apply them; do not retype them. From the
  repository's root:

  ```sh
  PLAN=docs/superpowers/plans/2026-10-21-phase-4-7-fullstack-blog.md   # this file, committed
  block() {
    awk -v name="$1" '
      $0 == "<!-- patch " name " -->" { grab = 1; next }
      grab && /^````diff$/ { inside = 1; next }
      inside && /^````$/ { exit }
      inside { print }
    ' "$PLAN" > "$P/$1.patch"
  }
  patch_block() {
    block "$1" && git apply --check "$P/$1.patch" && git apply "$P/$1.patch"
  }
  ```

  `patch_block t1-tests` applies the block named `t1-tests`, and prints
  nothing; only `t3-tests` prints a warning, its trailing space (Task 3,
  Step 1). A block that does not apply means the tree differs from the
  plan's: stop, and record why in the ledger.
- **Expected outputs are written once, from a run, and read.** Each
  `.stdout` under `tests/runtime/` that 4.7 adds or changes is in its
  task's test patch: the replay wrote `bytes_hex.stdout`, `fs_rename.stdout`,
  `crypto_pbkdf2.stdout`, `http_routes.stdout`,
  `http_request_readers.stdout`, `http_responses.stdout`,
  `http_server_wire.stdout` and `http_server_dispatch.stdout`'s new lines
  from the program's native run, and each was read against the spec; the
  two PBKDF2 vectors are RFC 7914's, checked against Python's `hashlib`.
- **Each task** is test first: its tests' patch, their run and the expected
  failure, then its code's patch, its run and the expected pass. Unit tests
  in the files a task changes ride in its code's patch, but for
  `nova-runtime`'s `crypto.rs` and `nova-mir`'s `lower_tests.rs`, which
  split from their code; Task 9's mutants show they bite. Task 7's patch is
  the browser test and CI alone, and Task 8's the records. Each task then
  runs its checks:
  - `cargo fmt --all -- --check`, which prints nothing;
  - the workspace's clippy, `cargo clippy --locked --all-targets
    --all-features -- -D warnings`, as CI runs it, which ends with
    `Finished` and no warning;
  - `cargo run --locked -q -p nova-cli -- fmt --check std examples
    tests/wasm tests/dom tests/browser/router-app tests/browser/ssr-app
    tests/ssr`, which prints nothing. All seven exist at the base, and 4.7
    adds none: the blog is under `examples`, `dist.nova` under `tests/ssr`.

  It ends with one commit. A difference from an `Expected:` is fixed in
  the smallest way and recorded in the ledger as a ruling, with its cost if
  wrong.


## Review Focus

The inputs most likely to bite a user, most likely first. Each has its test
in the task that owns the code:

1. **The blog's CSRF check and sessions:** every `POST` route goes through
   `checked`, which finds the session, then compares the form's `csrf`
   field with the session's token by `hmac_sha256_verify` before it looks at
   any other field, and refuses a missing session, a missing or wrong token
   and, for an admin's form, a visitor's session; the token is
   `hmac_sha256(key, "csrf:" + id)`, worth nothing in another session; a
   cookie's tag checked, its id in the table and not ended; a new id at
   login and the old one ended; logout ending the session and expiring the
   cookie; `HttpOnly`, `SameSite=Lax` and a `Max-Age` of the session's life;
   the admin's pages and forms behind the admin's session; the refusal
   pages, which start no session (decision 29); the ended sessions dropped
   only once the table holds at least 64 and twice what the last scan
   kept, `kept` set by each scan, so a request without a cookie costs the
   same on average however full the table is, the one at each doubling
   scanning it all, and the table holds at most twice the sessions live at
   the last scan, or 64 (decision 38) → the blog's `sessions`
   tests (Task 5), `a_comment_needs_its_sessions_csrf_token`,
   `logging_in_takes_the_right_password_and_a_new_session`,
   `the_admin_pages_need_the_admin`, `logging_out_ends_the_session` and
   `every_page_answers_with_its_status` (Task 6), and `blog.mjs`'s form
   without its token (Task 7); mutants 70 to 82 and 98 to 102.
2. **Escaping of every visitor's text, on both renderers:** a title, a
   body, a name and a comment reach the page only as text nodes and
   attribute values, which `std/ui` escapes on the server and sets as text
   on the client, and in the page's data, whose `<` `page_with` writes as an
   escape; nothing a visitor controls reaches an `href`, a header or a
   cookie: a post's link is built from its id, a redirect's location from
   ids → `visitor_text_comes_back_escaped` (Task 6), and `blog.mjs`'s
   script in a comment and in a title, which would set a global and open a
   dialog (Task 7); mutants 94 to 96.
3. **Routes against the router:** a pattern's pieces and a request's path,
   its target before any `?`, split at `/`, empty pieces dropped, each
   percent-decoded as `std/ui/router` decodes, whole or not at all
   (decision 14), so `/admin/` matches `/admin`; `:name` binding any one
   piece; the first route whose method and pattern match answering, with
   `req.params`; then `405`, its `allow` naming each matching method once,
   in the order added, its body `method not allowed`; then the fallback,
   for a `GET` alone; then `404`; `get` no longer seeing the query, and
   example 03's `POST /` now `405`; the blog's `GET` routes giving its
   `POST`-only paths the not-found page → `http_routes_run`,
   `http_server_dispatch_run` and `http_server_example_serves_and_exits_cleanly`
   (Task 2), `every_page_answers_with_its_status` (Task 6); mutants 18 to
   32 and 92.
4. **The readers:** `query` after the first `?`, and `form` only for a
   `content-type` whose media type, before any `;`, trimmed and in any
   case, is `application/x-www-form-urlencoded`, and a body that is UTF-8;
   both split at `&`, an empty part skipped, a part split at its first `=`
   or else an empty value, `+` a space, a half that does not decode kept as
   written, the last of two names winning; `cookies` split at `;`, each part
   trimmed, a part with no `=` skipped, the first of two names winning, the
   values not decoded → `http_request_readers_run` (Task 2); mutants 33 to
   42.
5. **Cookies on the wire, and what they refuse:** one `set-cookie` line per
   cookie, in order, after the other headers; `Path`, `Max-Age`,
   `HttpOnly`, `SameSite` and `Secure` each written only when it applies;
   a cookie left out whose name is not a token, whose value holds anything
   but RFC 6265's cookie-octets, a `"` or a CR or LF included, whose path
   holds a control character or `;`, or whose `same_site` is not a token,
   so no cookie can inject a header or an attribute (decision 17);
   `with_cookie` leaving its receiver's cookies alone (decision 20);
   `Cookie::new`'s defaults, `Cookie::expired`; `html`'s type, `redirect`'s
   `303`, its location and its empty body; the reason phrases →
   `http_responses_run` and `http_server_wire_run` (Task 3); mutants 43 to
   62.
6. **PBKDF2 and its use:** the password and the salt in `ring`'s order,
   the count and the length passed through; each bound, a count from 1 to
   4,294,967,295 and a length from 1 to 65,536, checked before anything is
   derived or allocated, and a panic in std's words; `verify` true for the
   start of the right key (decision 16), so the blog refuses a stored hash
   that is not 32 bytes; a name no user has spending, against a fixed
   salt, the largest count of the users' hashes that PBKDF2 takes
   (decisions 26 and 41); `--set-password`'s refusals, each with its
   status and before anything is written: a blank or padded name 2, a
   password that is not UTF-8, short or blank 1 (decisions 27 and 40); the
   key file refused unless it holds 32 bytes in hex, and `users.json`
   unless it holds users, by the server and by `--set-password`; the
   server's login reading the users of `users.json` and the largest count
   among them (decisions 37 and 41); the hex a key, a
   salt, a tag and a token are written in →
   `crypto_pbkdf2_run`, `crypto_pbkdf2_refuses_its_bad_arguments`,
   `nova-runtime`'s three tests and `bytes_hex_run` (Task 1), the blog's
   `passwords` tests (Task 5), `set_password_writes_the_admin`,
   `a_key_file_without_32_bytes_in_hex_is_refused` and
   `a_name_no_user_has_takes_as_long_as_a_wrong_password` (Task 6); mutants
   1 to 3, 7 to 17, 83 to 85, 104 to 111 and 114 to 116.
7. **`Dist` against `serve`'s old behaviour:** `open`'s template and
   manifest read with `serve`'s checks and errors, its root emptied, a
   manifest's path that could leave `dist` dropped; `file` for a listed
   path alone, by any spelling `serve` took, with `404` for a listed file
   not on disk, and `None` for anything else, whatever the method
   (decision 19); `respond`'s status and type; `serve`, rebuilt on it,
   answering as before → `ssr_dist_answers_files_and_pages` (Task 4), and
   4.6's `ssr_serve_*` and `ssr_tests.rs`, unchanged (Tasks 3 and 4);
   mutants 63 to 69.
8. **The store's writes:** each change made in memory, then the whole store
   written to `blog.json.tmp` and renamed over `blog.json` by a spawned
   write that runs after the response, in order; `rename` replacing the old
   file, `NotFound` for a missing source or directory; a deleted post's
   comments deleted; one writer per file: the server writes `blog.json`
   from its own copy, reading no file, and never `users.json`, which only
   `--set-password` writes, hashing first and then reading, setting and
   renaming it in one go, never touching `blog.json`, so neither writes
   over the other's change, and a `--set-password` run during another's
   hash kept (decision 37); a crash between the response and the write
   losing that change, and two `--set-password` runs whose reads and
   writes meet after both hashes losing one's user, which stand →
   `fs_rename_run` (Task 1), the blog's `store` tests (Task 5),
   `the_posts_survive_a_restart`, `set_password_writes_the_admin`,
   `a_password_set_while_the_server_runs_loses_nothing` and
   `overlapping_set_password_runs_keep_both_users` (Task 6); mutants 4 to
   6, 86, 87, 97, 112 and 113.
9. **Hydration on every page, and forms without JavaScript:** the server
   renders the enabled submit button and no messages, and the client's
   checks start in `on_mount` (decisions 13 and 24); stored text trimmed,
   its CRLFs made LF and its NULs dropped, so a `textarea` reads back as
   the server wrote it and a NUL cannot break a page for everyone
   (decisions 25 and 39), and the checks measuring that text on both sides
   (decision 44); a refused or failed form rendered at its `POST` path,
   which `routes()` maps on both sides (decision 23) → `blog.mjs`, which
   keeps a handle to every node of each page, a failed comment's `400` page
   at `/posts/1/comments` among them, and finds each still there once it
   hydrates (Task 7), and the blog's `forms` tests (Task 5); mutants 88 to
   91, 93 and 103.
10. **A test that passes without its code:** each mutant names one that
    would. The two gaps the prototype closed show the kind: no test posted
    an admin's form in the admin's own session without its token, and none
    measured a login's time (decisions 35 and 36); and, after the fixes,
    none showed ended sessions staying under 64 in a table that had
    doubled, until `3e49a31` (mutant 100; decision 38), and none ran two
    `--set-password` at once, until `64064e8` (mutant 113; decision
    37).
11. **Windows in `blog_tests.rs`:** the server's path joined by components
    with `EXE_SUFFIX`; the copy's files, which a Windows checkout can give
    CRLF line ends and which no test compares as text; the server killed and
    waited for before its directory is removed, on `Drop` too, since
    Windows locks a running executable and its open files; `rename` over an
    existing `blog.json`; the `.tmp` file left behind after a failed write.
    This machine cannot run them: CI's Windows leg does.

## Decisions

Made while writing the spec (its §12), prototyping, splitting, replaying
and fixing what the fact-check found, each as built, and each read against
the code. Where one departs from the
spec or fills in what it left open, it says so and why, and is marked
**(differs)**; ADR 0046 records them, and the departures from
`60-EXAMPLES.md` §7 and `20-STDLIB.md` §6.

**The spec's decisions (its §12), as built**

1. **A route's pattern is matched without the query, piece by piece,
   decoded, by the router's rules, and `get` takes patterns too** (spec
   §12.1, §3.1). `path_pieces` splits a pattern and a request's path, its
   target before any `?`, at `/`, drops the empty pieces and decodes each
   as `std/ui/router` does (decision 14), so `/admin/` and `//admin` match
   `/admin`; a `:name` piece matches any one piece and binds it. `get`'s
   doc and `http_server_dispatch.nova` change with it: `/echo?x=1` reaches
   `/echo`'s handler (mutants 18 to 27).
2. **`405` with `allow`, before the fallback** (spec §12.2, §3.1).
   `dispatch` tries the routes in the order they were added; the first
   whose method and pattern match answers; otherwise, when some route's
   pattern matches, `405 Method Not Allowed` with an `allow` header naming
   the methods of the routes that match, each once, in the order added, as
   `GET, POST`, and the body of decision 18; otherwise a `GET` goes to the
   fallback; otherwise `404`. A method `std/http` has no variant for, such
   as `PURGE`, is compared by its name; `HEAD` gets no special treatment.
   `examples/03-http-server` now answers `POST /` with `405` and `allow:
   GET`, which its test asserts (mutants 28 to 32).
3. **`Request.params`** (spec §12.3, §3.1), `req.params.get("id")`, empty
   in a request `parse_request_head` or `read_request` returns. Every
   `Request` literal names it: std's, `http_server_dispatch.nova`'s and the
   new programs' (mutant 23).
4. **`query` and `form` share one decoder; `cookies` keeps the first**
   (spec §12.4, §3.2). `form_fields` splits at `&`, skips an empty part,
   splits a part at its first `=`, a part with none being a name whose value
   is `""`, and decodes each half with `+` as a space (decision 14); the
   last of two names wins. `form` reads the body only when the
   `content-type`'s media type, before any `;`, trimmed and lower-cased, is
   `application/x-www-form-urlencoded` and the body is UTF-8, and is
   otherwise empty. `cookies` splits the `cookie` header at `;`, trims each
   part, skips one with no `=`, keeps the first of two names, and does not
   decode values (mutants 33 to 42).
5. **`Response.cookies`, a field of its own** (spec §12.5, §3.3), of type
   `[Cookie]` (decision 20). `to_bytes` writes one `set-cookie` line per
   cookie that passes decision 17's rules, in order, after the other
   headers; a `set-cookie` put in `headers` by hand is still written, as a
   header, before them. Every `Response` literal names `cookies`: std's,
   `examples/05-json-api`'s, `ui::ssr`'s and `http_serialise.nova`'s
   (mutant 50).
6. **`Cookie::new`'s defaults are the safe ones** (spec §12.6, §3.3): path
   `/`, no `Max-Age`, `HttpOnly`, `SameSite=Lax` and not `Secure`.
   `Cookie::expired(name)` is the same with the value `""` and `Max-Age=0`.
   Each attribute is written only when it applies: `Path` when the path is
   not `""`, `Max-Age` when it is `Some`, `SameSite` when it is not `""`
   (mutants 43 to 49).
7. **`ui::ssr::Dist`, the parts of `serve`** (spec §12.7, §4). `Dist::open(dir)`
   reads `<dir>/index.html` and `<dir>/manifest.json` with every check and
   error `serve` made, the template's root emptied and a manifest's path
   that could leave `dir` dropped; `file(req)` answers a listed file, as
   `serve` did, or returns `None`; `respond(status, page)` is
   `Response::html(status, document(template, page))`, which a route can add
   a cookie to. `serve` keeps its shape, built on `Dist`, and its tests do
   not change (mutants 63 to 69).
8. **Hex in `std/bytes`, for both targets** (spec §12.8, §5), in Nova:
   `Bytes::to_hex`, two lower-case digits a byte, and `bytes_from_hex`,
   which takes either case and gives `None` for an odd length or any other
   character, each byte of the text's UTF-8 checked, so a character outside
   ASCII is refused. `bytes_hex` runs in the WASM gate too (mutants 1 to 3).
9. **The CSRF token is derived from the session** (spec §12.9, §6.4):
   `hmac_sha256(key, "csrf:" + id)` in hex, in a hidden `csrf` field of
   every form. `checked`, which every `POST` route calls, compares it with
   `hmac_sha256_verify` before it looks at any other field, and answers
   `403` when it is missing or wrong (decision 29) (mutants 70 to 74).
10. **A failed form is rendered again with `400`; only success redirects**
    (spec §12.10, §6.2): with its values and its messages, which `Data`'s
    `draft` carries (decision 23).
11. **A visitor's session lasts a day, and the admin's eight hours** (spec
    §12.11, §6.4): 86,400 and 28,800 seconds. Ended sessions are dropped
    from the table once it has doubled (decision 38); logging in ends the
    old session and starts a new one, so an id set before the login is
    worth nothing after it (mutants 76 to 78).
12. **A login with an unknown name costs as much as one with a known
    name** (spec §12.12, §6.5): `password_ok` runs `pbkdf2_sha256_verify`
    against a fixed salt, `fullstack-blog:no-such-user`, and 32 zero bytes,
    over decision 26's count, and answers `false`; the page says the same,
    "Wrong name or password.", with `401` (mutant 84; decision 36).
13. **The forms' checks start after `on_mount`** (spec §12.13, §6.6): each
    form's `live` signal is set in `on_mount`, which runs after hydration's
    walk, so the server renders no message and the enabled button
    (decision 24), and hydration finds what it expects.

**std (the prototype's part A)**

14. **Percent-decoding is the router's, whole or not at all.** **(differs)**
    A path's piece, and a query's or form's name or value, is decoded as
    `std/ui/router`'s `decoded` decodes it, copied as `percent_decoded`,
    since `std/http` cannot import `std/ui`: kept as written when it holds a
    `%` not followed by two hex digits or its bytes are not UTF-8, but that
    a query's or form's `+` is still a space. So `%zz%41` stays `%zz%41`,
    where spec §3.2 had only the bad `%` stay as it is. A server's route and
    a client's agree on every path (mutants 24, 36 and 37).
15. **`pbkdf2_sha256` panics for a length above 65,536 too.** **(differs)**
    Spec §5 had it panic for a length below 1 only. 65,536 is
    `random_bytes`' cap; the runtime checks the count and the length before
    it allocates or derives anything, so no length can ask for an
    allocation the process cannot make, or reach the length past which
    `ring::pbkdf2::derive` unwraps an error. The panics read
    `<function>: iterations must be from 1 to 4294967295, not <n>` and
    `pbkdf2_sha256: length must be from 1 to 65536, not <n>`
    (`pbkdf2_refused`) (mutants 12 to 17).
16. **`pbkdf2_sha256_verify` takes the start of the right key as the key.**
    **(differs)** Spec §5 said only that it is `ring`'s, in constant time.
    `ring::pbkdf2::verify` derives a key of `expected`'s own length, and
    PBKDF2's shorter output is the start of its longer one, so the right
    key's first 31 bytes verify; an empty `expected` gives `false`. Its doc
    tells a caller to keep and check the whole key and its length, and the
    blog's `password_ok` refuses a stored hash that is not 32 bytes
    (`passwords::a_hash_cut_short_never_matches`; mutant 83).
17. **Two cookie rules beyond the spec's.** **(differs)** Spec §3.3 left
    out a cookie whose name is not a token or whose value or path holds a
    character a cookie cannot carry. Besides, a `same_site` must be empty
    or an HTTP token, so it cannot carry a `;` and an attribute of its own;
    and a value in double quotes, which RFC 6265 allows, is refused, so a
    value has one rule, RFC 6265's cookie-octets: no space, `"`, `,`, `;`,
    `\`, control character or anything outside ASCII. A path may hold any
    ASCII character but a control character or `;`. A cookie that breaks a
    rule is left out of the response, without an error (mutants 51 to 53).
18. **The `405`'s body is `method not allowed`,** as
    `text/plain; charset=utf-8`. **(differs)** The spec did not name it.
19. **`Dist::file` does not look at the method.** **(differs)** Spec §4
    did not say; `Server` asks its fallback only for a `GET`, and the
    blog's fallback is where `file` is called.
20. **`Response.cookies` is `[Cookie]`, an array.** **(differs)** Spec
    §3.3 named the field, not its type. `cookies: []` is inferred from the
    field's type, and `with_cookie` builds a new array, so the response it
    was called on keeps its own cookies; a `Vec` is shared on assignment,
    and would need copying, as `Server::get` copies its routes
    (`http_responses.nova`'s `two keeps` and `one again` lines).

**The blog (the prototype's part B)**

21. **The blog's `GET` routes answer `404` for its paths that have only a
    `POST` route:** `/posts/:id/comments`, `/logout`, `/admin/posts`,
    `/admin/posts/:id`, `/admin/posts/:id/delete` and
    `/admin/comments/:id/delete`, each with the not-found page, as spec
    §6.2's table asks of any other `GET`. **(differs)** Without them
    `std/http` answers such a `GET` with `405` (decision 2), which the
    spec's table did not foresee (mutant 92).
22. **`.gitignore` lists the example's `target/`, `data/blog.json.tmp`,
    `data/users.json` and `data/users.json.tmp` too,** beside the spec's
    `dist/`, `data/blog.json` and `data/secret`. **(differs)** Spec §6.1
    named three. A build in place makes `target/`, a write that stops
    before its rename leaves a `.tmp` file, and `users.json` holds the
    users (decision 37).
23. **`Data` gains `missing` and `refused`, and `routes()` maps the paths
    forms are posted to.** **(differs)** Spec §6.3 listed `Data`'s contents
    without these. `missing` renders the not-found page, for a path with no
    page and for a post that does not exist; `refused` renders a refused
    form's reason in place of the route's page, with `403`. A form that
    fails a check is rendered again at the path it was posted to, which is
    then the browser's location, so `routes()` gives `/posts/:id/comments`,
    `/admin/posts` and `/admin/posts/:id` their pages, the post's and the
    editor's, on both sides, and the client hydrates what the server
    rendered. A form's values and messages are `draft`, whose `failed`
    shows the messages from the start and whose `error` holds a message for
    the whole form, such as a wrong password.
24. **The submit button is two buttons, each under a `show`.** **(differs)**
    Spec §6.6 has one button, disabled while the form is not valid.
    `std/ui` writes every attribute it is given and never removes one, and
    `disabled` disables a button by being there at all, whatever its value,
    so one button cannot be both. The server's HTML holds the enabled one,
    which a page without JavaScript keeps; the client swaps them as the
    form's checks change (`forms.nova`'s `submit`).
25. **Stored text is trimmed, and its CRLFs made LF** (`cleaned`), and it
    holds no NUL (decision 39). **(differs)** Spec §6.6 trimmed only to
    measure. A browser's parser drops a line feed that begins a
    `textarea`'s text, so a body stored with one would come back to the
    editor as another text and mismatch on every hydration; and a browser
    sends a `textarea`'s lines ending in CRLF, where the field's value,
    which the client's checks measure, holds LF. The checks measure a
    field as `cleaned` leaves it, on both sides (decision 44). The login's
    name is trimmed, and its password is not (mutants 90 and 91).
26. **A login with a name no user has uses the largest count of the users'
    hashes,** the users the server read from `users.json` when it started
    (decision 37), skipping a count PBKDF2 refuses, or, when there is none, the
    server's `--iterations`, 600,000 unless given (`auth::no_user_rounds`;
    `password_ok` would still make a count outside PBKDF2's range 1).
    **(differs)** Spec §6.5's "the same count" left open whose. As first
    built it took the first user's count, which decision 41 changes.
27. **`--set-password` refuses a blank name, and one with spaces around
    it, with status 2,** before it reads the password, as it refuses a
    password that is not UTF-8, one shorter than eight characters, or one
    that is blank (decision 40), with status 1, writing nothing; and,
    with status 1, a `users.json` that does not hold users, which the
    server refuses too, when it starts (decision 37). **(differs)** Spec
    §6.5 named only the short password. `set_password_writes_the_admin`
    checks each refusal's status and message, that neither `users.json`
    nor `blog.json` is written, and that a refused `users.json` is left as
    it is (mutants 85, 104, 108 to 111 and 115).
28. **The session's cookie lasts as long as its session:** `Max-Age` 86,400
    for a visitor and 28,800 for the admin, so a browser drops it when the
    server would; logging out sends `Cookie::expired("sid")`. **(differs)**
    Spec §6.4 did not say, and `Cookie::new` has no `Max-Age`.
    `every_page_answers_with_its_status`,
    `logging_in_takes_the_right_password_and_a_new_session` and
    `logging_out_ends_the_session` check each cookie whole; mutants 79 and
    80 break its `HttpOnly` and its `SameSite`.
29. **A refusal's details.** **(differs)** Spec §6.2 said a `POST` without a
    logged-in session to an admin's path is `403`, and one with a missing or
    wrong token `403` with the expired-form page. As built, in `checked`:
    - a `POST` with no session, to an admin's path or not, gets `403` with
      the expired-form page;
    - one whose token is missing or wrong gets the same, shown in its
      session;
    - one with a visitor's session and its right token, to an admin's path,
      gets `403` with "Only the admin can do that. Log in first.";
    - a `403` page starts no session and sets no cookie;
    - a `POST` that names a post or a comment that does not exist, once its
      token is checked, gets the not-found page with `404`.

    Mutants 70 to 73 and 82.
30. **`lib.nova` is one-line wrappers,** such as `store_from`,
    `users_from`, `no_users`, `sessions`, `user_with`, `logs_in` and
    `message_for`. **(differs)** Spec §6.1 had it
    be the package's library, for its tests. Nova has no re-exports:
    `tests/*.nova` import the library as `fullstack_blog`, can name only
    what it declares, and use what those functions return through its
    methods and fields.
31. **The tests share one build** (spec §10, risk 6). `blog_tests.rs` copies
    the example once per run of the file, less `dist/`, `target/` and the
    data files a run in place leaves, into `nova-blog-tests/fullstack-blog`,
    builds it there with `nova build --ssr` behind a `OnceLock`, with a
    fresh `NOVA_HOME` beside it, and fails every test with the build's
    output when it fails. Each of its thirteen tests has a data directory
    of its own, holding the seed; nine set the admin's password with
    `--set-password admin --iterations 1000`, and the timing test another
    user's over 1,000 rounds, then the admin's over 100,000 (decision 36);
    eleven start the server on port 0, speak HTTP over `TcpStream`, one
    connection a request, and kill the server when they end, pass or fail,
    one of them posting comments while `--set-password` hashes over
    600,000 rounds, and another first running a `--set-password` at 1,000
    rounds to its end while one at 600,000 hashes (decision 37). The server's refusal to start is checked
    on a key file that does not hold 32 bytes in hex, and on a
    `users.json` that does not hold users. A run of the server that should
    end and has not within 30 seconds is killed, and its test fails. The
    example's `nova test` runs 21 tests.

**The split and the replay**

32. **`post_card.nova` comes whole in Task 5,** a client component among
    the server's modules: `lib.nova` reaches its `paragraphs` and
    `first_paragraph`, which the blog's `forms` tests check, and it needs
    only `app`'s `Post` and `ui`. The other client modules, `home`, `post`,
    `login`, `admin`, `header` and `main`, wait for Task 6.
33. **The blog's `nova.toml` comes with its tests, in Task 5's tests:**
    `tests/*.nova` import the package's library by its name, and without the
    manifest `nova test` finds no package. With it, and no `src/`, `nova
    test` refuses with M0013, which is Task 5's failing step.
34. **`nova test` runs in a copy of the example,** `$P/blog-47`: run in
    place it would leave `target/` in the example, which `.gitignore` names
    only from Task 6. `run_tests.rs`'s `the_blog_example_passes_its_own_tests`
    copies `src/`, `tests/` and `nova.toml` into
    `nova-blog-nova-test/fullstack-blog`, with a fresh `NOVA_HOME` beside it.

**The gap fix**

35. **The admin's own forms are refused without their token.** The
    mutants' first run found that `checked` could skip the token for an
    admin's form posted in the admin's own session, and pass (mutant 73).
    `the_admin_pages_need_the_admin` now logs in as the admin and posts each
    admin form, to create, edit and delete a post and delete a comment, and
    the logout, first without its `csrf` field, then with the visitor's
    token: each must be `403` with the expired-form page and no cookie, and
    `/`, `/posts/1` and `/admin` must be the same after as before (spec
    §6.4: each `POST` handler checks the token before it reads anything
    else).
36. **A login's time is measured.** **(differs)** Spec §7.4 lists no such
    test; spec §6.5 and §12.12 require that a name no user has take as
    long. `a_name_no_user_has_takes_as_long_as_a_wrong_password` uses its
    own data directory, sets another user's password over 1,000 rounds,
    then the admin's over 100,000, so that hashing, not HTTP, is most of a
    login's time, and a name no user has must cost the admin's count, not
    the first user's (decision 41), and times five
    logins of each kind, the admin with a wrong password and `nobody`,
    alternating which goes first; each median must be at least half the
    other. Run alone, with the shared build, it takes about 7 seconds;
    the whole of `blog_tests`, whose tests run side by side, about 9
    (mutants 84 and 116).

**The fact-check's fixes**

A fact-check of the plan read the prototype against the spec and probed
the running blog; it found nine things (Checked on). The prototype's
`5bae2a7` ("4.7 prototype: the fact-check's fixes") fixes them, and each
fix's test fails with its fix broken. A second check found five more
(Checked on), which `2cc807b` ("4.7 prototype: the second check's fixes")
fixes: decision 37 is its design, in place of the first fix's, and it
corrects decisions 38 and 41.

37. **The users have a file of their own, `<data>/users.json`, which only
    `--set-password` writes; a password it sets takes effect when the
    server next starts.** **(differs)** Spec §6.3 has the store hold the
    users, in `blog.json`, and §6.5 has `--set-password` write them there.
    Two writers of one file lost data each time it was checked. First, the
    server's next write put its own users over a password `--set-password`
    had set while it ran, so the old password stayed good. Then the fix
    for that, each server write keeping the users the file held, let
    `--set-password` write back the store it had read before its hash,
    erasing what the server saved during the hash, about two seconds at
    600,000 rounds, and under steady writes its own user too while it
    exited 0; and a server write that found no `blog.json` wrote the
    seed's users, none, while an unreadable one stopped every write. Now
    each file has one writer. `--set-password` hashes first, then reads
    `users.json`, no users when it does not exist, sets the user, writes
    `users.json.tmp` and renames it over `users.json`; it never reads or
    writes `blog.json`. The server reads `users.json` once, when it starts,
    refuses to start when it does not hold users, and never writes it;
    `saved` writes the server's own copy of the store to `blog.json`,
    reading no file. `seed.json` has no `users` key, and the store ignores
    one, so an old `blog.json`'s users are ignored (Checked on). The server
    checks logins against the users it read when it started, so the README
    says to stop the server, run `--set-password`, then start it. A
    `--set-password` that runs from start to end while another hashes is
    kept, since the slower one reads `users.json` after its hash; two runs
    can still lose one's user when one reads `users.json` after the other
    has read it and before the other has renamed its own over it, a window
    of milliseconds after both hashes; the README says to run one at a time
    (Checked on).
    `a_password_set_while_the_server_runs_loses_nothing` posts comments
    while `--set-password` hashes over 600,000 rounds, each awaited until
    `blog.json` holds it, then finds every one there, both users in
    `users.json`, the new user refused by the running server, and both
    users' logins and every comment after a restart;
    `set_password_writes_the_admin` checks that no `blog.json` is written
    and that a `users.json` that does not hold users is refused, by both;
    `store::the_users_have_a_file_of_their_own` checks both JSONs; and
    `overlapping_set_password_runs_keep_both_users` (`64064e8`) sets `ada`
    over 600,000 rounds and, half a second in, `bea` over 1,000, needs
    `ada`'s run to end at least 250 ms after `bea`'s, saying the runner is
    too slow otherwise, then finds both in `users.json` and logs both in
    (mutants 97 and 112 to 115).
38. **Ended sessions are dropped only once the table has doubled, and
    holds at least 64.** **(differs)** Spec §6.4 drops them each time a
    session starts, and its §10, risk 5, has the table hold at most a day
    of visitors. Every page asked for without a cookie starts a session,
    so a scan at every start made the cost of a request grow with the
    table: about 1.3 ms more for every 1,000 sessions on the debug runtime.
    Now `Sessions.kept` is how many sessions the table held after the last
    scan, and `start` scans only when the table holds at least 64
    (`FEWEST_TO_DROP`) and at least `2 * kept`: half of what each scan
    reads started since the last, so the scans cost the same per session
    on average, however many the table holds. Measured with `GET /login`
    and no cookie on the debug runtime, 200 to 8,200 sessions in the table:
    1.59 to 12.78 ms before, 1.52 to 1.87 ms after. But the start that
    finds the table doubled scans all of it, in time linear in its size:
    18.4 ms for the 16,385th session and 35.7 ms for the 32,769th. The
    table holds at most twice the sessions live at the last scan, or 64 if
    that is more: after a burst, the sessions that end stay until it
    doubles again, and a steady flood of clients that keep no cookie keeps
    up to two days' worth, the live day and as many ended, where risk 5
    expected one. A session takes about 658 bytes: about 22 GB a day at the
    391 requests a second one client made in a probe (Checked on; the
    second check corrected the first fix's records, which said at most a
    day's sessions and the same cost for every start). `sessions::ended_sessions_are_dropped_once_the_table_has_doubled`,
    with visitors' sessions that end at once, checks a fresh table, which
    has kept none, at 63, 64 and 1, so the floor holds however much the
    table has grown since it was scanned (`3e49a31`); then, with forty of
    the admin's sessions that do not end, a table at 64, 41, 80 and 41
    (mutants 98 to 102).
39. **Stored text holds no NUL.** **(differs)** Decision 25 made stored
    text the text a `textarea` reads back; a NUL broke that too. A
    browser's parser drops a NUL from a page's text and makes it U+FFFD in
    a `textarea` or an attribute, so one comment holding a NUL made its
    post's page mismatch, and keep none of the server's nodes, for every
    visitor. `cleaned` drops U+0000 first. In Chromium no other character
    below U+0020, nor U+007F, broke hydration, in a post's title and body,
    a comment's name and text, or a failed comment's attribute and
    `textarea`; `std/ui` writes a lone CR as `&#13;`. Tested by
    `forms::the_server_keeps_no_nul`, by `visitor_text_comes_back_escaped`,
    which posts a comment holding NULs, and by `blog.mjs`, whose comment
    holding a NUL comes back without it (mutant 103).
40. **`--set-password` refuses a blank password,** one that is blank once
    trimmed, with status 1 and `blog: a password is not blank`, writing
    nothing. **(differs)** Spec §6.5 refuses only a password shorter than
    eight characters; the login's check refuses a blank password, so a
    user given one could never log in (mutant 104).
41. **A name no user has costs the largest count of the users' hashes,**
    not the first user's. **(differs)** It changes decision 26 as first
    built: with users of different counts, the first user's count let a
    login's time tell a name no user has from the name of a user with more
    rounds. `no_user_rounds` takes the largest count PBKDF2 takes, so a
    count from a file changed by hand, such as 2^32, is skipped, and
    `--iterations` when there is none. A user whose hash has fewer rounds
    than the largest still answers sooner than a name no user has
    (Checked on). `passwords::a_name_no_user_has_costs_the_most_rounds_a_user_has`,
    through `lib.nova`'s `rounds_for_no_user` (mutants 105 and 106); and,
    since the second check found the server's use of it untested,
    `a_name_no_user_has_takes_as_long_as_a_wrong_password` sets another
    user at 1,000 rounds before the admin at 100,000: with `log_in` on the
    first user's count, its medians were 454.9 ms for a wrong password and
    6.26 ms for a name no user has (mutant 116).
42. **`Dist::open`'s doc example is `ignore`.** **(differs)** Spec §7.2
    asks a doc test of each new public function. `open`'s example reads a
    site `nova build --ssr` wrote into `dist` and serves it, which a doc
    test has not; the doc tests of `file` and `respond` call `Dist::open`
    on a site they write, so it runs there, and
    `ssr_dist_answers_files_and_pages` checks its errors.
43. **`auth.nova` imports `store`,** for `User`. **(differs)** Spec §6.1
    has only `server.nova` and `lib.nova` import the server's modules.
    Both are server modules; the client's `main.nova` reaches neither, as
    `index_complete.rs`'s count of the client's eight modules shows.
44. **The forms' checks measure a field as `cleaned` leaves it, on both
    sides.** **(differs)** Spec §6.6 measures characters after trimming.
    `sized`, behind the title's, body's, name's and comment's checks,
    counts `cleaned(value)`, so a CRLF counts as one character and a NUL
    as none, on the server and in the client alike, and a field the
    server would store blank is refused as blank. The login's checks still
    trim the name and refuse a blank password. The client grew from 49,423
    to 49,472 bytes gzipped.

---

### Task 0: The baseline

The branch is at this plan's commit, whose parent is `c8d6196`, the
approved spec. The plan's commit adds this file alone, so the patches apply
to it as to `c8d6196`.

- [ ] **Step 1: The tree.**
  - `git status --short` prints nothing, on
    `claude/loving-archimedes-frtf5q-47`.
  - `git diff --stat c8d6196 HEAD` lists only
    `docs/superpowers/plans/2026-10-21-phase-4-7-fullstack-blog.md`.
  - `BASE=$(git rev-parse --short HEAD)`: the plan's commit, recorded in
    the ledger. Task 9's review reads from it.
  - `df -h /` shows at least 8G free; if not, remove scratch directories
    of earlier sub-phases first.
- [ ] **Step 2: Node, the browser and `clang`.** `node --version` prints
  `v22.` and more; `ls -d /opt/node-tools/node_modules/playwright
  /opt/pw-browsers` lists both, for Task 7's local run (skip that run where
  either is missing, and say so in the ledger); `command -v clang` prints a
  path, so 4.6's `release_applies_to_the_client_and_the_server` runs (where
  it prints nothing, that test passes without running: say so in the
  ledger).
- [ ] **Step 3: The suite.** In a shell set up as the conventions say:

  ```sh
  cargo build --locked --workspace
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/base-47.txt" 2>&1
  grep '^test result' "$P/base-47.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/base-47.txt"
  ```

  Expected: `2392 1 10`, and one failure, `test
  net::tests::an_ipv6_connect_parks_on_its_first_poll_and_then_establishes
  ... FAILED`. It needs IPv6, which this machine lacks, as in every
  earlier phase. (`c8d6196`'s tree is `9ad78cd`'s and the spec. These
  figures are 4.6's last full run, at `b4cd43e` "4.6: the review's fixes"
  on 4.6's branch; `b4cd43e` is not an ancestor of `c8d6196`, but its tree
  is that of `782f77c`, the commit of the same name that is, and `git diff
  b4cd43e c8d6196 -- crates/` holds only `bundle.rs`'s `cfg_attr` for
  Windows' clippy and `ssr_tests.rs`'s fix for a CRLF checkout, which add
  no test. No full run was made at `c8d6196` itself.)
- [ ] **Step 4: The ledger.** Start `$P/ledger-47x.md`, headed with the
  base commit and the baseline's figures.

### Task 1: Hex, `rename` and PBKDF2

Spec §5, §8; decisions 8, 15 and 16.

**Files:**
- Modify:
  - `std/bytes/lib.nova` (`Bytes::to_hex`, `bytes_from_hex`,
    `hex_digit_value`)
  - `std/fs/lib.nova` (`rename`)
  - `std/crypto/lib.nova` (`pbkdf2_sha256`, `pbkdf2_sha256_verify`,
    `pbkdf2_refused`, two statuses)
  - `crates/nova-resolver/src/lib.rs` (three builtins, `STD_ONLY`)
  - `crates/nova-typeck/src/check.rs` (their signatures, and its test's
    arms)
  - `crates/nova-mir/src/lib.rs`, `lower.rs` (three `RtFunc`s)
  - `crates/nova-runtime/src/crypto.rs` (`iterations_of`, `derive_key`,
    `verify_key`, two intrinsics), `fs.rs` (`nova_rt_fs_rename`), `lib.rs`
    (three symbols)
- Test:
  - `tests/runtime/bytes_hex.nova`, `bytes_hex.stdout`
  - `tests/runtime/fs_rename.nova`, `fs_rename.stdout`
  - `tests/runtime/crypto_pbkdf2.nova`, `crypto_pbkdf2.stdout`,
    `crypto_pbkdf2_refused.nova`
  - `crates/nova-cli/tests/run_tests.rs`, `wasm_tests.rs`
  - `crates/nova-runtime/src/crypto.rs` (its three tests alone)
  - `crates/nova-mir/tests/lower_tests.rs`
  - `crates/nova-driver/tests/index_complete.rs`

- [ ] **Step 1: The tests.** `patch_block t1-tests`. The patch holds:
  - `tests/runtime/bytes_hex.nova`: `to_hex` of no bytes, of each nibble's
    edges, of `Nova ✓`'s UTF-8 and of all 256 bytes, read back by
    `bytes_from_hex` as written and in upper case; and `None` for an odd
    length, one digit, `g`, `G`, a space, a leading `0x`, a minus, a
    character outside ASCII, and a bad last or first digit. `bytes_hex_run`
    runs it as it is and under GC stress, through
    `run_with_and_without_stress`, which this patch adds; the gate runs it
    on WASM as `gate::bytes_hex`.
  - `tests/runtime/fs_rename.nova`, in `nova-fs-rename` under `temp_dir()`,
    which `fs_rename_run` points at its own `nova-fs-rename-run`: a file
    renamed over an existing one, which it replaces, the temporary name
    gone; a rename to a free name; and `NotFound` for a missing source and
    for a missing directory, the source kept. The test then checks on disk
    that only `moved.json` is left, holding what was written last.
  - `tests/runtime/crypto_pbkdf2.nova`: RFC 7914's two
    PBKDF2-HMAC-SHA256 vectors, 64 bytes each, in hex; a 20-byte key, the
    64's start; one byte from empty inputs; and `pbkdf2_sha256_verify` true
    for the key, the 64-byte vector and the key's first 31 bytes, and false
    for another password, salt or count, a key one byte longer and an empty
    one (`crypto_pbkdf2_run`).
  - `tests/runtime/crypto_pbkdf2_refused.nova` and
    `crypto_pbkdf2_refuses_its_bad_arguments`: with no argument, the longest
    and the shortest length, 65,536 and 1, run; each of eight cases, a count
    of 0, -1 or 4,294,967,296 for `pbkdf2_sha256` and of 0 or 4,294,967,296
    for `pbkdf2_sha256_verify`, and a length of 0, -32 or 65,537, panics
    with its text and prints nothing.
  - `wasm_tests.rs`: `bytes_hex` in the gate; `crypto_pbkdf2`,
    `crypto_pbkdf2_refused` and `fs_rename` in `LEFT_OUT`.
  - `crypto.rs`'s three unit tests alone: RFC 7914's vectors through
    `derive_key`; `verify_key` true and false as above; and `iterations_of`
    refusing 0, -1 and 2^32, and taking `u32::MAX`.
  - `lower_tests.rs`: `crypto_builtins_reach_their_runtime_functions` gains
    two stubs, which call `crypto_pbkdf2_sha256` and
    `crypto_pbkdf2_sha256_verify` and must lower to their `RtFunc`s.
  - `index_complete.rs`: 160 programs and 186 files.
- [ ] **Step 2: Run, and see them fail.** First the build (Conventions).
  - `R fs_rename_run` (about 11 s). Expected: `0 passed; 1 failed`:
    "Unexpected failure." of `nova run tests/runtime/fs_rename.nova`:
    ``error[E0001]: cannot find function `rename` in this scope``, `could not
    compile due to 4 previous errors`.
  - `R bytes_hex_run`. Expected: `0 passed; 1 failed`: "Unexpected
    failure.": ``error[E0014]: no method `to_hex` on type `Bytes` ``, `could
    not compile due to 21 previous errors`.
  - `R crypto_pbkdf2`. Expected: `0 passed; 2 failed`: `crypto_pbkdf2_run`
    and `crypto_pbkdf2_refuses_its_bad_arguments`, each "Unexpected
    failure.": ``error[E0001]: cannot find function `pbkdf2_sha256` in this
    scope``, then the same for `pbkdf2_sha256_verify`: `could not compile
    due to 13 previous errors` and `due to 10 previous errors`.
  - `W bytes_hex`. Expected: `0 passed; 1 failed`: `gate::bytes_hex`,
    "Unexpected failure." of its `nova build --target wasm`: the same
    E0014, 21 errors.
  - `W every_runtime_program_is_in_the_gate_or_left_out`. Expected: `1
    passed`: the programs and the gate's lists arrive together.
  - `X` (about 13 s). Expected: `0 passed; 1 failed`:
    `every_name_in_the_corpora_is_indexed` (`index_complete.rs:258`):
    "…/tests/runtime/bytes_hex.nova does not check cleanly: [\"E0014 no
    method `to_hex` on type `Bytes`\", …]".
  - `cargo test -p nova-runtime --lib crypto::tests::pbkdf2`. Expected: it
    does not build: ``error[E0433]: cannot find type `NonZeroU32` in this
    scope`` (once), ``error[E0425]: cannot find function `iterations_of` in
    this scope`` (8 times), the same for `derive_key` (3) and `verify_key`
    (7): ``could not compile `nova-runtime` (lib test) due to 19 previous
    errors``.
  - `cargo test -p nova-mir --test lower_tests
    crypto_builtins_reach_their_runtime_functions`. Expected: it does not
    build: ``error[E0599]: no variant, associated function, or constant
    named `CryptoPbkdf2Sha256` found for enum `RtFunc` in the current
    scope`` (`lower_tests.rs:1348`), and the same for
    `CryptoPbkdf2Sha256Verify` (`:1360`): ``could not compile `nova-mir`
    (test "lower_tests") due to 2 previous errors``.
- [ ] **Step 3: The code.** `patch_block t1-code`:
  - **`std/bytes/lib.nova`:** `Bytes::to_hex`, `bytes_from_hex` and
    `hex_digit_value`, in Nova, with their doc tests; the module's doc
    names them (decision 8).
  - **`std/fs/lib.nova`:** `rename`, an `async fn` that never suspends, over
    `fs_rename`, its errors `std/io`'s kinds, with its doc test.
  - **`std/crypto/lib.nova`:** `pbkdf2_sha256` and `pbkdf2_sha256_verify`
    over the two intrinsics, `PBKDF2_BAD_ITERATIONS` and
    `PBKDF2_BAD_LENGTH`, and `pbkdf2_refused`'s panics (decisions 15 and
    16), with their doc tests; the module's doc names them, and says
    `Bytes::to_hex` writes a hash as hex.
  - **The builtins:** `nova-resolver`'s `FsRename`, `CryptoPbkdf2Sha256` and
    `CryptoPbkdf2Sha256Verify`, each std-only (`STD_ONLY`, 90);
    `nova-typeck`'s signatures, and its test's arms; `nova-mir`'s three
    `RtFunc`s, their symbols and signatures, and their lowering.
  - **The runtime:** `nova_rt_fs_rename` on `std::fs::rename`, which
    replaces the target on Windows too; `nova_rt_crypto_pbkdf2_sha256` and
    `nova_rt_crypto_pbkdf2_sha256_verify` on `ring::pbkdf2`, through
    `iterations_of`, `derive_key` and `verify_key`, with
    `ERR_INVALID_ITERATIONS`, `ERR_INVALID_OUTPUT_LENGTH` and
    `MAX_PBKDF2_BYTES`, each bound checked before anything is derived or
    allocated; the three symbols in `lib.rs`'s table.
- [ ] **Step 4: Run.** First the build, which now builds the runtime
  library with the three new symbols (about 8 s).
  - The same eight commands. Expected: `R fs_rename_run` `1 passed`; `R
    bytes_hex_run` `1 passed`; `R crypto_pbkdf2` `2 passed` (about 2 s); `W
    bytes_hex` `1 passed`; `W
    every_runtime_program_is_in_the_gate_or_left_out` `1 passed`; `X` `1
    passed` (about 32 s); the runtime's units `3 passed; 0 failed; 0
    ignored; 0 measured; 272 filtered out`; the MIR test `1 passed`.
  - The whole binaries:
    - `cargo test -p nova-runtime -- --skip
      an_ipv6_connect_parks_on_its_first_poll_and_then_establishes`.
      Expected: the library `266 passed; 0 failed; 8 ignored; 0 measured;
      1 filtered out` (263 before; the IPv6 test, skipped, fails here at
      the base too), and its five integration binaries pass.
    - `cargo test -p nova-mir`. Expected: the library `36`, `lower_tests`
      `61`.
    - `cargo test -p nova-resolver`. Expected: `54 passed`.
    - `cargo test -p nova-typeck`. Expected: `410 passed` (about 20 s).
    - `cargo test -p nova-std`. Expected: `2 passed`.
    - `cargo test -p nova-doc --lib`. Expected: `61 passed`.
    - `cargo test -p nova-cli --test doc_test`. Expected: `17 passed`:
      `std/bytes`'s, `std/fs`'s and `std/crypto`'s new examples run inside
      `doc_std_runs_stds_doc_tests_inside_a_project_or_not`.
    - `R`. Expected: `321 passed; 0 failed; 1 ignored` (317 before), in
      about 50 seconds.
    - `W`. Expected: `230 passed` (229 before).
- [ ] **Step 5: The checks,** as the conventions list them, and `cargo run
  --locked -q -p nova-cli -- fmt --check tests/runtime/bytes_hex.nova
  tests/runtime/fs_rename.nova tests/runtime/crypto_pbkdf2.nova
  tests/runtime/crypto_pbkdf2_refused.nova`, which prints nothing.
- [ ] **Step 6: Commit** `std: hex in std/bytes, std/fs::rename, and PBKDF2 in std/crypto`.

**Patch `t1-tests`:**

<!-- patch t1-tests -->

````diff
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index 2f12249b6cf0018787376add43663811c7e45b61..f71932a8fa0f951830838a717e056c3a20775368 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -6514,6 +6514,47 @@ fn fs_roundtrip_run() {
     );
 }
 
+/// `std/fs`'s `rename` (spec
+/// `docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md`
+/// §5): a file renamed over an existing one replaces it, a rename to a free
+/// name moves it, and a missing source or target directory is `NotFound`.
+/// The fixture works under `temp_dir()`, which this points at a directory of
+/// fixed name of the test's own; and the files left behind are checked here
+/// on the disk itself, so a `rename` that copied, or reported success and
+/// did nothing, fails here even if the fixture's own reads agreed with it.
+#[test]
+fn fs_rename_run() {
+    let tmp = std::env::temp_dir().join("nova-fs-rename-run");
+    let _ = std::fs::remove_dir_all(&tmp);
+    std::fs::create_dir_all(&tmp).expect("the test's directory");
+    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/fs_rename.stdout"))
+        .expect("expected-output fixture exists")
+        .replace("\r\n", "\n");
+    nova()
+        .arg("run")
+        .arg(repo_root().join("tests/runtime/fs_rename.nova"))
+        .env("TMPDIR", &tmp)
+        .env("TMP", &tmp)
+        .env("TEMP", &tmp)
+        .assert()
+        .success()
+        .stdout(expected);
+    let dir = tmp.join("nova-fs-rename");
+    let left: Vec<String> = std::fs::read_dir(&dir)
+        .expect("the fixture's directory")
+        .map(|e| {
+            e.expect("an entry")
+                .file_name()
+                .to_string_lossy()
+                .into_owned()
+        })
+        .collect();
+    let moved = std::fs::read_to_string(dir.join("moved.json")).unwrap_or_default();
+    let _ = std::fs::remove_dir_all(&tmp);
+    assert_eq!(left, ["moved.json"], "only the renamed file is left");
+    assert_eq!(moved, "{\"posts\": 2}", "it holds what was written last");
+}
+
 /// A second `create_dir` on an existing directory reports `AlreadyExists`.
 /// The fixture's own leading `remove_dir_all` (discarding the result: absent
 /// is fine, present is cleaned up) makes this independent of a previous
@@ -6778,6 +6819,16 @@ fn bytes_api_under_gc_stress() {
         .stdout(expected);
 }
 
+/// `Bytes::to_hex` and `bytes_from_hex` (spec
+/// `docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md`
+/// §5): every byte both ways, either case read, and the refused inputs: an
+/// odd length and each kind of character that is not a hex digit. As it is
+/// and under GC stress here; `wasm_tests.rs`'s gate runs it on WASM.
+#[test]
+fn bytes_hex_run() {
+    run_with_and_without_stress("bytes_hex");
+}
+
 /// Review finding P2 (Important): `bytes_from_ints`'s `0..=255` contract is
 /// documented in three places (the plan, `bytes.rs`'s doc comment, and
 /// `std/bytes/lib.nova`'s), and nothing exercised it -- replacing the
@@ -9577,6 +9628,24 @@ fn http_listen_on_run() {
     }
 }
 
+/// `tests/runtime/<name>.nova` run as it is and under GC stress, its stdout
+/// its `.stdout` file each time, as `http_listen_on_run` runs its program.
+fn run_with_and_without_stress(name: &str) {
+    let source = repo_root().join(format!("tests/runtime/{name}.nova"));
+    let expected = std::fs::read_to_string(source.with_extension("stdout"))
+        .expect("expected-output fixture exists")
+        .replace("\r\n", "\n");
+    for stress in [false, true] {
+        let mut cmd = nova();
+        if stress {
+            cmd.env("NOVA_GC_STRESS", "1");
+        }
+        let a = cmd.arg("run").arg(&source).assert().success();
+        let out = String::from_utf8_lossy(&a.get_output().stdout).replace("\r\n", "\n");
+        assert_eq!(out, expected, "{name}, stress {stress}");
+    }
+}
+
 /// `std/process`'s `args()` under `nova run FILE -- ARGS`: index 0 is FILE as
 /// written, and an argument with spaces and an empty one each arrive whole.
 /// Run from the repository root so FILE is a fixed relative path. Design:
@@ -9895,6 +9964,76 @@ fn crypto_random_run() {
         .stdout(expected);
 }
 
+/// `pbkdf2_sha256` against RFC 7914 section 11's two PBKDF2-HMAC-SHA-256
+/// vectors (`passwd`/`salt`, c=1, and `Password`/`NaCl`, c=80000, both 64
+/// bytes), and `pbkdf2_sha256_verify` true and false (spec
+/// `docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md`
+/// §5). Both vectors were cross-checked against Python's `hashlib`.
+#[test]
+fn crypto_pbkdf2_run() {
+    let expected = std::fs::read_to_string(repo_root().join("tests/runtime/crypto_pbkdf2.stdout"))
+        .expect("expected-output fixture exists")
+        .replace("\r\n", "\n");
+    nova()
+        .arg("run")
+        .arg(repo_root().join("tests/runtime/crypto_pbkdf2.nova"))
+        .assert()
+        .success()
+        .stdout(expected);
+}
+
+/// The panics of PBKDF2 (spec 4.7 §5), one run per case: an iteration count
+/// below 1 or above 4,294,967,295 for either function, and a length below 1
+/// or above 65,536 for `pbkdf2_sha256`, each with its text and nothing after
+/// it run; and the largest length, 65,536, and the smallest, 1, accepted.
+#[test]
+fn crypto_pbkdf2_refuses_its_bad_arguments() {
+    let source = repo_root().join("tests/runtime/crypto_pbkdf2_refused.nova");
+    nova()
+        .arg("run")
+        .arg(&source)
+        .assert()
+        .success()
+        .stdout("65536 bytes: 65536\n1 byte: 1\nnot reached for a refused case\n");
+    let iterations = "iterations must be from 1 to 4294967295, not";
+    let length = "length must be from 1 to 65536, not";
+    for (case, says) in [
+        ("zero", format!("pbkdf2_sha256: {iterations} 0")),
+        ("negative", format!("pbkdf2_sha256: {iterations} -1")),
+        (
+            "too-many",
+            format!("pbkdf2_sha256: {iterations} 4294967296"),
+        ),
+        ("no-length", format!("pbkdf2_sha256: {length} 0")),
+        ("negative-length", format!("pbkdf2_sha256: {length} -32")),
+        ("too-long", format!("pbkdf2_sha256: {length} 65537")),
+        (
+            "verify-zero",
+            format!("pbkdf2_sha256_verify: {iterations} 0"),
+        ),
+        (
+            "verify-too-many",
+            format!("pbkdf2_sha256_verify: {iterations} 4294967296"),
+        ),
+    ] {
+        let a = nova()
+            .env("RUST_BACKTRACE", "0")
+            .arg("run")
+            .arg(&source)
+            .arg("--")
+            .arg(case)
+            .assert()
+            .failure();
+        let o = a.get_output();
+        let err = String::from_utf8_lossy(&o.stderr);
+        assert!(
+            err.contains(&format!("nova: panic: {says}\n")),
+            "{case}: {err}"
+        );
+        assert_eq!(String::from_utf8_lossy(&o.stdout), "", "{case}");
+    }
+}
+
 /// `examples/05-json-api` driven over a real socket: its three routes and the
 /// error paths the assertions below name, each by status code and whole
 /// response body.
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index 82253fba02a475489de61f4cb4d6ce1242039881..8e4b2314bb9901b7dcad92b7d1ea9b95ca730bac 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -1869,6 +1869,7 @@ mod gate {
         strings,
         // Bytes.
         bytes_basics,
+        bytes_hex,
         // Formatting.
         fmt_float_edge,
         fmt_float_fixed,
@@ -2777,6 +2778,8 @@ fn ui_imports_check_on_both_targets() {
 const LEFT_OUT: &[(&str, &str)] = &[
     ("bytes_reserved", "E0089 (its error is the point)"),
     ("crypto_hashes", "std/crypto"),
+    ("crypto_pbkdf2", "std/crypto"),
+    ("crypto_pbkdf2_refused", "std/crypto, std/process"),
     ("crypto_random", "std/crypto"),
     ("extern_ffi", "E0090 (an extern block)"),
     ("file_errors", "std/fs, std/io"),
@@ -2792,6 +2795,7 @@ const LEFT_OUT: &[(&str, &str)] = &[
     ("fs_not_found", "std/fs, std/io"),
     ("fs_permission_denied", "std/fs, std/io"),
     ("fs_read_dir", "std/fs"),
+    ("fs_rename", "std/fs, std/io"),
     ("fs_roundtrip", "std/fs"),
     ("fs_write_then_read", "std/fs"),
     ("http_keepalive", "std/http, std/net"),
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index 683ac6c0bfa7cdf218790391bae3755dd2d57677..b0140141fc1130ea140172efbd6f00a484bb4a9e 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -241,10 +241,10 @@ fn every_name_is_covered(a: &Analysis, file: FileId, failures: &mut Vec<String>)
 #[test]
 fn every_name_in_the_corpora_is_indexed() {
     let programs = programs();
-    // 8 examples, 147 top-level runtime programs and the modules program,
-    // on 2026-10-19, after 4.6 added `ui_hydratable`, `ui_router_server` and
-    // `http_listen_on`.
-    assert_eq!(programs.len(), 156, "{programs:#?}");
+    // 8 examples, 151 top-level runtime programs and the modules program,
+    // on 2026-10-21, after 4.7 added `fs_rename`, `crypto_pbkdf2`,
+    // `crypto_pbkdf2_refused` and `bytes_hex`.
+    assert_eq!(programs.len(), 160, "{programs:#?}");
     let mut files = 0;
     let mut failures: Vec<String> = Vec::new();
     for (k, path) in programs.iter().enumerate() {
@@ -271,9 +271,9 @@ fn every_name_in_the_corpora_is_indexed() {
             every_name_is_covered(&a, file, &mut failures);
         }
     }
-    // 8 + 147 + 3 files of user code, 05-geometry's library's 2, and
+    // 8 + 151 + 3 files of user code, 05-geometry's library's 2, and
     // native std's 22, `std/ui/ssr` among them.
-    assert_eq!(files, 182);
+    assert_eq!(files, 186);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/crates/nova-mir/tests/lower_tests.rs b/crates/nova-mir/tests/lower_tests.rs
index 2d1b61e871d14672c9a112a37f468057361cbcce..993687381f26d2297dc52d9d0ab7bdf84d96e5a6 100644
--- a/crates/nova-mir/tests/lower_tests.rs
+++ b/crates/nova-mir/tests/lower_tests.rs
@@ -1221,7 +1221,8 @@ fn http_parse_request_lowers_to_a_runtime_call() {
     );
 }
 
-/// `crypto_hash`, `crypto_random_bytes` and `crypto_random_int` are all
+/// `crypto_hash`, `crypto_random_bytes`, `crypto_random_int`,
+/// `crypto_pbkdf2_sha256` and `crypto_pbkdf2_sha256_verify` are all
 /// `STD_ONLY` (`crates/nova-resolver/src/lib.rs`), so pinning that a Nova call
 /// reaches each one's `RtFunc` needs the same `resolve_program`/`extra_std`
 /// machinery as `http_parse_request_lowers_to_a_runtime_call` above — read
@@ -1236,14 +1237,15 @@ fn http_parse_request_lowers_to_a_runtime_call() {
 /// diagnostic, the call in `main` would resolve to the real function
 /// instead, and this test would keep passing while pinning nothing. That is
 /// why each stub below is named `lower_tests_crypto_*_stub` rather than
-/// `hash`, `random_bytes` or `random_int` — the names `std/crypto` itself is
-/// expected to export.
+/// `hash`, `random_bytes`, `random_int`, `pbkdf2_sha256` or
+/// `pbkdf2_sha256_verify` — the names `std/crypto` itself is expected to
+/// export.
 ///
-/// Three separate `resolve_program` calls, each under its own `$test.*` key,
-/// rather than one wrapper module holding all three stubs: the point of the
+/// Separate `resolve_program` calls, each under its own `$test.*` key,
+/// rather than one wrapper module holding all the stubs: the point of the
 /// synthetic module is to isolate this test from everything unrelated to the
-/// one builtin it is checking, and folding all three into one module would
-/// let a change to one intrinsic's wrapper affect the others' resolution.
+/// one builtin it is checking, and folding them into one module would let a
+/// change to one intrinsic's wrapper affect the others' resolution.
 #[test]
 fn crypto_builtins_reach_their_runtime_functions() {
     use nova_mir::{RtFunc, Stmt};
@@ -1333,6 +1335,30 @@ fn crypto_builtins_reach_their_runtime_functions() {
         "lower_tests_crypto_random_int_stub",
         RtFunc::CryptoRandomInt,
     );
+
+    assert_stub_reaches(
+        "$test.crypto_pbkdf2_sha256_stub",
+        "pub fn lower_tests_crypto_pbkdf2_sha256_stub(p: Bytes, s: Bytes, n: Int, len: Int) -> Int { crypto_pbkdf2_sha256(p, s, n, len) }\n",
+        "fn main() {\n\
+             let p = bytes_from_ints([])\n\
+             let s = bytes_from_ints([])\n\
+             let _ = lower_tests_crypto_pbkdf2_sha256_stub(p, s, 1, 32)\n\
+         }",
+        "lower_tests_crypto_pbkdf2_sha256_stub",
+        RtFunc::CryptoPbkdf2Sha256,
+    );
+
+    assert_stub_reaches(
+        "$test.crypto_pbkdf2_sha256_verify_stub",
+        "pub fn lower_tests_crypto_pbkdf2_sha256_verify_stub(p: Bytes, s: Bytes, n: Int, e: Bytes) -> Int { crypto_pbkdf2_sha256_verify(p, s, n, e) }\n",
+        "fn main() {\n\
+             let p = bytes_from_ints([])\n\
+             let s = bytes_from_ints([])\n\
+             let _ = lower_tests_crypto_pbkdf2_sha256_verify_stub(p, s, 1, p)\n\
+         }",
+        "lower_tests_crypto_pbkdf2_sha256_verify_stub",
+        RtFunc::CryptoPbkdf2Sha256Verify,
+    );
 }
 
 /// How many resume states `<name>`'s poll function dispatches between; 1 when
diff --git a/crates/nova-runtime/src/crypto.rs b/crates/nova-runtime/src/crypto.rs
index 4c0908d7fc8fcf498d028374f4d19657758fe289..ba1a1353d8e8bc70fbef0ecff55d8c22e59ca69a 100644
--- a/crates/nova-runtime/src/crypto.rs
+++ b/crates/nova-runtime/src/crypto.rs
@@ -443,6 +443,61 @@ mod tests {
         assert!(!verify_tag(&key, b"Hi There", &[]), "empty tag");
     }
 
+    /// RFC 7914 section 11's two PBKDF2-HMAC-SHA-256 vectors, each 64
+    /// bytes. Both are also asserted from Nova by
+    /// `tests/runtime/crypto_pbkdf2.nova`.
+    #[test]
+    fn pbkdf2_matches_rfc_7914s_vectors() {
+        let one = iterations_of(1).expect("1 is a valid count");
+        assert_eq!(
+            hex(&derive_key(b"passwd", b"salt", one, 64)),
+            "55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc\
+             49ca9cccf179b645991664b39d77ef317c71b845b1e30bd509112041d3a19783"
+        );
+        let many = iterations_of(80_000).expect("80,000 is a valid count");
+        assert_eq!(
+            hex(&derive_key(b"Password", b"NaCl", many, 64)),
+            "4ddcd8f60b98be21830cee5ef22701f9641a4418d04c0414aeff08876b34ab56\
+             a1d425a1225833549adb841b51c9b3176a272bdebba1d078478f62b397f33c8d"
+        );
+    }
+
+    #[test]
+    fn pbkdf2_verify_accepts_its_key_and_rejects_others() {
+        let rounds = iterations_of(2).expect("2 is a valid count");
+        let key = derive_key(b"pw", b"salt", rounds, 32);
+        assert!(verify_key(b"pw", b"salt", rounds, &key));
+        assert!(
+            !verify_key(b"pW", b"salt", rounds, &key),
+            "another password"
+        );
+        assert!(!verify_key(b"pw", b"salT", rounds, &key), "another salt");
+        let other = iterations_of(3).expect("3 is a valid count");
+        assert!(!verify_key(b"pw", b"salt", other, &key), "another count");
+        // A key is derived of `expected`'s own length, and PBKDF2's shorter
+        // output is its longer output's start, so the start of the key
+        // matches: `std/crypto`'s doc tells a caller to check the length.
+        assert!(
+            verify_key(b"pw", b"salt", rounds, &key[..31]),
+            "the key's start"
+        );
+        let mut longer = key.clone();
+        longer.push(0);
+        assert!(!verify_key(b"pw", b"salt", rounds, &longer), "a longer key");
+        assert!(!verify_key(b"pw", b"salt", rounds, &[]), "an empty key");
+    }
+
+    #[test]
+    fn pbkdf2_iteration_counts_outside_u32_are_refused() {
+        assert!(iterations_of(0).is_none());
+        assert!(iterations_of(-1).is_none());
+        assert!(iterations_of(i64::from(u32::MAX) + 1).is_none());
+        assert_eq!(
+            iterations_of(i64::from(u32::MAX)).map(NonZeroU32::get),
+            Some(u32::MAX)
+        );
+    }
+
     fn hex(v: &[u8]) -> String {
         let mut s = String::with_capacity(v.len() * 2);
         for b in v {
diff --git a/tests/runtime/bytes_hex.nova b/tests/runtime/bytes_hex.nova
new file mode 100644
index 0000000000000000000000000000000000000000..0bf4e2e04e63e6ca384db5a18e4b58cfd20d91d3
--- /dev/null
+++ b/tests/runtime/bytes_hex.nova
@@ -0,0 +1,47 @@
+// `std/bytes`'s hex (spec
+// docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md §5),
+// on both targets: `Bytes::to_hex`, two lower-case digits per byte, and
+// `bytes_from_hex`, which reads either case back, and refuses an odd length
+// or any character that is not a hex digit.
+
+fn shown(b: Option<Bytes>) -> String {
+    match b {
+        Some(bytes) => "Some(${bytes.to_hex()})"
+        None => "None"
+    }
+}
+
+fn main() {
+    println("empty: [${bytes_from_string("").to_hex()}]")
+    println(
+        "every nibble: ${bytes_from_ints([0, 1, 9, 10, 15, 16, 127, 128, 160, 254, 255]).to_hex()}",
+    )
+    println("text: ${bytes_from_string("Nova ✓").to_hex()}")
+    let mut all = [0; 256]
+    let mut i = 0
+    while i < 256 {
+        all[i] = i
+        i = i + 1
+    }
+    let every = bytes_from_ints(all)
+    let written = every.to_hex()
+    println(
+        "256 bytes: ${written.len()} digits, round trip ${bytes_from_hex(written).unwrap().eq(every)}",
+    )
+    println("upper case read: ${bytes_from_hex(written.to_upper()).unwrap().eq(every)}")
+
+    println("from empty: ${shown(bytes_from_hex(""))}")
+    println("lower: ${shown(bytes_from_hex("00ff7f"))}")
+    println("upper: ${shown(bytes_from_hex("ABCDEF"))}")
+    println("mixed: ${shown(bytes_from_hex("aBcD"))}")
+    println("odd length: ${shown(bytes_from_hex("abc"))}")
+    println("one digit: ${shown(bytes_from_hex("a"))}")
+    println("g: ${shown(bytes_from_hex("0g"))}")
+    println("G: ${shown(bytes_from_hex("G0"))}")
+    println("space: ${shown(bytes_from_hex("00 ff"))}")
+    println("leading 0x: ${shown(bytes_from_hex("0x00"))}")
+    println("minus: ${shown(bytes_from_hex("-1"))}")
+    println("non-ascii: ${shown(bytes_from_hex("é0"))}")
+    println("bad last digit: ${shown(bytes_from_hex("00f:"))}")
+    println("bad first digit: ${shown(bytes_from_hex("/0"))}")
+}
diff --git a/tests/runtime/bytes_hex.stdout b/tests/runtime/bytes_hex.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..d458790d462700be9521da96342c61b780217592
--- /dev/null
+++ b/tests/runtime/bytes_hex.stdout
@@ -0,0 +1,19 @@
+empty: []
+every nibble: 0001090a0f107f80a0feff
+text: 4e6f766120e29c93
+256 bytes: 512 digits, round trip true
+upper case read: true
+from empty: Some()
+lower: Some(00ff7f)
+upper: Some(abcdef)
+mixed: Some(abcd)
+odd length: None
+one digit: None
+g: None
+G: None
+space: None
+leading 0x: None
+minus: None
+non-ascii: None
+bad last digit: None
+bad first digit: None
diff --git a/tests/runtime/crypto_pbkdf2.nova b/tests/runtime/crypto_pbkdf2.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4656ba21f773eb0a34bf0ea1bb422a15f8692cf3
--- /dev/null
+++ b/tests/runtime/crypto_pbkdf2.nova
@@ -0,0 +1,43 @@
+// `std/crypto`'s PBKDF2-HMAC-SHA-256 (spec
+// docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md §5),
+// over `ring::pbkdf2`: RFC 7914 section 11's two PBKDF2-HMAC-SHA-256
+// vectors, each 64 bytes, written with `Bytes::to_hex`; a shorter key, which
+// is the longer one's start; and `pbkdf2_sha256_verify`, true for the key
+// and for its start, and false for another password, salt or count, a
+// longer key, and an empty one.
+//
+// **Provenance:** both vectors were cross-checked against Python's
+// `hashlib.pbkdf2_hmac` before being written down; RFC 7914 publishes them,
+// and this project has verified the cross-check, not the document.
+// `crypto_pbkdf2_refused.nova` holds the panics.
+
+fn main() {
+    let first = pbkdf2_sha256(bytes_from_string("passwd"), bytes_from_string("salt"), 1, 64)
+    println("passwd/salt c=1: ${first.to_hex()}")
+    let second = pbkdf2_sha256(bytes_from_string("Password"), bytes_from_string("NaCl"), 80000, 64)
+    println("Password/NaCl c=80000: ${second.to_hex()}")
+    let short = pbkdf2_sha256(bytes_from_string("passwd"), bytes_from_string("salt"), 1, 20)
+    println("20 bytes: ${short.to_hex()}, the start of the 64: ${first.slice(0, 20).eq(short)}")
+    let one = pbkdf2_sha256(bytes_from_string(""), bytes_from_string(""), 1, 1)
+    println("1 byte of empty inputs: ${one.to_hex()}")
+
+    let salt = bytes_from_ints([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16])
+    let key = pbkdf2_sha256(bytes_from_string("correct horse"), salt, 1000, 32)
+    let pw = bytes_from_string("correct horse")
+    println("verify the key: ${pbkdf2_sha256_verify(pw, salt, 1000, key)}")
+    println(
+        "verify the 64-byte vector: ${pbkdf2_sha256_verify(bytes_from_string("Password"), bytes_from_string("NaCl"), 80000, second)}",
+    )
+    println(
+        "another password: ${pbkdf2_sha256_verify(bytes_from_string("correct horsE"), salt, 1000, key)}",
+    )
+    println("another salt: ${pbkdf2_sha256_verify(pw, salt.slice(0, 15), 1000, key)}")
+    println("another count: ${pbkdf2_sha256_verify(pw, salt, 999, key)}")
+    // `ring` derives a key of `expected`'s own length, and a shorter key is
+    // the longer one's start, so a key cut short still verifies.
+    println("the key's first 31 bytes: ${pbkdf2_sha256_verify(pw, salt, 1000, key.slice(0, 31))}")
+    println(
+        "a key one byte long: ${pbkdf2_sha256_verify(pw, salt, 1000, key.concat(bytes_from_ints([0])))}",
+    )
+    println("an empty key: ${pbkdf2_sha256_verify(pw, salt, 1000, bytes_from_string(""))}")
+}
diff --git a/tests/runtime/crypto_pbkdf2.stdout b/tests/runtime/crypto_pbkdf2.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..8f3b2434ccdad15e345c1752a7dbc9fc317c1eea
--- /dev/null
+++ b/tests/runtime/crypto_pbkdf2.stdout
@@ -0,0 +1,12 @@
+passwd/salt c=1: 55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc49ca9cccf179b645991664b39d77ef317c71b845b1e30bd509112041d3a19783
+Password/NaCl c=80000: 4ddcd8f60b98be21830cee5ef22701f9641a4418d04c0414aeff08876b34ab56a1d425a1225833549adb841b51c9b3176a272bdebba1d078478f62b397f33c8d
+20 bytes: 55ac046e56e3089fec1691c22544b605f9418521, the start of the 64: true
+1 byte of empty inputs: f7
+verify the key: true
+verify the 64-byte vector: true
+another password: false
+another salt: false
+another count: false
+the key's first 31 bytes: true
+a key one byte long: false
+an empty key: false
diff --git a/tests/runtime/crypto_pbkdf2_refused.nova b/tests/runtime/crypto_pbkdf2_refused.nova
new file mode 100644
index 0000000000000000000000000000000000000000..77c2f05796a5a1acc92d653691c9591d8c5ce626
--- /dev/null
+++ b/tests/runtime/crypto_pbkdf2_refused.nova
@@ -0,0 +1,26 @@
+// The panics of `std/crypto`'s PBKDF2 (spec
+// docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md §5):
+// an iteration count below 1 or above 4,294,967,295, for either function,
+// and a length below 1 or above 65,536 for `pbkdf2_sha256`. The first
+// argument names the case, and `run_tests.rs` runs each, checking the panic's
+// text and that nothing after it ran; with no argument, the largest values
+// allowed run and print.
+
+fn main() {
+    let pw = bytes_from_string("pw")
+    let salt = bytes_from_string("salt")
+    let case = args().get(1).unwrap_or("")
+    if case == "zero" { let _ = pbkdf2_sha256(pw, salt, 0, 32) }
+    if case == "negative" { let _ = pbkdf2_sha256(pw, salt, -1, 32) }
+    if case == "too-many" { let _ = pbkdf2_sha256(pw, salt, 4294967296, 32) }
+    if case == "no-length" { let _ = pbkdf2_sha256(pw, salt, 1, 0) }
+    if case == "negative-length" { let _ = pbkdf2_sha256(pw, salt, 1, -32) }
+    if case == "too-long" { let _ = pbkdf2_sha256(pw, salt, 1, 65537) }
+    if case == "verify-zero" { let _ = pbkdf2_sha256_verify(pw, salt, 0, pw) }
+    if case == "verify-too-many" { let _ = pbkdf2_sha256_verify(pw, salt, 4294967296, pw) }
+    if case == "" {
+        println("65536 bytes: ${pbkdf2_sha256(pw, salt, 1, 65536).len()}")
+        println("1 byte: ${pbkdf2_sha256(pw, salt, 1, 1).len()}")
+    }
+    println("not reached for a refused case")
+}
diff --git a/tests/runtime/fs_rename.nova b/tests/runtime/fs_rename.nova
new file mode 100644
index 0000000000000000000000000000000000000000..a3a42f93804f5d19fc25d55346f54ba8b733b774
--- /dev/null
+++ b/tests/runtime/fs_rename.nova
@@ -0,0 +1,66 @@
+// `std/fs`'s `rename` (spec
+// docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md §5),
+// in a directory of fixed name under `temp_dir()`, which `run_tests.rs`
+// points at a directory of the test's own: a file written whole under a
+// temporary name, then renamed over an existing one, which it replaces; a
+// rename to a name that is free; and the errors, `NotFound` for a missing
+// source and for a missing directory to hold the target.
+
+fn kind(e: IoError) -> String {
+    match e.kind {
+        NotFound => "NotFound"
+        PermissionDenied => "PermissionDenied"
+        AlreadyExists => "AlreadyExists"
+        _ => "another kind"
+    }
+}
+
+async fn show(path: String) -> String {
+    match read_to_string(path).await {
+        Ok(s) => s
+        Err(e) => "<${kind(e)}>"
+    }
+}
+
+async fn main() {
+    let dir = "${temp_dir()}/nova-fs-rename"
+    let _ = remove_dir_all(dir).await
+    create_dir_all(dir).await.unwrap()
+    let target = "${dir}/blog.json"
+    let temporary = "${dir}/blog.json.tmp"
+
+    write_string(target, "{\"posts\": 1}").await.unwrap()
+    write_string(temporary, "{\"posts\": 2}").await.unwrap()
+    match rename(temporary, target).await {
+        Ok(_) => println("renamed over the old file")
+        Err(e) => println("rename failed: ${kind(e)} ${e.message}")
+    }
+    println("target holds: ${show(target).await}")
+    println("temporary still there: ${exists(temporary).await}")
+
+    let moved = "${dir}/moved.json"
+    match rename(target, moved).await {
+        Ok(_) => println("renamed to a free name")
+        Err(e) => println("rename failed: ${kind(e)} ${e.message}")
+    }
+    println("moved holds: ${show(moved).await}, old name there: ${exists(target).await}")
+
+    match rename("${dir}/missing.json", target).await {
+        Ok(_) => println("a missing source renamed")
+        Err(e) => println("missing source: ${kind(e)}, message empty: ${e.message == ""}")
+    }
+    match rename(moved, "${dir}/no/such/dir/blog.json").await {
+        Ok(_) => println("renamed into a missing directory")
+        Err(e) => println(
+            "missing directory: ${kind(e)}, moved still there: ${exists(moved).await}",
+        )
+    }
+    let entries = read_dir(dir).await.unwrap()
+    let mut names = ""
+    let mut i = 0
+    while i < entries.len() {
+        names = "${names} ${entries[i].name}"
+        i = i + 1
+    }
+    println("left in the directory:${names}")
+}
diff --git a/tests/runtime/fs_rename.stdout b/tests/runtime/fs_rename.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..068b940dda4268b5eb4c1b437f6f39919ce291e6
--- /dev/null
+++ b/tests/runtime/fs_rename.stdout
@@ -0,0 +1,8 @@
+renamed over the old file
+target holds: {"posts": 2}
+temporary still there: false
+renamed to a free name
+moved holds: {"posts": 2}, old name there: false
+missing source: NotFound, message empty: false
+missing directory: NotFound, moved still there: true
+left in the directory: moved.json
````

**Patch `t1-code`:**

<!-- patch t1-code -->

````diff
diff --git a/crates/nova-mir/src/lib.rs b/crates/nova-mir/src/lib.rs
index b3595164247037b5928b3d79886889b9567e0a94..a7ede53b2c88bcb52111497ca815ad81180ac155 100644
--- a/crates/nova-mir/src/lib.rs
+++ b/crates/nova-mir/src/lib.rs
@@ -295,6 +295,10 @@ rt_funcs! {
     /// `(str) -> i64` — remove the directory and everything under it.
     /// Status code, as `FsReadToString`.
     FsRemoveDirAll,
+    /// `(str, str) -> i64` — rename the file named by the first string to
+    /// the second, replacing it when it exists. Status code, as
+    /// `FsReadToString`.
+    FsRename,
     /// `(str) -> i64` — list the directory's entry names, sorted. Status
     /// code, as `FsReadToString`; on success the names are waiting in
     /// `FsTakeStringArray`.
@@ -507,6 +511,15 @@ rt_funcs! {
     /// negative, naming one of `crypto.rs`'s `ERR_*` kinds
     /// (`crates/nova-runtime/src/crypto.rs`).
     CryptoRandomInt,
+    /// `(ptr, ptr, i64, i64) -> i64` — `std/crypto`'s PBKDF2-HMAC-SHA-256
+    /// intrinsic. Status word, not the key: `0` on success, with the key
+    /// waiting in `FsTakeBytes`; otherwise negative, naming one of
+    /// `crypto.rs`'s `ERR_*` kinds (`crates/nova-runtime/src/crypto.rs`).
+    CryptoPbkdf2Sha256,
+    /// `(ptr, ptr, i64, ptr) -> i64` — PBKDF2-HMAC-SHA-256's constant-time
+    /// check: `0` for a match, `1` for a mismatch (an answer, not an error),
+    /// negative for an iteration count out of range.
+    CryptoPbkdf2Sha256Verify,
     /// `() -> i64` — nanoseconds since the runtime's process epoch.
     TimeNowNanos,
     /// `() -> i64` — nanoseconds since the **Unix** epoch. Distinct from
@@ -597,6 +610,7 @@ impl RtFunc {
             RtFunc::FsCreateDirAll => "nova_rt_fs_create_dir_all",
             RtFunc::FsRemoveFile => "nova_rt_fs_remove_file",
             RtFunc::FsRemoveDirAll => "nova_rt_fs_remove_dir_all",
+            RtFunc::FsRename => "nova_rt_fs_rename",
             RtFunc::FsReadDir => "nova_rt_fs_read_dir",
             RtFunc::FsTakeStringArray => "nova_rt_fs_take_string_array",
             RtFunc::FsKind => "nova_rt_fs_kind",
@@ -636,6 +650,8 @@ impl RtFunc {
             RtFunc::CryptoHash => "nova_rt_crypto_hash",
             RtFunc::CryptoRandomBytes => "nova_rt_crypto_random_bytes",
             RtFunc::CryptoRandomInt => "nova_rt_crypto_random_int",
+            RtFunc::CryptoPbkdf2Sha256 => "nova_rt_crypto_pbkdf2_sha256",
+            RtFunc::CryptoPbkdf2Sha256Verify => "nova_rt_crypto_pbkdf2_sha256_verify",
             RtFunc::TimeNowNanos => "nova_rt_time_now_nanos",
             RtFunc::TimeNowEpochNanos => "nova_rt_time_now_epoch_nanos",
             RtFunc::LogConfigLevel => "nova_rt_log_config_level",
@@ -691,7 +707,7 @@ impl RtFunc {
             RtFunc::TaskJoinFuture => (vec![MirTy::Ptr], MirTy::Ptr),
             RtFunc::TaskTimeoutFuture => (vec![MirTy::I64, MirTy::Ptr], MirTy::Ptr),
             RtFunc::FsReadToString => (vec![MirTy::Ptr], MirTy::I64),
-            RtFunc::FsWriteString => (vec![MirTy::Ptr, MirTy::Ptr], MirTy::I64),
+            RtFunc::FsWriteString | RtFunc::FsRename => (vec![MirTy::Ptr, MirTy::Ptr], MirTy::I64),
             RtFunc::FsTakeString
             | RtFunc::FsLastErrorMessage
             | RtFunc::FsTempDir
@@ -832,6 +848,14 @@ impl RtFunc {
             ),
             RtFunc::CryptoRandomBytes => (vec![MirTy::I64], MirTy::I64),
             RtFunc::CryptoRandomInt => (vec![MirTy::I64, MirTy::I64], MirTy::I64),
+            RtFunc::CryptoPbkdf2Sha256 => (
+                vec![MirTy::Ptr, MirTy::Ptr, MirTy::I64, MirTy::I64],
+                MirTy::I64,
+            ),
+            RtFunc::CryptoPbkdf2Sha256Verify => (
+                vec![MirTy::Ptr, MirTy::Ptr, MirTy::I64, MirTy::Ptr],
+                MirTy::I64,
+            ),
             RtFunc::TimeNowNanos => (vec![], MirTy::I64),
             RtFunc::TimeNowEpochNanos => (vec![], MirTy::I64),
             RtFunc::LogConfigLevel => (vec![], MirTy::I64),
@@ -978,6 +1002,7 @@ impl RtFunc {
             | RtFunc::FsCreateDirAll
             | RtFunc::FsRemoveFile
             | RtFunc::FsRemoveDirAll
+            | RtFunc::FsRename
             | RtFunc::FsReadDir
             | RtFunc::FsTakeStringArray
             | RtFunc::FsKind
@@ -1006,6 +1031,8 @@ impl RtFunc {
             | RtFunc::CryptoHash
             | RtFunc::CryptoRandomBytes
             | RtFunc::CryptoRandomInt
+            | RtFunc::CryptoPbkdf2Sha256
+            | RtFunc::CryptoPbkdf2Sha256Verify
             | RtFunc::ShutdownRequested
             | RtFunc::ProcessArgCount
             | RtFunc::ProcessArg
diff --git a/crates/nova-mir/src/lower.rs b/crates/nova-mir/src/lower.rs
index 240c702704f9cff91b7226bc6cdff8e451759f67..5479f41773a98ed7ec734d8992e88123719b8613 100644
--- a/crates/nova-mir/src/lower.rs
+++ b/crates/nova-mir/src/lower.rs
@@ -737,6 +737,7 @@ impl<'a> Lowerer<'a> {
                     Builtin::FsCreateDirAll => Lowering::Runtime(RtFunc::FsCreateDirAll),
                     Builtin::FsRemoveFile => Lowering::Runtime(RtFunc::FsRemoveFile),
                     Builtin::FsRemoveDirAll => Lowering::Runtime(RtFunc::FsRemoveDirAll),
+                    Builtin::FsRename => Lowering::Runtime(RtFunc::FsRename),
                     Builtin::FsReadDir => Lowering::Runtime(RtFunc::FsReadDir),
                     Builtin::FsTakeStringArray => Lowering::Runtime(RtFunc::FsTakeStringArray),
                     Builtin::FsKind => Lowering::Runtime(RtFunc::FsKind),
@@ -778,6 +779,10 @@ impl<'a> Lowerer<'a> {
                     Builtin::CryptoHash => Lowering::Runtime(RtFunc::CryptoHash),
                     Builtin::CryptoRandomBytes => Lowering::Runtime(RtFunc::CryptoRandomBytes),
                     Builtin::CryptoRandomInt => Lowering::Runtime(RtFunc::CryptoRandomInt),
+                    Builtin::CryptoPbkdf2Sha256 => Lowering::Runtime(RtFunc::CryptoPbkdf2Sha256),
+                    Builtin::CryptoPbkdf2Sha256Verify => {
+                        Lowering::Runtime(RtFunc::CryptoPbkdf2Sha256Verify)
+                    }
                     Builtin::TimeNowNanos => Lowering::Runtime(RtFunc::TimeNowNanos),
                     Builtin::TimeNowEpochNanos => Lowering::Runtime(RtFunc::TimeNowEpochNanos),
                     Builtin::LogConfigLevel => Lowering::Runtime(RtFunc::LogConfigLevel),
diff --git a/crates/nova-resolver/src/lib.rs b/crates/nova-resolver/src/lib.rs
index 2482334084add76e3ba95a1b40977125eadeaea6..f3df01270d5370c04f28e633ed9b071ccfe2a0f1 100644
--- a/crates/nova-resolver/src/lib.rs
+++ b/crates/nova-resolver/src/lib.rs
@@ -360,6 +360,11 @@ builtins! {
     /// as [`Builtin::FsReadToString`]. Backs `std/fs`'s `remove_dir_all`.
     /// Std-only.
     FsRemoveDirAll,
+    /// `fs_rename(from: String, to: String) -> Int` — rename the file
+    /// `from` to `to`, replacing `to` when it exists, on Windows too. A
+    /// status code, for the same reason as [`Builtin::FsReadToString`].
+    /// Backs `std/fs`'s `rename`. Std-only.
+    FsRename,
     /// `fs_read_dir(path: String) -> Int` — list `path`'s entry names,
     /// sorted. A status code, for the same reason as
     /// [`Builtin::FsReadToString`]; on success the names are waiting in
@@ -854,6 +859,31 @@ builtins! {
     /// `nova_rt_crypto_random_int`
     /// (`crates/nova-runtime/src/crypto.rs`). Std-only.
     CryptoRandomInt,
+    /// `crypto_pbkdf2_sha256(password: Bytes, salt: Bytes, iterations: Int,
+    /// length: Int) -> Int` — `std/crypto`'s PBKDF2-HMAC-SHA-256 intrinsic,
+    /// `ring::pbkdf2::derive`. Nova has no way to reach a vetted
+    /// implementation from source, and one written in Nova runs far too slow
+    /// for the iteration counts a password needs, so this crosses into Rust.
+    ///
+    /// **The return is a status word, not the key.** `0` on success, with
+    /// the `length` derived bytes waiting in [`Builtin::FsTakeBytes`];
+    /// otherwise **negative**, naming one of `crypto.rs`'s `ERR_*` kinds:
+    /// `iterations` outside `1..=u32::MAX`, or `length` outside
+    /// `1..=65,536`. `std/crypto` turns either into a panic. Runtime symbol
+    /// `nova_rt_crypto_pbkdf2_sha256`
+    /// (`crates/nova-runtime/src/crypto.rs`). Std-only.
+    CryptoPbkdf2Sha256,
+    /// `crypto_pbkdf2_sha256_verify(password: Bytes, salt: Bytes,
+    /// iterations: Int, expected: Bytes) -> Int` — `ring::pbkdf2::verify`,
+    /// whose comparison is constant-time (inherited, not demonstrated here).
+    ///
+    /// **The return is a status word.** `0` when `expected` is the key
+    /// derived from the password, `1` when it is not (an answer, not an
+    /// error, as [`Builtin::CryptoHash`]'s tag check gives it), and negative
+    /// for `iterations` outside `1..=u32::MAX`, which `std/crypto` turns into
+    /// a panic. Runtime symbol `nova_rt_crypto_pbkdf2_sha256_verify`
+    /// (`crates/nova-runtime/src/crypto.rs`). Std-only.
+    CryptoPbkdf2Sha256Verify,
     /// `time_now_nanos() -> Int` — nanoseconds since the runtime's single
     /// process epoch, monotonic and never negative.
     ///
@@ -1026,6 +1056,7 @@ impl Builtin {
             Builtin::FsCreateDirAll => "fs_create_dir_all",
             Builtin::FsRemoveFile => "fs_remove_file",
             Builtin::FsRemoveDirAll => "fs_remove_dir_all",
+            Builtin::FsRename => "fs_rename",
             Builtin::FsReadDir => "fs_read_dir",
             Builtin::FsTakeStringArray => "fs_take_string_array",
             Builtin::FsKind => "fs_kind",
@@ -1071,6 +1102,8 @@ impl Builtin {
             Builtin::CryptoHash => "crypto_hash",
             Builtin::CryptoRandomBytes => "crypto_random_bytes",
             Builtin::CryptoRandomInt => "crypto_random_int",
+            Builtin::CryptoPbkdf2Sha256 => "crypto_pbkdf2_sha256",
+            Builtin::CryptoPbkdf2Sha256Verify => "crypto_pbkdf2_sha256_verify",
             Builtin::TimeNowNanos => "time_now_nanos",
             Builtin::TimeNowEpochNanos => "time_now_epoch_nanos",
             Builtin::LogConfigLevel => "log_config_level",
@@ -1126,7 +1159,7 @@ impl Builtin {
     /// consecutive review rounds (see the Phase 2.2b whole-branch review),
     /// because the roster is duplicated information that only this array
     /// needs to stay exact.
-    pub const STD_ONLY: [Builtin; 87] = [
+    pub const STD_ONLY: [Builtin; 90] = [
         Builtin::StrCmp,
         Builtin::StrHash,
         Builtin::CharToInt,
@@ -1160,6 +1193,7 @@ impl Builtin {
         Builtin::FsCreateDirAll,
         Builtin::FsRemoveFile,
         Builtin::FsRemoveDirAll,
+        Builtin::FsRename,
         Builtin::FsReadDir,
         Builtin::FsTakeStringArray,
         Builtin::FsKind,
@@ -1199,6 +1233,8 @@ impl Builtin {
         Builtin::CryptoHash,
         Builtin::CryptoRandomBytes,
         Builtin::CryptoRandomInt,
+        Builtin::CryptoPbkdf2Sha256,
+        Builtin::CryptoPbkdf2Sha256Verify,
         Builtin::TimeNowNanos,
         Builtin::TimeNowEpochNanos,
         Builtin::LogConfigLevel,
diff --git a/crates/nova-runtime/src/crypto.rs b/crates/nova-runtime/src/crypto.rs
index ba1a1353d8e8bc70fbef0ecff55d8c22e59ca69a..70bcc0995bc10766e16c4d304542bfe373e9653d 100644
--- a/crates/nova-runtime/src/crypto.rs
+++ b/crates/nova-runtime/src/crypto.rs
@@ -1,5 +1,6 @@
 //! `std/crypto`'s intrinsics: SHA-256, SHA-512, HMAC-SHA-256 and its
-//! constant-time check, plus random bytes and a bounded random integer.
+//! constant-time check, PBKDF2-HMAC-SHA-256 and its constant-time check,
+//! plus random bytes and a bounded random integer.
 //!
 //! # The boundary
 //!
@@ -25,7 +26,8 @@ use crate::bytes::{as_bytes, gc_bytes};
 use crate::fs::{stash, Slot};
 use crate::NovaStr;
 use ring::rand::SecureRandom;
-use ring::{digest, hmac, rand};
+use ring::{digest, hmac, pbkdf2, rand};
+use std::num::NonZeroU32;
 
 /// Which operation `nova_rt_crypto_hash` performs.
 ///
@@ -71,6 +73,14 @@ const ERR_ENTROPY_UNAVAILABLE: i64 = 1;
 const ERR_INVALID_LENGTH: i64 = 2;
 const ERR_REQUEST_TOO_LARGE: i64 = 3;
 const ERR_INVALID_RANGE: i64 = 4;
+/// PBKDF2's iteration count is outside `1..=u32::MAX`, the range
+/// `ring::pbkdf2` takes as a `NonZeroU32`. Returned by the two PBKDF2
+/// intrinsics only, which `std/crypto` turns into a panic rather than a
+/// `CryptoErrorKind`.
+const ERR_INVALID_ITERATIONS: i64 = 5;
+/// PBKDF2's output length is outside `1..=MAX_PBKDF2_BYTES`. Returned by
+/// `nova_rt_crypto_pbkdf2_sha256` only, and turned into a panic the same way.
+const ERR_INVALID_OUTPUT_LENGTH: i64 = 6;
 
 /// The largest `random_bytes` request served.
 ///
@@ -83,6 +93,15 @@ const ERR_INVALID_RANGE: i64 = 4;
 /// other platform allocations still leak until exit.
 const MAX_RANDOM_BYTES: usize = 64 * 1024;
 
+/// The longest key `nova_rt_crypto_pbkdf2_sha256` derives, the same cap as
+/// [`MAX_RANDOM_BYTES`] and for the same reason: it is checked before the
+/// output is allocated, so no length a Nova program passes can ask for an
+/// allocation the process cannot make. It also keeps the length far below
+/// `ring::pbkdf2::derive`'s own limit, `u32::MAX` blocks of 32 bytes, past
+/// which that function unwraps an error -- so `derive` cannot fail for
+/// anything this intrinsic hands it. A password hash is 32 bytes.
+const MAX_PBKDF2_BYTES: usize = 64 * 1024;
+
 /// Status for the tag-check operation when the tag does not match.
 ///
 /// Not an error: a mismatch is the answer the caller asked for.
@@ -212,6 +231,117 @@ pub unsafe extern "C" fn nova_rt_crypto_hash(
     }
 }
 
+/// The iteration count `iterations`, as `ring::pbkdf2` takes it, or `None`
+/// when it is outside `1..=u32::MAX`.
+fn iterations_of(iterations: i64) -> Option<NonZeroU32> {
+    NonZeroU32::new(u32::try_from(iterations).ok()?)
+}
+
+/// The PBKDF2-HMAC-SHA-256 key of `length` bytes derived from `password`
+/// and `salt` over `iterations` rounds.
+///
+/// `ring::pbkdf2::derive` returns nothing and unwraps its errors inside:
+/// a secret or salt near 2^61 bytes, which no Nova `Bytes` reaches, or an
+/// output longer than `u32::MAX` blocks, which `MAX_PBKDF2_BYTES` rules out
+/// before this is called.
+fn derive_key(password: &[u8], salt: &[u8], iterations: NonZeroU32, length: usize) -> Vec<u8> {
+    let mut out = vec![0u8; length];
+    pbkdf2::derive(
+        pbkdf2::PBKDF2_HMAC_SHA256,
+        iterations,
+        salt,
+        password,
+        &mut out,
+    );
+    out
+}
+
+/// Whether `expected` is the PBKDF2-HMAC-SHA-256 key derived from
+/// `password` and `salt` over `iterations` rounds.
+///
+/// `ring::pbkdf2::verify` compares in constant time, inherited here and not
+/// demonstrated by any test in this crate; an empty `expected` is a
+/// mismatch, never an error.
+fn verify_key(password: &[u8], salt: &[u8], iterations: NonZeroU32, expected: &[u8]) -> bool {
+    pbkdf2::verify(
+        pbkdf2::PBKDF2_HMAC_SHA256,
+        iterations,
+        salt,
+        password,
+        expected,
+    )
+    .is_ok()
+}
+
+/// PBKDF2-HMAC-SHA-256 (RFC 8018 section 5.2) through `ring`: the key of
+/// `length` bytes derived from `password` and `salt` over `iterations`
+/// rounds, into `Slot::Buffer`.
+///
+/// Returns `0`, or `-ERR_INVALID_ITERATIONS` for `iterations` outside
+/// `1..=u32::MAX` and `-ERR_INVALID_OUTPUT_LENGTH` for `length` outside
+/// `1..=MAX_PBKDF2_BYTES`, each checked before anything is derived or
+/// allocated. It runs for as long as `iterations` says, on the calling
+/// thread: measured on 2026-10-21, about 0.5 ms per thousand rounds for a
+/// 32-byte key with `ring` built optimized, and about 4.4 ms in this crate's
+/// debug build.
+///
+/// # Safety
+/// `password` and `salt` must each be a valid, non-null `NovaStr` pointer,
+/// which is what the compiler emits for a `Bytes` argument.
+#[no_mangle]
+pub unsafe extern "C" fn nova_rt_crypto_pbkdf2_sha256(
+    password: *const NovaStr,
+    salt: *const NovaStr,
+    iterations: i64,
+    length: i64,
+) -> i64 {
+    let Some(rounds) = iterations_of(iterations) else {
+        return -ERR_INVALID_ITERATIONS;
+    };
+    let want = match usize::try_from(length) {
+        Ok(n) if (1..=MAX_PBKDF2_BYTES).contains(&n) => n,
+        _ => return -ERR_INVALID_OUTPUT_LENGTH,
+    };
+    // SAFETY: forwarding this function's own contract.
+    let p = unsafe { as_bytes(password) };
+    let s = unsafe { as_bytes(salt) };
+    stash(Slot::Buffer, gc_bytes(&derive_key(p, s, rounds, want)));
+    0
+}
+
+/// PBKDF2-HMAC-SHA-256's constant-time check of `expected` against the key
+/// derived from `password` and `salt` over `iterations` rounds, of
+/// `expected`'s own length.
+///
+/// Returns `0` for a match, `TAG_MISMATCH` for a mismatch, an empty
+/// `expected` included, and `-ERR_INVALID_ITERATIONS` for `iterations`
+/// outside `1..=u32::MAX`.
+///
+/// # Safety
+/// `password`, `salt` and `expected` must each be a valid, non-null
+/// `NovaStr` pointer, which is what the compiler emits for a `Bytes`
+/// argument.
+#[no_mangle]
+pub unsafe extern "C" fn nova_rt_crypto_pbkdf2_sha256_verify(
+    password: *const NovaStr,
+    salt: *const NovaStr,
+    iterations: i64,
+    expected: *const NovaStr,
+) -> i64 {
+    let Some(rounds) = iterations_of(iterations) else {
+        return -ERR_INVALID_ITERATIONS;
+    };
+    // SAFETY: forwarding this function's own contract.
+    let p = unsafe { as_bytes(password) };
+    let s = unsafe { as_bytes(salt) };
+    let e = unsafe { as_bytes(expected) };
+    if verify_key(p, s, rounds, e) {
+        0
+    } else {
+        TAG_MISMATCH
+    }
+}
+
 /// `n` random bytes into `Slot::Buffer`.
 ///
 /// # Safety
diff --git a/crates/nova-runtime/src/fs.rs b/crates/nova-runtime/src/fs.rs
index a1464b85d0f4ebaeba752e2f44ddb9f96ec6eaf1..be969665dc9cdb5c790fd56ea28132e2a1e3422b 100644
--- a/crates/nova-runtime/src/fs.rs
+++ b/crates/nova-runtime/src/fs.rs
@@ -62,7 +62,8 @@ use std::cell::RefCell;
 /// from a second Rust source.** Per kind, now:
 /// - `NotFound`: `fs_not_found.nova` (`read_to_string` on a missing path),
 ///   and independently `file_errors.nova` (`open` on a path under a missing
-///   parent directory).
+///   parent directory) and `fs_rename.nova` (`rename` from a missing path,
+///   and into a missing directory).
 /// - `AlreadyExists`: `fs_already_exists.nova` (`create_dir` on an existing
 ///   path, twice), and independently `file_errors.nova` (`open` with
 ///   `create_new` on an existing path).
@@ -670,6 +671,27 @@ pub unsafe extern "C" fn nova_rt_fs_remove_dir_all(path: *const NovaStr) -> i64
     }
 }
 
+/// Rename the file `from` to `to`, replacing `to` when it exists.
+///
+/// `std::fs::rename` replaces an existing `to` on every system this project
+/// supports: on Unix it is `rename(2)`, which does so atomically, and on
+/// Windows Rust's standard library asks for the replacement itself (it
+/// passes `MOVEFILE_REPLACE_EXISTING`, or renames with POSIX semantics where
+/// the system has them), so `std/fs`'s `rename` needs nothing of its own
+/// there. Errors take the kinds every other operation here maps (`fail`).
+///
+/// # Safety
+/// Both arguments must point to live `NovaStr`s.
+#[no_mangle]
+pub unsafe extern "C" fn nova_rt_fs_rename(from: *const NovaStr, to: *const NovaStr) -> i64 {
+    // SAFETY: forwarding this function's own contract.
+    let (f, t) = unsafe { (crate::as_str(from), crate::as_str(to)) };
+    match std::fs::rename(f, t) {
+        Ok(()) => OK,
+        Err(e) => fail(&e),
+    }
+}
+
 /// List `path`'s entry names, sorted. On `OK` the names are waiting in
 /// `nova_rt_fs_take_string_array`.
 ///
diff --git a/crates/nova-runtime/src/lib.rs b/crates/nova-runtime/src/lib.rs
index 2fea72e1b0578dd67b16dba1fa4bd595e7fb3dae..55af2cd05458c9728f473417a45f4b93a09318b0 100644
--- a/crates/nova-runtime/src/lib.rs
+++ b/crates/nova-runtime/src/lib.rs
@@ -1096,6 +1096,7 @@ pub fn symbols() -> Vec<(&'static str, *const u8)> {
             "nova_rt_fs_remove_dir_all",
             fs::nova_rt_fs_remove_dir_all as *const u8,
         ),
+        ("nova_rt_fs_rename", fs::nova_rt_fs_rename as *const u8),
         ("nova_rt_fs_read_dir", fs::nova_rt_fs_read_dir as *const u8),
         (
             "nova_rt_fs_take_string_array",
@@ -1207,6 +1208,14 @@ pub fn symbols() -> Vec<(&'static str, *const u8)> {
             "nova_rt_crypto_random_int",
             crypto::nova_rt_crypto_random_int as *const u8,
         ),
+        (
+            "nova_rt_crypto_pbkdf2_sha256",
+            crypto::nova_rt_crypto_pbkdf2_sha256 as *const u8,
+        ),
+        (
+            "nova_rt_crypto_pbkdf2_sha256_verify",
+            crypto::nova_rt_crypto_pbkdf2_sha256_verify as *const u8,
+        ),
         (
             "nova_rt_time_now_nanos",
             time::nova_rt_time_now_nanos as *const u8,
diff --git a/crates/nova-typeck/src/check.rs b/crates/nova-typeck/src/check.rs
index 44dffe9b84fb304f10cd9b1c28c8297e85dcf0ed..39a929de23a18415eedd38e8b3cc86d711753c14 100644
--- a/crates/nova-typeck/src/check.rs
+++ b/crates/nova-typeck/src/check.rs
@@ -4224,6 +4224,7 @@ impl<'a> Checker<'a> {
             | Builtin::FsCreateDirAll
             | Builtin::FsRemoveFile
             | Builtin::FsRemoveDirAll
+            | Builtin::FsRename
             | Builtin::FsReadDir
             | Builtin::FsTakeStringArray
             | Builtin::FsKind
@@ -4263,6 +4264,8 @@ impl<'a> Checker<'a> {
             | Builtin::CryptoHash
             | Builtin::CryptoRandomBytes
             | Builtin::CryptoRandomInt
+            | Builtin::CryptoPbkdf2Sha256
+            | Builtin::CryptoPbkdf2Sha256Verify
             | Builtin::TimeNowNanos
             | Builtin::TimeNowEpochNanos
             | Builtin::LogConfigLevel
@@ -8057,6 +8060,7 @@ pub(crate) fn builtin_signature(builtin: Builtin) -> (Vec<Ty>, Ty) {
         Builtin::FsCreateDirAll => (vec![Ty::String], Ty::Int),
         Builtin::FsRemoveFile => (vec![Ty::String], Ty::Int),
         Builtin::FsRemoveDirAll => (vec![Ty::String], Ty::Int),
+        Builtin::FsRename => (vec![Ty::String, Ty::String], Ty::Int),
         Builtin::FsReadDir => (vec![Ty::String], Ty::Int),
         Builtin::FsTakeStringArray => (vec![], Ty::Array(Box::new(Ty::String))),
         Builtin::FsKind => (vec![Ty::String], Ty::Int),
@@ -8144,6 +8148,10 @@ pub(crate) fn builtin_signature(builtin: Builtin) -> (Vec<Ty>, Ty) {
         Builtin::CryptoHash => (vec![Ty::Int, Ty::Bytes, Ty::Bytes, Ty::Bytes], Ty::Int),
         Builtin::CryptoRandomBytes => (vec![Ty::Int], Ty::Int),
         Builtin::CryptoRandomInt => (vec![Ty::Int, Ty::Int], Ty::Int),
+        Builtin::CryptoPbkdf2Sha256 => (vec![Ty::Bytes, Ty::Bytes, Ty::Int, Ty::Int], Ty::Int),
+        Builtin::CryptoPbkdf2Sha256Verify => {
+            (vec![Ty::Bytes, Ty::Bytes, Ty::Int, Ty::Bytes], Ty::Int)
+        }
         Builtin::TimeNowNanos => (vec![], Ty::Int),
         Builtin::TimeNowEpochNanos => (vec![], Ty::Int),
         Builtin::LogConfigLevel => (vec![], Ty::Int),
@@ -16140,7 +16148,7 @@ mod tests {
                     (vec![], Ty::String),
                     "`fs_last_error_message()` in every fallible `std/fs` wrapper: \
                      `read_to_string`, `write_string`, `create_dir`, \
-                     `create_dir_all`, `remove_file`, `remove_dir_all`, `read_dir`",
+                     `create_dir_all`, `remove_file`, `remove_dir_all`, `rename`, `read_dir`",
                 ),
                 Builtin::FsTempDir => (
                     (vec![], Ty::String),
@@ -16166,6 +16174,10 @@ mod tests {
                     (vec![Ty::String], Ty::Int),
                     "`fs_remove_dir_all(path)` in `std/fs`'s `remove_dir_all`",
                 ),
+                Builtin::FsRename => (
+                    (vec![Ty::String, Ty::String], Ty::Int),
+                    "`fs_rename(from, to)` in `std/fs`'s `rename`",
+                ),
                 Builtin::FsReadDir => (
                     (vec![Ty::String], Ty::Int),
                     "`fs_read_dir(path)` in `std/fs`'s `read_dir`",
@@ -16351,6 +16363,15 @@ mod tests {
                     (vec![Ty::Int, Ty::Int], Ty::Int),
                     "`crypto_random_int(min, max)` in `std/crypto`",
                 ),
+                Builtin::CryptoPbkdf2Sha256 => (
+                    (vec![Ty::Bytes, Ty::Bytes, Ty::Int, Ty::Int], Ty::Int),
+                    "`crypto_pbkdf2_sha256(password, salt, iterations, length)` in `std/crypto`",
+                ),
+                Builtin::CryptoPbkdf2Sha256Verify => (
+                    (vec![Ty::Bytes, Ty::Bytes, Ty::Int, Ty::Bytes], Ty::Int),
+                    "`crypto_pbkdf2_sha256_verify(password, salt, iterations, expected)` in \
+                     `std/crypto`",
+                ),
                 Builtin::TimeNowNanos => (
                     (vec![], Ty::Int),
                     "`time_now_nanos()` in `std/time`'s `Instant::now`",
diff --git a/std/bytes/lib.nova b/std/bytes/lib.nova
index e5b4587d1ad4a70b3b9ce5d824aea1e973517be0..120d29552250981b16edc52ffedb23db709c42c1 100644
--- a/std/bytes/lib.nova
+++ b/std/bytes/lib.nova
@@ -5,8 +5,9 @@
 //! `bytes_from_string` and `bytes_from_ints` make a `Bytes`, and its methods
 //! measure, index, slice, join and search it, all counting in bytes; none
 //! changes it. `to_string` turns bytes that are valid UTF-8 back into a
-//! `String`. `Bytes` implements `Eq`: `a.eq(b)` compares two buffers byte for
-//! byte.
+//! `String`, and `to_hex` writes any bytes as hex, which `bytes_from_hex`
+//! reads back. `Bytes` implements `Eq`: `a.eq(b)` compares two buffers byte
+//! for byte.
 //!
 //! Making `Bytes` from a string and from integers, measuring, slicing,
 //! joining and searching them, and turning them back into a `String`:
@@ -121,6 +122,67 @@ impl Bytes {
     /// Returns `true` if `needle` occurs in these bytes. An empty `needle`
     /// always does.
     pub fn contains(self, needle: Bytes) -> Bool { self.index_of(needle).is_some() }
+
+    // Hex in Nova, on both targets (spec
+    // docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md
+    // §5): two characters per byte need no intrinsic, and a key or a tag is
+    // tens of bytes.
+    /// Returns the bytes written in hexadecimal, two lower-case digits per
+    /// byte, most significant first: `0x0a 0xff` is `"0aff"`. Empty bytes
+    /// give `""`. `bytes_from_hex` reads it back.
+    ///
+    /// ```
+    /// assert_eq(bytes_from_ints([0, 10, 255]).to_hex(), "000aff")
+    /// assert_eq(bytes_from_string("hi").to_hex(), "6869")
+    /// ```
+    pub fn to_hex(self) -> String {
+        let digits = "0123456789abcdef".chars()
+        let bs = self.to_ints()
+        let mut out = [""; bs.len()]
+        let mut i = 0
+        while i < bs.len() {
+            out[i] = "${digits[bs[i] / 16]}${digits[bs[i] % 16]}"
+            i = i + 1
+        }
+        "".join(out)
+    }
+}
+
+// Hex read back, in Nova, as `Bytes::to_hex` writes it.
+/// Returns the bytes `s` writes in hexadecimal, two digits per byte, most
+/// significant first, as `Bytes::to_hex` writes them; the digits `a` to `f`
+/// may be in either case. Returns `None` when `s` has an odd number of
+/// characters, or holds any character other than a hex digit, a space or a
+/// `0x` included. `""` gives empty bytes.
+///
+/// ```
+/// assert(bytes_from_hex("000aFF").unwrap().eq(bytes_from_ints([0, 10, 255])), "either case")
+/// assert(bytes_from_hex("abc").is_none(), "an odd length")
+/// assert(bytes_from_hex("0g").is_none(), "not a hex digit")
+/// ```
+pub fn bytes_from_hex(s: String) -> Option<Bytes> {
+    // Each byte of `s`'s UTF-8 is checked, so a character outside ASCII,
+    // whose bytes are all 0x80 or more, is refused like any other.
+    let cs = bytes_from_string(s).to_ints()
+    if cs.len() % 2 != 0 { return None }
+    let mut out = [0; cs.len() / 2]
+    let mut i = 0
+    while i < out.len() {
+        let hi = hex_digit_value(cs[2 * i])
+        let lo = hex_digit_value(cs[2 * i + 1])
+        if hi < 0 || lo < 0 { return None }
+        out[i] = hi * 16 + lo
+        i = i + 1
+    }
+    Some(bytes_from_ints(out))
+}
+
+// The value of the hex digit whose ASCII code is `b`, in either case, or -1.
+fn hex_digit_value(b: Int) -> Int {
+    if b >= 48 && b <= 57 { return b - 48 }
+    if b >= 65 && b <= 70 { return b - 55 }
+    if b >= 97 && b <= 102 { return b - 87 }
+    0 - 1
 }
 
 // A `Bytes` holding `s`'s UTF-8 bytes.
diff --git a/std/crypto/lib.nova b/std/crypto/lib.nova
index f54af77d30e29b80aa0f92bc3deae00763789262..d6df0d630dfdfabc3275f67f671f9f9cb905998c 100644
--- a/std/crypto/lib.nova
+++ b/std/crypto/lib.nova
@@ -1,11 +1,13 @@
 //! Hashes, message authentication codes and secure random values.
 //!
 //! `sha256` and `sha512` hash `Bytes`, `hmac_sha256` computes an HMAC-SHA-256
-//! tag, and `hmac_sha256_verify` checks one. `random_bytes` and `random_int`
-//! draw from the operating system's cryptographically secure random source.
-//! Hashes and tags are raw `Bytes`: nothing here encodes them as hex or
-//! base64. Every module sees these names without an import, and a module's
-//! own definition of one of them shadows the one here.
+//! tag, and `hmac_sha256_verify` checks one. `pbkdf2_sha256` derives a key
+//! from a password, for storing a password's hash, and
+//! `pbkdf2_sha256_verify` checks a password against one. `random_bytes` and
+//! `random_int` draw from the operating system's cryptographically secure
+//! random source. Hashes and tags are raw `Bytes`: `Bytes::to_hex` writes
+//! them as hex. Every module sees these names without an import, and a
+//! module's own definition of one of them shadows the one here.
 //!
 //! There is no encryption, no hash other than SHA-256 and SHA-512, and no way
 //! to hash input that arrives in pieces: each function takes all of its
@@ -34,7 +36,8 @@
 //! assert_eq(random_bytes(16).unwrap().len(), 16)
 //! ```
 
-// `std/crypto`: hashes, HMAC and random, over three runtime intrinsics.
+// `std/crypto`: hashes, HMAC, PBKDF2 and random, over five runtime
+// intrinsics.
 //
 // **Digest lengths are documented, not typed.** `nova-spec/20-STDLIB.md`
 // section 8 declares these as `[u8; 32]` and `[u8; 64]`; neither is
@@ -148,6 +151,87 @@ pub fn hmac_sha256_verify(key: Bytes, data: Bytes, tag: Bytes) -> Bool {
     crypto_hash(OP_HMAC_SHA256_VERIFY, key, data, tag) == 0
 }
 
+// PBKDF2-HMAC-SHA-256 (spec
+// docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md §5),
+// over `ring::pbkdf2`: written in Nova it ran about 0.84 s per 100,000
+// iterations, during which a single-threaded server answers nothing. The
+// runtime checks the iteration count and the length, and returns these two
+// statuses, which `crates/nova-runtime/src/crypto.rs` defines as well; each
+// is a caller's error, so a panic, as `bytes_from_ints` panics on a value
+// out of range, rather than a `CryptoError` every call would have to unwrap.
+const PBKDF2_BAD_ITERATIONS: Int = -5
+
+const PBKDF2_BAD_LENGTH: Int = -6
+
+// The panics both functions share, for the statuses above.
+fn pbkdf2_refused(name: String, status: Int, iterations: Int, length: Int) {
+    if status == PBKDF2_BAD_ITERATIONS {
+        panic("${name}: iterations must be from 1 to 4294967295, not ${iterations}")
+    }
+    if status == PBKDF2_BAD_LENGTH {
+        panic("${name}: length must be from 1 to 65536, not ${length}")
+    }
+    panic("${name}: the runtime refused it with status ${status}")
+}
+
+/// Returns the key of `length` bytes that PBKDF2-HMAC-SHA-256 (RFC 8018)
+/// derives from `password` and `salt` over `iterations` rounds.
+///
+/// It is made for storing a password: keep the key, the salt, which should
+/// be random and differ from one password to the next, as from
+/// `random_bytes(16)`, and the count, and check a password later with
+/// `pbkdf2_sha256_verify`. Each round makes the key costlier to guess:
+/// OWASP recommends 600,000 for a password. It runs on the calling task for
+/// as long as the count says, and every other task waits.
+///
+/// Panics when `iterations` is below 1 or above 4,294,967,295, with
+/// `pbkdf2_sha256: iterations must be from 1 to 4294967295, not <n>`, and
+/// when `length` is below 1 or above 65,536, with
+/// `pbkdf2_sha256: length must be from 1 to 65536, not <n>`.
+///
+/// ```
+/// // RFC 7914's first PBKDF2-HMAC-SHA-256 vector, its first 32 bytes.
+/// let key = pbkdf2_sha256(bytes_from_string("passwd"), bytes_from_string("salt"), 1, 32)
+/// assert_eq(key.to_hex(), "55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc")
+/// ```
+pub fn pbkdf2_sha256(password: Bytes, salt: Bytes, iterations: Int, length: Int) -> Bytes {
+    let status = crypto_pbkdf2_sha256(password, salt, iterations, length)
+    if status != 0 { pbkdf2_refused("pbkdf2_sha256", status, iterations, length) }
+    fs_take_bytes()
+}
+
+// Constant-time, and that property is INHERITED from `ring::pbkdf2::verify`
+// rather than demonstrated here, as `hmac_sha256_verify`'s is.
+/// Returns `true` if `expected` is the key PBKDF2-HMAC-SHA-256 derives from
+/// `password` and `salt` over `iterations` rounds, of `expected`'s own
+/// length, as `pbkdf2_sha256` derives it.
+///
+/// The comparison is made in constant time, as `ring`'s is, so how long it
+/// takes does not reveal how much of a wrong key matched. An empty
+/// `expected` gives `false`. A shorter key is the start of a longer one, so
+/// the start of the right key gives `true` too: keep and check the whole
+/// key, and its length.
+///
+/// Panics when `iterations` is below 1 or above 4,294,967,295, with
+/// `pbkdf2_sha256_verify: iterations must be from 1 to 4294967295, not <n>`.
+///
+/// ```
+/// let salt = random_bytes(16).unwrap()
+/// let stored = pbkdf2_sha256(bytes_from_string("correct horse"), salt, 1000, 32)
+/// assert(pbkdf2_sha256_verify(bytes_from_string("correct horse"), salt, 1000, stored), "the password")
+/// assert(!pbkdf2_sha256_verify(bytes_from_string("wrong horse"), salt, 1000, stored), "another")
+/// ```
+pub fn pbkdf2_sha256_verify(
+    password: Bytes,
+    salt: Bytes,
+    iterations: Int,
+    expected: Bytes,
+) -> Bool {
+    let status = crypto_pbkdf2_sha256_verify(password, salt, iterations, expected)
+    if status < 0 { pbkdf2_refused("pbkdf2_sha256_verify", status, iterations, expected.len()) }
+    status == 0
+}
+
 // `n` cryptographically random bytes.
 //
 // Fallible where section 8 declared it infallible: the OS entropy source can
diff --git a/std/fs/lib.nova b/std/fs/lib.nova
index 8ff907c338fffc56030a1cb97df649995e006709..1f3d037977ac45eb30bd5e8f699570ebbecbff7f 100644
--- a/std/fs/lib.nova
+++ b/std/fs/lib.nova
@@ -4,8 +4,9 @@
 //!
 //! Paths are `String`s, and a relative path is resolved against the process's
 //! current directory. `read_to_string` and `write_string` move a whole file
-//! as text, and `read` and `write` as `Bytes`. `open` returns a `File`, which
-//! reads and writes through `std/io`'s `Read` and `Write`. Failures are
+//! as text, and `read` and `write` as `Bytes`. `rename` moves a file into
+//! place, replacing what is there. `open` returns a `File`, which reads and
+//! writes through `std/io`'s `Read` and `Write`. Failures are
 //! `std/io`'s `IoError`s. Every module sees these names without an import,
 //! and a module's own definition of one of them shadows the one here.
 //!
@@ -243,6 +244,46 @@ pub async fn remove_dir_all(path: String) -> Result<(), IoError> {
     Err(IoError { kind: io_error_kind_of(status), message: fs_last_error_message() })
 }
 
+// Rename the file `from` to `to`, replacing `to` (spec
+// docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md §5):
+// a whole file written beside its target, then renamed over it, so the
+// target is never half written.
+/// Renames the file `from` to `to`, replacing the file at `to` when there is
+/// one, on Windows as on other systems.
+///
+/// Writing a file whole under another name, then renaming it over the one
+/// it replaces, leaves the old file or the new one at `to`, never half of
+/// one, should the program stop in between; on Unix the replacement itself
+/// is atomic. Returns an `IoError` of kind `NotFound` if nothing exists at
+/// `from` or the directory to hold `to` does not exist, and
+/// `PermissionDenied` if the system denies the rename. Renaming to another
+/// file system, or a file over a directory, fails with a kind that depends
+/// on the system.
+///
+/// An `async fn` that never suspends: it blocks every task until it
+/// finishes.
+///
+/// ```
+/// async fn main() {
+///     let dir = "${temp_dir()}/nova-std-fs-rename"
+///     create_dir_all(dir).await.unwrap()
+///     let path = "${dir}/data.json"
+///     write_string(path, "{\"n\": 1}").await.unwrap()
+///
+///     // The new contents, whole, beside the old, then moved over them.
+///     write_string("${path}.tmp", "{\"n\": 2}").await.unwrap()
+///     rename("${path}.tmp", path).await.unwrap()
+///     assert_eq(read_to_string(path).await.unwrap(), "{\"n\": 2}")
+///     assert(!exists("${path}.tmp").await, "the temporary name is gone")
+///     # remove_dir_all(dir).await.unwrap()
+/// }
+/// ```
+pub async fn rename(from: String, to: String) -> Result<(), IoError> {
+    let status = fs_rename(from, to)
+    if status == 0 { return Ok(()) }
+    Err(IoError { kind: io_error_kind_of(status), message: fs_last_error_message() })
+}
+
 // One entry in a directory listing.
 //
 // `is_file` and `is_dir` come from a single `fs_kind` call, so they cannot
````


### Task 2: Routes, and reading a request

Spec §3.1, §3.2; decisions 1 to 4, 14 and 18.

**Files:**
- Modify: `std/http/lib.nova` (the routes and the readers)
- Test:
  - `tests/runtime/http_routes.nova`, `http_routes.stdout`
  - `tests/runtime/http_request_readers.nova`, `http_request_readers.stdout`
  - `tests/runtime/http_server_dispatch.nova`, `http_server_dispatch.stdout`
  - `crates/nova-cli/tests/run_tests.rs`, `wasm_tests.rs`
  - `crates/nova-driver/tests/index_complete.rs`

- [ ] **Step 1: The tests.** `patch_block t2-tests`. The patch holds:
  - `tests/runtime/http_routes.nova`, with no socket, through `dispatch`:
    each method, `get`, `post`, `put`, `delete`, and `route` with `PATCH`,
    `PURGE` and `HEAD`; patterns with one and two `:name` pieces and
    `req.params`, a trailing `/`, doubled `/`s, escapes decoded (`a%20b`,
    `a%2Fb`, `%E2%9C%93`, `%70osts`), a bad `%` and bytes that are not UTF-8
    kept as written, and paths of the wrong length or case; a query taking
    no part, and `get("/add")` answering `/add?x=1`; the first route to
    match answering; `405` with its `allow`, each method once and in the
    order added, its type, its length and its status line; the fallback,
    for a `GET` alone, its query kept; `404`; a route method leaving its
    receiver alone; and `parse_request_head`'s empty `params`.
    `http_routes_run` runs it as it is and under stress.
  - `tests/runtime/http_request_readers.nova`: `query`, with no query or
    an empty one, pairs, `+` and escapes, names decoded too, UTF-8, empty
    parts, no `=`, the first `=` splitting, the last of two winning, bad
    escapes and bytes that are not UTF-8 kept, and only the first `?`;
    `form`, for a form, with a charset, in any case, with spaces, the last
    winning, an empty body, and empty for JSON, text, multipart, a longer
    media type, no `content-type` and a body that is not UTF-8, and apart
    from the query; `cookies`, with no header, two, the first of two
    winning, trimmed, a part with no `=` skipped, the first `=` splitting,
    an empty value, values not decoded, and a header from the wire. Each map
    is printed by its sorted keys. `http_request_readers_run`, likewise.
  - `tests/runtime/http_server_dispatch.nova`: `params: Map::new()` in its
    `Request`; `GET /echo?x=1` reaching `/echo`; and `POST /`, which was
    `404`, now `405 allow GET`.
  - `run_tests.rs`: the two tests, and
    `http_server_example_serves_and_exits_cleanly`'s `POST /`, now `405`
    with `allow: GET`.
  - `wasm_tests.rs`: `http_request_readers` and `http_routes` in
    `LEFT_OUT`.
  - `index_complete.rs`: 162 programs and 188 files.
- [ ] **Step 2: Run, and see them fail.** First the build (Conventions).
  - `R http_routes_run`. Expected: `0 passed; 1 failed`: "Unexpected
    failure.": ``error[E0014]: record `Request` has no field `params` ``,
    ``error[E0014]: no field `params` on record `Request` `` and
    ``error[E0014]: no method `post` on type `Server` ``: `could not compile
    due to 9 previous errors`.
  - `R http_request_readers_run`. Expected: `0 passed; 1 failed`: the same
    `params` error, ``no method `form` on type `Request` `` and ``no method
    `cookies` on type `Request` ``: `could not compile due to 31 previous
    errors`.
  - `R http_server_dispatch_run`. Expected: `0 passed; 1 failed`:
    ``error[E0014]: record `Request` has no field `params` ``: `could not
    compile due to 1 previous error`.
  - `R http_server_example_serves_and_exits_cleanly`. Expected: `0 passed;
    1 failed`, at `run_tests.rs:10758`: ``assertion `left == right` failed:
    only GET routes are added``, `left: 404`, `right: 405`.
  - `W every_runtime_program_is_in_the_gate_or_left_out`. Expected: `1
    passed`.
  - `X` (about 8 s). Expected: `0 passed; 1 failed`, at
    `index_complete.rs:259`: "…/tests/runtime/http_request_readers.nova does
    not check cleanly: [\"E0014 record `Request` has no field `params`\",
    …]".
- [ ] **Step 3: The code.** `patch_block t2-code`, `std/http/lib.nova`'s
  routes and readers:
  - `Request.params`, which every `Request` literal in std names; `query`,
    `form` and `cookies`, each with its doc test, over `form_fields`,
    `form_decoded`, `percent_decoded` and `hex_value` (decisions 3, 4 and
    14);
  - `path_pieces`; `Route`, a method and a pattern's pieces; `get`, now on
    a pattern, `post`, `put`, `delete` and `route`, each with its doc;
    `fallback`'s doc; `dispatch`, with `params_of`, `method_name` and
    `holds`, and the `405` (decisions 1, 2 and 18);
  - the module's doc and `Server`'s. `post`'s and `dispatch`'s examples
    answer with `Response::text`, and the module's doc names none of Task
    3's builders, which Task 3 puts back (Checked on).
- [ ] **Step 4: Run.** First the build.
  - The same six commands. Expected: `R http_routes_run` `1 passed` (about
    9 s, the build of `nova` included); `R http_request_readers_run` `1
    passed`; `R http_server_dispatch_run` `1 passed`; `R
    http_server_example_serves_and_exits_cleanly` `1 passed`; `W
    every_runtime_program_is_in_the_gate_or_left_out` `1 passed`; `X` `1
    passed`.
  - The whole binaries:
    - `cargo test -p nova-std`. Expected: `2 passed`.
    - `cargo test -p nova-doc --lib`. Expected: `61 passed`.
    - `cargo test -p nova-cli --test doc_test`. Expected: `17 passed`:
      `std/http`'s new examples run inside
      `doc_std_runs_stds_doc_tests_inside_a_project_or_not`.
    - `R`. Expected: `323 passed; 0 failed; 1 ignored`.
    - `W`. Expected: `230 passed`.
- [ ] **Step 5: The checks,** as the conventions list them, and `cargo run
  --locked -q -p nova-cli -- fmt --check tests/runtime/http_routes.nova
  tests/runtime/http_request_readers.nova`, which prints nothing. Not
  `http_server_dispatch.nova`: `nova fmt` would reformat three places the
  base already had (`status_of`'s one-line body and two long `println`s),
  and the lines this task adds are in its layout.
- [ ] **Step 6: Commit** `std/http: routes by method, with :name patterns and req.params, 405, and query, form and cookies`.

**Patch `t2-tests`:**

<!-- patch t2-tests -->

````diff
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index f71932a8fa0f951830838a717e056c3a20775368..fe39cb0c0fbfd6fae92ed65ed44d0f3a7ceac460 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -9646,6 +9646,24 @@ fn run_with_and_without_stress(name: &str) {
     }
 }
 
+/// `std/http`'s routes by method (spec
+/// `docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md`
+/// §3.1), socket-free: `get`, `post`, `put`, `delete` and `route`, `:name`
+/// patterns and `req.params`, matched on the decoded path without its query,
+/// the first matching route answering, `405` with `allow` naming each method
+/// once in the order added, the fallback for `GET` only, and `404`.
+#[test]
+fn http_routes_run() {
+    run_with_and_without_stress("http_routes");
+}
+
+/// `Request::query`, `form` and its `content-type` rule, and `cookies` (spec
+/// 4.7 §3.2), socket-free.
+#[test]
+fn http_request_readers_run() {
+    run_with_and_without_stress("http_request_readers");
+}
+
 /// `std/process`'s `args()` under `nova run FILE -- ARGS`: index 0 is FILE as
 /// written, and an argument with spaces and an empty one each arrive whole.
 /// Run from the repository root so FILE is a fixed relative path. Design:
@@ -10735,7 +10753,10 @@ fn http_server_example_serves_and_exits_cleanly() {
     // A body is consumed exactly, so the connection stays usable (Review Focus 3).
     k.send("POST / HTTP/1.1\r\nhost: x\r\ncontent-length: 3\r\n\r\nabc");
     let post = http03_response(&mut server, &mut k, "POST / with a body");
-    assert_eq!(post.status, 404, "only GET routes ship");
+    // `/` is a `GET` route's path, so since 4.7 a `POST` to it is `405`
+    // (spec docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md §3.1).
+    assert_eq!(post.status, 405, "only GET routes are added");
+    assert_eq!(post.headers.get("allow").map(String::as_str), Some("GET"));
     k.send("GET / HTTP/1.1\r\nhost: x\r\n\r\n");
     let again = http03_response(&mut server, &mut k, "GET / after a body");
     assert_eq!(again.body, "Hello from Nova!");
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index 8e4b2314bb9901b7dcad92b7d1ea9b95ca730bac..6dc580d35f590e982a1caa5607826e867f9f67a3 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -2803,6 +2803,8 @@ const LEFT_OUT: &[(&str, &str)] = &[
     ("http_malformed", "std/http"),
     ("http_offsets", "std/http"),
     ("http_partial", "std/http"),
+    ("http_request_readers", "std/http"),
+    ("http_routes", "std/http"),
     ("http_serialise", "std/http"),
     ("http_listen_on", "std/http, std/net, std/process"),
     ("http_server_dispatch", "std/http"),
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index b0140141fc1130ea140172efbd6f00a484bb4a9e..bed3d271f8fd45f54a73117aa48163ffce21672b 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -241,10 +241,11 @@ fn every_name_is_covered(a: &Analysis, file: FileId, failures: &mut Vec<String>)
 #[test]
 fn every_name_in_the_corpora_is_indexed() {
     let programs = programs();
-    // 8 examples, 151 top-level runtime programs and the modules program,
+    // 8 examples, 153 top-level runtime programs and the modules program,
     // on 2026-10-21, after 4.7 added `fs_rename`, `crypto_pbkdf2`,
-    // `crypto_pbkdf2_refused` and `bytes_hex`.
-    assert_eq!(programs.len(), 160, "{programs:#?}");
+    // `crypto_pbkdf2_refused` and `bytes_hex`, then `http_routes` and
+    // `http_request_readers`.
+    assert_eq!(programs.len(), 162, "{programs:#?}");
     let mut files = 0;
     let mut failures: Vec<String> = Vec::new();
     for (k, path) in programs.iter().enumerate() {
@@ -271,9 +272,9 @@ fn every_name_in_the_corpora_is_indexed() {
             every_name_is_covered(&a, file, &mut failures);
         }
     }
-    // 8 + 151 + 3 files of user code, 05-geometry's library's 2, and
+    // 8 + 153 + 3 files of user code, 05-geometry's library's 2, and
     // native std's 22, `std/ui/ssr` among them.
-    assert_eq!(files, 186);
+    assert_eq!(files, 188);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/tests/runtime/http_request_readers.nova b/tests/runtime/http_request_readers.nova
new file mode 100644
index 0000000000000000000000000000000000000000..00c7597c702bda920a13973b704c21f60c428aea
--- /dev/null
+++ b/tests/runtime/http_request_readers.nova
@@ -0,0 +1,126 @@
+// `std/http`'s readers of a request (spec
+// docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md
+// §3.2): `query` and `form`, which share one decoder (`+` a space, `%XX` as
+// UTF-8, a half that does not decode kept as written, an empty part
+// skipped, a part with no `=` an empty value, the last of two names
+// winning), `form`'s `content-type` rule, and `cookies` (split on `;`, each
+// part trimmed, a part with no `=` skipped, the first of two names winning,
+// values not decoded). Each map is printed by its keys in sorted order, so
+// a `Map`'s own order never shows.
+
+fn target(t: String) -> Request {
+    Request {
+        method: Get,
+        path: t,
+        headers: Map::new(),
+        body: bytes_from_string(""),
+        params: Map::new(),
+    }
+}
+
+fn posted(content_type: Option<String>, body: Bytes) -> Request {
+    let mut h: Map<String, String> = Map::new()
+    match content_type {
+        Some(t) => h.insert("content-type", t)
+        None => None
+    }
+    Request { method: Post, path: "/form", headers: h, body: body, params: Map::new() }
+}
+
+fn with_cookie_header(value: String) -> Request {
+    let mut h: Map<String, String> = Map::new()
+    h.insert("cookie", value)
+    Request { method: Get, path: "/", headers: h, body: bytes_from_string(""), params: Map::new() }
+}
+
+// `m` as `{k=v, …}`, its keys in sorted order.
+fn shown(m: Map<String, String>) -> String {
+    let keys = m.keys()
+    let n = keys.len()
+    // An insertion sort: std has none.
+    let mut sorted = [""; n]
+    let mut i = 0
+    while i < n {
+        let k = keys[i]
+        let mut j = i
+        while j > 0 && after(sorted[j - 1], k) {
+            sorted[j] = sorted[j - 1]
+            j = j - 1
+        }
+        sorted[j] = k
+        i = i + 1
+    }
+    let mut parts = [""; n]
+    let mut p = 0
+    while p < n {
+        parts[p] = "${sorted[p]}=[${m.get(sorted[p]).unwrap_or("?")}]"
+        p = p + 1
+    }
+    "{${", ".join(parts)}}"
+}
+
+// Whether `a` sorts after `b`, by UTF-8 bytes.
+fn after(a: String, b: String) -> Bool {
+    match a.cmp(b) {
+        Greater => true
+        _ => false
+    }
+}
+
+fn form_of(content_type: String, body: String) -> String {
+    shown(posted(Some(content_type), bytes_from_string(body)).form())
+}
+
+fn main() {
+    println("-- query")
+    println("no query: ${shown(target("/search").query())}")
+    println("empty query: ${shown(target("/search?").query())}")
+    println("pairs: ${shown(target("/s?q=nova&page=2").query())}")
+    println("plus and escapes: ${shown(target("/s?q=nova+lang&tag=a%26b%3Dc&sp=%20x%2B").query())}")
+    println("names decode too: ${shown(target("/s?a+b=1&c%3Dd=2").query())}")
+    println("utf-8: ${shown(target("/s?check=%E2%9C%93&raw=✓").query())}")
+    println("empty parts: ${shown(target("/s?&&a=1&&b=2&").query())}")
+    println("no `=`: ${shown(target("/s?flag&x=").query())}")
+    println("first `=` splits: ${shown(target("/s?eq=a=b=c").query())}")
+    println("last of two wins: ${shown(target("/s?tag=a&tag=b&tag=c").query())}")
+    println("bad escape kept: ${shown(target("/s?p=100%&q=%zz%41&r=%4").query())}")
+    println("not utf-8 kept: ${shown(target("/s?b=%FF%FE").query())}")
+    println("only the first `?`: ${shown(target("/s?a=1?b=2").query())}")
+
+    println("-- form")
+    let urlencoded = "application/x-www-form-urlencoded"
+    println("a form: ${form_of(urlencoded, "name=Ada+L.&text=Hi%21+%E2%9C%93&csrf=ab12")}")
+    println("charset: ${form_of("application/x-www-form-urlencoded; charset=UTF-8", "a=1")}")
+    println("any case: ${form_of("Application/X-WWW-Form-URLEncoded", "a=1")}")
+    println("spaces: ${form_of("  application/x-www-form-urlencoded  ;charset=utf-8", "a=1")}")
+    println("last wins: ${form_of(urlencoded, "a=1&a=2&b")}")
+    println("empty body: ${form_of(urlencoded, "")}")
+    println("json: ${form_of("application/json", "a=1")}")
+    println("text: ${form_of("text/plain", "a=1")}")
+    println("multipart: ${form_of("multipart/form-data; boundary=x", "a=1")}")
+    println("a longer type: ${form_of("application/x-www-form-urlencoded-not", "a=1")}")
+    println("no content-type: ${shown(posted(None, bytes_from_string("a=1")).form())}")
+    let not_utf8 = posted(Some(urlencoded), bytes_from_ints([97, 61, 255]))
+    println("body not utf-8: ${shown(not_utf8.form())}")
+    let both = Request {
+        method: Post,
+        path: "/form?from=query",
+        headers: posted(Some(urlencoded), bytes_from_string("")).headers,
+        body: bytes_from_string("from=body"),
+        params: Map::new(),
+    }
+    println("query and form apart: ${shown(both.query())} ${shown(both.form())}")
+
+    println("-- cookies")
+    println("no header: ${shown(target("/").cookies())}")
+    println("two: ${shown(with_cookie_header("sid=abc.123; theme=dark").cookies())}")
+    println("first of two wins: ${shown(with_cookie_header("sid=new; sid=old").cookies())}")
+    println("trimmed: ${shown(with_cookie_header("  a=1 ;b=2;   c=3  ").cookies())}")
+    println("no `=` skipped: ${shown(with_cookie_header("flag; a=1; ;").cookies())}")
+    println("first `=` splits: ${shown(with_cookie_header("tok=a=b").cookies())}")
+    println("empty value: ${shown(with_cookie_header("empty=").cookies())}")
+    println("not decoded: ${shown(with_cookie_header("v=a%20b+c; q=\"x\"").cookies())}")
+    let raw = bytes_from_string("GET / HTTP/1.1\r\nCookie: sid=1; theme=light\r\n\r\n")
+    let parsed = parse_request_head(raw, Limits::default()).unwrap().unwrap()
+    println("from the wire: ${shown(parsed.cookies())}")
+}
diff --git a/tests/runtime/http_request_readers.stdout b/tests/runtime/http_request_readers.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..99de8e419a5f3ba482ac47d637d0b1e20deb5c20
--- /dev/null
+++ b/tests/runtime/http_request_readers.stdout
@@ -0,0 +1,38 @@
+-- query
+no query: {}
+empty query: {}
+pairs: {page=[2], q=[nova]}
+plus and escapes: {q=[nova lang], sp=[ x+], tag=[a&b=c]}
+names decode too: {a b=[1], c=d=[2]}
+utf-8: {check=[✓], raw=[✓]}
+empty parts: {a=[1], b=[2]}
+no `=`: {flag=[], x=[]}
+first `=` splits: {eq=[a=b=c]}
+last of two wins: {tag=[c]}
+bad escape kept: {p=[100%], q=[%zz%41], r=[%4]}
+not utf-8 kept: {b=[%FF%FE]}
+only the first `?`: {a=[1?b=2]}
+-- form
+a form: {csrf=[ab12], name=[Ada L.], text=[Hi! ✓]}
+charset: {a=[1]}
+any case: {a=[1]}
+spaces: {a=[1]}
+last wins: {a=[2], b=[]}
+empty body: {}
+json: {}
+text: {}
+multipart: {}
+a longer type: {}
+no content-type: {}
+body not utf-8: {}
+query and form apart: {from=[query]} {from=[body]}
+-- cookies
+no header: {}
+two: {sid=[abc.123], theme=[dark]}
+first of two wins: {sid=[new]}
+trimmed: {a=[1], b=[2], c=[3]}
+no `=` skipped: {a=[1]}
+first `=` splits: {tok=[a=b]}
+empty value: {empty=[]}
+not decoded: {q=["x"], v=[a%20b+c]}
+from the wire: {sid=[1], theme=[light]}
diff --git a/tests/runtime/http_routes.nova b/tests/runtime/http_routes.nova
new file mode 100644
index 0000000000000000000000000000000000000000..bb9b07bfe4449b9571cd48ef645df4260cc53ab8
--- /dev/null
+++ b/tests/runtime/http_routes.nova
@@ -0,0 +1,163 @@
+// `std/http`'s routes by method (spec
+// docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md
+// §3.1), with no socket: `get`, `post`, `put`, `delete` and `route`; patterns
+// with `:name` and `req.params`, matched on the path before `?`, empty
+// pieces dropped and each piece percent-decoded; the order routes answer
+// in; `405` and its `allow`; the fallback, for `GET` only; and `404`.
+
+fn request(method: Method, target: String) -> Request {
+    Request {
+        method: method,
+        path: target,
+        headers: Map::new(),
+        body: bytes_from_string(""),
+        params: Map::new(),
+    }
+}
+
+fn body_text(r: Response) -> String {
+    match r.body.to_string() {
+        Some(s) => s
+        None => "<not utf-8>"
+    }
+}
+
+fn header(r: Response, name: String) -> String {
+    match r.headers.get(name) {
+        Some(v) => v
+        None => "<absent>"
+    }
+}
+
+// One line for `method` and `target` on `s`: the status, the `allow` header
+// when there is one, and the body.
+fn ask(s: Server, method: Method, target: String) -> String {
+    let r = s.dispatch(request(method, target))
+    let allow = match r.headers.get("allow") {
+        Some(v) => " allow=[${v}]"
+        None => ""
+    }
+    "${r.status}${allow} ${body_text(r)}"
+}
+
+// A handler that names itself and every parameter its request holds, in a
+// fixed order of names.
+fn named(name: String) -> fn(Request) -> Response {
+    |r: Request| {
+        let keys = ["id", "c", "name", "a", "b"]
+        let mut shown = ""
+        let mut i = 0
+        while i < keys.len() {
+            match r.params.get(keys[i]) {
+                Some(v) => shown = "${shown} ${keys[i]}=${v}"
+                None => {}
+            }
+            i = i + 1
+        }
+        Response::text(200, "${name}${shown} (${r.params.len()} params)")
+    }
+}
+
+fn main() {
+    let app = Server::new()
+        .get("/", named("home"))
+        .get("/posts/:id", named("show post"))
+        .post("/posts/:id/comments", named("add comment"))
+        .put("/notes/:name", named("put note"))
+        .delete("/notes/:name", named("delete note"))
+        .route(Patch, "/notes/:name", named("patch note"))
+        .route(Unknown("PURGE"), "/cache", named("purge"))
+        .route(Head, "/health", named("head health"))
+        .get("/admin/posts/:id/edit", named("edit"))
+        .get("/a/:a/b/:b", named("two params"))
+
+    println("-- each method")
+    println("GET / -> ${ask(app, Get, "/")}")
+    println("GET /posts/7 -> ${ask(app, Get, "/posts/7")}")
+    println("POST /posts/7/comments -> ${ask(app, Post, "/posts/7/comments")}")
+    println("PUT /notes/todo -> ${ask(app, Put, "/notes/todo")}")
+    println("DELETE /notes/todo -> ${ask(app, Delete, "/notes/todo")}")
+    println("PATCH /notes/todo -> ${ask(app, Patch, "/notes/todo")}")
+    println("PURGE /cache -> ${ask(app, Unknown("PURGE"), "/cache")}")
+    println("HEAD /health -> ${ask(app, Head, "/health")}")
+
+    println("-- patterns and params")
+    println("GET /a/1/b/2 -> ${ask(app, Get, "/a/1/b/2")}")
+    println("GET /admin/posts/9/edit -> ${ask(app, Get, "/admin/posts/9/edit")}")
+    println("GET /posts/7/ -> ${ask(app, Get, "/posts/7/")}")
+    println("GET //posts//7 -> ${ask(app, Get, "//posts//7")}")
+    println("GET /posts/a%20b -> ${ask(app, Get, "/posts/a%20b")}")
+    println("GET /posts/a%2Fb -> ${ask(app, Get, "/posts/a%2Fb")}")
+    println("GET /posts/100% -> ${ask(app, Get, "/posts/100%")}")
+    println("GET /posts/%E2%9C%93 -> ${ask(app, Get, "/posts/%E2%9C%93")}")
+    println("GET /posts/%FF -> ${ask(app, Get, "/posts/%FF")}")
+    println("GET /%70osts/7 -> ${ask(app, Get, "/%70osts/7")}")
+    println("GET /posts -> ${ask(app, Get, "/posts")}")
+    println("GET /posts/7/edit -> ${ask(app, Get, "/posts/7/edit")}")
+    println("GET /Posts/7 -> ${ask(app, Get, "/Posts/7")}")
+
+    println("-- a query takes no part")
+    println("GET /?ref=x -> ${ask(app, Get, "/?ref=x")}")
+    println("GET /posts/7?tab=2 -> ${ask(app, Get, "/posts/7?tab=2")}")
+    println("GET /posts/7/?a=1/2 -> ${ask(app, Get, "/posts/7/?a=1/2")}")
+    let adder = Server::new().get("/add", |r: Request| Response::text(200, "add ${r.path}"))
+    println("get(\"/add\"): GET /add?x=1 -> ${ask(adder, Get, "/add?x=1")}")
+
+    println("-- the first route to match answers")
+    let order = Server::new()
+        .get("/posts/new", named("new post"))
+        .get("/posts/:id", named("post by id"))
+        .get("/posts/:c", named("never"))
+        .post("/items/:id", named("post item"))
+        .get("/items/:id", named("get item"))
+    println("GET /posts/new -> ${ask(order, Get, "/posts/new")}")
+    println("GET /posts/3 -> ${ask(order, Get, "/posts/3")}")
+    println("GET /items/4 -> ${ask(order, Get, "/items/4")}")
+
+    println("-- 405 and allow")
+    println("POST / -> ${ask(app, Post, "/")}")
+    println("GET /posts/7/comments -> ${ask(app, Get, "/posts/7/comments")}")
+    println("POST /notes/todo -> ${ask(app, Post, "/notes/todo")}")
+    println("GET /cache -> ${ask(app, Get, "/cache")}")
+    println("HEAD / -> ${ask(app, Head, "/")}")
+    println("GET /health -> ${ask(app, Get, "/health")}")
+    let twice = Server::new()
+        .post("/form", named("first post"))
+        .route(Unknown("PURGE"), "/form", named("purge"))
+        .post("/:x", named("second post"))
+        .put("/form", named("put"))
+    println("DELETE /form -> ${ask(twice, Delete, "/form")}")
+    let refused = twice.dispatch(request(Delete, "/form"))
+    println("405 content-type: ${header(refused, "content-type")}")
+    println("405 content-length: ${header(refused, "content-length")}")
+    let wire = refused.to_bytes().to_string().unwrap_or("")
+    println("405 status line: ${wire.slice(0, wire.index_of("\r\n").unwrap_or(0))}")
+
+    println("-- the fallback, for GET only")
+    let fb = app.fallback(
+        |r: Request| Response::text(404, "fallback ${r.path} (${r.params.len()} params)"),
+    )
+    println("GET /nope?x=1 -> ${ask(fb, Get, "/nope?x=1")}")
+    println("POST /nope -> ${ask(fb, Post, "/nope")}")
+    println("HEAD /nope -> ${ask(fb, Head, "/nope")}")
+    println("PUT /posts/7 -> ${ask(fb, Put, "/posts/7")}")
+    println("GET /posts/7 -> ${ask(fb, Get, "/posts/7")}")
+    println("GET /posts/7/comments -> ${ask(fb, Get, "/posts/7/comments")}")
+
+    println("-- 404")
+    println("GET /nope -> ${ask(app, Get, "/nope")}")
+    println("POST /nope -> ${ask(app, Post, "/nope")}")
+    println("empty server: GET / -> ${ask(Server::new(), Get, "/")}")
+
+    println("-- a route method leaves its receiver alone")
+    let base = Server::new().get("/a", named("a"))
+    let more = base.post("/a", named("post a"))
+    println("base: POST /a -> ${ask(base, Post, "/a")}")
+    println("more: POST /a -> ${ask(more, Post, "/a")}")
+
+    println("-- params are the router's")
+    let raw = bytes_from_string("GET /posts/7 HTTP/1.1\r\n\r\n")
+    let parsed = parse_request_head(raw, Limits::default()).unwrap().unwrap()
+    println("parse_request_head: ${parsed.params.len()} params")
+    println("dispatched: ${ask(app, Get, parsed.path)}")
+}
diff --git a/tests/runtime/http_routes.stdout b/tests/runtime/http_routes.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..d13b3dee490241cbba5592f96e718bbafc44a3c7
--- /dev/null
+++ b/tests/runtime/http_routes.stdout
@@ -0,0 +1,60 @@
+-- each method
+GET / -> 200 home (0 params)
+GET /posts/7 -> 200 show post id=7 (1 params)
+POST /posts/7/comments -> 200 add comment id=7 (1 params)
+PUT /notes/todo -> 200 put note name=todo (1 params)
+DELETE /notes/todo -> 200 delete note name=todo (1 params)
+PATCH /notes/todo -> 200 patch note name=todo (1 params)
+PURGE /cache -> 200 purge (0 params)
+HEAD /health -> 200 head health (0 params)
+-- patterns and params
+GET /a/1/b/2 -> 200 two params a=1 b=2 (2 params)
+GET /admin/posts/9/edit -> 200 edit id=9 (1 params)
+GET /posts/7/ -> 200 show post id=7 (1 params)
+GET //posts//7 -> 200 show post id=7 (1 params)
+GET /posts/a%20b -> 200 show post id=a b (1 params)
+GET /posts/a%2Fb -> 200 show post id=a/b (1 params)
+GET /posts/100% -> 200 show post id=100% (1 params)
+GET /posts/%E2%9C%93 -> 200 show post id=✓ (1 params)
+GET /posts/%FF -> 200 show post id=%FF (1 params)
+GET /%70osts/7 -> 200 show post id=7 (1 params)
+GET /posts -> 404 not found
+GET /posts/7/edit -> 404 not found
+GET /Posts/7 -> 404 not found
+-- a query takes no part
+GET /?ref=x -> 200 home (0 params)
+GET /posts/7?tab=2 -> 200 show post id=7 (1 params)
+GET /posts/7/?a=1/2 -> 200 show post id=7 (1 params)
+get("/add"): GET /add?x=1 -> 200 add /add?x=1
+-- the first route to match answers
+GET /posts/new -> 200 new post (0 params)
+GET /posts/3 -> 200 post by id id=3 (1 params)
+GET /items/4 -> 200 get item id=4 (1 params)
+-- 405 and allow
+POST / -> 405 allow=[GET] method not allowed
+GET /posts/7/comments -> 405 allow=[POST] method not allowed
+POST /notes/todo -> 405 allow=[PUT, DELETE, PATCH] method not allowed
+GET /cache -> 405 allow=[PURGE] method not allowed
+HEAD / -> 405 allow=[GET] method not allowed
+GET /health -> 405 allow=[HEAD] method not allowed
+DELETE /form -> 405 allow=[POST, PURGE, PUT] method not allowed
+405 content-type: text/plain; charset=utf-8
+405 content-length: 18
+405 status line: HTTP/1.1 405 Method Not Allowed
+-- the fallback, for GET only
+GET /nope?x=1 -> 404 fallback /nope?x=1 (0 params)
+POST /nope -> 404 not found
+HEAD /nope -> 404 not found
+PUT /posts/7 -> 405 allow=[GET] method not allowed
+GET /posts/7 -> 200 show post id=7 (1 params)
+GET /posts/7/comments -> 405 allow=[POST] method not allowed
+-- 404
+GET /nope -> 404 not found
+POST /nope -> 404 not found
+empty server: GET / -> 404 not found
+-- a route method leaves its receiver alone
+base: POST /a -> 405 allow=[GET] method not allowed
+more: POST /a -> 200 post a (0 params)
+-- params are the router's
+parse_request_head: 0 params
+dispatched: 200 show post id=7 (1 params)
diff --git a/tests/runtime/http_server_dispatch.nova b/tests/runtime/http_server_dispatch.nova
index 0d2d9d4bbd6821116daf972cc6c256d6092c61a6..f2f6841445248dabe6149e03db40c27e1005f7d1 100644
--- a/tests/runtime/http_server_dispatch.nova
+++ b/tests/runtime/http_server_dispatch.nova
@@ -5,6 +5,12 @@
 // §8.3): it answers a GET no route matches, its query included, and nothing
 // else; a route still answers its own path, before or after it; a second one
 // replaces the first; and the server it is called on keeps none.
+//
+// Since 4.7 (spec
+// docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md
+// §3.1) `get` matches the path without its query, so `/echo?x=1` reaches
+// `/echo`'s handler, and a `POST` to `/`, a `GET` route's path, is `405`
+// with `allow: GET` where it was `404`. `http_routes.nova` holds the rest.
 
 fn status_ok() -> JsonValue {
     let mut m: Map<String, JsonValue> = Map::new()
@@ -13,7 +19,13 @@ fn status_ok() -> JsonValue {
 }
 
 fn request(method: Method, path: String) -> Request {
-    Request { method: method, path: path, headers: Map::new(), body: bytes_from_string("") }
+    Request {
+        method: method,
+        path: path,
+        headers: Map::new(),
+        body: bytes_from_string(""),
+        params: Map::new(),
+    }
 }
 
 fn body_text(r: Response) -> String {
@@ -46,10 +58,12 @@ fn main() {
     println("GET /health -> ${b.status} ${header(b, "content-type")} ${header(b, "content-length")} ${body_text(b)}")
     let c = app.dispatch(request(Get, "/echo"))
     println("GET /echo -> ${c.status} ${body_text(c)}")
+    let q = app.dispatch(request(Get, "/echo?x=1"))
+    println("GET /echo?x=1 -> ${q.status} ${body_text(q)}")
     let d = app.dispatch(request(Get, "/nope"))
     println("GET /nope -> ${d.status}")
     let e = app.dispatch(request(Post, "/"))
-    println("POST / -> ${e.status}")
+    println("POST / -> ${e.status} allow ${header(e, "allow")}")
 
     // `get` leaves its receiver alone, so two servers built from one base do
     // not share the routes added to each other.
diff --git a/tests/runtime/http_server_dispatch.stdout b/tests/runtime/http_server_dispatch.stdout
index 0678fbbc667ba30b2affb80dee90506a1dd61892..712b89a7f6a69295c416a6aa65e0e246b6ebd848 100644
--- a/tests/runtime/http_server_dispatch.stdout
+++ b/tests/runtime/http_server_dispatch.stdout
@@ -1,8 +1,9 @@
 GET / -> 200 Hello from Nova!
 GET /health -> 200 application/json 15 {"status":"ok"}
 GET /echo -> 200 /echo
+GET /echo?x=1 -> 200 /echo?x=1
 GET /nope -> 404
-POST / -> 404
+POST / -> 405 allow GET
 s1: /a 200 /b 200 /c 404
 s2: /a 200 /b 404 /c 200
 base: /a 200 /b 404 /c 404
````

**Patch `t2-code`:**

<!-- patch t2-code -->

````diff
diff --git a/std/http/lib.nova b/std/http/lib.nova
index a0df53c90816f863e361a68d809c56eed6f99e3e..41d2e7089cf207b2daf7208e5d892a829ae7d713 100644
--- a/std/http/lib.nova
+++ b/std/http/lib.nova
@@ -1,14 +1,17 @@
 //! An HTTP/1.1 server: requests read and parsed, responses built and written,
-//! and a router that serves GET routes.
+//! and a router that sends each request to a handler by its method and path.
 //!
-//! `Server` is the simple way in: give it a handler for each path with `get`,
-//! and one for every other `GET` with `fallback`, then `listen` on an
-//! address, or `listen_on` a listener already bound. A handler is a plain
+//! `Server` is the simple way in: give it a handler for each method and path
+//! pattern with `get`, `post`, `put`, `delete` or `route`, and one for every
+//! other `GET` with `fallback`, then `listen` on an address, or `listen_on` a
+//! listener already bound. A pattern such as `/posts/:id` puts the piece of
+//! the path it matched in the request's `params`. A handler is a plain
 //! function from a `Request` to a `Response`, so it cannot await anything.
+//! `Request`'s `query`, `form` and `cookies` read what a browser sends.
 //! `read_request`, `write_response` and `parse_request_head` are the parts
-//! underneath, for serving a `std/net` `TcpStream` yourself. There is no HTTP client. Every
-//! module sees these names without an import, and a module's own definition
-//! of one of them shadows the one here.
+//! underneath, for serving a `std/net` `TcpStream` yourself. There is no
+//! HTTP client. Every module sees these names without an import, and a
+//! module's own definition of one of them shadows the one here.
 //!
 //! Only `Content-Length` says how long a request's body is: chunked transfer
 //! encoding is not supported, and a chunked request's body is misread as the
@@ -17,10 +20,12 @@
 //! change from one run of a program to the next.
 //!
 //! `Server::listen` serves until the process is stopped, so this example
-//! has the server `dispatch` a request parsed from bytes, with no socket:
+//! has the server `dispatch` requests parsed from bytes, with no socket:
 //!
 //! ```
-//! let app = Server::new().get("/hello", |_| Response::text(200, "hi"))
+//! let app = Server::new()
+//!     .get("/hello", |_| Response::text(200, "hi"))
+//!     .get("/posts/:id", |r: Request| Response::text(200, "post ${r.params.get("id").unwrap()}"))
 //!
 //! let raw = bytes_from_string("GET /hello HTTP/1.1\r\nHost: example\r\nX-Trace: abc\r\n\r\n")
 //! let req = parse_request_head(raw, Limits::default()).unwrap().unwrap()
@@ -32,6 +37,12 @@
 //! let wire = resp.to_bytes().to_string().unwrap()
 //! assert(wire.starts_with("HTTP/1.1 200 OK\r\n"), "the status line comes first")
 //! assert(wire.ends_with("\r\n\r\nhi"), "then a blank line and the body")
+//!
+//! // A pattern's `:id` takes the piece of the path it matched; the query
+//! // takes no part in matching.
+//! let post = bytes_from_string("GET /posts/7?tab=2 HTTP/1.1\r\n\r\n")
+//! let shown = app.dispatch(parse_request_head(post, Limits::default()).unwrap().unwrap())
+//! assert_eq(shown.body.to_string().unwrap(), "post 7")
 //! ```
 
 // Nova standard library -- HTTP/1.1, server side.
@@ -48,8 +59,10 @@
 // alias), found async` -- and a type alias of any kind is `E0900`, so
 // `Server` at the end of this file types each handler inline as
 // `fn(Request) -> Response`. A handler is therefore synchronous: it builds
-// its response without awaiting. Only GET routes ship, and there is no
-// client. `Server::listen` shuts down gracefully on SIGTERM or SIGINT
+// its response without awaiting. Routes by method, `:name` parameters
+// (`req.params`), `405`, and the form, query and cookie readers shipped in
+// Phase 4.7 (docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md
+// §3); there is no client. `Server::listen` shuts down gracefully on SIGTERM or SIGINT
 // (Ctrl+Break or Ctrl+C on Windows); see its own comment and
 // docs/adr/0022-process-shutdown-signals.md. See also
 // docs/adr/0019-offset-table-intrinsic-boundary.md and section 6's own dated
@@ -256,8 +269,12 @@ impl Default for Limits {
 }
 
 // A parsed request. `body` is the bytes after the head, already bounded by
-// `Content-Length` and by `Limits::max_body_bytes`.
-/// An HTTP request: its method, target, headers and body.
+// `Content-Length` and by `Limits::max_body_bytes`. `params` is the router's
+// (spec docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md
+// §3.1), as `nova-spec/20-STDLIB.md` section 6 has it, `req.params.get("id")`,
+// rather than a second argument to every handler.
+/// An HTTP request: its method, target, headers and body, and the parameters
+/// the route that answers it took from its path.
 pub record Request {
     /// The request's method.
     pub method: Method,
@@ -271,6 +288,229 @@ pub record Request {
     /// The body: as many bytes as `Content-Length` says, or none without that
     /// header. Empty in a request from `parse_request_head`.
     pub body: Bytes,
+    /// What the pattern of the route that answers the request took from its
+    /// path, by name, decoded: `/posts/:id` matching `/posts/7` puts `7`
+    /// under `id`. `Server::dispatch` sets it before it calls the route's
+    /// handler. It is empty in a request from `parse_request_head` or
+    /// `read_request`, and in one a `fallback` handler gets.
+    pub params: Map<String, String>,
+}
+
+impl Request {
+    /// Returns the query of the request's target, the text after its first
+    /// `?`, by name, decoded as a browser encodes a form's fields
+    /// (`application/x-www-form-urlencoded`), as `std/ui/router`'s
+    /// `use_query` decodes a location's query.
+    ///
+    /// The text is split at each `&`, and an empty part is skipped. A part is
+    /// split at its first `=` into a name and a value; a part with no `=` is
+    /// a name whose value is `""`. In each half `+` is a space, and each `%`
+    /// followed by two hex digits is the byte they write, the bytes read as
+    /// UTF-8; a half that holds a `%` not followed by two hex digits, or
+    /// whose bytes are not UTF-8, is kept as written. Of two parts with the
+    /// same name, the last wins. A target with no `?` gives an empty map.
+    ///
+    /// ```
+    /// let raw = bytes_from_string("GET /search?q=nova+lang&tag=a%26b&tag=c&flag HTTP/1.1\r\n\r\n")
+    /// let req = parse_request_head(raw, Limits::default()).unwrap().unwrap()
+    /// let q = req.query()
+    /// assert_eq(q.get("q").unwrap(), "nova lang")
+    /// assert_eq(q.get("tag").unwrap(), "c") // the last of two wins
+    /// assert_eq(q.get("flag").unwrap(), "") // no `=`, no value
+    /// assert_eq(q.len(), 3)
+    /// ```
+    pub fn query(self) -> Map<String, String> {
+        match self.path.index_of("?") {
+            Some(at) => form_fields(self.path.slice(at + 1, self.path.len()))
+            None => {
+                let none: Map<String, String> = Map::new()
+                none
+            }
+        }
+    }
+
+    /// Returns the fields of the form the request's body holds, by name,
+    /// decoded as `query` decodes a query, when the body is a form a browser
+    /// sent: its `content-type` header's media type, the part before any
+    /// `;`, trimmed and read in any case, is
+    /// `application/x-www-form-urlencoded`, and the body is UTF-8. Any other
+    /// request, one with no `content-type` or a form sent as
+    /// `multipart/form-data` included, gives an empty map.
+    ///
+    /// ```
+    /// let mut headers: Map<String, String> = Map::new()
+    /// headers.insert("content-type", "application/x-www-form-urlencoded; charset=UTF-8")
+    /// let req = Request {
+    ///     method: Post,
+    ///     path: "/comments",
+    ///     headers: headers,
+    ///     body: bytes_from_string("name=Ada+L.&text=Hi%21+%E2%9C%93"),
+    ///     params: Map::new(),
+    /// }
+    /// let form = req.form()
+    /// assert_eq(form.get("name").unwrap(), "Ada L.")
+    /// assert_eq(form.get("text").unwrap(), "Hi! \u{2713}")
+    ///
+    /// // The same body as JSON is no form.
+    /// let mut json: Map<String, String> = Map::new()
+    /// json.insert("content-type", "application/json")
+    /// let other = Request { method: Post, path: "/comments", headers: json, body: req.body, params: Map::new() }
+    /// assert(other.form().is_empty(), "not a form")
+    /// ```
+    pub fn form(self) -> Map<String, String> {
+        let none: Map<String, String> = Map::new()
+        let media = match self.headers.get("content-type") {
+            Some(v) => {
+                match v.index_of(";") {
+                    Some(at) => v.slice(0, at).trim().to_lower()
+                    None => v.trim().to_lower()
+                }
+            }
+            None => return none
+        }
+        if media != "application/x-www-form-urlencoded" { return none }
+        match self.body.to_string() {
+            Some(text) => form_fields(text)
+            None => none
+        }
+    }
+
+    /// Returns the cookies the request's `cookie` header holds, by name.
+    ///
+    /// The header is split at each `;`, and each part trimmed, then split at
+    /// its first `=` into a name and a value. A part with no `=` is skipped,
+    /// and of two cookies with the same name the first is kept, since a
+    /// browser sends the one set for the longest path first. Values are
+    /// returned as sent, not decoded. A request with no `cookie` header gives
+    /// an empty map.
+    ///
+    /// ```
+    /// let raw = bytes_from_string("GET / HTTP/1.1\r\nCookie: sid=abc.123; theme=dark; sid=old\r\n\r\n")
+    /// let req = parse_request_head(raw, Limits::default()).unwrap().unwrap()
+    /// let cookies = req.cookies()
+    /// assert_eq(cookies.get("sid").unwrap(), "abc.123") // the first of two
+    /// assert_eq(cookies.get("theme").unwrap(), "dark")
+    /// assert_eq(cookies.len(), 2)
+    /// ```
+    pub fn cookies(self) -> Map<String, String> {
+        let mut out: Map<String, String> = Map::new()
+        let header = match self.headers.get("cookie") {
+            Some(v) => v
+            None => return out
+        }
+        let parts = header.split(";")
+        let mut i = 0
+        while i < parts.len() {
+            let part = parts[i].trim()
+            match part.index_of("=") {
+                Some(eq) => {
+                    let name = part.slice(0, eq)
+                    if !out.contains_key(name) {
+                        let _ = out.insert(name, part.slice(eq + 1, part.len()))
+                    }
+                }
+                None => {}
+            }
+            i = i + 1
+        }
+        out
+    }
+}
+
+// The fields of `text`, a query or a form's body, as `Request::query` says,
+// by `std/ui/router`'s `query_of` rules, which this copies since std/http
+// cannot import std/ui: `&`-separated parts, each `name=value` or a `name`
+// whose value is `""`, both halves decoded with `+` as a space, an empty part
+// skipped, and a name's last value winning. `query` and `form` share it
+// (spec §12, decision 4).
+fn form_fields(text: String) -> Map<String, String> {
+    let mut out: Map<String, String> = Map::new()
+    let parts = text.split("&")
+    let mut i = 0
+    while i < parts.len() {
+        let part = parts[i]
+        if part != "" {
+            let mut name = part
+            let mut value = ""
+            match part.index_of("=") {
+                Some(eq) => {
+                    name = part.slice(0, eq)
+                    value = part.slice(eq + 1, part.len())
+                }
+                None => {}
+            }
+            let _ = out.insert(form_decoded(name), form_decoded(value))
+        }
+        i = i + 1
+    }
+    out
+}
+
+// A half of a form's part: `+` is a space, then percent-decoding.
+fn form_decoded(text: String) -> String { percent_decoded(" ".join(text.split("+"))) }
+
+// `text` percent-decoded as UTF-8, or as written when a `%` is not followed
+// by two hex digits or the bytes are not UTF-8: `std/ui/router`'s `decoded`,
+// copied, so a server and its client read a path and a query alike.
+fn percent_decoded(text: String) -> String {
+    if !text.contains("%") { return text }
+    let bs = bytes_from_string(text).to_ints()
+    let mut out = [0; bs.len()]
+    let mut n = 0
+    let mut i = 0
+    while i < bs.len() {
+        if bs[i] == 37 {
+            if i + 2 >= bs.len() { return text }
+            let hi = hex_value(bs[i + 1])
+            let lo = hex_value(bs[i + 2])
+            if hi < 0 || lo < 0 { return text }
+            out[n] = hi * 16 + lo
+            i = i + 3
+        } else {
+            out[n] = bs[i]
+            i = i + 1
+        }
+        n = n + 1
+    }
+    bytes_from_ints(out).slice(0, n).to_string().unwrap_or(text)
+}
+
+// The value of the hex digit whose ASCII code is `b`, or -1.
+fn hex_value(b: Int) -> Int {
+    if b >= 48 && b <= 57 {
+        b - 48
+    } else if b >= 65 && b <= 70 {
+        b - 55
+    } else if b >= 97 && b <= 102 {
+        b - 87
+    } else {
+        -1
+    }
+}
+
+// The pieces of `path`, a request's path or a route's pattern: split at `/`,
+// with the empty ones dropped, each percent-decoded, or kept as written when
+// it does not decode -- `std/ui/router`'s `pieces_of`, copied, so a server's
+// route and a client's agree on what a path is (spec §3.1).
+fn path_pieces(path: String) -> [String] {
+    let parts = path.split("/")
+    let mut n = 0
+    let mut i = 0
+    while i < parts.len() {
+        if parts[i] != "" { n = n + 1 }
+        i = i + 1
+    }
+    let mut out = [""; n]
+    let mut k = 0
+    let mut j = 0
+    while j < parts.len() {
+        if parts[j] != "" {
+            out[k] = percent_decoded(parts[j])
+            k = k + 1
+        }
+        j = j + 1
+    }
+    out
 }
 
 // The raw offset table for `buf`, exactly as
@@ -392,7 +632,17 @@ pub fn parse_request_head(buf: Bytes, limits: Limits) -> Result<Option<Request>,
         headers.insert(name, value)
         i = i + 1
     }
-    Ok(Some(Request { method: method, path: path, headers: headers, body: bytes_from_string("") }))
+    Ok(
+        Some(
+            Request {
+                method: method,
+                path: path,
+                headers: headers,
+                body: bytes_from_string(""),
+                params: Map::new(),
+            },
+        ),
+    )
 }
 
 // The bytes `buf[start .. start+len]` as text, defaulting to the empty
@@ -844,6 +1094,7 @@ pub async fn read_request(conn: TcpStream, limits: Limits) -> Result<Request, Ht
             path: req.path,
             headers: req.headers,
             body: body.slice(0, want),
+            params: req.params,
         },
     )
 }
@@ -1002,19 +1253,22 @@ const SERVER_TICK_MS: Int = 100
 const SERVER_REQUEST_MS: Int = 10000
 
 // A routing table, and the handler of a `GET` no route matches. Build one
-// with `Server::new()`, `get` and `fallback`, then `listen` or `listen_on`.
-/// A router that serves GET routes over HTTP/1.1.
+// with `Server::new()`, a route method for each route, and `fallback`, then
+// `listen` or `listen_on`. Routes by method, patterns, `405` and `params`
+// are spec docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md
+// §3.1.
+/// A router that serves HTTP/1.1, sending each request to a handler by its
+/// method and path.
 ///
-/// Build one with `Server::new()` and a call to `get` for each route, then
-/// serve it with `listen`, or `listen_on` a listener already bound. A request
-/// goes to the handler of the route whose path equals the request's target
-/// exactly, query string included; any other `GET` goes to the `fallback`
-/// handler, when there is one; and any other request, any method but `GET`
-/// included, gets `Response::not_found()`.
+/// Build one with `Server::new()` and a call to `get`, `post`, `put`,
+/// `delete` or `route` for each route, and `fallback` for the `GET`s no route
+/// matches, then serve it with `listen`, or `listen_on` a listener already
+/// bound. `dispatch` says which handler answers a request.
 pub record Server { routes: Vec<Route>, unmatched: Option<fn(Request) -> Response> }
 
-// One GET route: an exact path, and the handler that answers it.
-record Route { path: String, handler: fn(Request) -> Response }
+// One route: the method it answers, its pattern's pieces, as `path_pieces`
+// splits it once when the route is added, and its handler.
+record Route { method: Method, pattern: [String], handler: fn(Request) -> Response }
 
 // A request read off a connection, and the bytes that followed it: a
 // pipelined next request, or the start of one.
@@ -1027,20 +1281,111 @@ impl Server {
     // Adds a GET route. Takes `self` rather than `mut self`, and returns
     // `Server` rather than section 6's `Self`: a `mut self` call on the
     // temporary `Server::new()` returns is `E0060`, and `Self` as a type in an
-    // impl block is `E0001`.
-    //
-    // The new server gets a copy of the route table, and `self` is left
-    // alone. A `Vec` is shared, not copied, on assignment, so pushing onto
-    // `self.routes` would add the route to every server built from the same
-    // base as well.
-    /// Returns a copy of this server with one more route: `GET` requests for
-    /// `path` go to `handler`.
+    // impl block is `E0001`. Before 4.7 it matched the whole target, query
+    // included; it takes a pattern now, as every route does (spec §12,
+    // decision 1).
+    /// Returns a copy of this server with one more route: `GET` requests
+    /// whose path matches `pattern` go to `handler`. It is
+    /// `route(Get, pattern, handler)`, which says how a pattern matches.
     ///
     /// `self` is left unchanged, so calls chain, as in
-    /// `Server::new().get("/", home).get("/about", about)`. `path` must equal
-    /// a request's target exactly: `/a` matches neither `/a/` nor `/a?x=1`. Of
-    /// two routes with the same path, the one added first answers.
-    pub fn get(self, path: String, handler: fn(Request) -> Response) -> Server {
+    /// `Server::new().get("/", home).get("/posts/:id", post)`. The query
+    /// takes no part: `/add` matches `/add?x=1`, and so does `/add/`.
+    ///
+    /// ```
+    /// let app = Server::new()
+    ///     .get("/", |_| Response::text(200, "home"))
+    ///     .get("/posts/:id", |r: Request| Response::text(200, "post ${r.params.get("id").unwrap()}"))
+    /// let get = |target: String| parse_request_head(
+    ///     bytes_from_string("GET ${target} HTTP/1.1\r\n\r\n"),
+    ///     Limits::default(),
+    /// ).unwrap().unwrap()
+    /// assert_eq(app.dispatch(get("/?ref=mail")).body.to_string().unwrap(), "home")
+    /// assert_eq(app.dispatch(get("/posts/a%20b/")).body.to_string().unwrap(), "post a b")
+    /// assert_eq(app.dispatch(get("/posts/7/edit")).status, 404)
+    /// ```
+    pub fn get(self, pattern: String, handler: fn(Request) -> Response) -> Server {
+        self.route(Get, pattern, handler)
+    }
+
+    /// Returns a copy of this server with one more route: `POST` requests
+    /// whose path matches `pattern` go to `handler`. It is
+    /// `route(Post, pattern, handler)`, which says how a pattern matches.
+    ///
+    /// ```
+    /// let app = Server::new()
+    ///     .get("/posts/:id", |_| Response::text(200, "the post"))
+    ///     .post("/posts/:id", |r: Request| Response::text(201, "saved ${r.params.get("id").unwrap()}"))
+    /// let raw = bytes_from_string("POST /posts/7 HTTP/1.1\r\nContent-Length: 0\r\n\r\n")
+    /// let saved = app.dispatch(parse_request_head(raw, Limits::default()).unwrap().unwrap())
+    /// assert_eq(saved.status, 201)
+    /// assert_eq(saved.body.to_string().unwrap(), "saved 7")
+    /// ```
+    pub fn post(self, pattern: String, handler: fn(Request) -> Response) -> Server {
+        self.route(Post, pattern, handler)
+    }
+
+    /// Returns a copy of this server with one more route: `PUT` requests
+    /// whose path matches `pattern` go to `handler`. It is
+    /// `route(Put, pattern, handler)`, which says how a pattern matches.
+    ///
+    /// ```
+    /// let app = Server::new().put("/notes/:name", |r: Request| Response::text(200, "stored ${r.body.len()} bytes"))
+    /// let raw = bytes_from_string("PUT /notes/todo HTTP/1.1\r\n\r\n")
+    /// let req = parse_request_head(raw, Limits::default()).unwrap().unwrap()
+    /// assert_eq(app.dispatch(req).body.to_string().unwrap(), "stored 0 bytes")
+    /// ```
+    pub fn put(self, pattern: String, handler: fn(Request) -> Response) -> Server {
+        self.route(Put, pattern, handler)
+    }
+
+    /// Returns a copy of this server with one more route: `DELETE` requests
+    /// whose path matches `pattern` go to `handler`. It is
+    /// `route(Delete, pattern, handler)`, which says how a pattern matches.
+    ///
+    /// ```
+    /// let app = Server::new().delete("/notes/:name", |_| Response::text(204, ""))
+    /// let raw = bytes_from_string("DELETE /notes/todo HTTP/1.1\r\n\r\n")
+    /// let req = parse_request_head(raw, Limits::default()).unwrap().unwrap()
+    /// assert_eq(app.dispatch(req).status, 204)
+    /// ```
+    pub fn delete(self, pattern: String, handler: fn(Request) -> Response) -> Server {
+        self.route(Delete, pattern, handler)
+    }
+
+    // Adds a route. The new server gets a copy of the route table, and
+    // `self` is left alone. A `Vec` is shared, not copied, on assignment, so
+    // pushing onto `self.routes` would add the route to every server built
+    // from the same base as well.
+    /// Returns a copy of this server with one more route: requests whose
+    /// method is `method` and whose path matches `pattern` go to `handler`.
+    ///
+    /// `self` is left unchanged, so calls chain. A request's path is its
+    /// target before any `?`, split at each `/`, with the empty pieces
+    /// dropped and each piece percent-decoded, as `std/ui/router` reads a
+    /// location; a piece that does not decode, one with a `%` not followed by
+    /// two hex digits say, is kept as written. `pattern` is split the same
+    /// way, and each of its pieces is literal, matching a piece equal to it,
+    /// or `:name`, matching any one piece, which the handler finds in
+    /// `req.params` under `name`. A pattern matches a path with as many
+    /// pieces, each matching: `/posts/:id` matches `/posts/7`, `/posts/7/`
+    /// and `/posts/7?x=1`, with `id` set to `7`, but not `/posts` or
+    /// `/posts/7/edit`; and `/` matches only `/`. A method matches only
+    /// itself, as the request writes it: `Unknown("PURGE")` matches `PURGE`.
+    /// `dispatch` says which of several matching routes answers.
+    ///
+    /// ```
+    /// let app = Server::new().route(Patch, "/notes/:name", |r: Request| Response::text(200, "patched ${r.params.get("name").unwrap()}"))
+    /// let raw = bytes_from_string("PATCH /notes/to%2Fdo HTTP/1.1\r\n\r\n")
+    /// let req = parse_request_head(raw, Limits::default()).unwrap().unwrap()
+    /// assert_eq(app.dispatch(req).body.to_string().unwrap(), "patched to/do")
+    /// ```
+    pub fn route(
+        self,
+        method: Method,
+        pattern: String,
+        handler: fn(Request) -> Response,
+    ) -> Server {
         let mut rs: Vec<Route> = Vec::new()
         let mut i = 0
         while i < self.routes.len() {
@@ -1050,19 +1395,21 @@ impl Server {
             }
             i = i + 1
         }
-        rs.push(Route { path: path, handler: handler })
+        rs.push(Route { method: method, pattern: path_pieces(pattern), handler: handler })
         Server { routes: rs, unmatched: self.unmatched }
     }
 
-    // Sets the fallback. Like `get`, it takes `self` and returns a new
+    // Sets the fallback. Like `route`, it takes `self` and returns a new
     // server, so `self` keeps its own; the route table is shared between the
-    // two, which is safe since `get` copies it before it adds a route.
+    // two, which is safe since `route` copies it before it adds a route.
     /// Returns a copy of this server whose `handler` answers each `GET`
     /// request no route matches, in place of `Response::not_found()`.
     ///
-    /// `self` is left unchanged, and calls chain with `get`'s in any order:
-    /// a route still answers its own path. A second `fallback` replaces the
-    /// first. Any method but `GET` still gets `not_found`.
+    /// `self` is left unchanged, and calls chain with the route methods in
+    /// any order: a route still answers its own path. A second `fallback`
+    /// replaces the first. It never answers any method but `GET`, and a
+    /// `GET` of a path a route matches with another method gets `405`, as
+    /// `dispatch` says. The request it gets has no `params`.
     ///
     /// ```
     /// let app = Server::new()
@@ -1073,6 +1420,7 @@ impl Server {
     ///     path: path,
     ///     headers: Map::new(),
     ///     body: bytes_from_string(""),
+    ///     params: Map::new(),
     /// }
     /// assert_eq(app.dispatch(get("/")).body.to_string().unwrap(), "home")
     /// let missing = app.dispatch(get("/nope?x=1"))
@@ -1083,36 +1431,86 @@ impl Server {
         Server { routes: self.routes, unmatched: Some(handler) }
     }
 
-    // The response to `req`: the handler of the GET route whose path equals
-    // `req.path` exactly, else the fallback's for a GET, or
-    // `Response::not_found()` for any other path or method. Socket-free, so a
-    // test can call it directly.
-    /// Returns the response to `req`: what the handler of the route whose
-    /// path equals `req.path` returns, or if there is none what the
-    /// `fallback` handler returns, or `Response::not_found()` if there is no
-    /// fallback either.
+    // The response to `req`, by spec §3.1's dispatch: the first route whose
+    // method and pattern match, else `405` naming the methods of the routes
+    // whose pattern matches, else the fallback's for a GET, else
+    // `Response::not_found()`. Socket-free, so a test can call it directly.
+    /// Returns the response to `req`.
+    ///
+    /// The routes are tried in the order they were added, and the first
+    /// whose method and pattern both match `req` answers: its handler gets
+    /// `req` with `params` holding what the pattern took. Otherwise, when a
+    /// route's pattern matches but its method does not, the answer is
+    /// `405 Method Not Allowed`, with an `allow` header naming the methods of
+    /// the routes whose pattern matches, each once, in the order they were
+    /// added, as `GET, POST`. Otherwise a `GET` goes to the `fallback`
+    /// handler, when there is one, and any other request gets
+    /// `Response::not_found()`. `HEAD` is a method like any other, answered
+    /// only by a route added for it.
     ///
-    /// Only a `GET` request reaches a handler; any other method, `HEAD`
-    /// included, gets `not_found`. It touches no connection, so a test can
-    /// call it directly with a `Request` it builds.
+    /// It touches no connection, so a test can call it directly with a
+    /// `Request` it builds.
+    ///
+    /// ```
+    /// let app = Server::new()
+    ///     .get("/login", |_| Response::text(200, "the form"))
+    ///     .post("/login", |_| Response::text(200, "logged in"))
+    /// let raw = bytes_from_string("DELETE /login HTTP/1.1\r\n\r\n")
+    /// let refused = app.dispatch(parse_request_head(raw, Limits::default()).unwrap().unwrap())
+    /// assert_eq(refused.status, 405)
+    /// assert_eq(refused.headers.get("allow").unwrap(), "GET, POST")
+    /// ```
     pub fn dispatch(self, req: Request) -> Response {
-        match req.method {
-            Get => {
-                let mut i = 0
-                while i < self.routes.len() {
-                    match self.routes.get(i) {
-                        Some(r) => {
-                            if r.path == req.path {
+        let target = req.path
+        let path = match target.index_of("?") {
+            Some(at) => target.slice(0, at)
+            None => target
+        }
+        let pieces = path_pieces(path)
+        let mut allowed: Vec<String> = Vec::new()
+        let mut i = 0
+        while i < self.routes.len() {
+            match self.routes.get(i) {
+                Some(r) => {
+                    match params_of(r.pattern, pieces) {
+                        Some(params) => {
+                            let name = method_name(r.method)
+                            if name == method_name(req.method) {
                                 // Bound to a local first: calling a fn-typed
                                 // field directly, `r.handler(req)`, is `E0014`.
                                 let h = r.handler
-                                return h(req)
+                                return h(
+                                    Request {
+                                        method: req.method,
+                                        path: req.path,
+                                        headers: req.headers,
+                                        body: req.body,
+                                        params: params,
+                                    },
+                                )
                             }
+                            if !holds(allowed, name) { allowed.push(name) }
                         }
                         None => {}
                     }
-                    i = i + 1
                 }
+                None => {}
+            }
+            i = i + 1
+        }
+        if allowed.len() > 0 {
+            let mut names = [""; allowed.len()]
+            let mut k = 0
+            while k < allowed.len() {
+                names[k] = allowed.get(k).unwrap_or("")
+                k = k + 1
+            }
+            let mut refused = Response::text(405, "method not allowed")
+            refused.headers.insert("allow", ", ".join(names))
+            return refused
+        }
+        match req.method {
+            Get => {
                 match self.unmatched {
                     Some(h) => h(req)
                     None => Response::not_found()
@@ -1347,6 +1745,7 @@ async fn read_in_flight(
                 path: req.path,
                 headers: req.headers,
                 body: buf.slice(body_start, end),
+                params: req.params,
             },
             rest: buf.slice(end, buf.len()),
         },
@@ -1372,3 +1771,48 @@ fn head_window(buf: Bytes, max: Int) -> Bytes {
     if buf.len() <= max { return buf }
     buf.slice(0, max)
 }
+
+// The parameters `pattern`, a route's pieces, takes from `pieces`, a
+// request path's, or `None` when it does not match them: `std/ui/router`'s
+// `params_of`, less its catch-all, which a server's pattern does not have
+// (spec §3.1).
+fn params_of(pattern: [String], pieces: [String]) -> Option<Map<String, String>> {
+    if pattern.len() != pieces.len() { return None }
+    let mut params: Map<String, String> = Map::new()
+    let mut i = 0
+    while i < pattern.len() {
+        let p = pattern[i]
+        if p.starts_with(":") {
+            let _ = params.insert(p.slice(1, p.len()), pieces[i])
+        } else if p != pieces[i] {
+            return None
+        }
+        i = i + 1
+    }
+    Some(params)
+}
+
+// `m` as a request line writes it, which is how two methods are compared and
+// how `allow` names one: `Method` has no `Eq`.
+fn method_name(m: Method) -> String {
+    match m {
+        Get => "GET"
+        Post => "POST"
+        Put => "PUT"
+        Delete => "DELETE"
+        Patch => "PATCH"
+        Head => "HEAD"
+        Options => "OPTIONS"
+        Unknown(name) => name
+    }
+}
+
+// Whether `names` holds `name`.
+fn holds(names: Vec<String>, name: String) -> Bool {
+    let mut i = 0
+    while i < names.len() {
+        if names.get(i).unwrap_or("") == name { return true }
+        i = i + 1
+    }
+    false
+}
````


### Task 3: Answering

Spec §3.3; decisions 5, 6, 17 and 20.

**Files:**
- Modify:
  - `std/http/lib.nova` (`Cookie`, `Response.cookies`, `html`,
    `redirect`, `with_cookie`, the cookie lines, the reason phrases)
  - `examples/05-json-api/src/main.nova`, `std/ui/ssr/lib.nova`
    (`cookies: []`)
- Test:
  - `tests/runtime/http_responses.nova`, `http_responses.stdout`
  - `tests/runtime/http_server_wire.nova`, `http_server_wire.stdout`
  - `tests/runtime/http_serialise.nova`
  - `crates/nova-cli/tests/run_tests.rs`, `wasm_tests.rs`
  - `crates/nova-driver/tests/index_complete.rs`

- [ ] **Step 1: The tests.** `patch_block t3-tests`. Expected: it applies,
  and `git apply` prints `$P/t3-tests.patch:508: trailing whitespace.`,
  then the line `  body: ` with its trailing space, then `warning: 1 line
  adds whitespace errors.`: the golden of a `303`'s empty body (Global
  Constraints). The patch holds:
  - `tests/runtime/http_responses.nova`, with no socket: `Cookie::new`'s
    defaults and `Cookie::expired`, as fields and on the wire; each
    attribute only when it applies; several `set-cookie` lines, in order,
    after the other headers, `with_cookie` leaving its receiver alone, and
    a `set-cookie` header set by hand; a cookie left out for a name that is
    not a token (empty, a space, `;`, `=`, `,`, `"`, `/`, a character outside
    ASCII, CR LF), every `tchar` taken; for a value a cookie cannot carry (a
    space, `;`, `,`, `"`, `\`, a tab, DEL, outside ASCII, CR LF), every
    cookie-octet taken; for a path holding `;`, a CR or a character outside
    ASCII, a space and a comma taken; for a `same_site` that would inject
    an attribute; only the bad one of three left out; `html`, and
    `redirect`'s status, location, length and empty body, a location holding
    CR LF left out; and the reason phrases, old and new, and three by their
    class. `http_responses_run` runs it as it is and under stress.
  - `tests/runtime/http_server_wire.nova`, over TCP on a port the program
    bound itself, on one connection: a `GET` with a path parameter and a
    query, answered with two cookies; a form's `POST` with a cookie,
    answered with `303`, its `location` and two `set-cookie` lines in order;
    a `DELETE` answered `405` with `allow: GET, POST`; and the fallback.
    `http_server_wire_run`, likewise.
  - `tests/runtime/http_serialise.nova`: `cookies: []` in its `Response`
    literal.
  - `wasm_tests.rs`: `http_responses` and `http_server_wire` in
    `LEFT_OUT`.
  - `index_complete.rs`: 164 programs and 190 files.
- [ ] **Step 2: Run, and see them fail.** First the build (Conventions).
  - `R http_responses_run`. Expected: `0 passed; 1 failed`: "Unexpected
    failure.": ``error[E0001]: cannot find type `Cookie` ``,
    ``error[E0014]: no method `with_cookie` on type `Response` ``,
    `error[E0900]: module-qualified paths are not supported yet`, for
    `Cookie::new`, and ``error[E0001]: no variant `redirect` on type
    `Response` ``: `could not compile due to 56 previous errors`.
  - `R http_server_wire_run`. Expected: `0 passed; 1 failed`: E0900,
    ``error[E0014]: cannot access field `max_age` on `?6` ``, ``no variant
    `redirect` `` and ``no variant `html` on type `Response` ``: `could not
    compile due to 14 previous errors`.
  - `R http_serialise_run`. Expected: `0 passed; 1 failed`:
    ``error[E0014]: record `Response` has no field `cookies` ``: `could not
    compile due to 1 previous error`.
  - `W every_runtime_program_is_in_the_gate_or_left_out`. Expected: `1
    passed`.
  - `X` (about 8 s). Expected: `0 passed; 1 failed`, at
    `index_complete.rs:259`: "…/tests/runtime/http_responses.nova does not
    check cleanly: [\"E0001 cannot find type `Cookie`\", \"E0014 no method
    `with_cookie` on type `Response`\", \"E0900 module-qualified paths are
    not supported yet\", …]".
- [ ] **Step 3: The code.** `patch_block t3-code`:
  - **`std/http/lib.nova`,** final: `Cookie`, its fields' docs and its
    rules, `Cookie::new` and `Cookie::expired` (decisions 6 and 17);
    `Response.cookies` (decisions 5 and 20); `Response::html`,
    `Response::redirect` and `with_cookie`, each with its doc test;
    `to_bytes`' `set-cookie` lines and its doc, through `cookie_line`,
    `is_token`, `is_tchar`, `is_cookie_octet`, `is_path_char` and
    `all_chars`; the reason phrases of 301, 302, 303, 401 and 403; `cookies:
    []` in each `Response` literal; and the three texts Task 2 wrote
    without the builders, back on `html` and `redirect`.
  - **`examples/05-json-api/src/main.nova`** and **`std/ui/ssr/lib.nova`:**
    `cookies: []` in their `Response` literals.
- [ ] **Step 4: Run.** First the build.
  - The same five commands. Expected: `R http_responses_run` `1 passed`;
    `R http_server_wire_run` `1 passed`; `R http_serialise_run` `1 passed`;
    `W every_runtime_program_is_in_the_gate_or_left_out` `1 passed`; `X` `1
    passed`.
  - The whole binaries:
    - `cargo test -p nova-std`. Expected: `2 passed`.
    - `cargo test -p nova-doc --lib`. Expected: `61 passed`.
    - `cargo test -p nova-cli --test doc_test`. Expected: `17 passed`.
    - `R`. Expected: `325 passed; 0 failed; 1 ignored`, in about 50
      seconds, example 05's test among them.
    - `W`. Expected: `230 passed`.
    - `cargo test -p nova-cli --test ssr_tests`. Expected: `9 passed`:
      `ui::ssr`'s two literals.
- [ ] **Step 5: The checks,** as the conventions list them, and `cargo run
  --locked -q -p nova-cli -- fmt --check tests/runtime/http_responses.nova
  tests/runtime/http_server_wire.nova`, which prints nothing. Not
  `http_serialise.nova`: `nova fmt` would reformat four places the base
  already had, and the `Response` literal this task gives `cookies: []`,
  one field to a line; `tests/runtime` is in no check's paths.
- [ ] **Step 6: Commit** `std/http: Cookie, Response.cookies, html, redirect and five reason phrases`.

**Patch `t3-tests`:**

<!-- patch t3-tests -->

````diff
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index fe39cb0c0fbfd6fae92ed65ed44d0f3a7ceac460..0c2b79430b0f1844481e0627bcecc4c6b6798a8a 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -9664,6 +9664,23 @@ fn http_request_readers_run() {
     run_with_and_without_stress("http_request_readers");
 }
 
+/// `Cookie`, `with_cookie`, several `set-cookie` lines after the other
+/// headers, the cookies left out, `Response::html`, `Response::redirect` and
+/// the reason phrases (spec 4.7 §3.3), socket-free.
+#[test]
+fn http_responses_run() {
+    run_with_and_without_stress("http_responses");
+}
+
+/// The same over TCP on a port the program bound itself (spec 4.7 §3): a
+/// path parameter and a query, a form's `POST` with a cookie, answered with
+/// `303` and two `set-cookie` lines in order, a `405` with `allow`, and the
+/// fallback.
+#[test]
+fn http_server_wire_run() {
+    run_with_and_without_stress("http_server_wire");
+}
+
 /// `std/process`'s `args()` under `nova run FILE -- ARGS`: index 0 is FILE as
 /// written, and an argument with spaces and an empty one each arrive whole.
 /// Run from the repository root so FILE is a fixed relative path. Design:
diff --git a/crates/nova-cli/tests/wasm_tests.rs b/crates/nova-cli/tests/wasm_tests.rs
index 6dc580d35f590e982a1caa5607826e867f9f67a3..47306e064ace32e2ac0c5e2343f1038c5048cf08 100644
--- a/crates/nova-cli/tests/wasm_tests.rs
+++ b/crates/nova-cli/tests/wasm_tests.rs
@@ -2804,10 +2804,12 @@ const LEFT_OUT: &[(&str, &str)] = &[
     ("http_offsets", "std/http"),
     ("http_partial", "std/http"),
     ("http_request_readers", "std/http"),
+    ("http_responses", "std/http"),
     ("http_routes", "std/http"),
     ("http_serialise", "std/http"),
     ("http_listen_on", "std/http, std/net, std/process"),
     ("http_server_dispatch", "std/http"),
+    ("http_server_wire", "std/http, std/net, std/process"),
     ("io_broken_stderr", "std/io"),
     ("io_broken_stdout", "std/io"),
     ("io_read_stdin_write_only", "std/io"),
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index bed3d271f8fd45f54a73117aa48163ffce21672b..72346ae966e0e909d2bb9e6202395000d1b29416 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -241,11 +241,11 @@ fn every_name_is_covered(a: &Analysis, file: FileId, failures: &mut Vec<String>)
 #[test]
 fn every_name_in_the_corpora_is_indexed() {
     let programs = programs();
-    // 8 examples, 153 top-level runtime programs and the modules program,
+    // 8 examples, 155 top-level runtime programs and the modules program,
     // on 2026-10-21, after 4.7 added `fs_rename`, `crypto_pbkdf2`,
-    // `crypto_pbkdf2_refused` and `bytes_hex`, then `http_routes` and
-    // `http_request_readers`.
-    assert_eq!(programs.len(), 162, "{programs:#?}");
+    // `crypto_pbkdf2_refused` and `bytes_hex`, then `http_routes`,
+    // `http_request_readers`, `http_responses` and `http_server_wire`.
+    assert_eq!(programs.len(), 164, "{programs:#?}");
     let mut files = 0;
     let mut failures: Vec<String> = Vec::new();
     for (k, path) in programs.iter().enumerate() {
@@ -272,9 +272,9 @@ fn every_name_in_the_corpora_is_indexed() {
             every_name_is_covered(&a, file, &mut failures);
         }
     }
-    // 8 + 153 + 3 files of user code, 05-geometry's library's 2, and
+    // 8 + 155 + 3 files of user code, 05-geometry's library's 2, and
     // native std's 22, `std/ui/ssr` among them.
-    assert_eq!(files, 188);
+    assert_eq!(files, 190);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/tests/runtime/http_responses.nova b/tests/runtime/http_responses.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1a9a833bfca671a64eaaeac7dc67a20cff36c848
--- /dev/null
+++ b/tests/runtime/http_responses.nova
@@ -0,0 +1,181 @@
+// `std/http`'s answers (spec
+// docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md
+// §3.3): `Cookie::new`'s safe defaults and `Cookie::expired`;
+// `with_cookie`, which leaves its receiver alone; several `set-cookie` lines
+// on the wire, in order, after the other headers, each attribute written
+// only when it applies; a cookie left out for a name that is not a token, a
+// value or a path holding what a cookie cannot carry, or a `same_site` that
+// is not a token; `Response::html`, `Response::redirect`; and the reason
+// phrases. Headers are read back by name, and the wire's lines are printed
+// only where a `Map`'s order cannot move them.
+
+// The lines of `r` on the wire, its head only.
+fn head_lines(r: Response) -> [String] {
+    let wire = r.to_bytes().to_string().unwrap_or("")
+    let end = wire.index_of("\r\n\r\n").unwrap_or(wire.len())
+    wire.slice(0, end).split("\r\n")
+}
+
+// The `set-cookie` lines of `r` on the wire, in order, each after a `|`;
+// and whether every one comes after every other header.
+fn set_cookies(r: Response) -> String {
+    let lines = head_lines(r)
+    let mut out = ""
+    let mut seen = false
+    let mut last = true
+    let mut i = 1
+    while i < lines.len() {
+        if lines[i].starts_with("set-cookie: ") {
+            out = "${out} | ${lines[i].slice(12, lines[i].len())}"
+            seen = true
+        } else if seen {
+            last = false
+        }
+        i = i + 1
+    }
+    "${out} (after the other headers: ${last})"
+}
+
+fn status_line(r: Response) -> String { head_lines(r)[0] }
+
+fn header(r: Response, name: String) -> String {
+    match r.headers.get(name) {
+        Some(v) => v
+        None => "<absent>"
+    }
+}
+
+// One line for `c` alone on a response: what is written, or that it was
+// left out.
+fn written(c: Cookie) -> String {
+    let line = set_cookies(Response::ok(bytes_from_string("")).with_cookie(c))
+    if line.starts_with(" | ") { line.slice(3, line.len()) } else { "left out${line}" }
+}
+
+fn named(name: String) -> String { written(Cookie::new(name, "v")) }
+
+fn valued(value: String) -> String { written(Cookie::new("c", value)) }
+
+fn pathed(path: String) -> String {
+    let mut c = Cookie::new("c", "v")
+    c.path = path
+    written(c)
+}
+
+fn main() {
+    println("-- Cookie::new and Cookie::expired")
+    let c = Cookie::new("sid", "abc.123")
+    println(
+        "new: ${c.name}=${c.value} path=${c.path} max_age none=${c.max_age.is_none()} http_only=${c.http_only} same_site=${c.same_site} secure=${c.secure}",
+    )
+    let e = Cookie::expired("sid")
+    println(
+        "expired: ${e.name}=[${e.value}] path=${e.path} max_age=${e.max_age.unwrap_or(-1)} http_only=${e.http_only} same_site=${e.same_site} secure=${e.secure}",
+    )
+    println("written: ${written(c)}")
+    println("written: ${written(e)}")
+
+    println("-- each attribute only when it applies")
+    let mut all = Cookie::new("all", "1")
+    all.max_age = Some(3600)
+    all.secure = true
+    all.same_site = "Strict"
+    println("all: ${written(all)}")
+    let mut bare = Cookie::new("bare", "")
+    bare.path = ""
+    bare.http_only = false
+    bare.same_site = ""
+    println("none: ${written(bare)}")
+    let mut neg = Cookie::new("neg", "x")
+    neg.max_age = Some(-5)
+    neg.path = "/admin"
+    neg.same_site = "None"
+    println("others: ${written(neg)}")
+
+    println("-- several set-cookie lines, in order")
+    let base = Response::redirect("/admin")
+    let one = base.with_cookie(Cookie::new("sid", "new"))
+    let two = one.with_cookie(Cookie::expired("old")).with_cookie(Cookie::new("theme", "dark"))
+    println("base:${set_cookies(base)}")
+    println("one:${set_cookies(one)}")
+    println("two:${set_cookies(two)}")
+    println(
+        "two keeps: ${status_line(two)}, location ${header(two, "location")}, ${two.cookies.len()} cookies",
+    )
+    let branch = one.with_cookie(Cookie::new("other", "1"))
+    println("one again:${set_cookies(one)}")
+    println("a branch:${set_cookies(branch)}")
+    // One header, so a `Map`'s order cannot move it.
+    let mut h: Map<String, String> = Map::new()
+    h.insert("set-cookie", "from=headers")
+    let by_hand = Response { status: 200, headers: h, body: bytes_from_string(""), cookies: [] }
+    println("a header by hand:${set_cookies(by_hand.with_cookie(Cookie::new("from", "cookies")))}")
+
+    println("-- a name that is not a token")
+    println("empty: ${named("")}")
+    println("space: ${named("a b")}")
+    println("semicolon: ${named("a;b")}")
+    println("equals: ${named("a=b")}")
+    println("comma: ${named("a,b")}")
+    println("quote: ${named("a\"b")}")
+    println("slash: ${named("a/b")}")
+    println("non-ascii: ${named("é")}")
+    println("crlf: ${named("a\r\nx-injected: 1")}")
+    println("every tchar: ${named("!#$%&'*+-.^_`|~09AZaz")}")
+
+    println("-- a value a cookie cannot carry")
+    println("empty: ${valued("")}")
+    println("space: ${valued("a b")}")
+    println("semicolon: ${valued("a;Path=/x")}")
+    println("comma: ${valued("a,b")}")
+    println("quote: ${valued("\"quoted\"")}")
+    println("backslash: ${valued("a\\b")}")
+    println("tab: ${valued("a\tb")}")
+    println("del: ${valued(bytes_from_ints([97, 127]).to_string().unwrap_or(""))}")
+    println("non-ascii: ${valued("caf\u{e9}")}")
+    println("crlf: ${valued("a\r\nx-injected: 1")}")
+    println("every octet: ${valued("!#$%&'()*+-./09:<=>?@AZ[]^_`az{|}~")}")
+
+    println("-- a path a cookie cannot carry")
+    println("semicolon: ${pathed("/a;Secure")}")
+    println("cr: ${pathed("/a\rb")}")
+    println("non-ascii: ${pathed("/caf\u{e9}")}")
+    println("space and comma: ${pathed("/a b,c")}")
+
+    println("-- a same_site that is not a token")
+    let mut lax = Cookie::new("c", "v")
+    lax.same_site = "Lax; Domain=evil.example"
+    println("injected: ${written(lax)}")
+
+    println("-- only the bad one is left out")
+    let mixed = Response::text(200, "ok")
+        .with_cookie(Cookie::new("good", "1"))
+        .with_cookie(Cookie::new("bad name", "2"))
+        .with_cookie(Cookie::new("also", "3"))
+    println("mixed:${set_cookies(mixed)}")
+    println("mixed holds ${mixed.cookies.len()}")
+
+    println("-- html and redirect")
+    let page = Response::html(404, "<p>caf\u{e9} &amp; more</p>")
+    println(
+        "html: ${status_line(page)}, ${header(page, "content-type")}, ${header(page, "content-length")}, ${page.body.to_string().unwrap_or("")}",
+    )
+    let go = Response::redirect("/posts/7#comments")
+    println(
+        "redirect: ${status_line(go)}, location ${header(go, "location")}, content-length ${header(go, "content-length")}, body ${go.body.len()} bytes, ${go.headers.len()} headers",
+    )
+    let wire = go.to_bytes().to_string().unwrap_or("")
+    println("redirect ends with a blank line: ${wire.ends_with("\r\n\r\n")}")
+    let split = Response::redirect("/x\r\nx-injected: 1")
+    println(
+        "a location with CRLF is left out: ${!split.to_bytes().contains(bytes_from_string("location"))}",
+    )
+
+    println("-- reason phrases")
+    let codes = [200, 201, 204, 301, 302, 303, 400, 401, 403, 404, 405, 413, 500, 307, 418, 503]
+    let mut i = 0
+    while i < codes.len() {
+        println(status_line(Response::text(codes[i], "")))
+        i = i + 1
+    }
+}
diff --git a/tests/runtime/http_responses.stdout b/tests/runtime/http_responses.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..cd7f1ed564b776d0958bbacea89ac5dc80311889
--- /dev/null
+++ b/tests/runtime/http_responses.stdout
@@ -0,0 +1,72 @@
+-- Cookie::new and Cookie::expired
+new: sid=abc.123 path=/ max_age none=true http_only=true same_site=Lax secure=false
+expired: sid=[] path=/ max_age=0 http_only=true same_site=Lax secure=false
+written: sid=abc.123; Path=/; HttpOnly; SameSite=Lax (after the other headers: true)
+written: sid=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax (after the other headers: true)
+-- each attribute only when it applies
+all: all=1; Path=/; Max-Age=3600; HttpOnly; SameSite=Strict; Secure (after the other headers: true)
+none: bare= (after the other headers: true)
+others: neg=x; Path=/admin; Max-Age=-5; HttpOnly; SameSite=None (after the other headers: true)
+-- several set-cookie lines, in order
+base: (after the other headers: true)
+one: | sid=new; Path=/; HttpOnly; SameSite=Lax (after the other headers: true)
+two: | sid=new; Path=/; HttpOnly; SameSite=Lax | old=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax | theme=dark; Path=/; HttpOnly; SameSite=Lax (after the other headers: true)
+two keeps: HTTP/1.1 303 See Other, location /admin, 3 cookies
+one again: | sid=new; Path=/; HttpOnly; SameSite=Lax (after the other headers: true)
+a branch: | sid=new; Path=/; HttpOnly; SameSite=Lax | other=1; Path=/; HttpOnly; SameSite=Lax (after the other headers: true)
+a header by hand: | from=headers | from=cookies; Path=/; HttpOnly; SameSite=Lax (after the other headers: true)
+-- a name that is not a token
+empty: left out (after the other headers: true)
+space: left out (after the other headers: true)
+semicolon: left out (after the other headers: true)
+equals: left out (after the other headers: true)
+comma: left out (after the other headers: true)
+quote: left out (after the other headers: true)
+slash: left out (after the other headers: true)
+non-ascii: left out (after the other headers: true)
+crlf: left out (after the other headers: true)
+every tchar: !#$%&'*+-.^_`|~09AZaz=v; Path=/; HttpOnly; SameSite=Lax (after the other headers: true)
+-- a value a cookie cannot carry
+empty: c=; Path=/; HttpOnly; SameSite=Lax (after the other headers: true)
+space: left out (after the other headers: true)
+semicolon: left out (after the other headers: true)
+comma: left out (after the other headers: true)
+quote: left out (after the other headers: true)
+backslash: left out (after the other headers: true)
+tab: left out (after the other headers: true)
+del: left out (after the other headers: true)
+non-ascii: left out (after the other headers: true)
+crlf: left out (after the other headers: true)
+every octet: c=!#$%&'()*+-./09:<=>?@AZ[]^_`az{|}~; Path=/; HttpOnly; SameSite=Lax (after the other headers: true)
+-- a path a cookie cannot carry
+semicolon: left out (after the other headers: true)
+cr: left out (after the other headers: true)
+non-ascii: left out (after the other headers: true)
+space and comma: c=v; Path=/a b,c; HttpOnly; SameSite=Lax (after the other headers: true)
+-- a same_site that is not a token
+injected: left out (after the other headers: true)
+-- only the bad one is left out
+mixed: | good=1; Path=/; HttpOnly; SameSite=Lax | also=3; Path=/; HttpOnly; SameSite=Lax (after the other headers: true)
+mixed holds 3
+-- html and redirect
+html: HTTP/1.1 404 Not Found, text/html; charset=utf-8, 23, <p>café &amp; more</p>
+redirect: HTTP/1.1 303 See Other, location /posts/7#comments, content-length 0, body 0 bytes, 2 headers
+redirect ends with a blank line: true
+a location with CRLF is left out: true
+-- reason phrases
+HTTP/1.1 200 OK
+HTTP/1.1 201 Created
+HTTP/1.1 204 No Content
+HTTP/1.1 301 Moved Permanently
+HTTP/1.1 302 Found
+HTTP/1.1 303 See Other
+HTTP/1.1 400 Bad Request
+HTTP/1.1 401 Unauthorized
+HTTP/1.1 403 Forbidden
+HTTP/1.1 404 Not Found
+HTTP/1.1 405 Method Not Allowed
+HTTP/1.1 413 Payload Too Large
+HTTP/1.1 500 Internal Server Error
+HTTP/1.1 307 Redirection
+HTTP/1.1 418 Client Error
+HTTP/1.1 503 Server Error
diff --git a/tests/runtime/http_serialise.nova b/tests/runtime/http_serialise.nova
index 8ce62fad52e55c51e0a35b72ef18e0862bcf217b..8e465f86011b68f89a385b6bb01c4d72e051b9ba 100644
--- a/tests/runtime/http_serialise.nova
+++ b/tests/runtime/http_serialise.nova
@@ -107,7 +107,7 @@ fn main() {
     let mut evil: Map<String, String> = Map::new()
     evil.insert("content-length", "0")
     evil.insert("x-evil", "safe\r\nX-Injected: yes")
-    let evil_resp = Response { status: 200, headers: evil, body: bytes_from_string("") }
+    let evil_resp = Response { status: 200, headers: evil, body: bytes_from_string(""), cookies: [] }
     let evil_wire = evil_resp.to_bytes()
     println("evil header dropped ${!evil_wire.contains(bytes_from_string("X-Injected"))}")
     println("evil wire keeps content-length ${evil_wire.contains(bytes_from_string("content-length: 0"))}")
diff --git a/tests/runtime/http_server_wire.nova b/tests/runtime/http_server_wire.nova
new file mode 100644
index 0000000000000000000000000000000000000000..7762e0ab9a973ffc5787204bfcfd9e602ce356f5
--- /dev/null
+++ b/tests/runtime/http_server_wire.nova
@@ -0,0 +1,132 @@
+// `std/http`'s routes, readers and answers over TCP (spec
+// docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md §3),
+// as `http_listen_on.nova` serves: the program binds port 0, serves that
+// listener, and a client sends, on one connection, a `GET` with a path
+// parameter and a query, a form's `POST` with a cookie, a `DELETE` that gets
+// `405`, and a `GET` the fallback answers. Each answer's status line, its
+// `set-cookie` lines in order, its `location` and `allow`, and its body are
+// printed; the other headers are not, since a `Map` orders them.
+
+// One request on `s`, and its answer, as `shown` writes it.
+async fn ask(s: TcpStream, request: String) -> String {
+    let _ = match s.write(bytes_from_string(request)).await {
+        Ok(n) => n
+        Err(e) => panic("client write: ${e.message}")
+    }
+    let mut text = ""
+    let mut done = false
+    while !done {
+        let chunk = match s.read(4096).await {
+            Ok(b) => b
+            Err(e) => panic("client read: ${e.message}")
+        }
+        if chunk.len() == 0 { panic("the server closed the connection") }
+        text = "${text}${chunk.to_string().unwrap_or("<not utf-8>")}"
+        done = framed(text)
+    }
+    shown(text)
+}
+
+// The status line of `text`, a whole response, then its `set-cookie`,
+// `location` and `allow` lines in the order they came, then its body.
+fn shown(text: String) -> String {
+    let end = text.index_of("\r\n\r\n").unwrap_or(0)
+    let lines = text.slice(0, end).split("\r\n")
+    let mut out = lines[0]
+    let mut i = 1
+    while i < lines.len() {
+        let line = lines[i]
+        let name = line.to_lower()
+        if name.starts_with("set-cookie:")
+            || name.starts_with("location:")
+            || name.starts_with("allow:") {
+            out = "${out}\n  ${line}"
+        }
+        i = i + 1
+    }
+    "${out}\n  body: ${text.slice(end + 4, text.len())}"
+}
+
+// Whether `text` holds a whole response: its head, and as many bytes of body
+// as its `content-length` says.
+fn framed(text: String) -> Bool {
+    let end = match text.index_of("\r\n\r\n") {
+        Some(at) => at
+        None => return false
+    }
+    let lines = text.slice(0, end).split("\r\n")
+    let mut want = 0
+    let mut i = 0
+    while i < lines.len() {
+        let line = lines[i].to_lower()
+        if line.starts_with("content-length: ") { want = number(line.slice(16, line.len())) }
+        i = i + 1
+    }
+    text.len() - end - 4 >= want
+}
+
+// The decimal number `digits` writes.
+fn number(digits: String) -> Int {
+    let cs = digits.chars()
+    let mut n = 0
+    let mut i = 0
+    while i < cs.len() {
+        n = n * 10 + "0123456789".index_of("${cs[i]}").unwrap_or(0)
+        i = i + 1
+    }
+    n
+}
+
+// The answer to a login form: who it names, the session it came with, and
+// two cookies back, one the browser keeps and one that removes another.
+fn log_in(r: Request) -> Response {
+    let form = r.form()
+    let sid = r.cookies().get("sid").unwrap_or("none")
+    let mut session = Cookie::new("sid", "new.${form.get("name").unwrap_or("?")}")
+    session.max_age = Some(28800)
+    let to = "/admin?from=${sid}&password=${form.get("password").unwrap_or("?")}"
+    Response::redirect(to).with_cookie(session).with_cookie(Cookie::expired("visitor"))
+}
+
+async fn main() {
+    let l = match bind("127.0.0.1:0") {
+        Ok(l) => l
+        Err(e) => panic("bind: ${e.message}")
+    }
+    let port = match l.local_port() {
+        Ok(p) => p
+        Err(e) => panic("local_port: ${e.message}")
+    }
+    let app = Server::new()
+        .get(
+            "/posts/:id",
+            |r: Request| {
+                let tab = r.query().get("tab").unwrap_or("none")
+                Response::html(200, "<p>post ${r.params.get("id").unwrap_or("?")}, tab ${tab}</p>")
+                    .with_cookie(Cookie::new("seen", "1"))
+                    .with_cookie(Cookie::new("bad name", "2"))
+                    .with_cookie(Cookie::new("theme", "dark"))
+            },
+        )
+        .get("/login", |_| Response::html(200, "<form method=\"post\"></form>"))
+        .post("/login", log_in)
+        .fallback(|r: Request| Response::text(404, "no page at ${r.path}"))
+    let _ = spawn(app.listen_on(l))
+
+    let s = match connect("127.0.0.1:${port}").await {
+        Ok(s) => s
+        Err(e) => panic("client connect: ${e.message}")
+    }
+    println(ask(s, "GET /posts/a%20b/?tab=2 HTTP/1.1\r\nHost: h\r\n\r\n").await)
+    let form = "name=ada&password=x%26y+z"
+    println(
+        ask(
+            s,
+            "POST /login HTTP/1.1\r\nHost: h\r\nCookie: sid=old.1; theme=dark\r\nContent-Type: application/x-www-form-urlencoded\r\nContent-Length: ${form.len()}\r\n\r\n${form}",
+        ).await,
+    )
+    println(ask(s, "DELETE /login HTTP/1.1\r\nHost: h\r\n\r\n").await)
+    println(ask(s, "GET /nope?x=1 HTTP/1.1\r\nHost: h\r\n\r\n").await)
+    let _ = s.close().await
+    exit(0)
+}
diff --git a/tests/runtime/http_server_wire.stdout b/tests/runtime/http_server_wire.stdout
new file mode 100644
index 0000000000000000000000000000000000000000..27c27a046f671656c22d47e7cab06da01c1e663e
--- /dev/null
+++ b/tests/runtime/http_server_wire.stdout
@@ -0,0 +1,14 @@
+HTTP/1.1 200 OK
+  set-cookie: seen=1; Path=/; HttpOnly; SameSite=Lax
+  set-cookie: theme=dark; Path=/; HttpOnly; SameSite=Lax
+  body: <p>post a b, tab 2</p>
+HTTP/1.1 303 See Other
+  location: /admin?from=old.1&password=x&y z
+  set-cookie: sid=new.ada; Path=/; Max-Age=28800; HttpOnly; SameSite=Lax
+  set-cookie: visitor=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax
+  body: 
+HTTP/1.1 405 Method Not Allowed
+  allow: GET, POST
+  body: method not allowed
+HTTP/1.1 404 Not Found
+  body: no page at /nope?x=1
````

**Patch `t3-code`:**

<!-- patch t3-code -->

````diff
diff --git a/examples/05-json-api/src/main.nova b/examples/05-json-api/src/main.nova
index a2132b36c71950b91d466ed120e3076d1bf45788..0f0e8f745e18dc008681114304611fb5ab5aed69 100644
--- a/examples/05-json-api/src/main.nova
+++ b/examples/05-json-api/src/main.nova
@@ -129,7 +129,7 @@ fn json_response(status: Int, body: String) -> Response {
     let mut h: Map<String, String> = Map::new()
     h.insert("content-length", "${b.len()}")
     h.insert("content-type", "application/json")
-    Response { status: status, headers: h, body: b }
+    Response { status: status, headers: h, body: b, cookies: [] }
 }
 
 fn error_json(msg: String) -> String { "{\"error\":${stringify(String(msg))}}" }
diff --git a/std/http/lib.nova b/std/http/lib.nova
index 41d2e7089cf207b2daf7208e5d892a829ae7d713..d98f76215f68193715d0b8b4ab5bdd2907de6c09 100644
--- a/std/http/lib.nova
+++ b/std/http/lib.nova
@@ -7,10 +7,11 @@
 //! listener already bound. A pattern such as `/posts/:id` puts the piece of
 //! the path it matched in the request's `params`. A handler is a plain
 //! function from a `Request` to a `Response`, so it cannot await anything.
-//! `Request`'s `query`, `form` and `cookies` read what a browser sends.
-//! `read_request`, `write_response` and `parse_request_head` are the parts
-//! underneath, for serving a `std/net` `TcpStream` yourself. There is no
-//! HTTP client. Every module sees these names without an import, and a
+//! `Request`'s `query`, `form` and `cookies` read what a browser sends, and
+//! `Response`'s `html`, `redirect` and `with_cookie` build what it is sent
+//! back. `read_request`, `write_response` and `parse_request_head` are the
+//! parts underneath, for serving a `std/net` `TcpStream` yourself. There is
+//! no HTTP client. Every module sees these names without an import, and a
 //! module's own definition of one of them shadows the one here.
 //!
 //! Only `Content-Length` says how long a request's body is: chunked transfer
@@ -729,12 +730,13 @@ fn method_from(token: String) -> Method {
 // cannot move instead: the exact status line, the exact tail (the blank line
 // and the body), and that each expected header line occurs somewhere
 // between them.
-/// An HTTP response: a status code, headers and a body.
+/// An HTTP response: a status code, headers, a body, and the cookies it
+/// sets.
 ///
-/// `ok`, `text`, `json` and `not_found` build the common ones, and each sets
-/// `content-length`. A response built by hand should set it too: `Server`
-/// keeps the connection open after a response, so without it a client cannot
-/// tell where the body ends.
+/// `ok`, `text`, `html`, `json`, `redirect` and `not_found` build the common
+/// ones, and each sets `content-length`. A response built by hand should set
+/// it too: `Server` keeps the connection open after a response, so without it
+/// a client cannot tell where the body ends.
 pub record Response {
     /// The status code, such as 200 or 404. It is written as given, without
     /// a check.
@@ -744,6 +746,10 @@ pub record Response {
     pub headers: Map<String, String>,
     /// The body.
     pub body: Bytes,
+    /// The cookies the response sets, in order, each written as a
+    /// `set-cookie` header of its own: a `Map` of headers holds one value
+    /// per name, so they cannot be headers. `with_cookie` adds one.
+    pub cookies: [Cookie],
 }
 
 impl Response {
@@ -753,7 +759,7 @@ impl Response {
     pub fn ok(body: Bytes) -> Response {
         let mut h: Map<String, String> = Map::new()
         h.insert("content-length", "${body.len()}")
-        Response { status: 200, headers: h, body: body }
+        Response { status: 200, headers: h, body: body, cookies: [] }
     }
 
     // `status` carrying `s` as UTF-8 text.
@@ -765,7 +771,84 @@ impl Response {
         let mut h: Map<String, String> = Map::new()
         h.insert("content-length", "${body.len()}")
         h.insert("content-type", "text/plain; charset=utf-8")
-        Response { status: status, headers: h, body: body }
+        Response { status: status, headers: h, body: body, cookies: [] }
+    }
+
+    // `text`'s twin for HTML (spec
+    // docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md
+    // §3.3). `s` is written as given: escaping is the caller's, as `std/ui`'s
+    // renderers do it.
+    /// Returns a response with status `status` carrying `s` as an HTML page,
+    /// with `content-type` set to `text/html; charset=utf-8` and
+    /// `content-length` to its length in bytes. `s` is sent as given, so
+    /// text from a visitor in it must already be escaped, as `std/ui`
+    /// escapes what it renders.
+    ///
+    /// ```
+    /// let r = Response::html(201, "<p>Saved</p>")
+    /// assert_eq(r.status, 201)
+    /// assert_eq(r.headers.get("content-type").unwrap(), "text/html; charset=utf-8")
+    /// assert_eq(r.headers.get("content-length").unwrap(), "12")
+    /// assert_eq(r.body.to_string().unwrap(), "<p>Saved</p>")
+    /// ```
+    pub fn html(status: Int, s: String) -> Response {
+        let body = bytes_from_string(s)
+        let mut h: Map<String, String> = Map::new()
+        h.insert("content-length", "${body.len()}")
+        h.insert("content-type", "text/html; charset=utf-8")
+        Response { status: status, headers: h, body: body, cookies: [] }
+    }
+
+    // `303 See Other`, the answer to a form's `POST` that succeeded, so the
+    // browser follows it with a `GET` (post, redirect, get).
+    /// Returns a `303 See Other` response sending the browser to `location`,
+    /// which it then asks for with a `GET`: the `location` header set to it,
+    /// `content-length` to `0`, and no body. A `location` holding a carriage
+    /// return or a line feed is left out by `to_bytes`, as any such header
+    /// is.
+    ///
+    /// ```
+    /// let r = Response::redirect("/posts/7#comments")
+    /// assert_eq(r.status, 303)
+    /// assert_eq(r.headers.get("location").unwrap(), "/posts/7#comments")
+    /// assert_eq(r.headers.get("content-length").unwrap(), "0")
+    /// assert(r.to_bytes().to_string().unwrap().starts_with("HTTP/1.1 303 See Other\r\n"), "303")
+    /// ```
+    pub fn redirect(location: String) -> Response {
+        let mut h: Map<String, String> = Map::new()
+        h.insert("location", location)
+        h.insert("content-length", "0")
+        Response { status: 303, headers: h, body: bytes_from_string(""), cookies: [] }
+    }
+
+    // A copy with one more cookie. The new array is built whole, so `self`'s
+    // own stays as it was, as `Server::get` leaves its receiver's routes.
+    /// Returns a copy of this response that also sets `c`, after the cookies
+    /// it already sets.
+    ///
+    /// `self` is left unchanged, so calls chain. Each cookie is written as a
+    /// `set-cookie` header of its own, after the other headers; one that
+    /// `Cookie` says cannot be written is left out.
+    ///
+    /// ```
+    /// let r = Response::redirect("/")
+    ///     .with_cookie(Cookie::new("sid", "abc.123"))
+    ///     .with_cookie(Cookie::expired("old"))
+    /// let wire = r.to_bytes().to_string().unwrap()
+    /// assert(wire.contains("\r\nset-cookie: sid=abc.123; Path=/; HttpOnly; SameSite=Lax\r\n"), wire)
+    /// assert(wire.contains("\r\nset-cookie: old=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax\r\n"), wire)
+    /// ```
+    pub fn with_cookie(self, c: Cookie) -> Response {
+        let n = self.cookies.len()
+        // `[c; n + 1]` holds `c` in every slot; each slot but the last is
+        // overwritten with the cookie already there.
+        let mut all = [c; n + 1]
+        let mut i = 0
+        while i < n {
+            all[i] = self.cookies[i]
+            i = i + 1
+        }
+        Response { status: self.status, headers: self.headers, body: self.body, cookies: all }
     }
 
     /// Returns a `404 Not Found` response whose body is the text `not found`.
@@ -788,7 +871,7 @@ impl Response {
         let mut h: Map<String, String> = Map::new()
         h.insert("content-length", "${body.len()}")
         h.insert("content-type", "application/json")
-        Response { status: 200, headers: h, body: body }
+        Response { status: 200, headers: h, body: body, cookies: [] }
     }
 
     // The response as it goes on the wire.
@@ -798,15 +881,18 @@ impl Response {
     // That is the design's decision rather than a gap -- see
     // docs/adr/0019-offset-table-intrinsic-boundary.md.
     /// Returns the response as it is sent: the status line, one line per
-    /// header, a blank line, and the body. Each line ends with a carriage
-    /// return and a line feed.
+    /// header, one `set-cookie` line per cookie, a blank line, and the body.
+    /// Each line ends with a carriage return and a line feed.
     ///
     /// The status line holds `HTTP/1.1`, the status code and a reason phrase:
-    /// the usual one for 200, 201, 204, 400, 404, 405, 413 and 500, and for
-    /// any other code a phrase for its class, such as `Client Error`. Each
-    /// header is written `name: value`. A header whose name or value holds a
-    /// carriage return or a line feed is left out, so it cannot add lines of
-    /// its own. Nothing else is added: no header the response does not hold.
+    /// the usual one for 200, 201, 204, 301, 302, 303, 400, 401, 403, 404,
+    /// 405, 413 and 500, and for any other code a phrase for its class, such
+    /// as `Client Error`. Each header is written `name: value`. A header
+    /// whose name or value holds a carriage return or a line feed is left
+    /// out, so it cannot add lines of its own. The cookies come after the
+    /// headers, in order, each written as `Cookie` says, and one it says
+    /// cannot be written is left out. Nothing else is added: no header the
+    /// response does not hold.
     pub fn to_bytes(self) -> Bytes {
         let mut head = "HTTP/1.1 ${self.status} ${reason_phrase(self.status)}\r\n"
         let names = self.headers.keys()
@@ -824,10 +910,194 @@ impl Response {
             }
             i = i + 1
         }
+        // After the headers, so the order of the cookies, which a `Map` does
+        // not decide, is the order they were added in.
+        let mut c = 0
+        while c < self.cookies.len() {
+            match cookie_line(self.cookies[c]) {
+                Some(line) => head = "${head}set-cookie: ${line}\r\n"
+                None => {}
+            }
+            c = c + 1
+        }
         bytes_from_string("${head}\r\n").concat(self.body)
     }
 }
 
+// A cookie a response sets: one `Set-Cookie` header (RFC 6265 section 4.1),
+// a field of `Response` of its own rather than a header map holding several
+// values (spec docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md
+// §3.3 and §12, decision 5). `Cookie::new`'s defaults are the safe ones
+// (decision 6). There is no `Domain` and no `Expires`: a cookie is the
+// host's own, and `Max-Age` says how long it lasts, since std/time writes no
+// HTTP date.
+//
+// **What is refused, and why each rule is the one it is.** The serialiser
+// is the right layer to refuse a cookie that would forge a header or an
+// attribute, for the reason `is_crlf_free` gives for headers: it cannot see
+// where a value came from. A name must be an HTTP token (RFC 9110 section
+// 5.6.2), as RFC 6265's cookie-name is; a value must be RFC 6265's
+// `*cookie-octet`, so no space, `"`, `,`, `;` or `\`, no control character
+// and nothing outside ASCII -- the quoted form RFC 6265 also allows is not
+// written, a choice that keeps one rule; a path must be RFC 6265's
+// path-value, any CHAR but a control character or `;`; and `same_site`,
+// which the spec leaves open, must be empty or a token, so it cannot carry
+// a `;` and an attribute of its own either.
+/// A cookie for a response to set, with `Response::with_cookie`.
+///
+/// `Cookie::new` gives the safe attributes, which the fields can change
+/// afterwards in a `let mut` binding. `Response::to_bytes` writes a cookie
+/// as one `set-cookie` header: `name=value`, then each attribute that
+/// applies, in this order: `Path` when `path` is not empty, `Max-Age` when
+/// `max_age` holds a number, `HttpOnly`, `SameSite` when `same_site` is not
+/// empty, and `Secure`. There is no `Domain` and no `Expires`.
+///
+/// A cookie is left out, as a header holding a carriage return or a line
+/// feed is, when one of its parts holds what a cookie cannot carry:
+///
+/// - a name that is not an HTTP token: one or more characters, each an ASCII
+///   letter or digit or one of ``!#$%&'*+-.^_`|~``;
+/// - a value holding anything but RFC 6265's cookie-octets, the ASCII
+///   characters from `!` to `~` less `"`, `,`, `;` and `\`: so no space, no
+///   control character and nothing outside ASCII. An empty value is a
+///   value, and one in double quotes is refused;
+/// - a path holding a control character (U+0000 to U+001F, or U+007F), a
+///   `;`, or anything outside ASCII;
+/// - a `same_site` that is neither empty nor a token.
+///
+/// A value that needs other characters can be encoded first, as
+/// `Bytes::to_hex` encodes bytes.
+pub record Cookie {
+    /// The cookie's name, such as `sid`.
+    pub name: String,
+    /// Its value, written as given.
+    pub value: String,
+    /// The path whose requests the browser sends it with, and those below
+    /// it: `/` for every path. `""` writes no `Path`, and the browser takes
+    /// the directory of the page that set it.
+    pub path: String,
+    /// How many seconds the cookie lasts, written as `Max-Age`; `Some(0)`
+    /// removes it. `None` writes no `Max-Age`, so it lasts until the
+    /// browser's session ends.
+    pub max_age: Option<Int>,
+    /// Whether the page's scripts are kept from reading it: `HttpOnly`.
+    pub http_only: Bool,
+    /// When the browser sends it with a request another site starts:
+    /// `"Strict"`, `"Lax"` or `"None"`, written as `SameSite`. `""` writes no
+    /// `SameSite`.
+    pub same_site: String,
+    /// Whether the browser sends it only over HTTPS: `Secure`.
+    pub secure: Bool,
+}
+
+impl Cookie {
+    /// Returns the cookie `name` holding `value`, with the safe attributes:
+    /// path `/`, no `Max-Age`, so it lasts until the browser's session ends,
+    /// `HttpOnly`, `SameSite=Lax`, and not `Secure`, so it is sent over HTTP
+    /// too.
+    ///
+    /// ```
+    /// let c = Cookie::new("theme", "dark")
+    /// assert_eq(c.path, "/")
+    /// assert(c.max_age.is_none(), "a cookie for the browser's session")
+    /// assert(c.http_only && !c.secure, "HttpOnly, and not only over HTTPS")
+    /// assert_eq(c.same_site, "Lax")
+    ///
+    /// // Fields change in a `let mut` binding: a cookie that lasts a day.
+    /// let mut day = Cookie::new("seen", "1")
+    /// day.max_age = Some(86400)
+    /// let wire = Response::ok(bytes_from_string("")).with_cookie(day).to_bytes().to_string().unwrap()
+    /// assert(wire.contains("set-cookie: seen=1; Path=/; Max-Age=86400; HttpOnly; SameSite=Lax\r\n"), wire)
+    /// ```
+    pub fn new(name: String, value: String) -> Cookie {
+        Cookie {
+            name: name,
+            value: value,
+            path: "/",
+            max_age: None,
+            http_only: true,
+            same_site: "Lax",
+            secure: false,
+        }
+    }
+
+    /// Returns the cookie that removes the cookie `name` from the browser:
+    /// `Cookie::new(name, "")` with `Max-Age=0`. A cookie is replaced only by
+    /// one of the same name and path, so this one's path, `/`, must be the
+    /// path the cookie was set with.
+    ///
+    /// ```
+    /// let c = Cookie::expired("sid")
+    /// assert_eq(c.value, "")
+    /// assert_eq(c.max_age.unwrap(), 0)
+    /// assert_eq(c.path, "/")
+    /// ```
+    pub fn expired(name: String) -> Cookie {
+        let mut c = Cookie::new(name, "")
+        c.max_age = Some(0)
+        c
+    }
+}
+
+// `c` as a `Set-Cookie` header's value, or `None` when a part of it holds
+// what a cookie cannot carry (`Cookie`'s comment).
+fn cookie_line(c: Cookie) -> Option<String> {
+    if !is_token(c.name) { return None }
+    if !all_chars(c.value, is_cookie_octet) { return None }
+    if !all_chars(c.path, is_path_char) { return None }
+    if c.same_site != "" && !is_token(c.same_site) { return None }
+    let mut line = "${c.name}=${c.value}"
+    if c.path != "" { line = "${line}; Path=${c.path}" }
+    match c.max_age {
+        Some(n) => line = "${line}; Max-Age=${n}"
+        None => {}
+    }
+    if c.http_only { line = "${line}; HttpOnly" }
+    if c.same_site != "" { line = "${line}; SameSite=${c.same_site}" }
+    if c.secure { line = "${line}; Secure" }
+    Some(line)
+}
+
+// Whether `s` is an HTTP token (RFC 9110 section 5.6.2): one or more
+// `tchar`s, ASCII letters and digits and ``!#$%&'*+-.^_`|~``.
+fn is_token(s: String) -> Bool { s != "" && all_chars(s, is_tchar) }
+
+fn is_tchar(code: Int) -> Bool {
+    // Digits, then upper- and lower-case letters.
+    if code >= 48 && code <= 57 { return true }
+    if code >= 65 && code <= 90 { return true }
+    if code >= 97 && code <= 122 { return true }
+    // ! # $ % & ' * + - . ^ _ ` | ~
+    if code == 33 || (code >= 35 && code <= 39) || code == 42 || code == 43 { return true }
+    if code == 45 || code == 46 || code == 94 || code == 95 || code == 96 { return true }
+    code == 124 || code == 126
+}
+
+// RFC 6265's cookie-octet: %x21, %x23-2B, %x2D-3A, %x3C-5B and %x5D-7E,
+// which leaves out controls, space, `"`, `,`, `;`, `\`, DEL and all beyond
+// ASCII.
+fn is_cookie_octet(code: Int) -> Bool {
+    if code == 33 || (code >= 35 && code <= 43) || (code >= 45 && code <= 58) { return true }
+    // A line may not start with `(`: it would call the `if` above.
+    let upper = code >= 60 && code <= 91
+    upper || (code >= 93 && code <= 126)
+}
+
+// RFC 6265's path-value: any CHAR (US-ASCII) but a control character or
+// `;`, so U+0020 to U+007E less `;`.
+fn is_path_char(code: Int) -> Bool { code >= 32 && code <= 126 && code != 59 }
+
+// Whether `ok` holds for the code of each character of `s`.
+fn all_chars(s: String, ok: fn(Int) -> Bool) -> Bool {
+    let cs = s.chars()
+    let mut i = 0
+    while i < cs.len() {
+        if !ok(char_to_int(cs[i])) { return false }
+        i = i + 1
+    }
+    true
+}
+
 // Whether `s` contains no CR and no LF, and so is safe to place verbatim in
 // one header line.
 //
@@ -871,7 +1141,12 @@ fn reason_phrase(status: Int) -> String {
     if status == 200 { return "OK" }
     if status == 201 { return "Created" }
     if status == 204 { return "No Content" }
+    if status == 301 { return "Moved Permanently" }
+    if status == 302 { return "Found" }
+    if status == 303 { return "See Other" }
     if status == 400 { return "Bad Request" }
+    if status == 401 { return "Unauthorized" }
+    if status == 403 { return "Forbidden" }
     if status == 404 { return "Not Found" }
     if status == 405 { return "Method Not Allowed" }
     if status == 413 { return "Payload Too Large" }
@@ -1314,12 +1589,12 @@ impl Server {
     ///
     /// ```
     /// let app = Server::new()
-    ///     .get("/posts/:id", |_| Response::text(200, "the post"))
-    ///     .post("/posts/:id", |r: Request| Response::text(201, "saved ${r.params.get("id").unwrap()}"))
+    ///     .get("/posts/:id", |_| Response::html(200, "<form method=post>…</form>"))
+    ///     .post("/posts/:id", |r: Request| Response::redirect("/posts/${r.params.get("id").unwrap()}"))
     /// let raw = bytes_from_string("POST /posts/7 HTTP/1.1\r\nContent-Length: 0\r\n\r\n")
     /// let saved = app.dispatch(parse_request_head(raw, Limits::default()).unwrap().unwrap())
-    /// assert_eq(saved.status, 201)
-    /// assert_eq(saved.body.to_string().unwrap(), "saved 7")
+    /// assert_eq(saved.status, 303)
+    /// assert_eq(saved.headers.get("location").unwrap(), "/posts/7")
     /// ```
     pub fn post(self, pattern: String, handler: fn(Request) -> Response) -> Server {
         self.route(Post, pattern, handler)
@@ -1453,8 +1728,8 @@ impl Server {
     ///
     /// ```
     /// let app = Server::new()
-    ///     .get("/login", |_| Response::text(200, "the form"))
-    ///     .post("/login", |_| Response::text(200, "logged in"))
+    ///     .get("/login", |_| Response::html(200, "<form>…</form>"))
+    ///     .post("/login", |_| Response::redirect("/admin"))
     /// let raw = bytes_from_string("DELETE /login HTTP/1.1\r\n\r\n")
     /// let refused = app.dispatch(parse_request_head(raw, Limits::default()).unwrap().unwrap())
     /// assert_eq(refused.status, 405)
diff --git a/std/ui/ssr/lib.nova b/std/ui/ssr/lib.nova
index f0be9d53ed9213bb71fadf9837f49d0230e3be11..7f50ce9df35b164f5bfed16eb36f0796bb7e7ea6 100644
--- a/std/ui/ssr/lib.nova
+++ b/std/ui/ssr/lib.nova
@@ -355,7 +355,7 @@ fn answer(s: Served, req: Request) -> Response {
     h.insert("content-length", "${body.len()}")
     h.insert("content-type", "text/html; charset=utf-8")
     let status = if found(s.routes, target) { 200 } else { 404 }
-    Response { status: status, headers: h, body: body }
+    Response { status: status, headers: h, body: body, cookies: [] }
 }
 
 // `200` and the file at `path` under `dist`, read now, or `404` when it
@@ -368,7 +368,7 @@ fn file(dist: String, path: String) -> Response {
     h.insert("content-length", "${body.len()}")
     h.insert("content-type", content_type(path))
     h.insert("x-content-type-options", "nosniff")
-    Response { status: 200, headers: h, body: body }
+    Response { status: 200, headers: h, body: body, cookies: [] }
 }
 
 // The `Content-Type` of a file named `path`, by its extension in any case,
````

### Task 4: `ui::ssr::Dist`

Spec §4; decisions 7 and 19.

**Files:**
- Modify: `std/ui/ssr/lib.nova` (`Dist`, `open`, `file`, `respond`,
  `site_path`; `Served`, `serve` and `answer` on a `Dist`)
- Test:
  - `tests/ssr/site/src/dist.nova`
  - `crates/nova-cli/tests/run_tests.rs`

- [ ] **Step 1: The tests.** `patch_block t4-tests`. The patch holds:
  - **`tests/ssr/site/src/dist.nova`,** a program of 4.6's project, with
    no socket: it opens the built site in the directory its first argument
    names, `dist` when there is none, and prints the error and exits with 3
    when that fails; then prints what `file` answers for each of fourteen
    targets, whether `file` answers a `POST`, and the page `respond` makes
    for `/posts/7` with the data and the status `201`.
  - **`run_tests.rs`'s `ssr_dist_answers_files_and_pages`,** in a fresh
    copy of the project with `dist/` written by hand, as 4.6's `ssr_*`
    tests do: `file` answers `app.js`, asked for as it is, as `/app%2ejs`,
    with a query and with no leading `/`, and the SVG, the WASM module and
    `index.html`, each `200` with its content type, `nosniff` and its
    `content-length`; `/gone.js`, listed and not on disk, `404`; and `None`
    for `/../secret`, `/..%2fsecret`, which the manifest lists as
    `../secret` and `file` never reads, a file in `dist/` the manifest does
    not list, the manifest, `/` and `/posts/7`; a `POST` of `/app.js` is
    answered too. `respond` puts the page into the template with its status
    and type, and a template whose root SSG filled is emptied first. Then
    `Dist::open`'s errors, each with exit 3 and `serve`'s words: no
    template, a template with no root, no manifest, and a manifest with no
    `files`.
- [ ] **Step 2: Run, and see them fail.** First the build (Conventions).
  - `R ssr_dist_answers_files_and_pages`. Expected: `0 passed; 1 failed`:
    "Unexpected failure." of `nova run src/dist.nova`: ``error[E0001]:
    `Dist` is not a public item of module `ui::ssr` ``, `could not compile
    due to 1 previous error`.
  - `R ssr_` (about 10 s). Expected: `4 passed; 1 failed`: the same; 4.6's
    four `ssr_` tests pass, since the project's new program touches none of
    them.
- [ ] **Step 3: The code.** `patch_block t4-code`, `std/ui/ssr/lib.nova`:
  `pub record Dist`, its fields private; `Dist::open`, an `async fn` that
  never suspends, which reads the template, empties its root, and reads the
  manifest, dropping a path that could leave the directory, with `serve`'s
  errors; `file`, through `site_path`, the target before `?`, decoded, less
  one `/`, which answers a listed file as `serve` did, or `None`, whatever
  the method (decision 19); and `respond`, `Response::html(status,
  document(template, page))` (decision 7); each with its doc test, but
  `open`'s, which is `ignore`. `Served` holds a `Dist`, and `serve` and
  `answer` use it, answering as before. The module's doc names `Dist`.
- [ ] **Step 4: Run.** First the build.
  - The same two commands. Expected: `R ssr_dist_answers_files_and_pages`
    `1 passed` (about 12 s); `R ssr_` `5 passed`.
  - The whole binaries:
    - `cargo test -p nova-std`. Expected: `2 passed`.
    - `cargo test -p nova-doc --lib`. Expected: `61 passed`.
    - `cargo test -p nova-cli --test doc_test`. Expected: `17 passed`:
      `Dist::file`'s and `respond`'s examples run.
    - `R`. Expected: `326 passed; 0 failed; 1 ignored`.
    - `W`. Expected: `230 passed`.
    - `cargo test -p nova-cli --test ssr_tests`. Expected: `9 passed`:
      `serve`, rebuilt on `Dist`, answers as before.
- [ ] **Step 5: The checks,** as the conventions list them; `nova fmt`'s
  `tests/ssr` takes `dist.nova`.
- [ ] **Step 6: Commit** `std/ui/ssr: Dist, the built site for a hand-written server`.

**Patch `t4-tests`:**

<!-- patch t4-tests -->

````diff
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index 0c2b79430b0f1844481e0627bcecc4c6b6798a8a..dda89575a6728e4804b47f0b83dfeceb30bbe957 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -12299,6 +12299,136 @@ fn ssr_serve_reports_what_stops_it_starting() {
     );
 }
 
+/// `ui::ssr::Dist` (spec
+/// `docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md`
+/// §4), with no socket, through `tests/ssr/site`'s `src/dist.nova`: `file`
+/// answers a listed file, asked for as it is, with an escape, with a query
+/// or with no leading `/`, with its content type, `nosniff` and its
+/// `content-length`; a listed file not on disk with `404`; and `None` for a
+/// file in `dist/` the manifest does not list, for the manifest's path that
+/// would leave `dist/`, which it never reads, and for a page's path. It does
+/// not look at the method. `respond` puts a page into the template with its
+/// status, a template whose root holds an old page emptied first. Then
+/// `Dist::open`'s errors, each naming its file in `serve`'s words: no
+/// template, a template with no root, no manifest, and a manifest with no
+/// `files`.
+#[test]
+fn ssr_dist_answers_files_and_pages() {
+    let (site, home) = ssr_site("dist");
+    let run = |args: &[&str]| {
+        nova()
+            .current_dir(&site)
+            .env("NOVA_HOME", &home)
+            .env("NO_COLOR", "1")
+            .env("RUST_BACKTRACE", "0")
+            .args(["run", "src/dist.nova", "--"])
+            .args(args)
+            .assert()
+    };
+    let respond = "respond: 201 text/html; charset=utf-8 true\n";
+    let page_view = ssr_view("<p>Post 7</p>");
+
+    let a = run(&[]).success();
+    let out = String::from_utf8_lossy(&a.get_output().stdout).replace("\r\n", "\n");
+    let (files, page) = out.split_once(respond).expect("the respond line");
+    let js = "200 text/javascript; charset=utf-8 nosniff 34 bytes true \
+              [export default function init() {}\n]";
+    let expected = [
+        format!("file /app.js: {js}"),
+        format!("file /app%2ejs: {js}"),
+        format!("file /app.js?v=1: {js}"),
+        format!("file app.js: {js}"),
+        "file /assets/logo.svg: 200 image/svg+xml nosniff 11 bytes true [<svg></svg>]".to_string(),
+        "file /app.wasm: 200 application/wasm nosniff 8 bytes true".to_string(),
+        format!(
+            "file /index.html: 200 text/html; charset=utf-8 nosniff {} bytes true [{SSR_TEMPLATE}]",
+            SSR_TEMPLATE.len()
+        ),
+        "file /gone.js: 404 text/plain; charset=utf-8 - 9 bytes true [not found]".to_string(),
+        "file /../secret: None".to_string(),
+        "file /..%2fsecret: None".to_string(),
+        "file /secret.txt: None".to_string(),
+        "file /manifest.json: None".to_string(),
+        "file /: None".to_string(),
+        "file /posts/7: None".to_string(),
+        "file of a POST: true".to_string(),
+    ];
+    assert_eq!(files, format!("{}\n", expected.join("\n")));
+    let page = page.strip_suffix('\n').unwrap_or(page).to_string();
+    assert!(
+        ssr_pages(SSR_TEMPLATE, &page_view).contains(&page),
+        "{page}"
+    );
+
+    // A template SSG filled: its root's page and data are dropped first.
+    let filled = site.join("filled");
+    std::fs::create_dir_all(&filled).expect("a directory");
+    std::fs::write(
+        filled.join("index.html"),
+        SSR_TEMPLATE.replacen(
+            "<div id=\"app\"></div>",
+            "<div id=\"app\"><p>old</p></div><script type=\"application/json\" id=\"nova-data\">[0]</script>",
+            1,
+        ),
+    )
+    .expect("a template");
+    std::fs::write(filled.join("manifest.json"), "{\"files\": {}}\n").expect("a manifest");
+    let a = run(&["filled"]).success();
+    let out = String::from_utf8_lossy(&a.get_output().stdout).replace("\r\n", "\n");
+    let (_, page) = out.split_once(respond).expect("the respond line");
+    let page = page.strip_suffix('\n').unwrap_or(page).to_string();
+    assert!(
+        ssr_pages(SSR_TEMPLATE, &page_view).contains(&page),
+        "{page}"
+    );
+
+    let rooted = "<div id=\"app\"></div>\n";
+    for (dir, template, manifest, says) in [
+        (
+            "missing",
+            None,
+            None,
+            "ssr: cannot read missing/index.html: ",
+        ),
+        (
+            "rootless",
+            Some("<div id=\"root\"></div>\n"),
+            Some("{\"files\": {}}\n"),
+            "ssr: rootless/index.html has no <div id=\"app\">\n",
+        ),
+        (
+            "unlisted",
+            Some(rooted),
+            None,
+            "ssr: cannot read unlisted/manifest.json: ",
+        ),
+        (
+            "fileless",
+            Some(rooted),
+            Some("{\"name\": \"site\"}\n"),
+            "ssr: fileless/manifest.json has no \"files\" object\n",
+        ),
+    ] {
+        let d = site.join(dir);
+        std::fs::create_dir_all(&d).expect("a directory");
+        if let Some(t) = template {
+            std::fs::write(d.join("index.html"), t).expect("a template");
+        }
+        if let Some(m) = manifest {
+            std::fs::write(d.join("manifest.json"), m).expect("a manifest");
+        }
+        let a = run(&[dir]).code(3);
+        let o = a.get_output();
+        assert_eq!(String::from_utf8_lossy(&o.stdout), "", "{dir}");
+        let err = String::from_utf8_lossy(&o.stderr).replace("\r\n", "\n");
+        if says.ends_with('\n') {
+            assert_eq!(err, says, "{dir}");
+        } else {
+            assert!(err.starts_with(says), "{dir}: {err}");
+        }
+    }
+}
+
 /// SSG (spec 4.6 §6.4): `serve` given `--ssg <dir>` binds nothing, and
 /// writes a page for each static route, `/` over the template it read, the
 /// others in directories it makes, skipping the route with a parameter and
diff --git a/tests/ssr/site/src/dist.nova b/tests/ssr/site/src/dist.nova
new file mode 100644
index 0000000000000000000000000000000000000000..368e019c66d2b883390cd09384559e554c9fb262
--- /dev/null
+++ b/tests/ssr/site/src/dist.nova
@@ -0,0 +1,86 @@
+//! `ui::ssr::Dist` with no socket (spec
+//! docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md
+//! §4): it opens the built site in the directory its first argument names,
+//! `dist` when there is none, printing the error and exiting with 3 when that
+//! fails; then prints what `file` answers for each target, and the page
+//! `respond` makes for `/posts/7` with the data, as `run_tests.rs` checks.
+
+import app::{App, data}
+import ui
+import ui::ssr::{Dist, page_with}
+
+fn get(target: String) -> Request {
+    Request {
+        method: Get,
+        path: target,
+        headers: Map::new(),
+        body: bytes_from_string(""),
+        params: Map::new(),
+    }
+}
+
+// What `file` answers for `target`: `None`, or the status, the content type,
+// `nosniff`, the body's length, whether `content-length` says it, and the
+// body itself when it is text.
+fn asked(dist: Dist, target: String) -> String {
+    match dist.file(get(target)) {
+        Some(r) => {
+            let kind = r.headers.get("content-type").unwrap_or("-")
+            let sniff = r.headers.get("x-content-type-options").unwrap_or("-")
+            let length = r.headers.get("content-length").unwrap_or("-") == "${r.body.len()}"
+            let head = "${r.status} ${kind} ${sniff} ${r.body.len()} bytes ${length}"
+            if kind.starts_with("text/") || kind.starts_with("image/svg") {
+                "${head} [${r.body.to_string().unwrap_or("<not utf-8>")}]"
+            } else {
+                head
+            }
+        }
+        None => "None"
+    }
+}
+
+async fn main() {
+    let dir = args().get(1).unwrap_or("dist")
+    let dist = match Dist::open(dir).await {
+        Ok(d) => d
+        Err(e) => {
+            eprintln(e.message)
+            exit(3)
+            return
+        }
+    }
+    let targets = [
+        "/app.js",
+        "/app%2ejs",
+        "/app.js?v=1",
+        "app.js",
+        "/assets/logo.svg",
+        "/app.wasm",
+        "/index.html",
+        "/gone.js",
+        "/../secret",
+        "/..%2fsecret",
+        "/secret.txt",
+        "/manifest.json",
+        "/",
+        "/posts/7",
+    ]
+    let mut i = 0
+    while i < targets.len() {
+        println("file ${targets[i]}: ${asked(dist, targets[i])}")
+        i = i + 1
+    }
+    let post = Request {
+        method: Post,
+        path: "/app.js",
+        headers: Map::new(),
+        body: bytes_from_string(""),
+        params: Map::new(),
+    }
+    println("file of a POST: ${dist.file(post).is_some()}")
+    let page = dist.respond(201, page_with("/posts/7", data(), App))
+    println(
+        "respond: ${page.status} ${page.headers.get("content-type").unwrap_or("-")} ${page.headers.get("content-length").unwrap_or("-") == "${page.body.len()}"}",
+    )
+    println(page.body.to_string().unwrap_or("<not utf-8>"))
+}
````

**Patch `t4-code`:**

<!-- patch t4-code -->

````diff
diff --git a/std/ui/ssr/lib.nova b/std/ui/ssr/lib.nova
index 7f50ce9df35b164f5bfed16eb36f0796bb7e7ea6..4b3256447aef2f5de9c1e94c20ca50351d9211c7 100644
--- a/std/ui/ssr/lib.nova
+++ b/std/ui/ssr/lib.nova
@@ -13,6 +13,12 @@
 //! with `--ssg` and the client's output directory, which may be absolute,
 //! and the server writes a page there for each static route.
 //!
+//! A server that needs more than `serve`, routes of its own, a cookie, or a
+//! status a page decides, is `std/http`'s `Server` with its own routes, and
+//! `Dist` is the built site it serves beside them: `Dist::open` reads it,
+//! `file` answers a request for one of its files, and `respond` answers with
+//! a page.
+//!
 //! `page` renders the view at a location, `page_with` with the server's data
 //! beside it, and `document` puts a page into the built `index.html`, in its
 //! `<div id="app">`:
@@ -45,9 +51,14 @@
 // docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md
 // §6). The low level is three functions of strings: `page` and `page_with`
 // render a view at a location through `std/ui/router`'s `with_location`,
-// and `document` fills the template. `serve` is `std/http`'s `Server` with
-// one fallback that answers every `GET`: a file the build's manifest lists,
-// a 404 for any other name with a `.`, or a page. A file is read when it is
+// and `document` fills the template. `Dist` (spec
+// docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md §4)
+// is what `serve` is made of, for a server written by hand: the template and
+// the manifest, read once by `Dist::open`; `file`, which answers for a file
+// the manifest lists; and `respond`, which answers with a page. `serve` is
+// `std/http`'s `Server` with one fallback, on a `Dist`, that answers every
+// `GET`: a file the build's manifest lists, a 404 for any other name with a
+// `.`, or a page. A file is read when it is
 // asked for, through the builtins `std/fs` is written on, since a handler
 // cannot await; no path outside the manifest is ever read, so there is no
 // traversal to refuse; and a manifest's path that would leave `dist`, with
@@ -269,28 +280,9 @@ pub async fn serve(site: Site) -> Result<(), IoError> {
         Some(dir) => return generate(site, dir).await
         None => {}
     }
-    let index = "${site.dist}/index.html"
-    let template = match read_to_string(index).await {
-        Ok(t) => {
-            match emptied(t) {
-                Some(t) => t
-                None => return Err(rootless("ssr", index))
-            }
-        }
-        Err(e) => return Err(unreadable("ssr", index, e))
-    }
-    let listing = "${site.dist}/manifest.json"
-    let text = match read_to_string(listing).await {
-        Ok(t) => t
-        Err(e) => return Err(unreadable("ssr", listing, e))
-    }
-    let files = match listed(text) {
-        Some(files) => files
-        None => {
-            return Err(
-                IoError { kind: InvalidData, message: "ssr: ${listing} has no \"files\" object" },
-            )
-        }
+    let dist = match Dist::open(site.dist).await {
+        Ok(d) => d
+        Err(e) => return Err(e)
     }
     // As `Server::listen` does before its `bind`: from here a signal stops
     // the server gracefully, even before `listen_on` runs.
@@ -312,63 +304,220 @@ pub async fn serve(site: Site) -> Result<(), IoError> {
     }
     println("ssr: serving http://${host_of(site.address)}:${port}/")
     let _ = stdout().flush().await
-    let served = Served {
-        dist: site.dist,
-        files: files,
-        template: template,
-        routes: site.routes,
-        render: site.render,
-    }
+    let served = Served { dist: dist, routes: site.routes, render: site.render }
     Server::new().fallback(|r: Request| answer(served, r)).listen_on(l).await
 }
 
-// What each request is answered from: the site, and what `serve` read when
-// it started.
-record Served {
-    dist: String,
+// What each request is answered from: what `serve` read when it started,
+// and the site's routes and pages.
+record Served { dist: Dist, routes: [Route], render: fn(String) -> Page }
+
+// The response to a `GET` of `req.path` (spec 4.6 §6.3). `Server` sends any
+// other method `not_found` before it asks.
+fn answer(s: Served, req: Request) -> Response {
+    match s.dist.file(req) {
+        Some(listed) => return listed
+        None => {}
+    }
+    let pieces = site_path(req.path).split("/")
+    if pieces[pieces.len() - 1].contains(".") { return Response::not_found() }
+    let render = s.render
+    let status = if found(s.routes, req.path) { 200 } else { 404 }
+    s.dist.respond(status, render(req.path))
+}
+
+/// The site `nova build --ssr` built, as a server written by hand serves it:
+/// the template each page is put into, its `index.html`, and the files its
+/// `manifest.json` lists. `Dist::open` reads both, `file` answers a request
+/// for one of the files, and `respond` answers with a page.
+///
+/// A program's own `Server` serves it beside routes of its own, which can
+/// set a cookie, choose a status or redirect, by asking `file` first in its
+/// fallback:
+///
+/// ```ignore
+/// let app = Server::new()
+///     .get("/posts/:id", |req: Request| post_page(store, dist, req))
+///     .post("/posts/:id/comments", |req: Request| add_comment(store, req))
+///     .fallback(|req: Request| match dist.file(req) {
+///         Some(file) => file
+///         None => dist.respond(404, page_with(req.path, not_found_data(), App))
+///     })
+/// ```
+pub record Dist {
+    // The directory, as `open` was given it.
+    dir: String,
+    // The manifest's `files`, less those `inside` leaves out.
     files: Map<String, JsonValue>,
+    // The template, emptied.
     template: String,
-    routes: [Route],
-    render: fn(String) -> Page,
 }
 
-// The response to a `GET` of `req.path` (spec §6.3). `Server` sends any
-// other method `not_found` before it asks.
-fn answer(s: Served, req: Request) -> Response {
-    let target = req.path
+impl Dist {
+    // `serve`'s reading of the template and the manifest (spec
+    // docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md
+    // §4), every check and every message as it was, so `serve`'s own tests
+    // pin this.
+    /// Reads the site `nova build --ssr` wrote into `dir`, such as `"dist"`:
+    /// `<dir>/index.html`, the template, and `<dir>/manifest.json`, whose
+    /// `files` are the files `file` serves.
+    ///
+    /// Returns an `IoError` naming the file when either cannot be read,
+    /// `ssr: cannot read <file>: ` and the reason, with the kind reading it
+    /// failed with; when the template has no `<div id="app">`,
+    /// `ssr: <dir>/index.html has no <div id="app">`; and when the manifest
+    /// has no `files` object, `ssr: <dir>/manifest.json has no "files"
+    /// object`, both of kind `InvalidData`. A template whose root holds a
+    /// page, as the one SSG writes for `/` does, or a placeholder, is a
+    /// template too: its root is emptied, and the data after it dropped, as
+    /// `document` empties it. A manifest's path with a piece that is empty,
+    /// `.` or `..`, or that holds `\` or NUL, which could name a file outside
+    /// `dir`, is left out, so `file` never serves it. No file is read again
+    /// but those `file` reads, so a change to the template or the manifest
+    /// takes effect once the program opens the site again.
+    ///
+    /// An `async fn` that never suspends, as `std/fs`'s functions are: call
+    /// it once, before serving.
+    ///
+    /// ```ignore
+    /// import ui
+    /// import ui::ssr::{Dist, page_with}
+    /// import app::{App, Data}
+    ///
+    /// async fn main() {
+    ///     let dist = match Dist::open("dist").await {
+    ///         Ok(d) => d
+    ///         Err(e) => {
+    ///             eprintln(e.message)
+    ///             exit(1)
+    ///             return
+    ///         }
+    ///     }
+    ///     let app = Server::new()
+    ///         .get("/", |req: Request| dist.respond(200, page_with(req.path, Data { names: [] }, App)))
+    ///         .fallback(|req: Request| dist.file(req).unwrap_or(Response::not_found()))
+    ///     app.listen("127.0.0.1:3000").await.unwrap()
+    /// }
+    /// ```
+    pub async fn open(dir: String) -> Result<Dist, IoError> {
+        let index = "${dir}/index.html"
+        let template = match read_to_string(index).await {
+            Ok(t) => {
+                match emptied(t) {
+                    Some(t) => t
+                    None => return Err(rootless("ssr", index))
+                }
+            }
+            Err(e) => return Err(unreadable("ssr", index, e))
+        }
+        let listing = "${dir}/manifest.json"
+        let text = match read_to_string(listing).await {
+            Ok(t) => t
+            Err(e) => return Err(unreadable("ssr", listing, e))
+        }
+        let files = match listed(text) {
+            Some(files) => files
+            None => {
+                return Err(
+                    IoError {
+                        kind: InvalidData,
+                        message: "ssr: ${listing} has no \"files\" object",
+                    },
+                )
+            }
+        }
+        Ok(Dist { dir: dir, files: files, template: template })
+    }
+
+    // The read is `std/fs`'s `read` without its `async`, which a handler
+    // cannot await: the builtin, then its slot, with nothing between.
+    /// Returns the answer to `req` when its path names a file the manifest
+    /// lists, and `None` when it names none, for the program to answer.
+    ///
+    /// The path is the request's target before any `?`, percent-decoded, less
+    /// its leading `/`, so `/app.js?v=2` and `/app%2Ejs` both name `app.js`.
+    /// A listed file is read now, and answered with `200`, the file, the
+    /// `Content-Type` its extension gives, in any case (`html`, `js` and
+    /// `mjs`, `wasm`, `css`, `json`, `svg`, `png`, `jpg`, `gif`, `webp`,
+    /// `ico`, `txt` and `woff2`; any other is `application/octet-stream`),
+    /// and `X-Content-Type-Options: nosniff`; or with `Response::not_found()`
+    /// when it cannot be read. No file the manifest does not list is ever
+    /// read. It does not look at the method: `Server` asks its fallback only
+    /// for a `GET`.
+    ///
+    /// ```
+    /// import ui
+    /// import ui::ssr::{Dist}
+    ///
+    /// async fn main() {
+    ///     let dir = "${temp_dir()}/nova-ssr-dist-file"
+    ///     create_dir_all(dir).await.unwrap()
+    ///     write_string("${dir}/index.html", "<div id=\"app\"></div>").await.unwrap()
+    ///     write_string("${dir}/manifest.json", "{\"files\": {\"app.js\": 3}}").await.unwrap()
+    ///     write_string("${dir}/app.js", "f()").await.unwrap()
+    ///     let dist = Dist::open(dir).await.unwrap()
+    ///     let get = |target: String| parse_request_head(
+    ///         bytes_from_string("GET ${target} HTTP/1.1\r\n\r\n"),
+    ///         Limits::default(),
+    ///     ).unwrap().unwrap()
+    ///
+    ///     let js = dist.file(get("/app.js?v=2")).unwrap()
+    ///     assert_eq(js.status, 200)
+    ///     assert_eq(js.headers.get("content-type").unwrap(), "text/javascript; charset=utf-8")
+    ///     assert_eq(js.body.to_string().unwrap(), "f()")
+    ///     assert(dist.file(get("/posts/7")).is_none(), "not a file: the program answers")
+    ///     # remove_dir_all(dir).await.unwrap()
+    /// }
+    /// ```
+    pub fn file(self, req: Request) -> Option<Response> {
+        let path = site_path(req.path)
+        if self.files.get(path).is_none() { return None }
+        if fs_read("${self.dir}/${path}") != 0 { return Some(Response::not_found()) }
+        let body = fs_take_bytes()
+        let mut h: Map<String, String> = Map::new()
+        h.insert("content-length", "${body.len()}")
+        h.insert("content-type", content_type(path))
+        h.insert("x-content-type-options", "nosniff")
+        Some(Response { status: 200, headers: h, body: body, cookies: [] })
+    }
+
+    /// Returns `page` put into the site's template, as `document` puts it,
+    /// with the status `status`, as an HTML response:
+    /// `Response::html(status, document(template, page))`, which a route can
+    /// add a cookie to.
+    ///
+    /// ```
+    /// import ui
+    /// import ui::ssr::{Dist, Page}
+    ///
+    /// async fn main() {
+    ///     let dir = "${temp_dir()}/nova-ssr-dist-respond"
+    ///     create_dir_all(dir).await.unwrap()
+    ///     write_string("${dir}/index.html", "<title>Blog</title><div id=\"app\"></div>").await.unwrap()
+    ///     write_string("${dir}/manifest.json", "{\"files\": {}}").await.unwrap()
+    ///     let dist = Dist::open(dir).await.unwrap()
+    ///
+    ///     let r = dist.respond(404, Page { body: "<p>No such post</p>", data: None })
+    ///     assert_eq(r.status, 404)
+    ///     assert_eq(r.headers.get("content-type").unwrap(), "text/html; charset=utf-8")
+    ///     assert_eq(r.body.to_string().unwrap(), "<title>Blog</title><div id=\"app\"><p>No such post</p></div>")
+    ///     # remove_dir_all(dir).await.unwrap()
+    /// }
+    /// ```
+    pub fn respond(self, status: Int, page: Page) -> Response {
+        Response::html(status, document(self.template, page))
+    }
+}
+
+// The path of the file `target`, a request's target, names in a built site:
+// all before any `?`, percent-decoded, less one leading `/` (spec 4.6 §6.3).
+fn site_path(target: String) -> String {
     let raw = match target.index_of("?") {
         Some(at) => target.slice(0, at)
         None => target
     }
-    let decoded_path = decoded(raw)
-    let path = if decoded_path.starts_with("/") {
-        decoded_path.slice(1, decoded_path.len())
-    } else {
-        decoded_path
-    }
-    if s.files.get(path).is_some() { return file(s.dist, path) }
-    let pieces = path.split("/")
-    if pieces[pieces.len() - 1].contains(".") { return Response::not_found() }
-    let render = s.render
-    let body = bytes_from_string(document(s.template, render(target)))
-    let mut h: Map<String, String> = Map::new()
-    h.insert("content-length", "${body.len()}")
-    h.insert("content-type", "text/html; charset=utf-8")
-    let status = if found(s.routes, target) { 200 } else { 404 }
-    Response { status: status, headers: h, body: body, cookies: [] }
-}
-
-// `200` and the file at `path` under `dist`, read now, or `404` when it
-// cannot be read. The read is `std/fs`'s `read` without its `async`, which
-// a handler cannot await: the builtin, then its slot, with nothing between.
-fn file(dist: String, path: String) -> Response {
-    if fs_read("${dist}/${path}") != 0 { return Response::not_found() }
-    let body = fs_take_bytes()
-    let mut h: Map<String, String> = Map::new()
-    h.insert("content-length", "${body.len()}")
-    h.insert("content-type", content_type(path))
-    h.insert("x-content-type-options", "nosniff")
-    Response { status: 200, headers: h, body: body, cookies: [] }
+    let path = decoded(raw)
+    if path.starts_with("/") { path.slice(1, path.len()) } else { path }
 }
 
 // The `Content-Type` of a file named `path`, by its extension in any case,
````


### Task 5: The blog's server side, and its `nova test`

Spec §6.1, §6.3 to §6.6, §7.3; decisions 8, 9, 11, 12, 16, 23 to 26, 30,
32 to 34, 37 to 39, 41, 43 and 44.

**Files:**
- Create:
  - `examples/07-fullstack-blog/nova.toml`
  - `examples/07-fullstack-blog/src/lib.nova`, `store.nova`, `auth.nova`,
    `forms.nova`, `post_card.nova`, `app.nova` (without its pages)
- Test:
  - `examples/07-fullstack-blog/tests/forms.nova`, `passwords.nova`,
    `sessions.nova`, `store.nova`
  - `crates/nova-cli/tests/run_tests.rs`

- [ ] **Step 1: The tests.** `patch_block t5-tests`. The patch holds:
  - **`nova.toml`:** the package `fullstack-blog`, version 0.1.0, edition
    2026 (decision 33).
  - **`tests/*.nova`,** 21 tests, which import the package's library as
    `fullstack_blog`:
    - `forms`, seven: each field's limits, in characters as the server
      stores the text, a title's 1 to 200 among them, and their messages
      (decision 44); a form passing when each field does; the text the
      server keeps, its line feeds and no spaces around; the text the
      server keeps holding no NUL, and a field of NULs blank (decision 39);
      a blank line separating paragraphs; and the page's data read back
      from its JSON;
    - `passwords`, four: a password kept only as its hash, with a new
      salt each time; the right password logging in, and another, none and
      a name no user has not, through the users' JSON too; a hash cut
      short never matching (decision 16); and the rounds a name no user has
      costs: `--iterations` with no user, then the largest count of three
      users', a count PBKDF2 refuses left out (decisions 26 and 41);
    - `sessions`, five: a cookie naming its session; a wrong tag, and an
      ended or unknown session, being no session; a CSRF token belonging to
      its session alone; and the ended sessions kept in a fresh table
      until it holds 64, then dropped at the next start, and, in a table
      with forty live sessions, dropped once it holds 64, the live ones
      kept, and then not before it holds 80, twice the 40 it kept
      (decision 38);
    - `store`, five: reading a seed; writing what it reads back; edits and
      deletes, a post's comments going with it; refusing what is not a
      store, with what is wrong; and the users in a file of their own: no
      users in the store's JSON, a `users` key ignored, the users' JSON
      both ways in the order they were set, and what is not a list of
      users refused, with what is wrong (decision 37).
  - **`run_tests.rs`'s `the_blog_example_passes_its_own_tests`:** it copies
    the example's `src/`, `tests/` and `nova.toml` into
    `nova-blog-nova-test/fullstack-blog`, with a fresh `NOVA_HOME` beside
    it, runs `nova test` there, and needs ten named cases `ok` and the
    line `test result: ok. 21 passed; 0 failed; 0 trapped; 21 total`
    (decision 34).
- [ ] **Step 2: Run, and see them fail.** First the build (Conventions).
  - `R the_blog_example_passes_its_own_tests`. Expected: `0 passed; 1
    failed`, at `run_tests.rs:12617`: `the example's files: Os { code: 2,
    kind: NotFound, message: "No such file or directory" }`: it copies
    `src/`, which does not exist yet.
  - `nova test` in a copy of the example, since in place it would leave
    `target/` there (decision 34):

    ```sh
    NOVA="$PWD/target/debug/nova"
    rm -rf "$P/blog-47" && cp -r examples/07-fullstack-blog "$P/blog-47"
    (cd "$P/blog-47" && "$NOVA" test); echo "exit $?"
    ```

    Expected: ``error[M0013]: package `fullstack-blog` has neither
    src/lib.nova nor src/main.nova``, pointing at `nova.toml` with `a package
    needs a library, a program, or both`, then `Error: could not compile
    due to 1 previous error`, and `exit 1`.
- [ ] **Step 3: The code.** `patch_block t5-code`:
  - **`src/lib.nova`:** the library `tests/*.nova` reach the server's
    modules through, one-line wrappers, such as `users_from`, `no_users`
    and `rounds_for_no_user`, since Nova has no re-exports (decision 30).
  - **`src/store.nova`:** `User`, `Store`, the posts, the comments and the
    next ids, and `Users`, each with its JSON both ways; posts and comments
    in the order written, `newest_first`, the edits, a deleted post's
    comments deleted with it; `load_store`, from `blog.json` or else
    `seed.json`; `load_users`, from `users.json`, none when it does not
    exist; `replace`, which writes `<name>.tmp` and renames it over
    `<name>`; `save_users`, which only `--set-password` calls; and `saved`,
    which a handler spawns, and which writes the server's copy of the store
    to `blog.json`, reading no file, and says on standard error when it
    cannot (decision 37).
  - **`src/auth.nova`,** which imports `store`'s `User` (decision 43):
    `Session` and `Sessions`, with `kept`: `new`, `lasting`, `start`, which
    drops the ended sessions first once the table holds at least 64 and
    twice what it kept (decision 38), `tag`, `cookie`, `find`, which checks
    the tag in constant time and the table, `end`, `token` and `token_ok`
    (decisions 9 and 11); `hash_password`, 16 random bytes of salt and 32
    of hash; `no_user_rounds`, the largest count of the users' hashes
    (decision 41); `password_ok`, with the 32-byte check and the work for a
    name no user has (decisions 12, 16 and 26); and `load_key`, which makes
    `<data>/secret` the first time, and refuses one that does not hold 32
    bytes in hex.
  - **`src/forms.nova`:** the checks both sides run, in characters of the
    text as `cleaned` leaves it (decision 44); `cleaned`, which drops NULs,
    makes CRLFs LF and trims (decisions 25 and 39); and a form's pieces,
    `csrf_field`, `message` and `submit`'s two buttons (decision 24).
  - **`src/post_card.nova`,** whole: `paragraphs`, `first_paragraph`, and
    the post's card (decision 32).
  - **`src/app.nova`,** without its pages (Checked on): `Post`, `Comment`,
    `Draft` and `Data`, with `missing` and `refused` (decision 23), their
    JSON written by hand, and `data_for`.
- [ ] **Step 4: Run.** First the build.
  - The same two. Expected: `R the_blog_example_passes_its_own_tests` `1
    passed` (about 3 s); `nova test` in a fresh copy, made by the same three
    lines, prints each test `ok` and ends with `test result: ok. 21 passed;
    0 failed; 0 trapped; 21 total`, and `exit 0`.
  - The whole binary: `R`. Expected: `327 passed; 0 failed; 1 ignored`.
- [ ] **Step 5: The checks,** as the conventions list them; `nova fmt`'s
  `examples` takes the blog's files.
- [ ] **Step 6: Commit** `examples/07-fullstack-blog: the server's modules, and their nova test`.

**Patch `t5-tests`:**

<!-- patch t5-tests -->

````diff
diff --git a/crates/nova-cli/tests/run_tests.rs b/crates/nova-cli/tests/run_tests.rs
index dda89575a6728e4804b47f0b83dfeceb30bbe957..381dc4d92108554d3b024a0b8fc534ef490d902a 100644
--- a/crates/nova-cli/tests/run_tests.rs
+++ b/crates/nova-cli/tests/run_tests.rs
@@ -12593,3 +12593,62 @@ fn ssr_ssg_writes_the_static_pages_and_refuses_a_bad_path() {
         "ssg: --ssg needs a directory\n"
     );
 }
+
+// ---------------------------------------------------------------------------
+// `examples/07-fullstack-blog`'s own tests (spec
+// docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md §7.3).
+
+/// `nova test` in a copy of `examples/07-fullstack-blog`, in a directory
+/// with a fixed name and a fresh `NOVA_HOME` beside it: the example's
+/// `tests/*.nova` reach its server's modules through `src/lib.nova`, which
+/// the client never imports, and pass: the store's JSON both ways, and the
+/// users' in a file of their own, sessions and their tags, when ended
+/// sessions are dropped, CSRF tokens, a password hashed and checked with a
+/// low count, the rounds a name no user has costs, the form checks, the
+/// text the server keeps, paragraphs and the page's data.
+#[test]
+fn the_blog_example_passes_its_own_tests() {
+    let base = std::env::temp_dir().join("nova-blog-nova-test");
+    let _ = std::fs::remove_dir_all(&base);
+    let dir = base.join("fullstack-blog");
+    let example = repo_root().join("examples/07-fullstack-blog");
+    for sub in ["src", "tests"] {
+        std::fs::create_dir_all(dir.join(sub)).expect("create the project");
+        for entry in std::fs::read_dir(example.join(sub)).expect("the example's files") {
+            let path = entry.expect("an entry").path();
+            let to = dir.join(sub).join(path.file_name().expect("a name"));
+            std::fs::copy(&path, to).expect("copy a file");
+        }
+    }
+    std::fs::copy(example.join("nova.toml"), dir.join("nova.toml")).expect("copy nova.toml");
+    let a = nova()
+        .current_dir(&dir)
+        .env("NOVA_HOME", base.join("home"))
+        .env("NO_COLOR", "1")
+        .env("RUST_BACKTRACE", "0")
+        .arg("test")
+        .assert()
+        .success();
+    let out = String::from_utf8_lossy(&a.get_output().stdout).replace("\r\n", "\n");
+    for name in [
+        "store::writes_what_it_reads_back",
+        "store::the_users_have_a_file_of_their_own",
+        "sessions::a_wrong_tag_is_no_session",
+        "sessions::a_csrf_token_belongs_to_its_session",
+        "sessions::ended_sessions_are_dropped_once_the_table_has_doubled",
+        "passwords::the_right_password_logs_in",
+        "passwords::a_name_no_user_has_costs_the_most_rounds_a_user_has",
+        "forms::a_title_has_1_to_200_characters",
+        "forms::the_server_keeps_no_nul",
+        "forms::a_blank_line_separates_paragraphs",
+    ] {
+        assert!(
+            out.contains(&format!("test {name} ... ok\n")),
+            "{name}: {out}"
+        );
+    }
+    assert!(
+        out.ends_with("test result: ok. 21 passed; 0 failed; 0 trapped; 21 total\n"),
+        "{out}"
+    );
+}
diff --git a/examples/07-fullstack-blog/nova.toml b/examples/07-fullstack-blog/nova.toml
new file mode 100644
index 0000000000000000000000000000000000000000..3d993f397d1eb766601a360dd0d35f6128e676ce
--- /dev/null
+++ b/examples/07-fullstack-blog/nova.toml
@@ -0,0 +1,5 @@
+[package]
+name = "fullstack-blog"
+version = "0.1.0"
+edition = "2026"
+description = "A blog that a visitor reads and comments on and its admin writes: a server on std/http that renders each page, and a client that hydrates it"
diff --git a/examples/07-fullstack-blog/tests/forms.nova b/examples/07-fullstack-blog/tests/forms.nova
new file mode 100644
index 0000000000000000000000000000000000000000..dccb65667a8f21e8ea789a8f7b7f874ac7290bbe
--- /dev/null
+++ b/examples/07-fullstack-blog/tests/forms.nova
@@ -0,0 +1,92 @@
+//! The checks both sides run, the text the server keeps, paragraphs, and
+//! the page's data as JSON.
+
+import fullstack_blog::{data_from, data_json, first_of, form_ok, message_for, paragraphs_of, stored}
+
+@test
+fn a_title_has_1_to_200_characters() {
+    assert_eq(message_for("title", ""), "Give the post a title.")
+    assert_eq(message_for("title", "  \n "), "Give the post a title.")
+    assert_eq(message_for("title", "x"), "")
+    assert_eq(message_for("title", "é".repeat(200)), "")
+    assert_eq(message_for("title", "  ${"x".repeat(200)}  "), "")
+    assert_eq(
+        message_for("title", "x".repeat(201)),
+        "A title has at most 200 characters; this one has 201.",
+    )
+}
+
+@test
+fn each_field_has_its_limits() {
+    assert_eq(message_for("body", "x".repeat(20000)), "")
+    assert(message_for("body", "x".repeat(20001)) != "", "a body of 20,001")
+    assert_eq(message_for("body", " "), "Write something in the post.")
+    assert_eq(message_for("name", "x".repeat(60)), "")
+    assert(message_for("name", "x".repeat(61)) != "", "a name of 61")
+    assert_eq(message_for("name", ""), "Say who you are.")
+    assert_eq(message_for("text", "x".repeat(2000)), "")
+    assert(message_for("text", "x".repeat(2001)) != "", "a comment of 2,001")
+    assert_eq(message_for("text", "\t"), "Write a comment.")
+    assert_eq(message_for("login", " "), "Give your name.")
+    assert_eq(message_for("password", ""), "Give your password.")
+    assert_eq(message_for("password", " secret "), "")
+}
+
+@test
+fn a_form_passes_when_each_field_does() {
+    assert(form_ok("post", "Title", "Body"), "a post")
+    assert(!form_ok("post", "Title", ""), "no body")
+    assert(!form_ok("post", "", "Body"), "no title")
+    assert(form_ok("comment", "Ada", "Hi"), "a comment")
+    assert(!form_ok("comment", "Ada", " "), "no text")
+    assert(form_ok("login", "admin", "pw"), "a login")
+    assert(!form_ok("login", "admin", ""), "no password")
+}
+
+@test
+fn the_server_keeps_line_feeds_and_no_spaces_around() {
+    assert_eq(stored("  One\r\ntwo\r\n\r\nThree\r\n"), "One\ntwo\n\nThree")
+}
+
+@test
+fn the_server_keeps_no_nul() {
+    assert_eq(stored("one\0two\0"), "onetwo")
+    assert_eq(stored("\0 a\r\0\nb \0"), "a\nb")
+    assert_eq(message_for("text", "\0\0"), "Write a comment.")
+    assert_eq(message_for("name", "${"x".repeat(60)}\0"), "")
+}
+
+@test
+fn a_blank_line_separates_paragraphs() {
+    let ps = paragraphs_of("One\nstill one\n\n  \nTwo <b>&amp;</b>\r\n\r\nThree")
+    assert_eq(ps.len(), 3)
+    assert_eq(ps[0], "One\nstill one")
+    assert_eq(ps[1], "Two <b>&amp;</b>")
+    assert_eq(ps[2], "Three")
+    assert_eq(paragraphs_of("\n\n  \n").len(), 0)
+    assert_eq(paragraphs_of("Only").len(), 1)
+    assert_eq(first_of("\n\nFirst\n\nSecond"), "First")
+    assert_eq(first_of(""), "")
+}
+
+@test
+fn the_pages_data_reads_back() {
+    let json = "{\"user\": \"admin\", \"csrf\": \"ab\", \"posts\": [],
+        \"post\": {\"id\": 2, \"title\": \"</script> & <b>\", \"body\": \"B\", \"created\": \"c\", \"updated\": \"u\"},
+        \"comments\": [{\"id\": 1, \"post\": 2, \"name\": \"N\", \"text\": \"T\", \"created\": \"c\"}],
+        \"draft\": {\"title\": \"\", \"body\": \"\", \"name\": \"Ada\", \"text\": \"\", \"failed\": true, \"error\": \"\"},
+        \"refused\": \"\", \"missing\": false}"
+    let d = data_from(json).unwrap()
+    assert_eq(d.user, "admin")
+    assert_eq(d.post.unwrap().title, "</script> & <b>")
+    assert_eq(d.comments[0].name, "N")
+    assert(d.draft.failed, "failed")
+    let back = data_from(data_json(d)).unwrap()
+    assert_eq(back.post.unwrap().id, 2)
+    assert_eq(back.draft.name, "Ada")
+    assert_eq(back.csrf, "ab")
+    // No post is `null`.
+    let none = data_from("\"post\": null, \"old\": {".join(json.split("\"post\": {"))).unwrap()
+    assert(none.post.is_none(), "no post")
+    assert(data_from("{}").is_err(), "no fields")
+}
diff --git a/examples/07-fullstack-blog/tests/passwords.nova b/examples/07-fullstack-blog/tests/passwords.nova
new file mode 100644
index 0000000000000000000000000000000000000000..306c1506ac3a93213f14d6a16ed45b13708fbdaf
--- /dev/null
+++ b/examples/07-fullstack-blog/tests/passwords.nova
@@ -0,0 +1,58 @@
+//! Passwords, hashed and checked with a low count.
+
+import fullstack_blog::{logs_in, no_users, rounds_for_no_user, user_with, users_from}
+
+@test
+fn a_password_is_kept_only_as_its_hash() {
+    let u = user_with("admin", "correct horse", 1000)
+    assert_eq(u.name, "admin")
+    assert_eq(u.iterations, 1000)
+    assert_eq(u.salt.len(), 32)
+    assert_eq(u.hash.len(), 64)
+    assert(!u.hash.contains("correct"), "not the password")
+    let again = user_with("admin", "correct horse", 1000)
+    assert(again.salt != u.salt && again.hash != u.hash, "a new salt each time")
+}
+
+@test
+fn the_right_password_logs_in() {
+    let mut users = no_users()
+    users.set_user(user_with("admin", "correct horse", 1000))
+    assert(logs_in(users, "admin", "correct horse", 1000), "the password")
+    assert(!logs_in(users, "admin", "correct horse ", 1000), "another")
+    assert(!logs_in(users, "admin", "", 1000), "none")
+    assert(!logs_in(users, "nobody", "correct horse", 1000), "no such user")
+    // Setting the password again replaces the user.
+    users.set_user(user_with("admin", "battery staple", 1000))
+    assert_eq(users.list.len(), 1)
+    assert(!logs_in(users, "admin", "correct horse", 1000), "the old password")
+    assert(logs_in(users, "admin", "battery staple", 1000), "the new one")
+    // The user survives the users' JSON.
+    let back = users_from(users.to_text()).unwrap()
+    assert(logs_in(back, "admin", "battery staple", 1000), "after the JSON")
+}
+
+@test
+fn a_hash_cut_short_never_matches() {
+    let mut users = no_users()
+    let mut u = user_with("admin", "correct horse", 1000)
+    // PBKDF2 would take the start of the key for the whole.
+    u.hash = u.hash.slice(0, 32)
+    users.set_user(u)
+    assert(!logs_in(users, "admin", "correct horse", 1000), "a 16-byte hash")
+}
+
+@test
+fn a_name_no_user_has_costs_the_most_rounds_a_user_has() {
+    let mut users = no_users()
+    assert_eq(rounds_for_no_user(users, 600000), 600000)
+    users.set_user(user_with("ada", "correct horse", 1000))
+    users.set_user(user_with("admin", "correct horse", 3000))
+    users.set_user(user_with("grace", "correct horse", 2000))
+    assert_eq(rounds_for_no_user(users, 600000), 3000)
+    // A count PBKDF2 does not take, from a file changed by hand, is left out.
+    let mut odd = user_with("eve", "correct horse", 1000)
+    odd.iterations = 4294967296
+    users.set_user(odd)
+    assert_eq(rounds_for_no_user(users, 600000), 3000)
+}
diff --git a/examples/07-fullstack-blog/tests/sessions.nova b/examples/07-fullstack-blog/tests/sessions.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b14a267c599f6632526c7151f998f354713581ce
--- /dev/null
+++ b/examples/07-fullstack-blog/tests/sessions.nova
@@ -0,0 +1,123 @@
+//! Sessions, their signed cookies, and the CSRF tokens bound to them.
+
+import fullstack_blog::{sessions}
+
+fn key(c: String) -> Bytes { bytes_from_string(c.repeat(32)) }
+
+@test
+fn a_cookie_names_its_session() {
+    let mut table = sessions(key("k"), 60, 60)
+    let s = table.start("")
+    assert_eq(s.id.len(), 64)
+    assert_eq(s.user, "")
+    let cookie = table.cookie(s)
+    // `<id>.<tag>`, the tag 32 bytes of HMAC-SHA256 in hex.
+    assert_eq(cookie.len(), 64 + 1 + 64)
+    assert_eq(cookie.slice(0, 64), s.id)
+    assert_eq(table.find(cookie).unwrap().id, s.id)
+    let admin = table.start("admin")
+    assert_eq(table.find(table.cookie(admin)).unwrap().user, "admin")
+    assert(admin.id != s.id, "a new id each time")
+    assert_eq(table.len(), 2)
+}
+
+@test
+fn a_wrong_tag_is_no_session() {
+    let mut table = sessions(key("k"), 60, 60)
+    let s = table.start("admin")
+    let cookie = table.cookie(s)
+    // The tag of another key, the tag cut short, changed, or missing, and an
+    // id of the table's with another id's tag.
+    let other = sessions(key("x"), 60, 60)
+    assert(table.find("${s.id}.${other.tag(s.id)}").is_none(), "another key's tag")
+    assert(table.find(cookie.slice(0, cookie.len() - 2)).is_none(), "a tag cut short")
+    let last = cookie.slice(cookie.len() - 1, cookie.len())
+    let flipped = if last == "0" { "1" } else { "0" }
+    assert(table.find("${cookie.slice(0, cookie.len() - 1)}${flipped}").is_none(), "a changed tag")
+    assert(table.find(s.id).is_none(), "no tag")
+    assert(table.find("${s.id}.").is_none(), "an empty tag")
+    assert(table.find("${s.id}.zz").is_none(), "a tag that is not hex")
+    let t = table.start("")
+    assert(table.find("${s.id}.${table.tag(t.id)}").is_none(), "another session's tag")
+}
+
+@test
+fn an_ended_or_unknown_session_is_no_session() {
+    let mut table = sessions(key("k"), 60, 60)
+    let s = table.start("admin")
+    let cookie = table.cookie(s)
+    table.end(s.id)
+    assert(table.find(cookie).is_none(), "ended")
+    // A tag the key signs, for an id the table never held.
+    let id = "ab".repeat(32)
+    assert(table.find("${id}.${table.tag(id)}").is_none(), "unknown")
+    // Sessions that last no time end at once.
+    let mut brief = sessions(key("k"), 0, 0)
+    let gone = brief.start("")
+    assert(brief.find(brief.cookie(gone)).is_none(), "already ended")
+}
+
+@test
+fn a_csrf_token_belongs_to_its_session() {
+    let mut table = sessions(key("k"), 60, 60)
+    let s = table.start("")
+    let t = table.start("")
+    let token = table.token(s)
+    assert_eq(token.len(), 64)
+    assert(table.token_ok(s, token), "its own")
+    assert(!table.token_ok(t, token), "another session's")
+    assert(table.token(s) == token, "the same each time")
+    assert(token != table.tag(s.id), "not the cookie's tag")
+    assert(!table.token_ok(s, ""), "none")
+    assert(!table.token_ok(s, "not hex"), "not hex")
+    assert(!table.token_ok(s, token.slice(0, 62)), "cut short")
+    let other = sessions(key("x"), 60, 60)
+    assert(!table.token_ok(s, other.token(s)), "another key's")
+}
+
+@test
+fn ended_sessions_are_dropped_once_the_table_has_doubled() {
+    // A visitor's session ends at once; the admin's lasts an hour.
+    // A fresh table has kept none, so it has always doubled since; still,
+    // under 64 sessions the ended ones stay.
+    let mut fresh = sessions(key("k"), 0, 3600)
+    let mut m = 0
+    while m < 63 {
+        let _ = fresh.start("")
+        m = m + 1
+    }
+    assert_eq(fresh.len(), 63)
+    let _ = fresh.start("")
+    assert_eq(fresh.len(), 64)
+    // At 64, the next start drops all 64.
+    let _ = fresh.start("")
+    assert_eq(fresh.len(), 1)
+
+    let mut table = sessions(key("k"), 0, 3600)
+    let mut live: Vec<String> = Vec::new()
+    while live.len() < 40 { live.push(table.cookie(table.start("admin"))) }
+    // Under 64 sessions, the ended ones stay.
+    let mut n = 0
+    while n < 24 {
+        let _ = table.start("")
+        n = n + 1
+    }
+    assert_eq(table.len(), 64)
+    // At 64, they are dropped as the next one starts, and the live ones stay.
+    let _ = table.start("")
+    assert_eq(table.len(), 41)
+    let mut i = 0
+    while i < live.len() {
+        assert(table.find(live.get(i).unwrap()).is_some(), "a live session")
+        i = i + 1
+    }
+    // 40 kept: the ended ones stay until the table holds twice as many.
+    n = 0
+    while n < 39 {
+        let _ = table.start("")
+        n = n + 1
+    }
+    assert_eq(table.len(), 80)
+    let _ = table.start("")
+    assert_eq(table.len(), 41)
+}
diff --git a/examples/07-fullstack-blog/tests/store.nova b/examples/07-fullstack-blog/tests/store.nova
new file mode 100644
index 0000000000000000000000000000000000000000..f1336cc30d137026e17fd1680d45b30b500ba431
--- /dev/null
+++ b/examples/07-fullstack-blog/tests/store.nova
@@ -0,0 +1,135 @@
+//! The store's JSON both ways, and its changes.
+
+import fullstack_blog::{store_from, user_with, users_from}
+
+// A store as `seed.json` holds one: two posts and a comment on each, and no
+// next ids, which come from the ids taken.
+fn seed() -> String {
+    let lines = [
+        "{\"posts\": [",
+        "  {\"id\": 1, \"title\": \"One\", \"body\": \"First.\\n\\nSecond.\",",
+        "   \"created\": \"2026-10-19T08:30:00.000Z\", \"updated\": \"2026-10-19T08:30:00.000Z\"},",
+        "  {\"id\": 5, \"title\": \"Two <b>\", \"body\": \"Text\",",
+        "   \"created\": \"2026-10-20T08:30:00.000Z\", \"updated\": \"2026-10-20T09:00:00.000Z\"}",
+        "], \"comments\": [",
+        "  {\"id\": 3, \"post\": 1, \"name\": \"Ada\", \"text\": \"Hi\", \"created\": \"2026-10-19T10:00:00.000Z\"},",
+        "  {\"id\": 4, \"post\": 5, \"name\": \"Grace\", \"text\": \"Yes\", \"created\": \"2026-10-20T10:00:00.000Z\"}",
+        "]}",
+    ]
+    "\n".join(lines)
+}
+
+@test
+fn reads_a_seed() {
+    let s = store_from(seed()).unwrap()
+    assert_eq(s.posts.len(), 2)
+    assert_eq(s.comments.len(), 2)
+    assert_eq(s.next_post, 6)
+    assert_eq(s.next_comment, 5)
+    let two = s.post(5).unwrap()
+    assert_eq(two.title, "Two <b>")
+    assert_eq(two.updated, "2026-10-20T09:00:00.000Z")
+    assert(s.post(2).is_none(), "no post 2")
+    let newest = s.newest_first()
+    assert_eq(newest[0].id, 5)
+    assert_eq(newest[1].id, 1)
+    assert_eq(s.comments_on(1)[0].name, "Ada")
+}
+
+@test
+fn writes_what_it_reads_back() {
+    let mut s = store_from(seed()).unwrap()
+    let made = s.add_post("Three \"quoted\"", "Line one\nline two", "2026-10-21T08:00:00.000Z")
+    assert_eq(made.id, 6)
+    let c = s.add_comment(6, "Eve", "<script>x</script>", "2026-10-21T09:00:00.000Z")
+    assert_eq(c.id, 5)
+    let again = store_from(s.to_text()).unwrap()
+    assert_eq(again.posts.len(), 3)
+    assert_eq(again.next_post, 7)
+    assert_eq(again.next_comment, 6)
+    let three = again.post(6).unwrap()
+    assert_eq(three.title, "Three \"quoted\"")
+    assert_eq(three.body, "Line one\nline two")
+    assert_eq(three.created, "2026-10-21T08:00:00.000Z")
+    assert_eq(again.comments_on(6)[0].text, "<script>x</script>")
+}
+
+@test
+fn edits_and_deletes() {
+    let mut s = store_from(seed()).unwrap()
+    assert(s.edit_post(1, "One, edited", "New", "2026-10-22T00:00:00.000Z"), "edited")
+    let one = s.post(1).unwrap()
+    assert_eq(one.title, "One, edited")
+    assert_eq(one.created, "2026-10-19T08:30:00.000Z")
+    assert_eq(one.updated, "2026-10-22T00:00:00.000Z")
+    assert(!s.edit_post(9, "x", "y", "z"), "no post 9")
+    // Deleting a post deletes its comments, and no other.
+    assert(s.delete_post(1), "deleted")
+    assert(s.post(1).is_none(), "gone")
+    assert_eq(s.comments.len(), 1)
+    assert(s.comment(3).is_none(), "its comment is gone")
+    assert(!s.delete_post(1), "already gone")
+    // A deleted id is not given again.
+    assert_eq(s.add_post("t", "b", "now").id, 6)
+    assert_eq(s.delete_comment(4).unwrap().post, 5)
+    assert(s.delete_comment(4).is_none(), "already gone")
+    assert_eq(s.comments.len(), 0)
+}
+
+// What is wrong with `text`, or `""` when it is a store.
+fn wrong(text: String) -> String {
+    match store_from(text) {
+        Ok(_) => ""
+        Err(why) => why
+    }
+}
+
+@test
+fn refuses_what_is_not_a_store() {
+    assert(store_from("not json").is_err(), "not JSON")
+    assert(store_from("[]").is_err(), "not an object")
+    assert_eq(wrong("{\"posts\": []}"), "comments: expected an array")
+    assert_eq(wrong("{\"posts\": [{\"id\": 1}], \"comments\": []}"), "title: expected a string")
+}
+
+// What is wrong with `text`, or `""` when it holds users.
+fn wrong_users(text: String) -> String {
+    match users_from(text) {
+        Ok(_) => ""
+        Err(why) => why
+    }
+}
+
+@test
+fn the_users_have_a_file_of_their_own() {
+    // The store's JSON holds no users, and the store ignores a `users` key.
+    let s = store_from("{\"posts\": [], \"comments\": [], \"users\": [1]}").unwrap()
+    assert(!s.to_text().contains("users"), "users in the store's JSON")
+    assert(!store_from(seed()).unwrap().to_text().contains("users"), "users in the seed's")
+    // The users' JSON both ways, in the order they were set.
+    let mut users = users_from("{\"users\": []}").unwrap()
+    assert_eq(users.list.len(), 0)
+    users.set_user(user_with("grace", "correct horse", 1000))
+    users.set_user(user_with("admin", "battery staple", 2000))
+    let back = users_from(users.to_text()).unwrap()
+    assert_eq(back.list.len(), 2)
+    assert_eq(back.list.get(0).unwrap().name, "grace")
+    let admin = back.user("admin").unwrap()
+    let was = users.user("admin").unwrap()
+    assert_eq(admin.iterations, 2000)
+    assert_eq(admin.salt, was.salt)
+    assert_eq(admin.hash, was.hash)
+    assert(back.user("nobody").is_none(), "no such user")
+    assert(!users.to_text().contains("posts"), "posts in the users' JSON")
+    // What is not a list of users.
+    assert(wrong_users("not json") != "", "not JSON")
+    assert_eq(wrong_users("[]"), "expected an object")
+    assert_eq(wrong_users("{\"posts\": []}"), "users: expected an array")
+    assert_eq(wrong_users("{\"users\": [1]}"), "expected a user")
+    assert_eq(
+        wrong_users(
+            "{\"users\": [{\"name\": \"a\", \"salt\": \"00\", \"iterations\": \"many\", \"hash\": \"00\"}]}",
+        ),
+        "iterations: expected a number",
+    )
+}
````

**Patch `t5-code`:**

<!-- patch t5-code -->

````diff
diff --git a/examples/07-fullstack-blog/src/app.nova b/examples/07-fullstack-blog/src/app.nova
new file mode 100644
index 0000000000000000000000000000000000000000..4d885c85fc19514682079d0fb18fc1983f835fae
--- /dev/null
+++ b/examples/07-fullstack-blog/src/app.nova
@@ -0,0 +1,387 @@
+//! What the client, `main.nova`, and the server, `server.nova`, share: the
+//! records a page is rendered from and their JSON, the routes, and the app.
+//!
+//! `hydrate_with` reads one value of one type for the whole app, so `Data`
+//! holds what any page needs: who is logged in, the CSRF token, the posts or
+//! the post and its comments, and a form's values. The server fills the
+//! parts the page at hand shows and renders it with `page_with`; the client
+//! reads the same `Data` back and hydrates the same view. Links are plain
+//! anchors, so each page arrives from the server with its own data.
+
+/// A post: written as plain text, a blank line between paragraphs, and when
+/// it was written and last changed, as `SystemTime::to_iso8601` writes them.
+pub record Post {
+    pub id: Int,
+    pub title: String,
+    pub body: String,
+    pub created: String,
+    pub updated: String,
+}
+
+/// A comment on the post whose id is `post`.
+pub record Comment {
+    pub id: Int,
+    pub post: Int,
+    pub name: String,
+    pub text: String,
+    pub created: String,
+}
+
+/// A form's values as the page shows them: the editor's title and body, the
+/// comment form's name and text, or the login form's name (a password is
+/// never sent back). `failed` says the server refused the form, so its
+/// messages show from the start; `error` is the server's message for the
+/// whole form, such as a wrong password.
+pub record Draft {
+    pub title: String,
+    pub body: String,
+    pub name: String,
+    pub text: String,
+    pub failed: Bool,
+    pub error: String,
+}
+
+/// What a page is rendered from, on the server and again on the client.
+pub record Data {
+    /// Who is logged in, or `""`.
+    pub user: String,
+    /// The CSRF token of the visitor's session, for every form's hidden
+    /// `csrf` field.
+    pub csrf: String,
+    /// The home page's posts, newest first, each with its first paragraph
+    /// only, and the admin's list, with no bodies.
+    pub posts: [Post],
+    /// The post the page shows or edits.
+    pub post: Option<Post>,
+    /// The post's comments, in the order they were written.
+    pub comments: [Comment],
+    /// The form's values.
+    pub draft: Draft,
+    /// Why the server refused a form, such as an expired token; the page
+    /// then says so instead of showing its route.
+    pub refused: String,
+    /// The page has nothing to show: no such post, or no such path.
+    pub missing: Bool,
+}
+
+/// A draft with nothing in it.
+pub fn empty_draft() -> Draft {
+    Draft { title: "", body: "", name: "", text: "", failed: false, error: "" }
+}
+
+/// Data with nothing in it but who is logged in and the token.
+pub fn data_for(user: String, csrf: String) -> Data {
+    Data {
+        user: user,
+        csrf: csrf,
+        posts: [],
+        post: None,
+        comments: [],
+        draft: empty_draft(),
+        refused: "",
+        missing: false,
+    }
+}
+
+// --- JSON, by hand, since there is no derive ---
+
+/// The member `k` of `m`, or `Null`.
+pub fn member(m: Map<String, JsonValue>, k: String) -> JsonValue { m.get(k).unwrap_or(Null) }
+
+/// The object `v` holds, or an error naming `what`.
+pub fn object_of(v: JsonValue, what: String) -> Result<Map<String, JsonValue>, JsonError> {
+    match v {
+        Object(m) => Ok(m)
+        _ => Err(JsonError { msg: "expected ${what}", at: 0 })
+    }
+}
+
+/// The string member `k` of `m`.
+pub fn string_of(m: Map<String, JsonValue>, k: String) -> Result<String, JsonError> {
+    match String::from_json(member(m, k)) {
+        Ok(s) => Ok(s)
+        Err(e) => Err(JsonError { msg: "${k}: ${e.msg}", at: 0 })
+    }
+}
+
+/// The integer member `k` of `m`.
+pub fn int_of(m: Map<String, JsonValue>, k: String) -> Result<Int, JsonError> {
+    match Int::from_json(member(m, k)) {
+        Ok(n) => Ok(n)
+        Err(e) => Err(JsonError { msg: "${k}: ${e.msg}", at: 0 })
+    }
+}
+
+/// The array member `k` of `m`.
+pub fn list_of(m: Map<String, JsonValue>, k: String) -> Result<[JsonValue], JsonError> {
+    match member(m, k) {
+        Array(xs) => Ok(xs)
+        _ => Err(JsonError { msg: "${k}: expected an array", at: 0 })
+    }
+}
+
+impl ToJson for Post {
+    fn to_json(self) -> JsonValue {
+        let mut m: Map<String, JsonValue> = Map::new()
+        m.insert("id", self.id.to_json())
+        m.insert("title", self.title.to_json())
+        m.insert("body", self.body.to_json())
+        m.insert("created", self.created.to_json())
+        m.insert("updated", self.updated.to_json())
+        Object(m)
+    }
+}
+
+impl FromJson for Post {
+    fn from_json(v: JsonValue) -> Result<Post, JsonError> {
+        let m = match object_of(v, "a post") {
+            Ok(m) => m
+            Err(e) => return Err(e)
+        }
+        let id = match int_of(m, "id") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let title = match string_of(m, "title") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let body = match string_of(m, "body") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let created = match string_of(m, "created") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let updated = match string_of(m, "updated") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        Ok(Post { id: id, title: title, body: body, created: created, updated: updated })
+    }
+}
+
+impl ToJson for Comment {
+    fn to_json(self) -> JsonValue {
+        let mut m: Map<String, JsonValue> = Map::new()
+        m.insert("id", self.id.to_json())
+        m.insert("post", self.post.to_json())
+        m.insert("name", self.name.to_json())
+        m.insert("text", self.text.to_json())
+        m.insert("created", self.created.to_json())
+        Object(m)
+    }
+}
+
+impl FromJson for Comment {
+    fn from_json(v: JsonValue) -> Result<Comment, JsonError> {
+        let m = match object_of(v, "a comment") {
+            Ok(m) => m
+            Err(e) => return Err(e)
+        }
+        let id = match int_of(m, "id") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let post = match int_of(m, "post") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let name = match string_of(m, "name") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let text = match string_of(m, "text") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let created = match string_of(m, "created") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        Ok(Comment { id: id, post: post, name: name, text: text, created: created })
+    }
+}
+
+/// `posts` as a JSON array.
+pub fn posts_json(posts: [Post]) -> JsonValue {
+    let mut xs = [Null; posts.len()]
+    let mut i = 0
+    while i < posts.len() {
+        xs[i] = posts[i].to_json()
+        i = i + 1
+    }
+    Array(xs)
+}
+
+/// `comments` as a JSON array.
+pub fn comments_json(comments: [Comment]) -> JsonValue {
+    let mut xs = [Null; comments.len()]
+    let mut i = 0
+    while i < comments.len() {
+        xs[i] = comments[i].to_json()
+        i = i + 1
+    }
+    Array(xs)
+}
+
+/// The posts a JSON array holds.
+pub fn posts_from(list: [JsonValue]) -> Result<[Post], JsonError> {
+    let mut posts = [Post { id: 0, title: "", body: "", created: "", updated: "" }; list.len()]
+    let mut i = 0
+    while i < list.len() {
+        posts[i] = match Post::from_json(list[i]) {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        i = i + 1
+    }
+    Ok(posts)
+}
+
+/// The comments a JSON array holds.
+pub fn comments_from(list: [JsonValue]) -> Result<[Comment], JsonError> {
+    let mut comments = [Comment { id: 0, post: 0, name: "", text: "", created: "" }; list.len()]
+    let mut i = 0
+    while i < list.len() {
+        comments[i] = match Comment::from_json(list[i]) {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        i = i + 1
+    }
+    Ok(comments)
+}
+
+impl ToJson for Draft {
+    fn to_json(self) -> JsonValue {
+        let mut m: Map<String, JsonValue> = Map::new()
+        m.insert("title", self.title.to_json())
+        m.insert("body", self.body.to_json())
+        m.insert("name", self.name.to_json())
+        m.insert("text", self.text.to_json())
+        m.insert("failed", self.failed.to_json())
+        m.insert("error", self.error.to_json())
+        Object(m)
+    }
+}
+
+impl FromJson for Draft {
+    fn from_json(v: JsonValue) -> Result<Draft, JsonError> {
+        let m = match object_of(v, "a draft") {
+            Ok(m) => m
+            Err(e) => return Err(e)
+        }
+        let title = match string_of(m, "title") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let body = match string_of(m, "body") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let name = match string_of(m, "name") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let text = match string_of(m, "text") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let failed = match Bool::from_json(member(m, "failed")) {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let error = match string_of(m, "error") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        Ok(Draft { title: title, body: body, name: name, text: text, failed: failed, error: error })
+    }
+}
+
+impl ToJson for Data {
+    fn to_json(self) -> JsonValue {
+        let mut m: Map<String, JsonValue> = Map::new()
+        m.insert("user", self.user.to_json())
+        m.insert("csrf", self.csrf.to_json())
+        m.insert("posts", posts_json(self.posts))
+        let post = match self.post {
+            Some(x) => x.to_json()
+            None => Null
+        }
+        m.insert("post", post)
+        m.insert("comments", comments_json(self.comments))
+        m.insert("draft", self.draft.to_json())
+        m.insert("refused", self.refused.to_json())
+        m.insert("missing", self.missing.to_json())
+        Object(m)
+    }
+}
+
+impl FromJson for Data {
+    fn from_json(v: JsonValue) -> Result<Data, JsonError> {
+        let m = match object_of(v, "the page's data") {
+            Ok(m) => m
+            Err(e) => return Err(e)
+        }
+        let user = match string_of(m, "user") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let csrf = match string_of(m, "csrf") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let listed = match list_of(m, "posts") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let posts = match posts_from(listed) {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let post = match member(m, "post") {
+            Null => None
+            x => {
+                match Post::from_json(x) {
+                    Ok(p) => Some(p)
+                    Err(e) => return Err(e)
+                }
+            }
+        }
+        let said = match list_of(m, "comments") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let comments = match comments_from(said) {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let draft = match Draft::from_json(member(m, "draft")) {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let refused = match string_of(m, "refused") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let missing = match Bool::from_json(member(m, "missing")) {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        Ok(
+            Data {
+                user: user,
+                csrf: csrf,
+                posts: posts,
+                post: post,
+                comments: comments,
+                draft: draft,
+                refused: refused,
+                missing: missing,
+            },
+        )
+    }
+}
diff --git a/examples/07-fullstack-blog/src/auth.nova b/examples/07-fullstack-blog/src/auth.nova
new file mode 100644
index 0000000000000000000000000000000000000000..278c8159f71ea20b6f16c753833d20372b589b80
--- /dev/null
+++ b/examples/07-fullstack-blog/src/auth.nova
@@ -0,0 +1,230 @@
+//! Sessions, CSRF tokens and passwords.
+//!
+//! - **A session** is a random id, 32 bytes in hex, in a table in memory
+//!   from the id to who is logged in, if anyone, and when it started. The
+//!   cookie `sid` carries `<id>.<tag>`, the tag `hmac_sha256(key, id)` in
+//!   hex, checked with `hmac_sha256_verify`, which takes the same time
+//!   however much of a wrong tag matches. A cookie whose tag is wrong, or
+//!   whose id the table does not hold, or holds as ended, is no session. A
+//!   visitor's session ends after a day, a logged-in one after eight hours,
+//!   and a restart ends them all, since the table is in memory. The ended
+//!   ones are dropped from the table once it has grown to twice the size it
+//!   had after they were last dropped, and to at least 64 sessions, so it
+//!   holds at most twice the sessions live at the last scan, or 64 if that
+//!   is more; those that end after a burst stay until it doubles again.
+//!   Each scan is paid for by the sessions started since the last, so
+//!   starting one costs the same on average however many the table holds,
+//!   but the start that finds it doubled scans it all, in time linear in
+//!   its size.
+//! - **The CSRF token** of a session is `hmac_sha256(key, "csrf:" + id)` in
+//!   hex: nothing more to store, and worth nothing with any other session.
+//! - **A password** is stored only as its PBKDF2-HMAC-SHA256 hash, with its
+//!   salt and its iteration count.
+//! - **The key** is 32 random bytes, written in hex to `<data>/secret` the
+//!   first time the server starts, and read from there afterwards.
+//!
+//! Native only: `std/crypto` and `std/fs` are not on WASM.
+
+import store::{User}
+
+/// How long a visitor's session lasts, in seconds: a day.
+const VISITOR_LIFE: Int = 86400
+
+/// How long the admin's session lasts: eight hours.
+const ADMIN_LIFE: Int = 28800
+
+/// The fewest sessions the table holds before the ended ones are dropped.
+const FEWEST_TO_DROP: Int = 64
+
+/// A session: its id, who is logged in, `""` for a visitor, when it started
+/// and how many seconds it lasts.
+pub record Session { pub id: String, pub user: String, pub started: Instant, pub life: Int }
+
+/// The sessions, by id, and the key their cookies and tokens are signed
+/// with; `kept` is how many sessions the table held after the ended ones
+/// were last dropped.
+pub record Sessions {
+    pub key: Bytes,
+    pub table: Map<String, Session>,
+    pub visitor_life: Int,
+    pub admin_life: Int,
+    pub kept: Int,
+}
+
+// `n` random bytes in hex. The system's random source failing ends the
+// program: nothing here can go on without it.
+fn random_hex(n: Int) -> String {
+    match random_bytes(n) {
+        Ok(b) => b.to_hex()
+        Err(e) => panic("blog: the system gave no random bytes: ${e.message}")
+    }
+}
+
+impl Sessions {
+    /// No sessions, signed with `key`, lasting a day for a visitor and eight
+    /// hours for the admin.
+    pub fn new(key: Bytes) -> Sessions { Sessions::lasting(key, VISITOR_LIFE, ADMIN_LIFE) }
+
+    /// No sessions, signed with `key`, lasting `visitor` seconds for a
+    /// visitor and `admin` for the admin.
+    pub fn lasting(key: Bytes, visitor: Int, admin: Int) -> Sessions {
+        Sessions { key: key, table: Map::new(), visitor_life: visitor, admin_life: admin, kept: 0 }
+    }
+
+    /// How many sessions the table holds, ended or not.
+    pub fn len(self) -> Int { self.table.len() }
+
+    /// Whether `s` has ended.
+    pub fn ended(self, s: Session) -> Bool { s.started.elapsed().as_millis() >= s.life * 1000 }
+
+    /// Starts a session for `user`, `""` for a visitor, and returns it.
+    /// First, when the table holds at least 64 sessions, and at least twice
+    /// as many as it kept the last time the ended ones were dropped, it
+    /// drops the ended ones: at least half the sessions it then scans have
+    /// started since the last scan, so the scans cost the same per session
+    /// on average, however many the table holds; this start scans them all.
+    pub fn start(mut self, user: String) -> Session {
+        let mut table = self.table
+        if table.len() >= FEWEST_TO_DROP && table.len() >= self.kept * 2 {
+            let ids = table.keys()
+            let mut i = 0
+            while i < ids.len() {
+                match table.get(ids[i]) {
+                    Some(s) => { if self.ended(s) { let _ = table.remove(ids[i]) } }
+                    None => {}
+                }
+                i = i + 1
+            }
+            self.kept = table.len()
+        }
+        let life = if user == "" { self.visitor_life } else { self.admin_life }
+        let s = Session { id: random_hex(32), user: user, started: Instant::now(), life: life }
+        let _ = table.insert(s.id, s)
+        s
+    }
+
+    /// The tag of the session id `id`, in hex.
+    pub fn tag(self, id: String) -> String { hmac_sha256(self.key, bytes_from_string(id)).to_hex() }
+
+    /// The value of `s`'s cookie: its id, a `.` and its tag.
+    pub fn cookie(self, s: Session) -> String { "${s.id}.${self.tag(s.id)}" }
+
+    /// The session a cookie's value names: `None` unless it is `<id>.<tag>`
+    /// with the right tag, and the table holds the id, not ended.
+    pub fn find(self, cookie: String) -> Option<Session> {
+        let dot = match cookie.index_of(".") {
+            Some(at) => at
+            None => return None
+        }
+        let id = cookie.slice(0, dot)
+        let tag = match bytes_from_hex(cookie.slice(dot + 1, cookie.len())) {
+            Some(t) => t
+            None => return None
+        }
+        if !hmac_sha256_verify(self.key, bytes_from_string(id), tag) { return None }
+        match self.table.get(id) {
+            Some(s) => { if self.ended(s) { None } else { Some(s) } }
+            None => None
+        }
+    }
+
+    /// Ends the session `id`.
+    pub fn end(self, id: String) {
+        let mut table = self.table
+        let _ = table.remove(id)
+    }
+
+    /// The CSRF token of `s`, in hex.
+    pub fn token(self, s: Session) -> String {
+        hmac_sha256(self.key, bytes_from_string("csrf:${s.id}")).to_hex()
+    }
+
+    /// Whether `token` is `s`'s CSRF token, compared in constant time.
+    pub fn token_ok(self, s: Session, token: String) -> Bool {
+        match bytes_from_hex(token) {
+            Some(t) => hmac_sha256_verify(self.key, bytes_from_string("csrf:${s.id}"), t)
+            None => false
+        }
+    }
+}
+
+/// The user `name` with the password `password`, hashed over `iterations`
+/// rounds with 16 new random bytes of salt into 32 bytes.
+pub fn hash_password(name: String, password: String, iterations: Int) -> User {
+    let salt = match random_bytes(16) {
+        Ok(b) => b
+        Err(e) => panic("blog: the system gave no random bytes: ${e.message}")
+    }
+    let hash = pbkdf2_sha256(bytes_from_string(password), salt, iterations, 32)
+    User { name: name, salt: salt.to_hex(), iterations: iterations, hash: hash.to_hex() }
+}
+
+// The salt for a name no user has: fixed, so that a wrong name costs the
+// same rounds as a wrong password.
+fn no_salt() -> Bytes { bytes_from_string("fullstack-blog:no-such-user") }
+
+// Whether `n` is a count PBKDF2 takes.
+fn count_ok(n: Int) -> Bool { n >= 1 && n <= 4294967295 }
+
+/// The rounds a login with a name no user has spends: the largest count of
+/// `users`' hashes that PBKDF2 takes, or `otherwise` when there is none, so
+/// that such a login takes as long as one with the name of the user whose
+/// hash has the most rounds.
+pub fn no_user_rounds(users: Vec<User>, otherwise: Int) -> Int {
+    let mut most = 0
+    let mut i = 0
+    while i < users.len() {
+        let n = users.get(i).unwrap().iterations
+        if count_ok(n) && n > most { most = n }
+        i = i + 1
+    }
+    if most == 0 { otherwise } else { most }
+}
+
+/// Whether `password` is `found`'s. For a name no user has, `found` is
+/// `None`: the same work is done against a fixed salt with `iterations`
+/// rounds, `no_user_rounds`'s count, and the answer is `false`, so the two
+/// cases take as long and say the same. A stored hash that is not 32
+/// bytes never matches, since `pbkdf2_sha256_verify` would take its start
+/// for the whole.
+pub fn password_ok(found: Option<User>, password: String, iterations: Int) -> Bool {
+    let given = bytes_from_string(password)
+    match found {
+        Some(u) => {
+            let salt = bytes_from_hex(u.salt)
+            let hash = bytes_from_hex(u.hash)
+            if salt.is_none() || hash.is_none() || !count_ok(u.iterations) { return false }
+            let key = hash.unwrap()
+            if key.len() != 32 { return false }
+            pbkdf2_sha256_verify(given, salt.unwrap(), u.iterations, key)
+        }
+        None => {
+            let rounds = if count_ok(iterations) { iterations } else { 1 }
+            let _ = pbkdf2_sha256_verify(given, no_salt(), rounds, bytes_from_ints([0; 32]))
+            false
+        }
+    }
+}
+
+/// The key in `<dir>/secret`: 32 bytes in hex. When the file does not exist
+/// yet, 32 random bytes, written there first. An error names the file.
+pub async fn load_key(dir: String) -> Result<Bytes, String> {
+    let path = "${dir}/secret"
+    if exists(path).await {
+        let text = match read_to_string(path).await {
+            Ok(t) => t
+            Err(e) => return Err("cannot read ${path}: ${e.message}")
+        }
+        let key = bytes_from_hex(text.trim()).unwrap_or(bytes_from_ints([]))
+        if key.len() != 32 { return Err("${path} does not hold 32 bytes in hex") }
+        return Ok(key)
+    }
+    let key = match random_bytes(32) {
+        Ok(b) => b
+        Err(e) => return Err("the system gave no random bytes: ${e.message}")
+    }
+    match write_string(path, "${key.to_hex()}\n").await {
+        Ok(_) => Ok(key)
+        Err(e) => Err("cannot write ${path}: ${e.message}")
+    }
+}
diff --git a/examples/07-fullstack-blog/src/forms.nova b/examples/07-fullstack-blog/src/forms.nova
new file mode 100644
index 0000000000000000000000000000000000000000..83242962c1db721d8899c46592593f00895385d4
--- /dev/null
+++ b/examples/07-fullstack-blog/src/forms.nova
@@ -0,0 +1,118 @@
+//! The checks both sides run on a form's fields: the server on every
+//! `POST`, which it answers with the form again and `400` when one fails;
+//! the client as the fields change, once the page has hydrated. Each check
+//! returns the field's message, or `""` when the value is good. A value is
+//! measured in characters as the server stores it, `cleaned`.
+//!
+//! Also the pieces every form is built from: the hidden CSRF field, a
+//! field's message, and the submit button, disabled while the form is not
+//! valid.
+
+import ui
+import ui::html::{attr, button, class, div, dyn_text, input, name, p, text, value}
+
+// The longest title, in characters.
+const TITLE_MAX: Int = 200
+
+// The longest body.
+const BODY_MAX: Int = 20000
+
+// The longest name on a comment.
+const NAME_MAX: Int = 60
+
+// The longest comment.
+const TEXT_MAX: Int = 2000
+
+// `what`'s message when `value`, cleaned, is empty or longer than `max`.
+fn sized(value: String, max: Int, blank: String, what: String) -> String {
+    let n = cleaned(value).len()
+    if n == 0 { return blank }
+    if n > max { return "${what} has at most ${max} characters; this one has ${n}." }
+    ""
+}
+
+/// A post's title: 1 to 200 characters.
+pub fn title_message(title: String) -> String {
+    sized(title, TITLE_MAX, "Give the post a title.", "A title")
+}
+
+/// A post's body: 1 to 20,000 characters.
+pub fn body_message(body: String) -> String {
+    sized(body, BODY_MAX, "Write something in the post.", "A post")
+}
+
+/// The name on a comment: 1 to 60 characters.
+pub fn name_message(name: String) -> String { sized(name, NAME_MAX, "Say who you are.", "A name") }
+
+/// A comment: 1 to 2,000 characters.
+pub fn text_message(text: String) -> String {
+    sized(text, TEXT_MAX, "Write a comment.", "A comment")
+}
+
+/// The name at the login: not blank.
+pub fn login_name_message(name: String) -> String {
+    if name.trim() == "" { "Give your name." } else { "" }
+}
+
+/// The password at the login: not blank. It is not trimmed when checked.
+pub fn password_message(password: String) -> String {
+    if password.trim() == "" { "Give your password." } else { "" }
+}
+
+/// Whether a post's title and body pass.
+pub fn post_ok(title: String, body: String) -> Bool {
+    title_message(title) == "" && body_message(body) == ""
+}
+
+/// Whether a comment's name and text pass.
+pub fn comment_ok(name: String, text: String) -> Bool {
+    name_message(name) == "" && text_message(text) == ""
+}
+
+/// Whether a login's name and password pass.
+pub fn login_ok(name: String, password: String) -> Bool {
+    login_name_message(name) == "" && password_message(password) == ""
+}
+
+/// `text` as the server stores it: with no NUL, which a browser's parser
+/// drops from a page's text and makes U+FFFD in a `textarea` or an
+/// attribute, so a page holding one would not be the page its data
+/// renders; with each carriage return and line feed pair as a line feed, as
+/// a browser sends a `textarea`'s lines; and trimmed.
+pub fn cleaned(text: String) -> String {
+    let kept = "".join(text.split("\0"))
+    "\n".join(kept.split("\r\n")).trim()
+}
+
+// --- The pieces of a form ---
+
+/// The hidden field that carries the session's CSRF token.
+pub fn csrf_field(token: String) -> View {
+    input([attr("type", "hidden"), name("csrf"), value(token)])
+}
+
+/// A field's message, under it: `check()`'s, while `shown` holds.
+pub fn message(shown: Signal<Bool>, check: fn() -> String) -> View {
+    p([class("message")], [dyn_text(|| if shown.value() { check() } else { "" })])
+}
+
+/// The submit button, enabled while `ok` holds and disabled otherwise.
+///
+/// `std/ui` writes every attribute it is given and never removes one, and
+/// `disabled` disables a button by being there at all, whatever its value.
+/// So the two states are two buttons, each under a `show`: the server's
+/// HTML holds the enabled one, which a page without JavaScript keeps, and
+/// the client swaps them as `ok` changes.
+pub fn submit(ok: Signal<Bool>, label: String) -> View {
+    let blocked = memo(|| !ok.value())
+    div(
+        [class("actions")],
+        [
+            show(ok, || button([attr("type", "submit")], [text(label)])),
+            show(
+                blocked,
+                || button([attr("type", "submit"), attr("disabled", "disabled")], [text(label)]),
+            ),
+        ],
+    )
+}
diff --git a/examples/07-fullstack-blog/src/lib.nova b/examples/07-fullstack-blog/src/lib.nova
new file mode 100644
index 0000000000000000000000000000000000000000..622d4510dbd229594acb121742a558fffcb9c729
--- /dev/null
+++ b/examples/07-fullstack-blog/src/lib.nova
@@ -0,0 +1,105 @@
+//! The blog's own modules as a library, so `nova test` reaches them: it runs
+//! the tests of the `src/` modules the program and the library reach, and
+//! the client, `main.nova`, reaches only the shared ones. `tests/*.nova`
+//! import this library as `fullstack_blog`, and can name only what it
+//! declares, so it gives them a way in to each module; what they get back
+//! they use through its methods and fields.
+
+import app::{Data}
+import auth::{Sessions, hash_password, no_user_rounds, password_ok}
+import forms::{
+    body_message,
+    cleaned,
+    comment_ok,
+    login_name_message,
+    login_ok,
+    name_message,
+    password_message,
+    post_ok,
+    text_message,
+    title_message,
+}
+import post_card::{first_paragraph, paragraphs}
+import store::{Store, User, Users}
+
+/// The store `text` holds, as `data/seed.json` and `data/blog.json` hold
+/// it, or what is wrong with it.
+pub fn store_from(text: String) -> Result<Store, String> { Store::from_text(text) }
+
+/// The users `text` holds, as `data/users.json` holds them, or what is
+/// wrong with it.
+pub fn users_from(text: String) -> Result<Users, String> { Users::from_text(text) }
+
+/// No users, as when `data/users.json` does not exist yet.
+pub fn no_users() -> Users { Users::new() }
+
+/// No sessions, signed with `key`, lasting `visitor` seconds for a visitor
+/// and `admin` for the admin.
+pub fn sessions(key: Bytes, visitor: Int, admin: Int) -> Sessions {
+    Sessions::lasting(key, visitor, admin)
+}
+
+/// The user `name` with `password` hashed over `iterations` rounds.
+pub fn user_with(name: String, password: String, iterations: Int) -> User {
+    hash_password(name, password, iterations)
+}
+
+/// Whether `password` is the password of the user `name` of `users`,
+/// spending `iterations` rounds when there is no such user.
+pub fn logs_in(users: Users, name: String, password: String, iterations: Int) -> Bool {
+    password_ok(users.user(name), password, iterations)
+}
+
+/// The rounds a login with a name no user has spends: the largest count of
+/// `users`' hashes, or `otherwise` when there is none.
+pub fn rounds_for_no_user(users: Users, otherwise: Int) -> Int {
+    no_user_rounds(users.list, otherwise)
+}
+
+/// The message of the form field `field`, one of `title`, `body`, `name`,
+/// `text`, `login` and `password`, for `value`: `""` when it passes.
+pub fn message_for(field: String, value: String) -> String {
+    if field == "title" { return title_message(value) }
+    if field == "body" { return body_message(value) }
+    if field == "name" { return name_message(value) }
+    if field == "text" { return text_message(value) }
+    if field == "login" { return login_name_message(value) }
+    if field == "password" { return password_message(value) }
+    panic("no field ${field}")
+}
+
+/// Whether the form `form`, one of `post`, `comment` and `login`, passes
+/// with the values `a` and `b`: a title and a body, a name and a text, or a
+/// name and a password.
+pub fn form_ok(form: String, a: String, b: String) -> Bool {
+    if form == "post" { return post_ok(a, b) }
+    if form == "comment" { return comment_ok(a, b) }
+    if form == "login" { return login_ok(a, b) }
+    panic("no form ${form}")
+}
+
+/// `text` as the server stores a field: NULs dropped, line ends as line
+/// feeds, trimmed.
+pub fn stored(text: String) -> String { cleaned(text) }
+
+/// The paragraphs of a post's body.
+pub fn paragraphs_of(body: String) -> [String] { paragraphs(body) }
+
+/// The first paragraph of a post's body.
+pub fn first_of(body: String) -> String { first_paragraph(body) }
+
+/// The page data `json` holds, as the client reads it, or what is wrong.
+pub fn data_from(json: String) -> Result<Data, String> {
+    match parse(json) {
+        Ok(v) => {
+            match Data::from_json(v) {
+                Ok(d) => Ok(d)
+                Err(e) => Err(e.msg)
+            }
+        }
+        Err(e) => Err(e.msg)
+    }
+}
+
+/// `data` as the JSON the server sends with a page.
+pub fn data_json(data: Data) -> String { stringify(data.to_json()) }
diff --git a/examples/07-fullstack-blog/src/post_card.nova b/examples/07-fullstack-blog/src/post_card.nova
new file mode 100644
index 0000000000000000000000000000000000000000..9dcc39f7aa78d9728778610517f0a6df972b6dfe
--- /dev/null
+++ b/examples/07-fullstack-blog/src/post_card.nova
@@ -0,0 +1,72 @@
+//! A post as the home page lists it: its title, linked to its page, its
+//! date, and its first paragraph. And the rules for a post's text, which is
+//! plain: a blank line separates paragraphs, and each paragraph becomes a
+//! `<p>` whose text `std/ui` escapes.
+
+import app::{Post}
+import ui
+import ui::html::{a, article, class, div, h2, p, text}
+
+/// The paragraphs of `body`, in order: its lines, with a carriage return and
+/// line feed pair read as a line feed, cut at each line that is empty or
+/// only spaces, each paragraph's lines joined by line feeds and trimmed, and
+/// no paragraph empty.
+pub fn paragraphs(body: String) -> [String] {
+    let lines = "\n".join(body.split("\r\n")).split("\n")
+    let mut kept: Vec<String> = Vec::new()
+    let mut para = ""
+    let mut i = 0
+    while i < lines.len() {
+        let line = lines[i]
+        if line.trim() == "" {
+            if para.trim() != "" { kept.push(para.trim()) }
+            para = ""
+        } else if para == "" {
+            para = line
+        } else {
+            para = "${para}\n${line}"
+        }
+        i = i + 1
+    }
+    if para.trim() != "" { kept.push(para.trim()) }
+    let mut out = [""; kept.len()]
+    let mut k = 0
+    while k < kept.len() {
+        out[k] = kept.get(k).unwrap_or("")
+        k = k + 1
+    }
+    out
+}
+
+/// The first paragraph of `body`, or `""` when it has none.
+pub fn first_paragraph(body: String) -> String {
+    let all = paragraphs(body)
+    if all.len() == 0 { "" } else { all[0] }
+}
+
+/// The day of `when`, a time as `SystemTime::to_iso8601` writes it, such as
+/// `2026-10-21` for `2026-10-21T08:30:00.000Z`.
+pub fn day(when: String) -> String { if when.len() < 10 { when } else { when.slice(0, 10) } }
+
+/// `body`'s paragraphs, each a `<p>`, in a `div` of class `body`.
+pub fn body_view(body: String) -> View {
+    let all = paragraphs(body)
+    let mut views = [text(""); all.len()]
+    let mut i = 0
+    while i < all.len() {
+        views[i] = p([], [text(all[i])])
+        i = i + 1
+    }
+    div([class("body")], views)
+}
+
+/// A post on the home page.
+pub component PostCard(post: Post) {
+    view {
+        article(class: "card") {
+            h2 { a(href: "/posts/${post.id}") { text(post.title) } }
+            p(class: "date") { text(day(post.created)) }
+            p { text(first_paragraph(post.body)) }
+        }
+    }
+}
diff --git a/examples/07-fullstack-blog/src/store.nova b/examples/07-fullstack-blog/src/store.nova
new file mode 100644
index 0000000000000000000000000000000000000000..e455b88a8ecfcdb88eb84a87e8520b4f57965922
--- /dev/null
+++ b/examples/07-fullstack-blog/src/store.nova
@@ -0,0 +1,423 @@
+//! The store: the posts, the comments and the next ids, kept in memory and
+//! written whole to `<data>/blog.json` after each change; and the users,
+//! in `<data>/users.json`.
+//!
+//! Each file has one writer. Only the server writes `blog.json`, from its
+//! own copy of the store: a handler changes the store, then spawns the
+//! write, which runs after the response: the store's JSON goes to
+//! `blog.json.tmp`, then `rename` puts it over `blog.json`, so a crash never
+//! leaves half a file. Spawned writes run in order, so the file ends as the
+//! last change left the store. Only `--set-password` writes `users.json`,
+//! the same way, through `users.json.tmp`. The server reads `users.json`
+//! once, when it starts, and `--set-password` never reads or writes
+//! `blog.json`, so neither writes over a change the other made.
+//!
+//! Posts and comments keep the order they were written in; a view that
+//! wants the newest first walks them backwards, and no view walks a `Map`,
+//! whose order changes from one run to the next.
+//!
+//! Native only: it reads and writes files with `std/fs`.
+
+import app::{
+    Comment,
+    Post,
+    comments_from,
+    comments_json,
+    int_of,
+    list_of,
+    object_of,
+    posts_from,
+    posts_json,
+    string_of,
+}
+
+/// A user who can log in: a name, and the password's PBKDF2-HMAC-SHA256
+/// hash of 32 bytes, with the 16 random bytes of salt and the iteration
+/// count that made it, the bytes in hex.
+pub record User { pub name: String, pub salt: String, pub iterations: Int, pub hash: String }
+
+/// The posts, the comments and the next ids.
+pub record Store {
+    pub posts: Vec<Post>,
+    pub comments: Vec<Comment>,
+    pub next_post: Int,
+    pub next_comment: Int,
+}
+
+/// The users who can log in, in the order they were set.
+pub record Users { pub list: Vec<User> }
+
+// `v`'s items as an array.
+fn posts_of(v: Vec<Post>) -> [Post] {
+    let mut out = [Post { id: 0, title: "", body: "", created: "", updated: "" }; v.len()]
+    let mut i = 0
+    while i < v.len() {
+        out[i] = v.get(i).unwrap()
+        i = i + 1
+    }
+    out
+}
+
+fn comments_of(v: Vec<Comment>) -> [Comment] {
+    let mut out = [Comment { id: 0, post: 0, name: "", text: "", created: "" }; v.len()]
+    let mut i = 0
+    while i < v.len() {
+        out[i] = v.get(i).unwrap()
+        i = i + 1
+    }
+    out
+}
+
+impl ToJson for User {
+    fn to_json(self) -> JsonValue {
+        let mut m: Map<String, JsonValue> = Map::new()
+        m.insert("name", self.name.to_json())
+        m.insert("salt", self.salt.to_json())
+        m.insert("iterations", self.iterations.to_json())
+        m.insert("hash", self.hash.to_json())
+        Object(m)
+    }
+}
+
+impl FromJson for User {
+    fn from_json(v: JsonValue) -> Result<User, JsonError> {
+        let m = match object_of(v, "a user") {
+            Ok(m) => m
+            Err(e) => return Err(e)
+        }
+        let name = match string_of(m, "name") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let salt = match string_of(m, "salt") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let iterations = match int_of(m, "iterations") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        let hash = match string_of(m, "hash") {
+            Ok(x) => x
+            Err(e) => return Err(e)
+        }
+        Ok(User { name: name, salt: salt, iterations: iterations, hash: hash })
+    }
+}
+
+// The larger of `a` and `b`.
+fn larger(a: Int, b: Int) -> Int { if a > b { a } else { b } }
+
+impl Store {
+    /// The store `text` holds, as `blog.json` and `seed.json` hold it: an
+    /// object with `posts` and `comments`, and optionally `next_post` and
+    /// `next_comment`; any other key is ignored. A next id is never one
+    /// already taken. An error says what is wrong.
+    pub fn from_text(text: String) -> Result<Store, String> {
+        let v = match parse(text) {
+            Ok(v) => v
+            Err(e) => return Err("not JSON: ${e.msg} at ${e.at}")
+        }
+        let m = match object_of(v, "an object") {
+            Ok(m) => m
+            Err(e) => return Err(e.msg)
+        }
+        let listed = match list_of(m, "posts") {
+            Ok(xs) => xs
+            Err(e) => return Err(e.msg)
+        }
+        let posts = match posts_from(listed) {
+            Ok(xs) => xs
+            Err(e) => return Err(e.msg)
+        }
+        let said = match list_of(m, "comments") {
+            Ok(xs) => xs
+            Err(e) => return Err(e.msg)
+        }
+        let comments = match comments_from(said) {
+            Ok(xs) => xs
+            Err(e) => return Err(e.msg)
+        }
+        let mut store = Store {
+            posts: Vec::new(),
+            comments: Vec::new(),
+            next_post: int_of(m, "next_post").unwrap_or(1),
+            next_comment: int_of(m, "next_comment").unwrap_or(1),
+        }
+        let mut ps = store.posts
+        let mut i = 0
+        while i < posts.len() {
+            ps.push(posts[i])
+            store.next_post = larger(store.next_post, posts[i].id + 1)
+            i = i + 1
+        }
+        let mut cs = store.comments
+        i = 0
+        while i < comments.len() {
+            cs.push(comments[i])
+            store.next_comment = larger(store.next_comment, comments[i].id + 1)
+            i = i + 1
+        }
+        Ok(store)
+    }
+
+    /// The store as JSON, which `from_text` reads back.
+    pub fn to_text(self) -> String {
+        let mut m: Map<String, JsonValue> = Map::new()
+        m.insert("posts", posts_json(posts_of(self.posts)))
+        m.insert("comments", comments_json(comments_of(self.comments)))
+        m.insert("next_post", self.next_post.to_json())
+        m.insert("next_comment", self.next_comment.to_json())
+        stringify(Object(m))
+    }
+
+    /// The posts, newest first.
+    pub fn newest_first(self) -> [Post] {
+        let all = posts_of(self.posts)
+        let n = all.len()
+        let mut out = [Post { id: 0, title: "", body: "", created: "", updated: "" }; n]
+        let mut i = 0
+        while i < n {
+            out[i] = all[n - 1 - i]
+            i = i + 1
+        }
+        out
+    }
+
+    /// The post `id`, if there is one.
+    pub fn post(self, id: Int) -> Option<Post> {
+        let mut i = 0
+        while i < self.posts.len() {
+            let p = self.posts.get(i).unwrap()
+            if p.id == id { return Some(p) }
+            i = i + 1
+        }
+        None
+    }
+
+    /// The comments on post `id`, in the order they were written.
+    pub fn comments_on(self, id: Int) -> [Comment] {
+        let mut kept: Vec<Comment> = Vec::new()
+        let mut i = 0
+        while i < self.comments.len() {
+            let c = self.comments.get(i).unwrap()
+            if c.post == id { kept.push(c) }
+            i = i + 1
+        }
+        comments_of(kept)
+    }
+
+    /// The comment `id`, if there is one.
+    pub fn comment(self, id: Int) -> Option<Comment> {
+        let mut i = 0
+        while i < self.comments.len() {
+            let c = self.comments.get(i).unwrap()
+            if c.id == id { return Some(c) }
+            i = i + 1
+        }
+        None
+    }
+
+    /// Adds a post written at `now`, and returns it.
+    pub fn add_post(mut self, title: String, body: String, now: String) -> Post {
+        let p = Post { id: self.next_post, title: title, body: body, created: now, updated: now }
+        self.next_post = self.next_post + 1
+        let mut ps = self.posts
+        ps.push(p)
+        p
+    }
+
+    /// Changes post `id`'s title and body at `now`; `false` when there is no
+    /// such post.
+    pub fn edit_post(mut self, id: Int, title: String, body: String, now: String) -> Bool {
+        let mut ps = self.posts
+        let mut i = 0
+        while i < ps.len() {
+            let p = ps.get(i).unwrap()
+            if p.id == id {
+                ps.set(
+                    i,
+                    Post { id: id, title: title, body: body, created: p.created, updated: now },
+                )
+                return true
+            }
+            i = i + 1
+        }
+        false
+    }
+
+    /// Deletes post `id` and its comments; `false` when there is no such
+    /// post.
+    pub fn delete_post(mut self, id: Int) -> Bool {
+        if self.post(id).is_none() { return false }
+        let mut ps: Vec<Post> = Vec::new()
+        let mut i = 0
+        while i < self.posts.len() {
+            let p = self.posts.get(i).unwrap()
+            if p.id != id { ps.push(p) }
+            i = i + 1
+        }
+        let mut cs: Vec<Comment> = Vec::new()
+        i = 0
+        while i < self.comments.len() {
+            let c = self.comments.get(i).unwrap()
+            if c.post != id { cs.push(c) }
+            i = i + 1
+        }
+        self.posts = ps
+        self.comments = cs
+        true
+    }
+
+    /// Adds a comment on post `post` written at `now`, and returns it.
+    pub fn add_comment(mut self, post: Int, name: String, text: String, now: String) -> Comment {
+        let c = Comment { id: self.next_comment, post: post, name: name, text: text, created: now }
+        self.next_comment = self.next_comment + 1
+        let mut cs = self.comments
+        cs.push(c)
+        c
+    }
+
+    /// Deletes comment `id`, and returns it, or `None` when there is none.
+    pub fn delete_comment(mut self, id: Int) -> Option<Comment> {
+        let gone = self.comment(id)
+        if gone.is_none() { return None }
+        let mut cs: Vec<Comment> = Vec::new()
+        let mut i = 0
+        while i < self.comments.len() {
+            let c = self.comments.get(i).unwrap()
+            if c.id != id { cs.push(c) }
+            i = i + 1
+        }
+        self.comments = cs
+        gone
+    }
+}
+
+impl Users {
+    /// No users.
+    pub fn new() -> Users { Users { list: Vec::new() } }
+
+    /// The users `text` holds, as `users.json` holds them: an object whose
+    /// `users` is an array of users. An error says what is wrong.
+    pub fn from_text(text: String) -> Result<Users, String> {
+        let v = match parse(text) {
+            Ok(v) => v
+            Err(e) => return Err("not JSON: ${e.msg} at ${e.at}")
+        }
+        let m = match object_of(v, "an object") {
+            Ok(m) => m
+            Err(e) => return Err(e.msg)
+        }
+        let people = match list_of(m, "users") {
+            Ok(xs) => xs
+            Err(e) => return Err(e.msg)
+        }
+        let users = Users::new()
+        let mut us = users.list
+        let mut i = 0
+        while i < people.len() {
+            match User::from_json(people[i]) {
+                Ok(u) => us.push(u)
+                Err(e) => return Err(e.msg)
+            }
+            i = i + 1
+        }
+        Ok(users)
+    }
+
+    /// The users as JSON, which `from_text` reads back.
+    pub fn to_text(self) -> String {
+        let mut users = [Null; self.list.len()]
+        let mut i = 0
+        while i < self.list.len() {
+            users[i] = self.list.get(i).unwrap().to_json()
+            i = i + 1
+        }
+        let mut m: Map<String, JsonValue> = Map::new()
+        m.insert("users", Array(users))
+        stringify(Object(m))
+    }
+
+    /// The user named `name`, if there is one.
+    pub fn user(self, name: String) -> Option<User> {
+        let mut i = 0
+        while i < self.list.len() {
+            let u = self.list.get(i).unwrap()
+            if u.name == name { return Some(u) }
+            i = i + 1
+        }
+        None
+    }
+
+    /// Adds `u`, in place of a user of the same name.
+    pub fn set_user(mut self, u: User) {
+        let mut us: Vec<User> = Vec::new()
+        let mut i = 0
+        while i < self.list.len() {
+            let old = self.list.get(i).unwrap()
+            if old.name != u.name { us.push(old) }
+            i = i + 1
+        }
+        us.push(u)
+        self.list = us
+    }
+}
+
+/// The store in `<dir>/blog.json`, or, when that does not exist yet, in
+/// `<dir>/seed.json`. An error names the file and says what is wrong.
+pub async fn load_store(dir: String) -> Result<Store, String> {
+    let live = "${dir}/blog.json"
+    let path = if exists(live).await { live } else { "${dir}/seed.json" }
+    let text = match read_to_string(path).await {
+        Ok(t) => t
+        Err(e) => return Err("cannot read ${path}: ${e.message}")
+    }
+    match Store::from_text(text) {
+        Ok(s) => Ok(s)
+        Err(why) => Err("${path}: ${why}")
+    }
+}
+
+/// The users in `<dir>/users.json`, or none when it does not exist yet. An
+/// error names the file and says what is wrong.
+pub async fn load_users(dir: String) -> Result<Users, String> {
+    let path = "${dir}/users.json"
+    if !exists(path).await { return Ok(Users::new()) }
+    let text = match read_to_string(path).await {
+        Ok(t) => t
+        Err(e) => return Err("cannot read ${path}: ${e.message}")
+    }
+    match Users::from_text(text) {
+        Ok(u) => Ok(u)
+        Err(why) => Err("${path}: ${why}")
+    }
+}
+
+// Writes `text` to `<dir>/<name>.tmp`, then renames it over `<dir>/<name>`.
+async fn replace(dir: String, name: String, text: String) -> Result<(), IoError> {
+    let tmp = "${dir}/${name}.tmp"
+    match write_string(tmp, text).await {
+        Ok(_) => {}
+        Err(e) => return Err(e)
+    }
+    rename(tmp, "${dir}/${name}").await
+}
+
+/// Writes `users` to `<dir>/users.json.tmp`, then renames it over
+/// `<dir>/users.json`. Only `--set-password` calls it.
+pub async fn save_users(dir: String, users: Users) -> Result<(), IoError> {
+    replace(dir, "users.json", users.to_text()).await
+}
+
+/// Writes `store` to `<dir>/blog.json.tmp`, then renames it over
+/// `<dir>/blog.json`, for a running server's handler to spawn. It reads no
+/// file: the server's copy of the store is the whole of what it writes.
+/// It says on standard error when it cannot write, since no one awaits it;
+/// the change stays in memory, for the next write.
+pub async fn saved(dir: String, store: Store) {
+    match replace(dir, "blog.json", store.to_text()).await {
+        Ok(_) => {}
+        Err(e) => eprintln("blog: cannot save ${dir}/blog.json: ${e.message}")
+    }
+}
````


### Task 6: The rest of the blog, and `blog_tests.rs`

Spec §6, §7.4; decisions 9 to 13, 21 to 24, 26 to 29, 31, 35 to 37, 40
and 41.

**Files:**
- Create:
  - `examples/07-fullstack-blog/src/home.nova`, `post.nova`, `login.nova`,
    `admin.nova`, `header.nova`, `main.nova`, `server.nova`
  - `examples/07-fullstack-blog/index.html`, `public/styles.css`,
    `data/seed.json`, `README.md`
- Modify:
  - `examples/07-fullstack-blog/src/app.nova` (its pages, `routes`, `App`)
  - `.gitignore`
  - `crates/nova-driver/tests/index_complete.rs`
- Test: `crates/nova-cli/tests/blog_tests.rs`

- [ ] **Step 1: The tests.** `patch_block t6-tests`, `blog_tests.rs`: one
  build shared by its thirteen tests, each with a data directory of its own;
  a run of the server that should end is killed after 30 seconds, and its
  test fails (decision 31):
  - `every_page_answers_with_its_status`: a first page sets a visitor's
    cookie, `Max-Age=86400`, `HttpOnly` and `SameSite=Lax`, and a page
    asked for with it sets none; each page's status, markup and data; the
    built site's files served; and a path with no page, a post that does
    not exist and a `GET` of a path that has only a `POST` route answering
    the not-found page with `404`;
  - `a_comment_needs_its_sessions_csrf_token`: a comment without a token,
    with a wrong one, with another session's or with no session is `403`
    with the expired-form page, and changes nothing; with its session's
    token it is `303` to the post's comments, and on the page; one with a
    blank field comes back with its values, its messages and `400`;
  - `visitor_text_comes_back_escaped`: a title and a comment holding
    `<script>` come back escaped, in the page and in its data; and a
    comment whose name and text hold NULs comes back without them
    (decision 39);
  - `logging_in_takes_the_right_password_and_a_new_session`: a wrong
    password, or a name no user has, is `401` with the form again; a blank
    one `400`; the right one `303` to `/admin` with a new `sid`, lasting
    eight hours, and the session before the login worth nothing after it;
  - `a_name_no_user_has_takes_as_long_as_a_wrong_password` (decision 36);
  - `the_admin_creates_edits_and_deletes`: a post's create, edit and
    delete, and a comment's delete; a post that fails a check comes back
    with its values and `400`;
  - `the_admin_pages_need_the_admin`: without the admin's session, an admin
    page is `303` to `/login`, and an admin's form `403`, even with the
    visitor's own token, and no `blog.json` is written; in the admin's
    session, each admin form and the
    logout are `403` without their token or with another session's
    (decision 35);
  - `logging_out_ends_the_session`: the session ends, its cookie expires,
    and the logout needs the session's token;
  - `the_posts_survive_a_restart`: after a stop and a start, the posts and
    comments saved are there, from `blog.json`, which holds no users, the
    admin from `users.json`, and the key is `secret`'s; the sessions are
    not;
  - `set_password_writes_the_admin`: `--set-password` refuses, writing
    nothing, with status 1 a password shorter than eight characters, two
    blank ones and one that is not UTF-8, and with status 2 a blank name
    and names with a space or a tab around them, each with its message
    (decisions 27 and 40); the server refuses `--iterations 0` with status
    2; `--set-password` writes `users.json`, no `.tmp` left and no
    `blog.json` written, and replaces the user of that name when run
    again; a `users.json` that does not hold users is refused with status
    1, by `--set-password` and by the server, and left as it is (decision
    37);
  - `a_key_file_without_32_bytes_in_hex_is_refused`: the server, given an
    empty key file, 31 or 33 bytes in hex, or a key that is not hex, ends
    with status 1 and `blog: <data>/secret does not hold 32 bytes in hex`,
    prints nothing on standard output, and leaves the file as it was;
  - `a_password_set_while_the_server_runs_loses_nothing`: while
    `--set-password grace` hashes over 600,000 rounds, comments posted one
    by one, each awaited until `blog.json` holds it, some saved before it
    ends; then every one still in `blog.json`, `grace`'s login refused by
    the running server, both users in `users.json`, and after a restart
    both logins and every comment on the post's page (decision 37);
  - `overlapping_set_password_runs_keep_both_users`: `--set-password ada`
    over 600,000 rounds; half a second in, `--set-password bea` over 1,000
    rounds, run to its end; `ada`'s run still going then, and ending at
    least 250 ms after `bea`'s, or the test fails saying the runner is too
    slow; then both users in `users.json`, and both logging in on a server
    started afterwards (decision 37).
- [ ] **Step 2: Run, and see them fail.** First the build (Conventions).
  `cargo test -p nova-cli --test blog_tests`. Expected: it builds; `0
  passed; 13 failed`, each at `blog_tests.rs:107`, the shared build:
  ``nova build --ssr failed: Error: `fullstack-blog` is a library: it has no
  src/main.nova; `nova check` and `nova test` work on it``.
- [ ] **Step 3: The code.** `patch_block t6-code`:
  - **`src/app.nova`,** final: its pages' imports, `NotFound`, `Refused`,
    `routes(data)`, which maps the forms' `POST` paths too (decision 23),
    `content`, and `App`, whose root's `data-state` reads `server` in the
    server's HTML and `hydrated` once `on_mount` has run.
  - **The pages and components:** `home.nova`; `post.nova`, the post, its
    comments, the comment form and the admin's buttons to delete each
    comment; `login.nova`; `admin.nova`, the list and the editor; and
    `header.nova`. Each form's checks start in `on_mount` (decision 13).
  - **`src/main.nova`,** the client: `hydrate_with("#app", data_for("",
    ""), App)`.
  - **`src/server.nova`:** its arguments, `--address`, `--data`, `--dist`,
    `--set-password`, which refuses a blank password too, and
    `--iterations` (decisions 26, 27 and 40); a visitor's session started
    on each page `GET` without one, its cookie lasting as long (decision
    28); `checked` (decisions 9 and 29); the login, whose name no user has
    costs `no_user_rounds` over the users read at start (decision 41); the
    pages and the forms, each change saved by a spawned write of the
    server's own store (decision 37); `server`, its routes, the
    `GET` routes of its `POST`-only paths (decision 21), and the fallback,
    which asks `Dist::file` first; and `main`, which loads the store, the
    users from `users.json` (decision 37) and the key, opens `dist`, binds,
    prints `blog: serving
    http://<host>:<port>/`, and serves until a signal.
  - **`index.html`,** the page, its stylesheet and its root;
    **`public/styles.css`**; **`data/seed.json`,** three posts, two
    comments and the next ids, with no `users` key; **`README.md`,**
    `60-EXAMPLES.md` §9's sections, which say to stop the server, run
    `--set-password`, then start it, to run one `--set-password` at a time,
    and to keep `data/` private.
  - **`.gitignore`:** the example's `dist`, `target`, `data/blog.json`,
    `data/blog.json.tmp`, `data/users.json`, `data/users.json.tmp` and
    `data/secret` (decision 22).
  - **`index_complete.rs`:** 165 programs and 199 files: the blog is a
    program, and its client's `main.nova` reaches eight modules; the
    server's four modules, which only `server.nova` and `lib.nova` reach,
    are not counted.
- [ ] **Step 4: Run.** First the build.
  - `cargo test -p nova-cli --test blog_tests`. Expected: `13 passed`, in
    about 9 seconds, the build included; the timing test's ten logins over
    100,000 rounds, about 0.4 s each, and the 600,000-round hash that
    `a_password_set_while_the_server_runs_loses_nothing` posts comments
    through, about 2.6 s, are most of it.
  - `R the_blog_example_passes_its_own_tests`. Expected: `1 passed`.
  - `X` (about 30 s). Expected: `1 passed`.
  - The whole binaries:
    - `cargo test -p nova-driver` (about a minute). Expected: each binary
      passes: the library `37`, `analyze` `12`, `broken` `1` (it cuts the
      blog's `main.nova` too), `doc_tests` `8`, `fixes` `33`, `index` `24`,
      `index_complete` `1`, `packages` `15`, `registry` `5`.
    - `NOVA_REQUIRE_NODE=1 cargo test -p nova-cli --no-fail-fast`, in two
      to three minutes. Expected: every binary passes: the unit tests `41`,
      `blog_tests` `13`, `build_support` `12`, `deps` `7`, `dev_tests` `6`,
      `doc` `13`, `doc_test` `17`, `fmt` `16`, `install_scripts` `9`, `lsp`
      `36` (1 ignored), `lsp_fixes` `16`, `lsp_navigation` `38`, `packages`
      `10`, `project` `25`, `publish_script` `13`, `registry` `22`,
      `release_manifests` `7`, `run_tests` `327` (1 ignored), `ssr_tests`
      `9`, `vscode_extension` `3`, `wasm_tests` `230`.
- [ ] **Step 5: The checks,** as the conventions list them, and `git status
  --short --ignored --untracked-files=all examples tests`, which prints
  nothing: the tests work in copies.
- [ ] **Step 6: Commit** `examples/07-fullstack-blog: the pages, the client and the server; blog_tests.rs`.

**Patch `t6-tests`:**

<!-- patch t6-tests -->

````diff
diff --git a/crates/nova-cli/tests/blog_tests.rs b/crates/nova-cli/tests/blog_tests.rs
new file mode 100644
index 0000000000000000000000000000000000000000..0b7dc2404c063e165fe25a3faf9ffcd7cc2290c4
--- /dev/null
+++ b/crates/nova-cli/tests/blog_tests.rs
@@ -0,0 +1,1595 @@
+//! `examples/07-fullstack-blog`, end to end over HTTP (spec
+//! `docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md`
+//! §7.4).
+//!
+//! The example is copied once per run of this file into a directory with a
+//! fixed name, less what a build or a run in place may have left there, and
+//! built there with `nova build --ssr`, with a fresh `NOVA_HOME` beside it:
+//! a server build is slow, so the tests share it. Each test then gives the
+//! server a data directory of its own, holding the seed, sets the admin's
+//! password through `--set-password admin --iterations 1000` with the
+//! password on standard input, and starts the server on `127.0.0.1:0`,
+//! reading the port it prints. It speaks HTTP over `TcpStream`, one
+//! connection per request, reading each response by its `content-length`,
+//! since the server keeps a connection open. A server is killed when its
+//! test ends, pass or fail, before anything is removed. Every wait polls,
+//! for 30 seconds at most. The copied files may have CRLF line ends on a
+//! Windows checkout; nothing here compares their text. The test that times
+//! logins sets another user's password over 1000 rounds first, then the
+//! admin's over 100000.
+
+use std::io::{BufRead, BufReader, Read, Write};
+use std::net::TcpStream;
+use std::path::{Path, PathBuf};
+use std::process::{Child, Command, Stdio};
+use std::sync::OnceLock;
+use std::time::{Duration, Instant};
+
+/// How long any wait may take.
+const LIMIT: Duration = Duration::from_secs(30);
+
+/// The admin's password, which no file holds: each test sets it.
+const PASSWORD: &str = "correct horse battery";
+
+/// What the page of a refused form says.
+const EXPIRED: &str = "This form has expired";
+
+fn repo_root() -> PathBuf {
+    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
+}
+
+/// The directory this file's runs use, emptied by the build.
+fn base() -> PathBuf {
+    std::env::temp_dir().join("nova-blog-tests")
+}
+
+/// What a build or a run of the example in place may leave there, which the
+/// copy leaves out.
+fn left_out(relative: &Path) -> bool {
+    let generated = [
+        Path::new("dist"),
+        Path::new("target"),
+        Path::new("data/blog.json"),
+        Path::new("data/blog.json.tmp"),
+        Path::new("data/users.json"),
+        Path::new("data/users.json.tmp"),
+        Path::new("data/secret"),
+    ];
+    generated.contains(&relative)
+}
+
+/// Copies the directory `from` into `to`, less what [`left_out`] names.
+fn copy_tree(root: &Path, from: &Path, to: &Path) {
+    std::fs::create_dir_all(to).expect("create a directory");
+    for entry in std::fs::read_dir(from).expect("read a directory") {
+        let path = entry.expect("an entry").path();
+        let relative = path.strip_prefix(root).expect("inside the example");
+        if left_out(relative) {
+            continue;
+        }
+        let into = to.join(path.file_name().expect("a name"));
+        if path.is_dir() {
+            copy_tree(root, &path, &into);
+        } else {
+            std::fs::copy(&path, &into).expect("copy a file");
+        }
+    }
+}
+
+/// The example, copied and built once: the project's directory.
+fn project() -> &'static Path {
+    static BUILT: OnceLock<Result<PathBuf, String>> = OnceLock::new();
+    let built = BUILT.get_or_init(|| {
+        let base = base();
+        let _ = std::fs::remove_dir_all(&base);
+        let dir = base.join("fullstack-blog");
+        let example = repo_root().join("examples/07-fullstack-blog");
+        copy_tree(&example, &example, &dir);
+        let out = Command::new(env!("CARGO_BIN_EXE_nova"))
+            .args(["build", "--ssr"])
+            .current_dir(&dir)
+            .env("NOVA_HOME", base.join("home"))
+            .env("NO_COLOR", "1")
+            .env("RUST_BACKTRACE", "0")
+            .output()
+            .expect("run nova build --ssr");
+        if !out.status.success() {
+            return Err(format!(
+                "nova build --ssr failed: {}{}",
+                String::from_utf8_lossy(&out.stdout),
+                String::from_utf8_lossy(&out.stderr)
+            ));
+        }
+        Ok(dir)
+    });
+    match built {
+        Ok(dir) => dir,
+        Err(why) => panic!("{why}"),
+    }
+}
+
+/// The server's executable.
+fn server_exe() -> PathBuf {
+    project().join("target").join("debug").join(format!(
+        "fullstack-blog-server{}",
+        std::env::consts::EXE_SUFFIX
+    ))
+}
+
+/// A data directory for test `name`, holding only the seed: made once the
+/// build, which empties [`base`] first, is done.
+fn data_dir(name: &str) -> PathBuf {
+    let seed = project().join("data").join("seed.json");
+    let dir = base().join(format!("data-{name}"));
+    let _ = std::fs::remove_dir_all(&dir);
+    std::fs::create_dir_all(&dir).expect("create the data directory");
+    std::fs::copy(seed, dir.join("seed.json")).expect("copy the seed");
+    dir
+}
+
+/// Reads `pipe` to its end on a thread of its own: what it held, as text.
+fn drain(mut pipe: impl Read + Send + 'static) -> std::thread::JoinHandle<String> {
+    std::thread::spawn(move || {
+        let mut bytes = Vec::new();
+        let _ = pipe.read_to_end(&mut bytes);
+        String::from_utf8_lossy(&bytes).replace("\r\n", "\n")
+    })
+}
+
+/// `fullstack-blog-server ARGS --data DATA`, `stdin` on its standard input,
+/// to its end: its exit status, `None` when a signal ended it, its stdout
+/// and its stderr. A run that has not ended within [`LIMIT`], such as a
+/// server that started when it should have refused to, is killed, and the
+/// test fails. The server may end before it reads `stdin`, so a failure to
+/// write it is not one.
+fn run_server(data: &Path, args: &[&str], stdin: &[u8]) -> (Option<i32>, String, String) {
+    let mut child = Command::new(server_exe())
+        .args(args)
+        .arg("--data")
+        .arg(data)
+        .current_dir(project())
+        .stdin(Stdio::piped())
+        .stdout(Stdio::piped())
+        .stderr(Stdio::piped())
+        .spawn()
+        .expect("spawn the server");
+    let out = drain(child.stdout.take().expect("stdout is piped"));
+    let err = drain(child.stderr.take().expect("stderr is piped"));
+    let mut input = child.stdin.take().expect("stdin is piped");
+    let _ = input.write_all(stdin);
+    drop(input);
+    let deadline = Instant::now() + LIMIT;
+    let status = loop {
+        if let Some(status) = child.try_wait().expect("wait for the server") {
+            break status;
+        }
+        if Instant::now() > deadline {
+            let _ = child.kill();
+            let _ = child.wait();
+            let (out, err) = (
+                out.join().unwrap_or_default(),
+                err.join().unwrap_or_default(),
+            );
+            panic!("fullstack-blog-server {args:?} did not end within {LIMIT:?}: {out}{err}");
+        }
+        std::thread::sleep(Duration::from_millis(20));
+    };
+    (
+        status.code(),
+        out.join().unwrap_or_default(),
+        err.join().unwrap_or_default(),
+    )
+}
+
+/// Sets the admin's password in `data`, with a low count.
+fn set_password(data: &Path) {
+    set_password_over(data, "1000");
+}
+
+/// Sets the admin's password in `data`, hashed over `iterations` rounds.
+fn set_password_over(data: &Path, iterations: &str) {
+    let (code, out, err) = run_server(
+        data,
+        &["--set-password", "admin", "--iterations", iterations],
+        format!("{PASSWORD}\n").as_bytes(),
+    );
+    assert_eq!(code, Some(0), "--set-password failed: {out}{err}");
+    assert_eq!(out, "blog: set the password of admin\n");
+}
+
+/// A running server, killed when dropped.
+struct Blog {
+    child: Child,
+    port: u16,
+    stderr: Option<std::thread::JoinHandle<String>>,
+}
+
+impl Blog {
+    /// The server on `data`, once it has printed its URL.
+    fn start(data: &Path) -> Blog {
+        let mut child = Command::new(server_exe())
+            .args(["--address", "127.0.0.1:0", "--data"])
+            .arg(data)
+            .current_dir(project())
+            .stdin(Stdio::null())
+            .stdout(Stdio::piped())
+            .stderr(Stdio::piped())
+            .spawn()
+            .expect("spawn the server");
+        let stdout = child.stdout.take().expect("stdout is piped");
+        let mut stderr = child.stderr.take().expect("stderr is piped");
+        let stderr = std::thread::spawn(move || {
+            let mut s = String::new();
+            let _ = stderr.read_to_string(&mut s);
+            s
+        });
+        let (send, lines) = std::sync::mpsc::channel();
+        std::thread::spawn(move || {
+            for line in BufReader::new(stdout).lines() {
+                let Ok(line) = line else { break };
+                if send.send(line).is_err() {
+                    break;
+                }
+            }
+        });
+        let mut blog = Blog {
+            child,
+            port: 0,
+            stderr: Some(stderr),
+        };
+        let line = match lines.recv_timeout(LIMIT) {
+            Ok(line) => line,
+            Err(e) => blog.fail(&format!("no URL on stdout ({e})")),
+        };
+        let port = line
+            .strip_prefix("blog: serving http://127.0.0.1:")
+            .and_then(|rest| rest.strip_suffix('/'))
+            .and_then(|port| port.parse::<u16>().ok());
+        match port {
+            Some(port) if port != 0 => blog.port = port,
+            _ => blog.fail(&format!("not the URL line: {line:?}")),
+        }
+        blog
+    }
+
+    fn fail(&mut self, why: &str) -> ! {
+        let _ = self.child.kill();
+        let _ = self.child.wait();
+        let err = self
+            .stderr
+            .take()
+            .map(|h| h.join().unwrap_or_default())
+            .unwrap_or_default();
+        panic!("{why}; the server's stderr: {err:?}");
+    }
+
+    /// Kills the server and waits for it to end.
+    fn stop(mut self) {
+        let _ = self.child.kill();
+        let _ = self.child.wait();
+    }
+
+    fn get(&self, path: &str, cookie: Option<&str>) -> Response {
+        exchange(self.port, "GET", path, cookie, None)
+    }
+
+    fn post(&self, path: &str, cookie: Option<&str>, fields: &[(&str, &str)]) -> Response {
+        exchange(self.port, "POST", path, cookie, Some(fields))
+    }
+}
+
+impl Drop for Blog {
+    fn drop(&mut self) {
+        let _ = self.child.kill();
+        let _ = self.child.wait();
+    }
+}
+
+/// A response: its status, its head and its body.
+struct Response {
+    status: u16,
+    head: String,
+    body: String,
+}
+
+impl Response {
+    /// The values of the headers named `name`, in any case.
+    fn headers(&self, name: &str) -> Vec<&str> {
+        self.head
+            .split("\r\n")
+            .skip(1)
+            .filter_map(|line| {
+                let (n, v) = line.split_once(':')?;
+                n.eq_ignore_ascii_case(name).then(|| v.trim())
+            })
+            .collect()
+    }
+
+    fn header(&self, name: &str) -> Option<&str> {
+        self.headers(name).first().copied()
+    }
+
+    /// The `sid` cookie this response sets, whole, attributes included.
+    fn set_sid(&self) -> Option<&str> {
+        self.headers("set-cookie")
+            .into_iter()
+            .find(|c| c.starts_with("sid="))
+    }
+
+    /// The `sid` cookie this response sets, as a request sends it back.
+    fn sid(&self) -> String {
+        let line = self
+            .set_sid()
+            .unwrap_or_else(|| panic!("no sid cookie in {}", self.head));
+        line.split(';').next().expect("a pair").to_string()
+    }
+
+    /// The value of the page's hidden `csrf` field.
+    fn csrf(&self) -> String {
+        let at = "name=\"csrf\" value=\"";
+        let Some(start) = self.body.find(at).map(|i| i + at.len()) else {
+            panic!("no csrf field in {}", self.body);
+        };
+        let end = start + self.body[start..].find('"').expect("the value ends");
+        self.body[start..end].to_string()
+    }
+
+    /// The CSRF token in the page's data, which every page has, with a form
+    /// or not.
+    fn data_csrf(&self) -> String {
+        let at = "\"csrf\":\"";
+        let Some(start) = self.body.find(at).map(|i| i + at.len()) else {
+            panic!("no token in the data of {}", self.body);
+        };
+        let end = start + self.body[start..].find('"').expect("the token ends");
+        self.body[start..end].to_string()
+    }
+
+    /// Asserts a `303` to `location`.
+    fn redirects_to(&self, location: &str) {
+        assert_eq!(self.status, 303, "{}{}", self.head, self.body);
+        assert_eq!(self.header("location"), Some(location), "{}", self.head);
+    }
+}
+
+/// `fields` as a browser encodes a form.
+fn encoded(fields: &[(&str, &str)]) -> String {
+    let mut out = String::new();
+    for (i, (name, value)) in fields.iter().enumerate() {
+        if i > 0 {
+            out.push('&');
+        }
+        for (j, text) in [name, value].iter().enumerate() {
+            if j > 0 {
+                out.push('=');
+            }
+            for b in text.bytes() {
+                match b {
+                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
+                        out.push(b as char)
+                    }
+                    b' ' => out.push('+'),
+                    _ => out.push_str(&format!("%{b:02X}")),
+                }
+            }
+        }
+    }
+    out
+}
+
+/// One request on a connection of its own, and its response, read by its
+/// `content-length`.
+fn exchange(
+    port: u16,
+    method: &str,
+    path: &str,
+    cookie: Option<&str>,
+    fields: Option<&[(&str, &str)]>,
+) -> Response {
+    let mut request = format!("{method} {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\n");
+    if let Some(c) = cookie {
+        request.push_str(&format!("Cookie: {c}\r\n"));
+    }
+    let body = fields.map(encoded).unwrap_or_default();
+    if fields.is_some() {
+        request.push_str("Content-Type: application/x-www-form-urlencoded\r\n");
+    }
+    request.push_str(&format!("Content-Length: {}\r\n\r\n{body}", body.len()));
+    let mut sock = TcpStream::connect(("127.0.0.1", port)).expect("connect to the server");
+    sock.set_read_timeout(Some(LIMIT)).expect("a read timeout");
+    sock.set_write_timeout(Some(LIMIT))
+        .expect("a write timeout");
+    sock.write_all(request.as_bytes())
+        .expect("send the request");
+    let mut read: Vec<u8> = Vec::new();
+    let mut chunk = [0u8; 8192];
+    loop {
+        if let Some(end) = read.windows(4).position(|w| w == b"\r\n\r\n") {
+            let head = String::from_utf8_lossy(&read[..end]).into_owned();
+            let length: usize = head
+                .split("\r\n")
+                .find_map(|line| {
+                    let (n, v) = line.split_once(':')?;
+                    n.eq_ignore_ascii_case("content-length")
+                        .then(|| v.trim().parse().ok())?
+                })
+                .unwrap_or_else(|| panic!("no content-length in {head}"));
+            if read.len() >= end + 4 + length {
+                let status = head
+                    .split(' ')
+                    .nth(1)
+                    .and_then(|s| s.parse().ok())
+                    .unwrap_or_else(|| panic!("no status in {head:?}"));
+                let body = String::from_utf8_lossy(&read[end + 4..end + 4 + length]).into_owned();
+                return Response { status, head, body };
+            }
+        }
+        match sock.read(&mut chunk) {
+            Ok(0) => panic!(
+                "{method} {path}: the server closed the connection after {:?}",
+                String::from_utf8_lossy(&read)
+            ),
+            Ok(n) => read.extend_from_slice(&chunk[..n]),
+            Err(e) => panic!(
+                "{method} {path}: reading failed ({e}) after {:?}",
+                String::from_utf8_lossy(&read)
+            ),
+        }
+    }
+}
+
+/// A visitor: the cookie a first page set, and that page's CSRF token.
+fn visitor(blog: &Blog, path: &str) -> (String, String) {
+    let page = blog.get(path, None);
+    assert_eq!(page.status, 200, "{}", page.body);
+    (page.sid(), page.csrf())
+}
+
+/// The user `name`, logged in through the form with `password`: the new
+/// session's cookie, and its token.
+fn log_in(blog: &Blog, name: &str, password: &str) -> (String, String) {
+    let (sid, csrf) = visitor(blog, "/login");
+    let r = blog.post(
+        "/login",
+        Some(&sid),
+        &[("csrf", &csrf), ("name", name), ("password", password)],
+    );
+    r.redirects_to("/admin");
+    let session = r.sid();
+    let page = blog.get("/admin", Some(&session));
+    assert_eq!(page.status, 200, "{}", page.body);
+    (session, page.csrf())
+}
+
+/// The admin, logged in through the form: the new session's cookie, and
+/// its token.
+fn admin(blog: &Blog) -> (String, String) {
+    log_in(blog, "admin", PASSWORD)
+}
+
+/// The number of comments on a post's page.
+fn comments(blog: &Blog, path: &str) -> usize {
+    let page = blog.get(path, None);
+    assert_eq!(page.status, 200, "{}", page.body);
+    page.body.matches("<article class=\"comment\">").count()
+}
+
+/// Polls until `test` holds of the store's file in `data`.
+fn until_saved(data: &Path, test: impl Fn(&str) -> bool) {
+    if let Err(text) = saved_within(data, test) {
+        panic!("blog.json never held the change: {text}");
+    }
+}
+
+/// Polls until `test` holds of the store's file in `data`, for [`LIMIT`] at
+/// most; then the file's text, as an error.
+fn saved_within(data: &Path, test: impl Fn(&str) -> bool) -> Result<(), String> {
+    let deadline = Instant::now() + LIMIT;
+    loop {
+        let text = std::fs::read_to_string(data.join("blog.json")).unwrap_or_default();
+        if test(&text) {
+            return Ok(());
+        }
+        if Instant::now() > deadline {
+            return Err(text);
+        }
+        std::thread::sleep(Duration::from_millis(50));
+    }
+}
+
+/// Every page's status, with its markup and its data: a first page sets a
+/// visitor's session cookie, `HttpOnly` and `SameSite=Lax`, lasting a day,
+/// and a page asked for with it sets none; the built site's files are
+/// served; a path with no page, a post that does not exist, and a `GET` of a
+/// path that has only a `POST` route answer the not-found page with `404`.
+#[test]
+fn every_page_answers_with_its_status() {
+    let data = data_dir("pages");
+    set_password(&data);
+    let blog = Blog::start(&data);
+
+    let home = blog.get("/", None);
+    assert_eq!(home.status, 200);
+    assert_eq!(
+        home.header("content-type"),
+        Some("text/html; charset=utf-8")
+    );
+    let set = home.set_sid().expect("a session cookie");
+    let value = set
+        .strip_prefix("sid=")
+        .and_then(|v| v.split(';').next())
+        .unwrap_or("");
+    let (id, tag) = value.split_once('.').expect("<id>.<tag>");
+    assert_eq!((id.len(), tag.len()), (64, 64), "{set}");
+    assert!(
+        set.ends_with("; Path=/; Max-Age=86400; HttpOnly; SameSite=Lax"),
+        "{set}"
+    );
+    // The posts, newest first, each with its first paragraph only.
+    let first = home
+        .body
+        .find("Sessions, tokens and passwords")
+        .expect("post 3");
+    let second = home.body.find("How the forms work").expect("post 2");
+    let third = home.body.find("Hello from a Nova server").expect("post 1");
+    assert!(first < second && second < third, "{}", home.body);
+    assert!(home.body.contains("<a href=\"/posts/3\">"), "{}", home.body);
+    assert!(home.body.contains("<p class=\"date\">2026-10-21</p>"));
+    assert!(!home.body.contains("stored only"), "{}", home.body);
+    assert!(home
+        .body
+        .contains("<link rel=\"stylesheet\" href=\"/assets/styles.css\">"));
+    assert!(home
+        .body
+        .contains("<div id=\"root\" class=\"page\" data-state=\"server\">"));
+    assert!(home
+        .body
+        .contains("<script type=\"application/json\" id=\"nova-data\">{"));
+    assert!(home.body.contains("<a href=\"/login\">Log in</a>"));
+    let sid = home.sid();
+    let again = blog.get("/", Some(&sid));
+    assert_eq!(again.status, 200);
+    assert!(again.set_sid().is_none(), "{}", again.head);
+    assert_eq!(again.data_csrf(), home.data_csrf());
+    assert_eq!(home.data_csrf().len(), 64);
+
+    let post = blog.get("/posts/1", Some(&sid));
+    assert_eq!(post.status, 200);
+    assert!(post.body.contains("<h1>Hello from a Nova server</h1>"));
+    // Three paragraphs in the post, and Ada's comment.
+    let article = post
+        .body
+        .split("<section id=\"comments\">")
+        .next()
+        .unwrap_or("");
+    assert_eq!(
+        article.matches("<div class=\"body\"><p>").count(),
+        1,
+        "{article}"
+    );
+    assert_eq!(article.matches("</p><p>").count(), 2, "{article}");
+    assert!(post.body.contains("<strong>Ada</strong>"), "{}", post.body);
+    assert!(post.body.contains(
+        "<form id=\"comment-form\" class=\"stacked\" method=\"post\" action=\"/posts/1/comments\">"
+    ));
+    assert!(post.body.contains(&format!(
+        "<input type=\"hidden\" name=\"csrf\" value=\"{}\">",
+        home.data_csrf()
+    )));
+    // The server's HTML has the enabled button, and no message.
+    assert!(post.body.contains("<div class=\"actions\"><!--[--><button type=\"submit\">Comment</button><!--]--><!--[--><!--]--></div>"), "{}", post.body);
+    assert_eq!(post.body.matches("<p class=\"message\"></p>").count(), 2);
+    assert!(post.body.contains("\"post\":{"), "{}", post.body);
+
+    let login = blog.get("/login", Some(&sid));
+    assert_eq!(login.status, 200);
+    assert!(login.body.contains("<form id=\"login-form\""));
+    assert!(login.body.contains("type=\"password\""));
+    assert_eq!(login.csrf(), home.data_csrf());
+
+    for path in [
+        "/nowhere",
+        "/posts/99",
+        "/posts/x",
+        "/posts/1/comments",
+        "/logout",
+        "/admin/posts",
+    ] {
+        let r = blog.get(path, Some(&sid));
+        assert_eq!(r.status, 404, "{path}");
+        assert!(r.body.contains("<h1>Not found</h1>"), "{path}: {}", r.body);
+    }
+    // A page with no session yet starts one, the not-found page too.
+    assert!(blog.get("/nowhere", None).set_sid().is_some());
+
+    let js = blog.get("/app.js", None);
+    assert_eq!(js.status, 200);
+    assert_eq!(
+        js.header("content-type"),
+        Some("text/javascript; charset=utf-8")
+    );
+    assert!(js.set_sid().is_none());
+    let css = blog.get("/assets/styles.css", None);
+    assert_eq!(css.status, 200);
+    assert_eq!(css.header("content-type"), Some("text/css; charset=utf-8"));
+    assert_eq!(blog.get("/manifest.json", None).status, 404);
+}
+
+/// A comment without a CSRF token, with a wrong one, with another session's,
+/// or with no session, is refused with `403` and the page saying the form
+/// has expired, and changes nothing; with its session's token it is `303`
+/// to the post's comments, and on the page. A comment with a blank field
+/// comes back with its values, its messages and `400`.
+#[test]
+fn a_comment_needs_its_sessions_csrf_token() {
+    let data = data_dir("csrf");
+    let blog = Blog::start(&data);
+    let (sid, csrf) = visitor(&blog, "/posts/1");
+    let (other_sid, other_csrf) = visitor(&blog, "/posts/1");
+    assert_ne!(csrf, other_csrf);
+    assert_eq!(comments(&blog, "/posts/1"), 1);
+
+    let comment = [("name", "Katherine"), ("text", "Refused")];
+    let without = blog.post("/posts/1/comments", Some(&sid), &comment);
+    let wrong = blog.post(
+        "/posts/1/comments",
+        Some(&sid),
+        &[
+            ("csrf", &"0".repeat(64)),
+            ("name", "Katherine"),
+            ("text", "Refused"),
+        ],
+    );
+    let others = blog.post(
+        "/posts/1/comments",
+        Some(&sid),
+        &[
+            ("csrf", &other_csrf),
+            ("name", "Katherine"),
+            ("text", "Refused"),
+        ],
+    );
+    let no_session = blog.post(
+        "/posts/1/comments",
+        None,
+        &[("csrf", &csrf), ("name", "Katherine"), ("text", "Refused")],
+    );
+    for (what, r) in [
+        ("no token", &without),
+        ("a wrong token", &wrong),
+        ("another session's token", &others),
+        ("no session", &no_session),
+    ] {
+        assert_eq!(r.status, 403, "{what}: {}", r.body);
+        assert!(r.body.contains(EXPIRED), "{what}: {}", r.body);
+        assert!(r.set_sid().is_none(), "{what}");
+    }
+    assert_eq!(comments(&blog, "/posts/1"), 1);
+    assert!(
+        !data.join("blog.json").exists(),
+        "a refused form wrote the store"
+    );
+
+    let blank = blog.post(
+        "/posts/1/comments",
+        Some(&sid),
+        &[
+            ("csrf", &csrf),
+            ("name", "  Katherine "),
+            ("text", " \r\n "),
+        ],
+    );
+    assert_eq!(blank.status, 400, "{}", blank.body);
+    assert!(
+        blank
+            .body
+            .contains("<p class=\"message\">Write a comment.</p>"),
+        "{}",
+        blank.body
+    );
+    assert!(blank.body.contains("value=\"Katherine\""), "{}", blank.body);
+    assert!(blank.body.contains("<h1>Hello from a Nova server</h1>"));
+    assert_eq!(comments(&blog, "/posts/1"), 1);
+
+    let posted = blog.post(
+        "/posts/1/comments",
+        Some(&sid),
+        &[
+            ("csrf", &csrf),
+            ("name", "Katherine"),
+            ("text", "One.\r\n\r\nTwo."),
+        ],
+    );
+    posted.redirects_to("/posts/1#comments");
+    let page = blog.get("/posts/1", Some(&other_sid));
+    assert_eq!(page.body.matches("<article class=\"comment\">").count(), 2);
+    assert!(
+        page.body.contains("<strong>Katherine</strong>"),
+        "{}",
+        page.body
+    );
+    assert!(
+        page.body
+            .contains("<div class=\"body\"><p>One.</p><p>Two.</p></div>"),
+        "{}",
+        page.body
+    );
+    // A comment on a post that does not exist.
+    let nowhere = blog.post(
+        "/posts/99/comments",
+        Some(&sid),
+        &[("csrf", &csrf), ("name", "Katherine"), ("text", "Lost")],
+    );
+    assert_eq!(nowhere.status, 404);
+}
+
+/// A title and a comment holding `<script>` come back escaped, in the page
+/// and in its data: no page holds the script as markup. A comment holding a
+/// NUL comes back without it.
+#[test]
+fn visitor_text_comes_back_escaped() {
+    let data = data_dir("escaping");
+    set_password(&data);
+    let blog = Blog::start(&data);
+    let script = "<script>alert(\"x\")</script>";
+    let escaped = "&lt;script&gt;alert(\"x\")&lt;/script&gt;";
+    let in_data = "\\u003cscript\\u003ealert(\\\"x\\\")\\u003c/script\\u003e";
+
+    let (sid, csrf) = visitor(&blog, "/posts/2");
+    blog.post(
+        "/posts/2/comments",
+        Some(&sid),
+        &[("csrf", &csrf), ("name", script), ("text", script)],
+    )
+    .redirects_to("/posts/2#comments");
+    let page = blog.get("/posts/2", None);
+    assert!(!page.body.contains(script), "{}", page.body);
+    assert_eq!(page.body.matches(escaped).count(), 2, "{}", page.body);
+    assert_eq!(page.body.matches(in_data).count(), 2, "{}", page.body);
+    // A NUL, which a browser's parser drops from the page's text, is not
+    // stored, so the page and the data it hydrates from agree.
+    blog.post(
+        "/posts/2/comments",
+        Some(&sid),
+        &[("csrf", &csrf), ("name", "Nu\0ll"), ("text", "one\0two")],
+    )
+    .redirects_to("/posts/2#comments");
+    let page = blog.get("/posts/2", None);
+    assert!(
+        !page.body.contains('\0'),
+        "a NUL came back: {:?}",
+        page.body
+    );
+    assert!(page.body.contains("<strong>Null</strong>"), "{}", page.body);
+    assert!(
+        page.body
+            .contains("<div class=\"body\"><p>onetwo</p></div>"),
+        "{}",
+        page.body
+    );
+    assert!(page.body.contains("\"text\":\"onetwo\""), "{}", page.body);
+
+    let (admin, token) = admin(&blog);
+    let title = format!("A title {script} & more");
+    blog.post(
+        "/admin/posts",
+        Some(&admin),
+        &[("csrf", &token), ("title", &title), ("body", script)],
+    )
+    .redirects_to("/admin");
+    for (path, cookie) in [
+        ("/", None),
+        ("/posts/4", None),
+        ("/admin", Some(admin.as_str())),
+    ] {
+        let page = blog.get(path, cookie);
+        assert_eq!(page.status, 200, "{path}");
+        assert!(!page.body.contains(script), "{path}: {}", page.body);
+        assert!(
+            page.body.contains(&format!("A title {escaped} &amp; more")),
+            "{path}: {}",
+            page.body
+        );
+        assert!(page.body.contains(in_data), "{path}: {}", page.body);
+    }
+    // The editor holds the title in an attribute, and the body as the text
+    // of its `textarea`.
+    let editor = blog.get("/admin/posts/4/edit", Some(&admin));
+    assert!(
+        editor.body.contains(
+            "value=\"A title &lt;script&gt;alert(&quot;x&quot;)&lt;/script&gt; &amp; more\""
+        ),
+        "{}",
+        editor.body
+    );
+    assert!(
+        editor.body.contains(&format!(
+            "<textarea name=\"body\" rows=\"16\">{escaped}</textarea>"
+        )),
+        "{}",
+        editor.body
+    );
+}
+
+/// A wrong password, or a name no user has, is `401` with the form again; a
+/// blank one is `400`; the right one is `303` to `/admin` with a new `sid`,
+/// lasting eight hours, and the session before the login is worth nothing
+/// after it.
+#[test]
+fn logging_in_takes_the_right_password_and_a_new_session() {
+    let data = data_dir("login");
+    set_password(&data);
+    let blog = Blog::start(&data);
+    let (sid, csrf) = visitor(&blog, "/login");
+
+    for (name, password) in [("admin", "wrong password"), ("nobody", PASSWORD)] {
+        let r = blog.post(
+            "/login",
+            Some(&sid),
+            &[("csrf", &csrf), ("name", name), ("password", password)],
+        );
+        assert_eq!(r.status, 401, "{name}: {}", r.body);
+        assert!(r.body.contains("Wrong name or password."), "{}", r.body);
+        assert!(r.body.contains(&format!("value=\"{name}\"")), "{}", r.body);
+        assert!(!r.body.contains(password), "the password came back");
+        assert!(r.set_sid().is_none());
+    }
+    let blank = blog.post(
+        "/login",
+        Some(&sid),
+        &[("csrf", &csrf), ("name", "admin"), ("password", "")],
+    );
+    assert_eq!(blank.status, 400);
+    assert!(blank.body.contains("Give your password."), "{}", blank.body);
+    let refused = blog.post(
+        "/login",
+        Some(&sid),
+        &[("name", "admin"), ("password", PASSWORD)],
+    );
+    assert_eq!(refused.status, 403);
+
+    let r = blog.post(
+        "/login",
+        Some(&sid),
+        &[("csrf", &csrf), ("name", "admin"), ("password", PASSWORD)],
+    );
+    r.redirects_to("/admin");
+    let set = r.set_sid().expect("a new session cookie");
+    assert!(
+        set.ends_with("; Path=/; Max-Age=28800; HttpOnly; SameSite=Lax"),
+        "{set}"
+    );
+    let admin = r.sid();
+    assert_ne!(admin, sid);
+    let page = blog.get("/admin", Some(&admin));
+    assert_eq!(page.status, 200);
+    assert!(page.body.contains("Log out admin"), "{}", page.body);
+    // The session from before the login has ended.
+    blog.get("/admin", Some(&sid)).redirects_to("/login");
+    let old = blog.post(
+        "/posts/1/comments",
+        Some(&sid),
+        &[("csrf", &csrf), ("name", "Ada"), ("text", "Late")],
+    );
+    assert_eq!(old.status, 403);
+    let fresh = blog.get("/", Some(&sid));
+    assert!(
+        fresh.set_sid().is_some(),
+        "the old cookie still names a session"
+    );
+    // The admin's id with any other tag is no session.
+    let (id, tag) = admin.split_once('.').expect("<id>.<tag>");
+    let other = if tag.starts_with('0') { "1" } else { "0" };
+    for forged in [
+        format!("{id}.{other}{}", &tag[1..]),
+        format!("{id}.{}", "0".repeat(64)),
+        id.to_string(),
+    ] {
+        blog.get("/admin", Some(&forged)).redirects_to("/login");
+        assert!(blog.get("/", Some(&forged)).set_sid().is_some(), "{forged}");
+    }
+}
+
+/// A login with a name no user has takes about as long as one with the
+/// admin's name and a wrong password: both run PBKDF2 over the most rounds
+/// a user's hash has, the admin's 100000, so that the hashing, not the HTTP
+/// around it, is most of a login's time. Another user, set first, has 1000,
+/// so a name no user has that cost the first user's count would answer in
+/// a few milliseconds. The two kinds alternate, five of each, and their
+/// medians are compared with a generous ratio, each at least half the
+/// other, so that a slow or busy runner does not fail it. Without the work
+/// for a name no user has, its login takes a few milliseconds against
+/// hundreds.
+#[test]
+fn a_name_no_user_has_takes_as_long_as_a_wrong_password() {
+    const ROUNDS: usize = 5;
+    let data = data_dir("timing");
+    let (code, out, err) = run_server(
+        &data,
+        &["--set-password", "grace", "--iterations", "1000"],
+        b"a password for grace\n",
+    );
+    assert_eq!(code, Some(0), "{out}{err}");
+    set_password_over(&data, "100000");
+    let blog = Blog::start(&data);
+    let (sid, csrf) = visitor(&blog, "/login");
+    let login = |name: &str| {
+        let started = Instant::now();
+        let r = blog.post(
+            "/login",
+            Some(&sid),
+            &[
+                ("csrf", &csrf),
+                ("name", name),
+                ("password", "wrong password"),
+            ],
+        );
+        let took = started.elapsed();
+        assert_eq!(r.status, 401, "{name}: {}", r.body);
+        took
+    };
+    let mut known = Vec::new();
+    let mut unknown = Vec::new();
+    for round in 0..ROUNDS {
+        if round % 2 == 0 {
+            known.push(login("admin"));
+            unknown.push(login("nobody"));
+        } else {
+            unknown.push(login("nobody"));
+            known.push(login("admin"));
+        }
+    }
+    known.sort();
+    unknown.sort();
+    eprintln!("a wrong password: {known:?}; a name no user has: {unknown:?}");
+    let (k, u) = (known[ROUNDS / 2], unknown[ROUNDS / 2]);
+    assert!(
+        u * 2 >= k && k * 2 >= u,
+        "medians: a wrong password {k:?}, a name no user has {u:?}; \
+         every time: {known:?}, {unknown:?}"
+    );
+}
+
+/// The admin's create, edit and delete of a post, and delete of a comment;
+/// a post that fails a check comes back with its values and `400`.
+#[test]
+fn the_admin_creates_edits_and_deletes() {
+    let data = data_dir("admin");
+    set_password(&data);
+    let blog = Blog::start(&data);
+    let (admin, token) = admin(&blog);
+
+    let editor = blog.get("/admin/new", Some(&admin));
+    assert_eq!(editor.status, 200);
+    assert!(
+        editor.body.contains(
+            "<form id=\"editor\" class=\"stacked\" method=\"post\" action=\"/admin/posts\">"
+        ),
+        "{}",
+        editor.body
+    );
+    let blank = blog.post(
+        "/admin/posts",
+        Some(&admin),
+        &[("csrf", &token), ("title", "Kept"), ("body", "")],
+    );
+    assert_eq!(blank.status, 400);
+    assert!(
+        blank.body.contains("Write something in the post."),
+        "{}",
+        blank.body
+    );
+    assert!(blank.body.contains("value=\"Kept\""), "{}", blank.body);
+
+    blog.post(
+        "/admin/posts",
+        Some(&admin),
+        &[
+            ("csrf", &token),
+            ("title", " A new post "),
+            ("body", "First.\r\n\r\nSecond."),
+        ],
+    )
+    .redirects_to("/admin");
+    let list = blog.get("/admin", Some(&admin));
+    assert!(
+        list.body.contains("<a href=\"/posts/4\">A new post</a>"),
+        "{}",
+        list.body
+    );
+    let first = list.body.find("A new post").expect("listed");
+    assert!(first < list.body.find("Sessions, tokens").expect("post 3"));
+    let post = blog.get("/posts/4", None);
+    assert_eq!(post.status, 200);
+    assert!(
+        post.body
+            .contains("<div class=\"body\"><p>First.</p><p>Second.</p></div>"),
+        "{}",
+        post.body
+    );
+
+    let edit = blog.get("/admin/posts/4/edit", Some(&admin));
+    assert_eq!(edit.status, 200);
+    assert!(
+        edit.body.contains("action=\"/admin/posts/4\""),
+        "{}",
+        edit.body
+    );
+    assert!(edit.body.contains("value=\"A new post\""), "{}", edit.body);
+    assert!(
+        edit.body.contains(">First.\n\nSecond.</textarea>"),
+        "{}",
+        edit.body
+    );
+    assert_eq!(blog.get("/admin/posts/99/edit", Some(&admin)).status, 404);
+    blog.post(
+        "/admin/posts/4",
+        Some(&admin),
+        &[
+            ("csrf", &token),
+            ("title", "An edited post"),
+            ("body", "Changed."),
+        ],
+    )
+    .redirects_to("/admin");
+    let post = blog.get("/posts/4", None);
+    assert!(
+        post.body.contains("<h1>An edited post</h1>"),
+        "{}",
+        post.body
+    );
+    assert!(post.body.contains("<p>Changed.</p>"), "{}", post.body);
+    let failed = blog.post(
+        "/admin/posts/4",
+        Some(&admin),
+        &[
+            ("csrf", &token),
+            ("title", &"x".repeat(201)),
+            ("body", "Changed."),
+        ],
+    );
+    assert_eq!(failed.status, 400);
+    assert!(failed
+        .body
+        .contains("A title has at most 200 characters; this one has 201."));
+
+    // A comment on post 4, comment 3, then post 4 deleted, with its
+    // comment, which is then no comment to delete.
+    let (sid, csrf) = visitor(&blog, "/posts/4");
+    blog.post(
+        "/posts/4/comments",
+        Some(&sid),
+        &[("csrf", &csrf), ("name", "Ada"), ("text", "Hi")],
+    )
+    .redirects_to("/posts/4#comments");
+    let page = blog.get("/posts/4", Some(&admin));
+    assert!(
+        page.body.contains("action=\"/admin/comments/3/delete\""),
+        "{}",
+        page.body
+    );
+    blog.post("/admin/posts/4/delete", Some(&admin), &[("csrf", &token)])
+        .redirects_to("/admin");
+    assert_eq!(blog.get("/posts/4", None).status, 404);
+    assert!(!blog
+        .get("/admin", Some(&admin))
+        .body
+        .contains("An edited post"));
+    assert_eq!(
+        blog.post("/admin/posts/4/delete", Some(&admin), &[("csrf", &token)])
+            .status,
+        404
+    );
+    assert_eq!(
+        blog.post(
+            "/admin/comments/3/delete",
+            Some(&admin),
+            &[("csrf", &token)]
+        )
+        .status,
+        404
+    );
+
+    // Ada's comment on post 1, deleted.
+    assert_eq!(comments(&blog, "/posts/1"), 1);
+    let page = blog.get("/posts/1", Some(&admin));
+    assert!(
+        page.body.contains("action=\"/admin/comments/1/delete\""),
+        "{}",
+        page.body
+    );
+    blog.post(
+        "/admin/comments/1/delete",
+        Some(&admin),
+        &[("csrf", &token)],
+    )
+    .redirects_to("/posts/1#comments");
+    assert_eq!(comments(&blog, "/posts/1"), 0);
+    assert_eq!(
+        blog.post(
+            "/admin/comments/1/delete",
+            Some(&admin),
+            &[("csrf", &token)]
+        )
+        .status,
+        404
+    );
+    // A visitor sees no delete buttons.
+    assert!(!blog.get("/posts/3", None).body.contains("/delete"));
+}
+
+/// Without the admin's session, an admin page is `303` to `/login`, and an
+/// admin's form `403`, even with the visitor's own token, and changes
+/// nothing. In the admin's own session, each of the admin's forms, and the
+/// logout, is `403` too without its token or with another session's, and
+/// changes nothing: the pages are the same after.
+#[test]
+fn the_admin_pages_need_the_admin() {
+    let data = data_dir("guarded");
+    set_password(&data);
+    let blog = Blog::start(&data);
+    let (sid, csrf) = visitor(&blog, "/login");
+    for path in ["/admin", "/admin/new", "/admin/posts/1/edit"] {
+        blog.get(path, None).redirects_to("/login");
+        blog.get(path, Some(&sid)).redirects_to("/login");
+    }
+    for (path, fields) in [
+        ("/admin/posts", vec![("title", "T"), ("body", "B")]),
+        ("/admin/posts/1", vec![("title", "T"), ("body", "B")]),
+        ("/admin/posts/1/delete", vec![]),
+        ("/admin/comments/1/delete", vec![]),
+    ] {
+        let mut with_token = vec![("csrf", csrf.as_str())];
+        with_token.extend(fields.iter().copied());
+        let r = blog.post(path, Some(&sid), &with_token);
+        assert_eq!(r.status, 403, "{path}: {}", r.body);
+        assert!(
+            r.body.contains("Only the admin can do that."),
+            "{path}: {}",
+            r.body
+        );
+        assert_eq!(blog.post(path, None, &fields).status, 403, "{path}");
+    }
+    let post = blog.get("/posts/1", None);
+    assert!(post.body.contains("<h1>Hello from a Nova server</h1>"));
+    assert_eq!(comments(&blog, "/posts/1"), 1);
+    assert!(
+        !data.join("blog.json").exists(),
+        "a refused form wrote the store"
+    );
+
+    // The admin, logged in, posting without the form's token, or with the
+    // visitor's above.
+    let (admin, _) = admin(&blog);
+    let paths = ["/", "/posts/1", "/admin"];
+    let pages = || -> Vec<String> {
+        paths
+            .iter()
+            .map(|path| {
+                let page = blog.get(path, Some(&admin));
+                assert_eq!(page.status, 200, "{path}: {}{}", page.head, page.body);
+                page.body
+            })
+            .collect()
+    };
+    let before = pages();
+    for (path, fields) in [
+        ("/admin/posts", vec![("title", "T"), ("body", "B")]),
+        ("/admin/posts/1", vec![("title", "T"), ("body", "B")]),
+        ("/admin/posts/1/delete", vec![]),
+        ("/admin/comments/1/delete", vec![]),
+        ("/logout", vec![]),
+    ] {
+        let mut others = vec![("csrf", csrf.as_str())];
+        others.extend(fields.iter().copied());
+        for (what, form) in [("no token", &fields), ("another session's token", &others)] {
+            let r = blog.post(path, Some(&admin), form);
+            assert_eq!(r.status, 403, "{path}, {what}: {}", r.body);
+            assert!(r.body.contains(EXPIRED), "{path}, {what}: {}", r.body);
+            assert!(r.set_sid().is_none(), "{path}, {what}");
+        }
+    }
+    for ((path, was), now) in paths.iter().zip(&before).zip(pages()) {
+        assert!(*was == now, "a refused form changed {path}: {now}");
+    }
+}
+
+/// Logging out ends the session and expires its cookie; it needs the
+/// session's token.
+#[test]
+fn logging_out_ends_the_session() {
+    let data = data_dir("logout");
+    set_password(&data);
+    let blog = Blog::start(&data);
+    let (admin, token) = admin(&blog);
+    assert_eq!(blog.post("/logout", Some(&admin), &[]).status, 403);
+    blog.get("/admin", Some(&admin));
+    let r = blog.post("/logout", Some(&admin), &[("csrf", &token)]);
+    r.redirects_to("/");
+    assert_eq!(
+        r.set_sid(),
+        Some("sid=; Path=/; Max-Age=0; HttpOnly; SameSite=Lax")
+    );
+    blog.get("/admin", Some(&admin)).redirects_to("/login");
+    let refused = blog.post(
+        "/admin/posts",
+        Some(&admin),
+        &[("csrf", &token), ("title", "T"), ("body", "B")],
+    );
+    assert_eq!(refused.status, 403);
+}
+
+/// After the server stops and starts again, the posts and comments it saved
+/// are still there, from `blog.json`, and the key is the one in `secret`;
+/// the sessions are not.
+#[test]
+fn the_posts_survive_a_restart() {
+    let data = data_dir("restart");
+    set_password(&data);
+    let blog = Blog::start(&data);
+    let (admin, token) = admin(&blog);
+    blog.post(
+        "/admin/posts",
+        Some(&admin),
+        &[
+            ("csrf", &token),
+            ("title", "Still here"),
+            ("body", "After a restart."),
+        ],
+    )
+    .redirects_to("/admin");
+    let (sid, csrf) = visitor(&blog, "/posts/1");
+    blog.post(
+        "/posts/1/comments",
+        Some(&sid),
+        &[("csrf", &csrf), ("name", "Grace"), ("text", "Me too.")],
+    )
+    .redirects_to("/posts/1#comments");
+    until_saved(&data, |text| {
+        text.contains("Still here") && text.contains("Me too.")
+    });
+    let saved = std::fs::read_to_string(data.join("blog.json")).expect("blog.json");
+    assert!(!saved.contains("\"users\""), "users in blog.json: {saved}");
+    let secret = std::fs::read_to_string(data.join("secret")).expect("the key's file");
+    assert_eq!(secret.trim().len(), 64);
+    assert!(!data.join("blog.json.tmp").exists());
+    blog.stop();
+
+    let blog = Blog::start(&data);
+    let home = blog.get("/", None);
+    assert!(
+        home.body.contains("<a href=\"/posts/4\">Still here</a>"),
+        "{}",
+        home.body
+    );
+    assert!(blog
+        .get("/posts/1", None)
+        .body
+        .contains("<strong>Grace</strong>"));
+    assert_eq!(
+        std::fs::read_to_string(data.join("secret")).expect("the key"),
+        secret
+    );
+    // A restart ends every session.
+    blog.get("/admin", Some(&admin)).redirects_to("/login");
+    // The admin's user is still in `users.json`, which the server never
+    // writes.
+    let (again, _) = self::admin(&blog);
+    assert_ne!(again, admin);
+}
+
+/// `--set-password` refuses, and writes nothing: with status 2, a name that
+/// is blank or has spaces around it, and an `--iterations` that is not a
+/// count; with status 1, a password shorter than eight characters, one that
+/// is blank, and one that is not UTF-8. It writes `users.json`, and no
+/// `blog.json`, which does not exist yet, and replaces the user of the same
+/// name when run again. A `users.json` that does not hold users is
+/// refused, with status 1, and left as it is, by `--set-password` and by
+/// the server, which reads it when it starts.
+#[test]
+fn set_password_writes_the_admin() {
+    let data = data_dir("set-password");
+    let (code, out, err) = run_server(&data, &["--set-password", "admin"], b"short\n");
+    assert_eq!(code, Some(1), "{out}");
+    assert_eq!(err, "blog: a password has at least eight characters\n");
+    for name in ["", " admin", "admin\t"] {
+        let (code, out, err) = run_server(&data, &["--set-password", name], b"a long password\n");
+        assert_eq!(code, Some(2), "{name:?}: {out}");
+        assert_eq!(
+            err, "blog: a user's name is not blank and has no spaces around it\n",
+            "{name:?}"
+        );
+    }
+    for (password, refused) in [
+        (&b"        \n"[..], "blog: a password is not blank\n"),
+        (&b"\t \t \t \t \r\n"[..], "blog: a password is not blank\n"),
+        (
+            &b"\xff\xfe a long password\n"[..],
+            "blog: the password is not UTF-8\n",
+        ),
+    ] {
+        let (code, out, err) = run_server(&data, &["--set-password", "admin"], password);
+        assert_eq!(code, Some(1), "{password:?}: {out}");
+        assert_eq!(err, refused, "{password:?}");
+    }
+    assert!(!data.join("users.json").exists());
+    assert!(!data.join("blog.json").exists());
+    let (code, _, err) = run_server(&data, &["--iterations", "0"], b"");
+    assert_eq!(code, Some(2));
+    assert!(
+        err.starts_with("blog: --iterations takes a number from 1 to 4294967295, not 0\n"),
+        "{err}"
+    );
+    set_password(&data);
+    let saved = std::fs::read_to_string(data.join("users.json")).expect("users.json");
+    assert!(saved.contains("\"iterations\":1000"), "{saved}");
+    assert!(!saved.contains(PASSWORD));
+    assert!(!saved.contains("Hello from a Nova server"), "{saved}");
+    assert!(!data.join("users.json.tmp").exists());
+    assert!(
+        !data.join("blog.json").exists(),
+        "--set-password wrote blog.json"
+    );
+    set_password(&data);
+    let again = std::fs::read_to_string(data.join("users.json")).expect("users.json");
+    assert_eq!(again.matches("\"name\":\"admin\"").count(), 1, "{again}");
+    assert!(!data.join("blog.json").exists());
+
+    std::fs::write(data.join("users.json"), "{").expect("break users.json");
+    let refused = format!("blog: {}/users.json: not JSON", data.display());
+    let (code, out, err) = run_server(
+        &data,
+        &["--set-password", "grace", "--iterations", "1000"],
+        b"a password for grace\n",
+    );
+    assert_eq!(code, Some(1), "{out}{err}");
+    assert!(err.starts_with(&refused), "{err}");
+    let (code, out, err) = run_server(&data, &["--address", "127.0.0.1:0"], b"");
+    assert_eq!(code, Some(1), "{out}{err}");
+    assert!(err.starts_with(&refused), "{err}");
+    assert_eq!(
+        std::fs::read_to_string(data.join("users.json")).expect("users.json"),
+        "{"
+    );
+}
+
+/// The server refuses to start on a key file that does not hold 32 bytes in
+/// hex, with status 1, and leaves the file as it is.
+#[test]
+fn a_key_file_without_32_bytes_in_hex_is_refused() {
+    let data = data_dir("bad-key");
+    let refused = format!(
+        "blog: {}/secret does not hold 32 bytes in hex\n",
+        data.display()
+    );
+    for key in [
+        String::new(),
+        "ab".repeat(31),
+        "ab".repeat(33),
+        format!("zz{}", "ab".repeat(31)),
+    ] {
+        let text = format!("{key}\n");
+        std::fs::write(data.join("secret"), &text).expect("write the key's file");
+        let (code, out, err) = run_server(&data, &["--address", "127.0.0.1:0"], b"");
+        assert_eq!(code, Some(1), "{key:?}: {out}{err}");
+        assert_eq!(out, "", "{key:?}");
+        assert_eq!(err, refused, "{key:?}");
+        assert_eq!(
+            std::fs::read_to_string(data.join("secret")).expect("the key's file"),
+            text
+        );
+    }
+}
+
+/// A running `--set-password`: a process that ends, killed when dropped,
+/// so that a failing test leaves none running.
+struct Setting(Child);
+
+impl Drop for Setting {
+    fn drop(&mut self) {
+        let _ = self.0.kill();
+        let _ = self.0.wait();
+    }
+}
+
+/// A password set while the server runs loses nothing, and nothing loses
+/// it: `--set-password` writes only `users.json`, and the server only
+/// `blog.json`. Comments are posted while `--set-password` hashes over
+/// 600,000 rounds, about two seconds on a debug runtime, each awaited
+/// until `blog.json` holds it, and some saved before it ends; once it has
+/// ended, `blog.json` holds every one, and `users.json`, after the server
+/// writes again, the new user and the admin. The running server checks
+/// logins against the users it read when it started, so the new user logs
+/// in once it starts again, and the comments are still on the post's page.
+#[test]
+fn a_password_set_while_the_server_runs_loses_nothing() {
+    const GRACE: &str = "a password for grace";
+    let data = data_dir("set-while-serving");
+    set_password(&data);
+    let blog = Blog::start(&data);
+    let (sid, csrf) = visitor(&blog, "/posts/1");
+    let mut setting = Setting(
+        Command::new(server_exe())
+            .args(["--set-password", "grace", "--data"])
+            .arg(&data)
+            .current_dir(project())
+            .stdin(Stdio::piped())
+            .stdout(Stdio::piped())
+            .stderr(Stdio::piped())
+            .spawn()
+            .expect("spawn --set-password"),
+    );
+    let out = drain(setting.0.stdout.take().expect("stdout is piped"));
+    let err = drain(setting.0.stderr.take().expect("stderr is piped"));
+    let mut input = setting.0.stdin.take().expect("stdin is piped");
+    input
+        .write_all(format!("{GRACE}\n").as_bytes())
+        .expect("write the password");
+    drop(input);
+    let deadline = Instant::now() + LIMIT;
+    let mut posted: Vec<String> = Vec::new();
+    let mut during = 0;
+    let status = loop {
+        if let Some(status) = setting.0.try_wait().expect("wait for --set-password") {
+            break status;
+        }
+        assert!(
+            Instant::now() < deadline,
+            "--set-password did not end within {LIMIT:?}"
+        );
+        let text = format!("Posted during the hash, number {}.", posted.len() + 1);
+        blog.post(
+            "/posts/1/comments",
+            Some(&sid),
+            &[("csrf", &csrf), ("name", "Ada"), ("text", &text)],
+        )
+        .redirects_to("/posts/1#comments");
+        if let Err(saved) = saved_within(&data, |saved| saved.contains(&text)) {
+            panic!("blog.json lost {text:?}, posted while --set-password ran: {saved}");
+        }
+        posted.push(text);
+        if setting
+            .0
+            .try_wait()
+            .expect("wait for --set-password")
+            .is_none()
+        {
+            during += 1;
+        }
+    };
+    let (out, err) = (
+        out.join().unwrap_or_default(),
+        err.join().unwrap_or_default(),
+    );
+    assert_eq!(status.code(), Some(0), "{out}{err}");
+    eprintln!(
+        "{during} of {} comments saved while --set-password ran",
+        posted.len()
+    );
+    assert!(
+        during > 0,
+        "--set-password ended before a comment was saved while it ran"
+    );
+    let saved = std::fs::read_to_string(data.join("blog.json")).expect("blog.json");
+    for text in &posted {
+        assert!(
+            saved.contains(text.as_str()),
+            "blog.json lost {text:?}, saved while --set-password ran ({during} of {} were): {saved}",
+            posted.len()
+        );
+    }
+    let (login, token) = visitor(&blog, "/login");
+    let early = blog.post(
+        "/login",
+        Some(&login),
+        &[("csrf", &token), ("name", "grace"), ("password", GRACE)],
+    );
+    assert_eq!(
+        early.status, 401,
+        "the running server knows grace: {}",
+        early.body
+    );
+    blog.post(
+        "/posts/1/comments",
+        Some(&sid),
+        &[
+            ("csrf", &csrf),
+            ("name", "Ada"),
+            ("text", "After the hash."),
+        ],
+    )
+    .redirects_to("/posts/1#comments");
+    until_saved(&data, |text| text.contains("After the hash."));
+    let users = std::fs::read_to_string(data.join("users.json")).expect("users.json");
+    assert!(users.contains("\"name\":\"grace\""), "{users}");
+    assert!(users.contains("\"name\":\"admin\""), "{users}");
+    blog.stop();
+
+    let blog = Blog::start(&data);
+    log_in(&blog, "grace", GRACE);
+    admin(&blog);
+    let page = blog.get("/posts/1", None);
+    for text in posted.iter().map(String::as_str).chain(["After the hash."]) {
+        assert!(page.body.contains(text), "{text:?} is not on the page");
+    }
+}
+
+/// Two `--set-password` runs that overlap both keep their user: each hashes
+/// first and reads `users.json` only then, so one that runs from start to
+/// end while another hashes is in the file the other reads. One run sets
+/// `ada` over 600,000 rounds, about two seconds on a debug runtime; half a
+/// second in, a second run sets `bea` over 1,000 rounds and ends. The test
+/// needs the first run still going when the second has ended, by a quarter
+/// of a second at least, and fails saying so when the runner is too slow
+/// for that, rather than judging the files. Then `users.json` holds both
+/// users, and both log in on a server started afterwards.
+#[test]
+fn overlapping_set_password_runs_keep_both_users() {
+    const ADA: &str = "a password for ada";
+    const BEA: &str = "a password for bea";
+    let data = data_dir("overlapping-set-password");
+    let started = Instant::now();
+    let mut slow = Setting(
+        Command::new(server_exe())
+            .args(["--set-password", "ada", "--iterations", "600000", "--data"])
+            .arg(&data)
+            .current_dir(project())
+            .stdin(Stdio::piped())
+            .stdout(Stdio::piped())
+            .stderr(Stdio::piped())
+            .spawn()
+            .expect("spawn the slow --set-password"),
+    );
+    let out = drain(slow.0.stdout.take().expect("stdout is piped"));
+    let err = drain(slow.0.stderr.take().expect("stderr is piped"));
+    let mut input = slow.0.stdin.take().expect("stdin is piped");
+    input
+        .write_all(format!("{ADA}\n").as_bytes())
+        .expect("write ada's password");
+    drop(input);
+    std::thread::sleep(Duration::from_millis(500));
+    let (code, fast_out, fast_err) = run_server(
+        &data,
+        &["--set-password", "bea", "--iterations", "1000"],
+        format!("{BEA}\n").as_bytes(),
+    );
+    let fast_done = started.elapsed();
+    assert_eq!(code, Some(0), "{fast_out}{fast_err}");
+    let still_hashing = slow
+        .0
+        .try_wait()
+        .expect("wait for the slow --set-password")
+        .is_none();
+    let status = loop {
+        if let Some(status) = slow.0.try_wait().expect("wait for the slow --set-password") {
+            break status;
+        }
+        assert!(
+            started.elapsed() < LIMIT,
+            "the slow --set-password did not end within {LIMIT:?}"
+        );
+        std::thread::sleep(Duration::from_millis(10));
+    };
+    let slow_done = started.elapsed();
+    let (out, err) = (
+        out.join().unwrap_or_default(),
+        err.join().unwrap_or_default(),
+    );
+    assert_eq!(status.code(), Some(0), "{out}{err}");
+    eprintln!("bea's run ended at {fast_done:?}, ada's at {slow_done:?}");
+    assert!(
+        still_hashing && slow_done >= fast_done + Duration::from_millis(250),
+        "this runner is too slow for the test: bea's run, at 1000 rounds, ended \
+         {fast_done:?} after ada's started, and ada's, at 600000, ended at \
+         {slow_done:?}; the test needs ada's to end at least 250ms after bea's"
+    );
+    let users = std::fs::read_to_string(data.join("users.json")).expect("users.json");
+    assert!(
+        users.contains("\"name\":\"bea\""),
+        "ada's run wrote over bea, who was set while it hashed: {users}"
+    );
+    assert!(users.contains("\"name\":\"ada\""), "{users}");
+    let blog = Blog::start(&data);
+    log_in(&blog, "bea", BEA);
+    log_in(&blog, "ada", ADA);
+}
````

**Patch `t6-code`:**

<!-- patch t6-code -->

````diff
diff --git a/.gitignore b/.gitignore
index b6fc45fc302bab7c94a01ab5291b0523be7f070c..4a7a5870952cc91b46903a7b6b7c424559fce292 100644
--- a/.gitignore
+++ b/.gitignore
@@ -6,6 +6,13 @@
 .env.local
 /examples/04-todo-cli/todos.json
 /examples/06-counter-spa/dist
+/examples/07-fullstack-blog/dist
+/examples/07-fullstack-blog/target
+/examples/07-fullstack-blog/data/blog.json
+/examples/07-fullstack-blog/data/blog.json.tmp
+/examples/07-fullstack-blog/data/users.json
+/examples/07-fullstack-blog/data/users.json.tmp
+/examples/07-fullstack-blog/data/secret
 /tools/vscode-nova/node_modules
 /tools/vscode-nova/out
 /tools/vscode-nova/.vscode-test
diff --git a/crates/nova-driver/tests/index_complete.rs b/crates/nova-driver/tests/index_complete.rs
index 72346ae966e0e909d2bb9e6202395000d1b29416..fd2ba68a56fc63b66ce3bd60e5deeef289a2a9b6 100644
--- a/crates/nova-driver/tests/index_complete.rs
+++ b/crates/nova-driver/tests/index_complete.rs
@@ -241,11 +241,11 @@ fn every_name_is_covered(a: &Analysis, file: FileId, failures: &mut Vec<String>)
 #[test]
 fn every_name_in_the_corpora_is_indexed() {
     let programs = programs();
-    // 8 examples, 155 top-level runtime programs and the modules program,
-    // on 2026-10-21, after 4.7 added `fs_rename`, `crypto_pbkdf2`,
-    // `crypto_pbkdf2_refused` and `bytes_hex`, then `http_routes`,
-    // `http_request_readers`, `http_responses` and `http_server_wire`.
-    assert_eq!(programs.len(), 164, "{programs:#?}");
+    // 9 examples, 155 top-level runtime programs and the modules program,
+    // on 2026-10-21, after 4.7 added `http_routes`, `http_request_readers`,
+    // `http_responses`, `http_server_wire`, `fs_rename`, `crypto_pbkdf2`,
+    // `crypto_pbkdf2_refused` and `bytes_hex`, and `07-fullstack-blog`.
+    assert_eq!(programs.len(), 165, "{programs:#?}");
     let mut files = 0;
     let mut failures: Vec<String> = Vec::new();
     for (k, path) in programs.iter().enumerate() {
@@ -272,9 +272,11 @@ fn every_name_in_the_corpora_is_indexed() {
             every_name_is_covered(&a, file, &mut failures);
         }
     }
-    // 8 + 155 + 3 files of user code, 05-geometry's library's 2, and
+    // 9 + 155 + 3 files of user code, 05-geometry's library's 2, the 8
+    // modules 07-fullstack-blog's client reaches from its `main.nova` (not
+    // the server's 4, which only `server.nova` and `lib.nova` reach), and
     // native std's 22, `std/ui/ssr` among them.
-    assert_eq!(files, 190);
+    assert_eq!(files, 199);
     failures.sort();
     failures.dedup();
     assert!(
diff --git a/examples/07-fullstack-blog/README.md b/examples/07-fullstack-blog/README.md
new file mode 100644
index 0000000000000000000000000000000000000000..e5651d12cfadc378434f79c8c05196feec372fe2
--- /dev/null
+++ b/examples/07-fullstack-blog/README.md
@@ -0,0 +1,138 @@
+# 07-fullstack-blog
+
+A blog that a visitor reads and comments on, and that its admin writes, from
+a browser: a server on `std/http` that renders every page, and a client that
+hydrates it.
+
+## What this demonstrates
+
+- **A hand-written server beside the built site.** `src/server.nova` is
+  `std/http`'s `Server` with the blog's routes, by method and with `:id`
+  parameters (`req.params`), and a fallback that asks `ui::ssr::Dist` for
+  the built site's files. Each page is rendered with `page_with`, its data
+  after it, and answered with the status and cookie it needs.
+- **Hydration.** `src/main.nova`, the client, built for WASM, adopts the
+  server's HTML with `hydrate_with`: one `Data` record, its JSON written by
+  hand, carries what any page needs. The same `routes()` in `src/app.nova`
+  give each path its page on both sides. Links are plain anchors, so each
+  page arrives from the server with its own data.
+- **Forms that work without JavaScript.** Every form is a plain HTML form
+  whose `POST` is answered with `303` when it succeeds (post, redirect,
+  get), and with the form again, its values and messages, and `400` when a
+  field fails a check. Once the page has hydrated, the client runs the same
+  checks (`src/forms.nova`) as the fields change: each field's message
+  shows under it, and the submit button is disabled while the form is not
+  valid.
+- **Sessions and CSRF tokens.** Every visitor gets a session: a random id in
+  a table in memory, carried by the cookie `sid` as `<id>.<tag>`, the tag
+  `hmac_sha256` of the id, `HttpOnly` and `SameSite=Lax`. Every form carries
+  a token derived from the session, and a `POST` without the right one is
+  refused with `403`. Logging in starts a new session, so an id set before
+  it is worth nothing after; logging out ends it (`src/auth.nova`).
+- **Passwords stored only as hashes,** PBKDF2-HMAC-SHA256 with a random
+  salt and OWASP's 600,000 rounds, the count kept with each hash. No
+  password is in the repository: the server sets the admin's from standard
+  input.
+- **Escaping by construction.** Every title, body, name and comment reaches
+  the page as a text node or an attribute's value, which `std/ui` escapes,
+  and no `href` holds a visitor's text. A post's body is plain text; a blank
+  line starts a new paragraph.
+- **A store in JSON files.** The posts and comments live in memory; each
+  change is written whole to `data/blog.json.tmp`, then renamed over
+  `data/blog.json`, so a crash never leaves half a file
+  (`src/store.nova`). `data/seed.json`, in the repository, is where the
+  first start reads from. The users are in `data/users.json`, which only
+  `--set-password` writes, the same way, and the server reads when it
+  starts.
+
+## Run it
+
+```bash
+cd examples/07-fullstack-blog
+nova build --ssr
+echo 'a long password' | target/debug/fullstack-blog-server --set-password admin
+target/debug/fullstack-blog-server
+```
+
+`nova build --ssr` builds the client into `dist/` and the server into
+`target/debug/fullstack-blog-server` (`.exe` on Windows). `--set-password`
+reads one line, of at least eight characters and not blank, writes the user
+`admin` and its hash into `data/users.json`, and ends.
+Then the server serves the blog on `127.0.0.1:3000`; open it in a browser,
+and log in at `/login`. Ctrl-C stops it, once the writes it has started are
+done.
+
+To set a password, stop the server, run `--set-password`, then start the
+server. The server reads `data/users.json` only when it starts, so a
+password set while it runs takes effect at its next start. Nothing is lost
+meanwhile: only `--set-password` writes `data/users.json`, and only the
+server writes `data/blog.json`. Run one `--set-password` at a time: each
+hashes first and reads `data/users.json` only then, so one that runs while
+another hashes is kept, but two that read and write the file in the same
+few milliseconds can lose one's user.
+
+The server's arguments: `--address <host:port>`, port 0 for one the system
+chooses; `--data <dir>` and `--dist <dir>`, `data` and `dist` unless given;
+and `--iterations <n>`, PBKDF2's rounds for `--set-password`, which the
+tests lower to 1,000.
+
+`nova test` runs `tests/*.nova`, which reach the server's modules through
+`src/lib.nova`.
+
+## Expected output
+
+```
+built dist
+built target/debug/fullstack-blog-server
+blog: set the password of admin
+blog: serving http://127.0.0.1:3000/
+```
+
+`/` lists three posts, newest first, each with its date and first
+paragraph. A post's page shows its paragraphs, its comments and a form to
+add one, whose button stays disabled until both fields are filled. After
+logging in, `/admin` lists the posts with a link to edit each and a button
+to delete it, and `Write a new post` opens the editor.
+
+## Notes
+
+- **What a public deployment adds:** TLS, with the session cookie made
+  `Secure` (`Cookie`'s `secure`, which `std/http` leaves off so the blog
+  works over plain HTTP on `127.0.0.1`); a limit on login attempts; and
+  keeping `data/` private: the key in `data/secret` and the password hashes
+  in `data/users.json` are written with the system's default permissions,
+  since `std/fs` sets no mode. PBKDF2's 600,000 rounds hold the
+  single-threaded server for each login, about 2.6 seconds on a debug
+  runtime.
+- **The session table is in memory,** so a restart logs everyone out. The
+  signing key is kept in `data/secret`, made at random on the first start.
+  Every page asked for without a cookie starts a session. The ended ones
+  are dropped once the table has grown to twice the size it had after they
+  were last dropped, and to at least 64, so it holds at most twice the
+  sessions live at the last scan, or 64 if that is more, and sessions that
+  end after a burst stay until it doubles again. The scans cost the same
+  per session on average: on a debug runtime, a page asked for without a
+  cookie took 1.5 to 1.9 ms with 200 to 8,200 sessions in the table, where
+  dropping them at every start took 1.6 ms at 200 and 12.8 ms at 8,200.
+  The request that finds the table doubled scans all of it, in time linear
+  in its size: the one that started the 16,385th session took 18.4 ms, and
+  the 32,769th's 35.7 ms. A session takes about 658 bytes, so a client
+  that keeps no cookie and asks for pages as fast as the server answers,
+  391 a second in a probe, would grow the server's memory by about 22 GB
+  in a day.
+- **`std/ui` does not refuse a `javascript:` URL** in an `href`. The blog
+  never puts a visitor's text in one: a post's link is built from its id.
+- **`src/` is flat,** unlike `nova-spec/60-EXAMPLES.md` §7's tree: a nested
+  module path is not supported yet. The store is a JSON file, not SQLite
+  (ADR 0038).
+- **The submit button** is two buttons, each under a `show`: `std/ui` never
+  removes an attribute, and `disabled` disables a button by being there at
+  all, so the server's HTML holds the enabled one, which a page without
+  JavaScript keeps, and the client swaps them as the form's checks change.
+- `nova dev` does not serve a server of its own yet: build with
+  `nova build --ssr` and start the server.
+- `cargo test` builds it and drives its server over HTTP
+  (`crates/nova-cli/tests/blog_tests.rs`), and runs its `nova test`
+  (`run_tests.rs`); CI's `browser` job drives it in Chromium
+  (`tests/browser/blog.mjs`). The 4.7 spec is
+  `docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md`.
diff --git a/examples/07-fullstack-blog/data/seed.json b/examples/07-fullstack-blog/data/seed.json
new file mode 100644
index 0000000000000000000000000000000000000000..5e0fd92acd5c95189e77588b9e20f3c76acced1f
--- /dev/null
+++ b/examples/07-fullstack-blog/data/seed.json
@@ -0,0 +1,43 @@
+{
+  "posts": [
+    {
+      "id": 1,
+      "title": "Hello from a Nova server",
+      "body": "This blog is a Nova program. Its server is built on std/http, and it renders every page you read before it sends it, so the page is whole even before any script runs.\n\nThe client is the same program's components, built for WebAssembly. When it loads, it hydrates the page: it adopts the elements the server wrote instead of building them again, and the forms come alive.\n\nNothing here needs a database. The posts and the comments live in memory, and each change is written to a JSON file.",
+      "created": "2026-10-19T08:30:00.000Z",
+      "updated": "2026-10-19T08:30:00.000Z"
+    },
+    {
+      "id": 2,
+      "title": "How the forms work",
+      "body": "Every form on this blog is a plain HTML form. It posts to the server, which checks it and answers with a redirect, so a reload never sends it twice.\n\nOnce the page has hydrated, the client checks the fields as you type. A blank or too long field is reported under it, and the button stays disabled until the form is valid.\n\nWithout JavaScript the forms still work: the server runs the same checks, and sends the form back with its messages when one fails.",
+      "created": "2026-10-20T09:15:00.000Z",
+      "updated": "2026-10-20T09:15:00.000Z"
+    },
+    {
+      "id": 3,
+      "title": "Sessions, tokens and passwords",
+      "body": "Every visitor gets a session: a random id in a cookie, signed with HMAC-SHA256, that scripts cannot read. Every form carries a token derived from that session, and a form without the right one is refused.\n\nThe admin's password is stored only as a PBKDF2 hash, with its salt and its iteration count. No password is in the repository: the server sets it from standard input.\n\nText you write here reaches the page only as text. A title or a comment that holds a script shows the script's text, and runs nothing.",
+      "created": "2026-10-21T07:45:00.000Z",
+      "updated": "2026-10-21T07:45:00.000Z"
+    }
+  ],
+  "comments": [
+    {
+      "id": 1,
+      "post": 1,
+      "name": "Ada",
+      "text": "The page was there before the script loaded. Nice.",
+      "created": "2026-10-19T10:02:00.000Z"
+    },
+    {
+      "id": 2,
+      "post": 3,
+      "name": "Grace",
+      "text": "So a reload after a comment does not post it again?\n\nGood.",
+      "created": "2026-10-21T08:10:00.000Z"
+    }
+  ],
+  "next_post": 4,
+  "next_comment": 3
+}
diff --git a/examples/07-fullstack-blog/index.html b/examples/07-fullstack-blog/index.html
new file mode 100644
index 0000000000000000000000000000000000000000..0fc16105aa46a3c894d10da1d65b9c2ea0b49f74
--- /dev/null
+++ b/examples/07-fullstack-blog/index.html
@@ -0,0 +1,16 @@
+<!doctype html>
+<html lang="en">
+<head>
+<meta charset="utf-8">
+<meta name="viewport" content="width=device-width, initial-scale=1">
+<title>A Nova blog</title>
+<link rel="stylesheet" href="/assets/styles.css">
+</head>
+<body>
+<div id="app"></div>
+<script type="module">
+import init from "/app.js";
+init();
+</script>
+</body>
+</html>
diff --git a/examples/07-fullstack-blog/public/styles.css b/examples/07-fullstack-blog/public/styles.css
new file mode 100644
index 0000000000000000000000000000000000000000..7de5b309089e4b8016e425ebbcf84b403afdc69e
--- /dev/null
+++ b/examples/07-fullstack-blog/public/styles.css
@@ -0,0 +1,191 @@
+/* The blog's one stylesheet, served as /assets/styles.css. System fonts,
+   no images. */
+
+:root {
+  --ink: #1f2328;
+  --muted: #59636e;
+  --line: #d1d9e0;
+  --accent: #0b5cad;
+  --error: #b42318;
+  --paper: #ffffff;
+  --wash: #f6f8fa;
+}
+
+* {
+  box-sizing: border-box;
+}
+
+body {
+  margin: 0;
+  background: var(--paper);
+  color: var(--ink);
+  font: 17px/1.6 system-ui, -apple-system, "Segoe UI", Roboto, sans-serif;
+}
+
+.page {
+  max-width: 42rem;
+  margin: 0 auto;
+  padding: 0 1rem 4rem;
+}
+
+a {
+  color: var(--accent);
+}
+
+header.site {
+  display: flex;
+  flex-wrap: wrap;
+  align-items: center;
+  justify-content: space-between;
+  gap: 0.5rem;
+  padding: 1rem 0;
+  margin-bottom: 1.5rem;
+  border-bottom: 1px solid var(--line);
+}
+
+header.site nav {
+  display: flex;
+  align-items: center;
+  gap: 1rem;
+}
+
+.brand {
+  font-weight: 700;
+  color: var(--ink);
+  text-decoration: none;
+}
+
+h1 {
+  font-size: 1.8rem;
+  line-height: 1.25;
+}
+
+h2 {
+  font-size: 1.3rem;
+  line-height: 1.3;
+}
+
+.card {
+  padding: 1rem 0;
+  border-bottom: 1px solid var(--line);
+}
+
+.card h2 {
+  margin: 0;
+}
+
+.date,
+.who,
+.hint {
+  color: var(--muted);
+  font-size: 0.9rem;
+}
+
+.card .date {
+  margin: 0.25rem 0;
+}
+
+.body p {
+  white-space: pre-line;
+}
+
+#comments {
+  margin-top: 2.5rem;
+  border-top: 1px solid var(--line);
+}
+
+.comment {
+  padding: 0.5rem 0.75rem;
+  margin: 0.75rem 0;
+  background: var(--wash);
+  border-radius: 6px;
+}
+
+.comment .who {
+  margin: 0;
+}
+
+.stacked {
+  display: grid;
+  gap: 0.25rem;
+  margin-top: 1.5rem;
+}
+
+.stacked label {
+  display: grid;
+  gap: 0.25rem;
+  font-weight: 600;
+}
+
+input,
+textarea {
+  width: 100%;
+  padding: 0.5rem;
+  border: 1px solid var(--line);
+  border-radius: 6px;
+  font: inherit;
+}
+
+textarea {
+  resize: vertical;
+}
+
+button {
+  padding: 0.4rem 1rem;
+  border: 1px solid var(--accent);
+  border-radius: 6px;
+  background: var(--accent);
+  color: var(--paper);
+  font: inherit;
+  cursor: pointer;
+}
+
+button:disabled {
+  opacity: 0.45;
+  cursor: not-allowed;
+}
+
+form.inline {
+  display: inline;
+}
+
+form.inline button {
+  padding: 0.1rem 0.6rem;
+  background: var(--paper);
+  color: var(--accent);
+  font-size: 0.9rem;
+}
+
+.message,
+.error {
+  min-height: 1.2rem;
+  margin: 0 0 0.5rem;
+  color: var(--error);
+  font-size: 0.9rem;
+}
+
+.actions {
+  margin-top: 0.5rem;
+}
+
+.rows {
+  padding: 0;
+  list-style: none;
+}
+
+.row {
+  display: flex;
+  flex-wrap: wrap;
+  align-items: baseline;
+  gap: 0.75rem;
+  padding: 0.5rem 0;
+  border-bottom: 1px solid var(--line);
+}
+
+.row > a:first-child {
+  flex: 1;
+}
+
+.notice {
+  padding: 2rem 0;
+}
diff --git a/examples/07-fullstack-blog/src/admin.nova b/examples/07-fullstack-blog/src/admin.nova
new file mode 100644
index 0000000000000000000000000000000000000000..7981a85b03a8d530e2b7fcef38ec7996eed1dfd8
--- /dev/null
+++ b/examples/07-fullstack-blog/src/admin.nova
@@ -0,0 +1,116 @@
+//! The admin's pages: the posts, each with a link to edit it and a button
+//! that deletes it, and the editor, empty for a new post and filled for one
+//! to edit. The server shows them only to the admin; anyone else is sent to
+//! the login.
+
+import app::{Data, Post}
+import forms::{body_message, csrf_field, message, post_ok, submit, title_message}
+import post_card::{day}
+import ui
+import ui::html::{
+    a,
+    attr,
+    button,
+    class,
+    div,
+    form,
+    h1,
+    href,
+    input,
+    label,
+    li,
+    p,
+    span,
+    text,
+    textarea,
+    ul,
+}
+
+// A post in the admin's list: its title, linked to its page, its date, a
+// link to edit it and a button that deletes it.
+fn row(data: Data, post: Post) -> View {
+    li(
+        [class("row")],
+        [
+            a([href("/posts/${post.id}")], [text(post.title)]),
+            span([class("date")], [text(day(post.created))]),
+            a([class("edit"), href("/admin/posts/${post.id}/edit")], [text("Edit")]),
+            form(
+                [
+                    class("inline"),
+                    attr("method", "post"),
+                    attr("action", "/admin/posts/${post.id}/delete"),
+                ],
+                [csrf_field(data.csrf), button([attr("type", "submit")], [text("Delete")])],
+            ),
+        ],
+    )
+}
+
+// The list, newest first, or a line saying there is nothing in it.
+fn rows(data: Data) -> View {
+    if data.posts.len() == 0 { return p([], [text("No posts yet.")]) }
+    let mut views = [text(""); data.posts.len()]
+    let mut i = 0
+    while i < data.posts.len() {
+        views[i] = row(data, data.posts[i])
+        i = i + 1
+    }
+    ul([class("rows")], views)
+}
+
+/// The admin's page.
+pub component AdminPage(data: Data) {
+    view {
+        div {
+            h1 { text("Admin") }
+            p { a(id: "new-post", href: "/admin/new") { text("Write a new post") } }
+            rows(data)
+        }
+    }
+}
+
+/// The editor: a new post's when `data` holds no post, and otherwise the
+/// post's, filled from `data.draft`.
+pub component Editor(data: Data) {
+    let action = match data.post {
+        Some(post) => "/admin/posts/${post.id}"
+        None => "/admin/posts"
+    }
+    let heading = match data.post {
+        Some(post) => "Edit a post"
+        None => "A new post"
+    }
+    let title = signal(data.draft.title)
+    let body = signal(data.draft.body)
+    let live = signal(false)
+    let shown = memo(|| live.value() || data.draft.failed)
+    let ok = memo(|| !live.value() || post_ok(title.value(), body.value()))
+    on_mount(|| live.set(true))
+    view {
+        div {
+            h1 { text(heading) }
+            form(id: "editor", class: "stacked", method: "post", action: action) {
+                csrf_field(data.csrf)
+                label {
+                    text("Title")
+                    input(
+                        name: "title",
+                        value: data.draft.title,
+                        on_input: |v: String| title.set(v),
+                    )
+                }
+                message(shown, || title_message(title.value()))
+                label {
+                    text("Text")
+                    textarea(name: "body", rows: "16", on_input: |v: String| body.set(v)) {
+                        text(data.draft.body)
+                    }
+                }
+                message(shown, || body_message(body.value()))
+                p(class: "hint") { text("A blank line starts a new paragraph.") }
+                submit(ok, "Save")
+            }
+        }
+    }
+}
diff --git a/examples/07-fullstack-blog/src/app.nova b/examples/07-fullstack-blog/src/app.nova
index 4d885c85fc19514682079d0fb18fc1983f835fae..8058bff33779e038431d507adeab18a5a0da657f 100644
--- a/examples/07-fullstack-blog/src/app.nova
+++ b/examples/07-fullstack-blog/src/app.nova
@@ -8,6 +8,15 @@
 //! reads the same `Data` back and hydrates the same view. Links are plain
 //! anchors, so each page arrives from the server with its own data.
 
+import admin::{AdminPage, Editor}
+import header::{Header}
+import home::{Home}
+import login::{LoginPage}
+import post::{PostPage}
+import ui
+import ui::html::{a, div, h1, p}
+import ui::router::{HistoryRouter, Route}
+
 /// A post: written as plain text, a blank line between paragraphs, and when
 /// it was written and last changed, as `SystemTime::to_iso8601` writes them.
 pub record Post {
@@ -385,3 +394,68 @@ impl FromJson for Data {
         )
     }
 }
+
+// --- The pages ---
+
+/// The page for a path that has none, and for a post that does not exist.
+component NotFound() {
+    view {
+        div(class: "notice") {
+            h1 { text("Not found") }
+            p { text("There is no page here.") }
+            p { a(href: "/") { text("Back to the posts") } }
+        }
+    }
+}
+
+/// The page for a form the server refused: `why` says what was wrong.
+component Refused(why: String) {
+    view {
+        div(class: "notice") {
+            h1 { text("Not sent") }
+            p(id: "refused") { text(why) }
+            p { a(href: "/") { text("Back to the posts") } }
+        }
+    }
+}
+
+/// Each path's page, the same on both sides. A form the server sends back
+/// with its messages is answered at the path it was posted to, so those
+/// paths have their pages too: a comment at `/posts/:id/comments`, a new
+/// post at `/admin/posts`, an edited one at `/admin/posts/:id`.
+pub fn routes(data: Data) -> [Route] {
+    [
+        Route { path: "/", component: || Home(data) },
+        Route { path: "/posts/:id", component: || PostPage(data) },
+        Route { path: "/posts/:id/comments", component: || PostPage(data) },
+        Route { path: "/login", component: || LoginPage(data) },
+        Route { path: "/admin", component: || AdminPage(data) },
+        Route { path: "/admin/new", component: || Editor(data) },
+        Route { path: "/admin/posts", component: || Editor(data) },
+        Route { path: "/admin/posts/:id/edit", component: || Editor(data) },
+        Route { path: "/admin/posts/:id", component: || Editor(data) },
+        Route { path: "*", component: NotFound },
+    ]
+}
+
+// The page's content: what refused the form, or that there is nothing here,
+// or the route's page.
+fn content(data: Data) -> View {
+    if data.refused != "" { return Refused(data.refused) }
+    if data.missing { return NotFound() }
+    HistoryRouter(routes(data))
+}
+
+/// The app: the header, then the page. The root's `data-state` reads
+/// `server` in the server's HTML and `hydrated` once the client has adopted
+/// the page, which a test can wait for.
+pub component App(data: Data) {
+    let state = signal("server")
+    on_mount(|| state.set("hydrated"))
+    view {
+        div(id: "root", class: "page", data_state: state.value()) {
+            Header(data)
+            content(data)
+        }
+    }
+}
diff --git a/examples/07-fullstack-blog/src/header.nova b/examples/07-fullstack-blog/src/header.nova
new file mode 100644
index 0000000000000000000000000000000000000000..99e4f32d597b42f8cbf5518216b624ab086c1ece
--- /dev/null
+++ b/examples/07-fullstack-blog/src/header.nova
@@ -0,0 +1,36 @@
+//! The header every page has: the blog's name, linked to the list of
+//! posts, and, for the admin, a link to the admin's page and a form that
+//! logs out, or, for anyone else, a link to the login.
+
+import app::{Data}
+import forms::{csrf_field}
+import ui
+import ui::html::{a, attr, button, class, form, header, href, nav, text}
+
+// The links for who is logged in, or for a visitor.
+fn links(data: Data) -> View {
+    if data.user == "" { return nav([], [a([href("/login")], [text("Log in")])]) }
+    nav(
+        [],
+        [
+            a([href("/admin")], [text("Admin")]),
+            form(
+                [class("inline"), attr("method", "post"), attr("action", "/logout")],
+                [
+                    csrf_field(data.csrf),
+                    button([attr("type", "submit")], [text("Log out ${data.user}")]),
+                ],
+            ),
+        ],
+    )
+}
+
+/// The header.
+pub component Header(data: Data) {
+    view {
+        header(class: "site") {
+            a(class: "brand", href: "/") { text("A Nova blog") }
+            links(data)
+        }
+    }
+}
diff --git a/examples/07-fullstack-blog/src/home.nova b/examples/07-fullstack-blog/src/home.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1c45ec7d36dff55e6f167e266c7aa1de38290b23
--- /dev/null
+++ b/examples/07-fullstack-blog/src/home.nova
@@ -0,0 +1,29 @@
+//! The home page: the posts, newest first, as the server sends them, each
+//! with its title, its date and its first paragraph.
+
+import app::{Data}
+import post_card::{PostCard}
+import ui
+import ui::html::{div, h1, p, section, text}
+
+// A card for each post, or a line saying there are none.
+fn cards(data: Data) -> View {
+    if data.posts.len() == 0 { return p([], [text("Nothing has been written yet.")]) }
+    let mut views = [text(""); data.posts.len()]
+    let mut i = 0
+    while i < data.posts.len() {
+        views[i] = PostCard(data.posts[i])
+        i = i + 1
+    }
+    div([], views)
+}
+
+/// The home page.
+pub component Home(data: Data) {
+    view {
+        section(id: "posts") {
+            h1 { text("Posts") }
+            cards(data)
+        }
+    }
+}
diff --git a/examples/07-fullstack-blog/src/login.nova b/examples/07-fullstack-blog/src/login.nova
new file mode 100644
index 0000000000000000000000000000000000000000..b941a07eb44ee3a4f9c2ba7943f32a4486460183
--- /dev/null
+++ b/examples/07-fullstack-blog/src/login.nova
@@ -0,0 +1,48 @@
+//! The login: the admin's name and password, checked as they are typed once
+//! the page has hydrated, and on the server, which answers a wrong name or
+//! password with the form again and `401`.
+
+import app::{Data}
+import forms::{csrf_field, login_name_message, login_ok, message, password_message, submit}
+import ui
+import ui::html::{div, form, h1, input, label, p, text}
+
+/// The login page.
+pub component LoginPage(data: Data) {
+    let name = signal(data.draft.name)
+    let password = signal("")
+    let live = signal(false)
+    let shown = memo(|| live.value() || data.draft.failed)
+    let ok = memo(|| !live.value() || login_ok(name.value(), password.value()))
+    on_mount(|| live.set(true))
+    view {
+        div {
+            h1 { text("Log in") }
+            p(id: "login-error", class: "error") { text(data.draft.error) }
+            form(id: "login-form", class: "stacked", method: "post", action: "/login") {
+                csrf_field(data.csrf)
+                label {
+                    text("Name")
+                    input(
+                        name: "name",
+                        autocomplete: "username",
+                        value: data.draft.name,
+                        on_input: |v: String| name.set(v),
+                    )
+                }
+                message(shown, || login_name_message(name.value()))
+                label {
+                    text("Password")
+                    input(
+                        type: "password",
+                        name: "password",
+                        autocomplete: "current-password",
+                        on_input: |v: String| password.set(v),
+                    )
+                }
+                message(shown, || password_message(password.value()))
+                submit(ok, "Log in")
+            }
+        }
+    }
+}
diff --git a/examples/07-fullstack-blog/src/main.nova b/examples/07-fullstack-blog/src/main.nova
new file mode 100644
index 0000000000000000000000000000000000000000..1475d135150acd1c4e88dc0f7648d333a7d6ad9f
--- /dev/null
+++ b/examples/07-fullstack-blog/src/main.nova
@@ -0,0 +1,9 @@
+//! The client: it hydrates the page the server rendered, with the data the
+//! server put after the page's root, so every node the server wrote is kept
+//! and the forms' checks come alive. On a page with no data it shows the
+//! empty data's page, with no posts.
+
+import app::{App, data_for}
+import ui
+
+fn main() { hydrate_with("#app", data_for("", ""), App) }
diff --git a/examples/07-fullstack-blog/src/post.nova b/examples/07-fullstack-blog/src/post.nova
new file mode 100644
index 0000000000000000000000000000000000000000..08e74c21c66cb00e1507fe060c9dd5fa15b1face
--- /dev/null
+++ b/examples/07-fullstack-blog/src/post.nova
@@ -0,0 +1,127 @@
+//! A post's page: the post, its comments, and the form that adds one; for
+//! the admin, a button under each comment that deletes it.
+//!
+//! The comment form's fields are signals that `on_input` sets. Once the page
+//! has hydrated, `on_mount` starts the checks: each field's message shows
+//! under it as it is typed, and the submit button is disabled while the
+//! form is not valid. Until then the page is what the server sent, which is
+//! all a browser without JavaScript gets, and still posts the form.
+
+import app::{Comment, Data, Post}
+import forms::{comment_ok, csrf_field, message, name_message, submit, text_message}
+import post_card::{body_view, day}
+import ui
+import ui::html::{
+    article,
+    attr,
+    button,
+    class,
+    div,
+    form,
+    h1,
+    h2,
+    id,
+    input,
+    label,
+    p,
+    section,
+    strong,
+    text,
+    textarea,
+}
+
+// The button that deletes comment `c`, for the admin, or nothing.
+fn delete_comment(data: Data, c: Comment) -> View {
+    if data.user == "" { return text("") }
+    form(
+        [class("inline"), attr("method", "post"), attr("action", "/admin/comments/${c.id}/delete")],
+        [csrf_field(data.csrf), button([attr("type", "submit")], [text("Delete")])],
+    )
+}
+
+// The comments, in the order they were written, or a line saying there are
+// none.
+fn comment_list(data: Data) -> View {
+    if data.comments.len() == 0 { return p([], [text("No comments yet.")]) }
+    let mut views = [text(""); data.comments.len()]
+    let mut i = 0
+    while i < data.comments.len() {
+        let c = data.comments[i]
+        views[i] = article(
+            [class("comment")],
+            [
+                p([class("who")], [strong([], [text(c.name)]), text(" on ${day(c.created)}")]),
+                body_view(c.text),
+                delete_comment(data, c),
+            ],
+        )
+        i = i + 1
+    }
+    div([], views)
+}
+
+/// The form that adds a comment to `post`.
+component CommentForm(data: Data, post: Post) {
+    let name = signal(data.draft.name)
+    let said = signal(data.draft.text)
+    let live = signal(false)
+    let shown = memo(|| live.value() || data.draft.failed)
+    let ok = memo(|| !live.value() || comment_ok(name.value(), said.value()))
+    on_mount(|| live.set(true))
+    view {
+        form(
+            id: "comment-form",
+            class: "stacked",
+            method: "post",
+            action: "/posts/${post.id}/comments",
+        ) {
+            h2 { text("Add a comment") }
+            csrf_field(data.csrf)
+            label {
+                text("Name")
+                input(name: "name", value: data.draft.name, on_input: |v: String| name.set(v))
+            }
+            message(shown, || name_message(name.value()))
+            label {
+                text("Comment")
+                textarea(name: "text", rows: "5", on_input: |v: String| said.set(v)) {
+                    text(data.draft.text)
+                }
+            }
+            message(shown, || text_message(said.value()))
+            submit(ok, "Comment")
+        }
+    }
+}
+
+// The post, its comments and the form.
+fn shown_post(data: Data, post: Post) -> View {
+    div(
+        [],
+        [
+            article(
+                [id("post")],
+                [
+                    h1([], [text(post.title)]),
+                    p([class("date")], [text(day(post.created))]),
+                    body_view(post.body),
+                ],
+            ),
+            section(
+                [id("comments")],
+                [h2([], [text("Comments")]), comment_list(data), CommentForm(data, post)],
+            ),
+        ],
+    )
+}
+
+/// The page of the post `data` holds.
+pub component PostPage(data: Data) {
+    let page = match data.post {
+        Some(post) => shown_post(data, post)
+        None => p([], [text("There is no such post.")])
+    }
+    view {
+        div { page }
+    }
+}
diff --git a/examples/07-fullstack-blog/src/server.nova b/examples/07-fullstack-blog/src/server.nova
new file mode 100644
index 0000000000000000000000000000000000000000..6258892ed030feb2a8c54407132bf1bfc09c9f33
--- /dev/null
+++ b/examples/07-fullstack-blog/src/server.nova
@@ -0,0 +1,728 @@
+//! The server: `std/http`'s `Server` with the blog's routes, beside the
+//! built site, `ui::ssr::Dist`, which it serves from its fallback.
+//!
+//! Every page is rendered here with `page_with`, its data after it, and the
+//! client hydrates it. Every page `GET` without a session starts one, a
+//! visitor's, and sets its cookie, so every form can carry a CSRF token.
+//! Every `POST` checks that token against its session before anything else,
+//! and answers `403` when it is missing or wrong; one that succeeds is
+//! answered with `303` (post, redirect, get), and one whose fields fail a
+//! check gets the form again with its values and messages, and `400`.
+//!
+//! The store lives in memory. A handler changes it there, then spawns the
+//! write of the whole store to `<data>/blog.json`, which runs after the
+//! response. The users are read from `<data>/users.json` once, at start;
+//! the server never writes them.
+//!
+//! Arguments:
+//!
+//! - `--address <host:port>`, `127.0.0.1:3000` unless given; port 0 lets
+//!   the system choose. Once bound, it prints
+//!   `blog: serving http://<host>:<port>/`.
+//! - `--data <dir>`, `data`, and `--dist <dir>`, `dist`, unless given.
+//! - `--set-password <name>` reads a password, one line of at least eight
+//!   characters, not blank, from standard input, and hashes it; then it
+//!   reads `<data>/users.json`, no users when it does not exist yet, sets
+//!   the user `<name>` with its hash, in place of one of that name, writes
+//!   `<data>/users.json`, prints `blog: set the password of <name>`, and
+//!   ends. It never reads or writes `blog.json`, so a server running
+//!   meanwhile loses nothing; but the server checks logins against the
+//!   users it read when it started, so the password takes effect when it
+//!   next starts.
+//! - `--iterations <n>`: PBKDF2's rounds for `--set-password`, and for a
+//!   login with a name no user has when there is no user; 600,000 unless
+//!   given.
+//!
+//! It stops on SIGTERM or SIGINT, as `Server` does, once the writes it has
+//! spawned are done.
+
+import app::{App, Data, Draft, Post, data_for}
+import auth::{Session, Sessions, hash_password, load_key, no_user_rounds, password_ok}
+import forms::{cleaned, comment_ok, login_ok, post_ok}
+import post_card::{first_paragraph}
+import store::{Store, Users, load_store, load_users, save_users, saved}
+import ui
+import ui::ssr::{Dist, page_with}
+
+/// OWASP's count of PBKDF2-HMAC-SHA256 rounds for a password.
+const ITERATIONS: Int = 600000
+
+/// What the page of a form says when its token is missing or wrong.
+const EXPIRED: String = "This form has expired, or it was not sent from this site. Open the page again and send it from there."
+
+/// What the page says when someone who is not logged in posts an admin's
+/// form.
+const NOT_ADMIN: String = "Only the admin can do that. Log in first."
+
+const USAGE: String = "usage: fullstack-blog-server [--address <host:port>] [--data <dir>] [--dist <dir>] [--set-password <name>] [--iterations <n>]"
+
+// The arguments.
+record Options {
+    address: String,
+    data: String,
+    dist: String,
+    set_password: Option<String>,
+    iterations: Int,
+}
+
+// What every handler shares: the store, the users read at start, the
+// sessions, the built site, the data directory, and the rounds for a name
+// no user has when there is no user.
+record Blog {
+    store: Store,
+    users: Users,
+    sessions: Sessions,
+    dist: Dist,
+    data: String,
+    iterations: Int,
+}
+
+// The session a page is shown in, and whether it was started for this page,
+// so its cookie goes with the response.
+record Visit { session: Session, fresh: Bool }
+
+// --- Arguments ---
+
+// The number `s` writes in decimal digits, or `None`.
+fn number(s: String) -> Option<Int> {
+    let codes = bytes_from_string(s).to_ints()
+    if codes.len() == 0 || codes.len() > 15 { return None }
+    let mut n = 0
+    let mut i = 0
+    while i < codes.len() {
+        let c = codes[i]
+        if c < 48 || c > 57 { return None }
+        n = n * 10 + c - 48
+        i = i + 1
+    }
+    Some(n)
+}
+
+// The options `argv` gives, after the program's name, or what is wrong.
+fn options(argv: Vec<String>) -> Result<Options, String> {
+    let mut o = Options {
+        address: "127.0.0.1:3000",
+        data: "data",
+        dist: "dist",
+        set_password: None,
+        iterations: ITERATIONS,
+    }
+    let mut i = 1
+    while i < argv.len() {
+        let flag = argv.get(i).unwrap_or("")
+        let given = match argv.get(i + 1) {
+            Some(v) => v
+            None => return Err("${flag} needs a value")
+        }
+        if flag == "--address" {
+            o.address = given
+        } else if flag == "--data" {
+            o.data = given
+        } else if flag == "--dist" {
+            o.dist = given
+        } else if flag == "--set-password" {
+            o.set_password = Some(given)
+        } else if flag == "--iterations" {
+            let n = number(given).unwrap_or(0)
+            if n < 1 || n > 4294967295 {
+                return Err("--iterations takes a number from 1 to 4294967295, not ${given}")
+            }
+            o.iterations = n
+        } else {
+            return Err("unknown argument ${flag}")
+        }
+        i = i + 2
+    }
+    Ok(o)
+}
+
+// The host of `address`, `host:port`: all before its last `:`.
+fn host_of(address: String) -> String {
+    let cs = address.chars()
+    let mut at = cs.len()
+    let mut i = 0
+    while i < cs.len() {
+        if cs[i] == ':' { at = i }
+        i = i + 1
+    }
+    address.slice(0, at)
+}
+
+// --- Sessions, pages and refusals ---
+
+// The session the request's cookie names, if it is one.
+fn session_of(b: Blog, req: Request) -> Option<Session> {
+    match req.cookies().get("sid") {
+        Some(value) => b.sessions.find(value)
+        None => None
+    }
+}
+
+// The session a page is shown in: the request's, or a visitor's started now.
+fn visit(b: Blog, req: Request) -> Visit {
+    match session_of(b, req) {
+        Some(s) => Visit { session: s, fresh: false }
+        None => {
+            let mut sessions = b.sessions
+            Visit { session: sessions.start(""), fresh: true }
+        }
+    }
+}
+
+// The cookie that carries `s`, lasting as long as it does.
+fn cookie_for(b: Blog, s: Session) -> Cookie {
+    let mut c = Cookie::new("sid", b.sessions.cookie(s))
+    c.max_age = Some(s.life)
+    c
+}
+
+// The data every page of `s` starts from: who is logged in and the token.
+fn base(b: Blog, s: Session) -> Data { data_for(s.user, b.sessions.token(s)) }
+
+// The page at the request's path for `data`, with `status`, and the cookie
+// of a session started for it.
+fn page(b: Blog, v: Visit, req: Request, status: Int, data: Data) -> Response {
+    let r = b.dist.respond(status, page_with(req.path, data, App))
+    if v.fresh { r.with_cookie(cookie_for(b, v.session)) } else { r }
+}
+
+// The not-found page, with `404`.
+fn missing(b: Blog, v: Visit, req: Request) -> Response {
+    let mut d = base(b, v.session)
+    d.missing = true
+    page(b, v, req, 404, d)
+}
+
+// The page saying why a form was refused, with `403`, shown in `s` when
+// there is one. Nothing changes.
+fn refused(b: Blog, req: Request, s: Option<Session>, why: String) -> Response {
+    let mut d = match s {
+        Some(x) => base(b, x)
+        None => data_for("", "")
+    }
+    d.refused = why
+    b.dist.respond(403, page_with(req.path, d, App))
+}
+
+// The post the request's `:id` names, if there is one.
+fn post_named(b: Blog, req: Request) -> Option<Post> {
+    match number(req.params.get("id").unwrap_or("")) {
+        Some(id) => b.store.post(id)
+        None => None
+    }
+}
+
+// A draft of a form that failed its checks.
+fn failed(title: String, body: String, name: String, text: String, error: String) -> Draft {
+    Draft { title: title, body: body, name: name, text: text, failed: true, error: error }
+}
+
+// The time now, as a post or a comment keeps it.
+fn now() -> String { SystemTime::now().to_iso8601() }
+
+// Spawns the write of the whole store, which runs after the response.
+fn save(b: Blog) { let _ = spawn(saved(b.data, b.store)) }
+
+// `posts` with each body cut to its first paragraph, for the home page.
+fn excerpts(posts: [Post]) -> [Post] {
+    let mut out = [Post { id: 0, title: "", body: "", created: "", updated: "" }; posts.len()]
+    let mut i = 0
+    while i < posts.len() {
+        let p = posts[i]
+        out[i] = Post {
+            id: p.id,
+            title: p.title,
+            body: first_paragraph(p.body),
+            created: p.created,
+            updated: p.updated,
+        }
+        i = i + 1
+    }
+    out
+}
+
+// `posts` with no bodies, for the admin's list.
+fn titles(posts: [Post]) -> [Post] {
+    let mut out = [Post { id: 0, title: "", body: "", created: "", updated: "" }; posts.len()]
+    let mut i = 0
+    while i < posts.len() {
+        let p = posts[i]
+        out[i] = Post { id: p.id, title: p.title, body: "", created: p.created, updated: p.updated }
+        i = i + 1
+    }
+    out
+}
+
+// The answer to a form posted to the request's path: `then`'s, given the
+// session and the fields, once the form's CSRF token is found to be the
+// session's, and, for an admin's form, the session to be the admin's;
+// otherwise `403`, and nothing changes. The token is checked before any
+// other field is looked at.
+fn checked(
+    b: Blog,
+    req: Request,
+    admin: Bool,
+    then: fn(Session, Map<String, String>) -> Response,
+) -> Response {
+    let found = session_of(b, req)
+    let fields = req.form()
+    let s = match found {
+        Some(s) => s
+        None => return refused(b, req, None, EXPIRED)
+    }
+    if !b.sessions.token_ok(s, fields.get("csrf").unwrap_or("")) {
+        return refused(b, req, Some(s), EXPIRED)
+    }
+    if admin && s.user == "" { return refused(b, req, Some(s), NOT_ADMIN) }
+    then(s, fields)
+}
+
+// --- The pages ---
+
+// `/`: the posts, newest first.
+fn home(b: Blog, req: Request) -> Response {
+    let v = visit(b, req)
+    let mut d = base(b, v.session)
+    d.posts = excerpts(b.store.newest_first())
+    page(b, v, req, 200, d)
+}
+
+// `/posts/:id`: the post, its comments and the form.
+fn post_page(b: Blog, req: Request) -> Response {
+    let v = visit(b, req)
+    match post_named(b, req) {
+        Some(p) => {
+            let mut d = base(b, v.session)
+            d.post = Some(p)
+            d.comments = b.store.comments_on(p.id)
+            page(b, v, req, 200, d)
+        }
+        None => missing(b, v, req)
+    }
+}
+
+// `/login`: the form.
+fn login_page(b: Blog, req: Request) -> Response {
+    let v = visit(b, req)
+    page(b, v, req, 200, base(b, v.session))
+}
+
+// The admin's session, if the request has one.
+fn admin_of(b: Blog, req: Request) -> Option<Session> {
+    match session_of(b, req) {
+        Some(s) => { if s.user == "" { None } else { Some(s) } }
+        None => None
+    }
+}
+
+// An admin's page: `then`'s, for the admin; anyone else goes to the login.
+fn for_admin(b: Blog, req: Request, then: fn(Visit) -> Response) -> Response {
+    match admin_of(b, req) {
+        Some(s) => then(Visit { session: s, fresh: false })
+        None => Response::redirect("/login")
+    }
+}
+
+// `/admin`: the posts, newest first, to edit or delete.
+fn admin_page(b: Blog, req: Request, v: Visit) -> Response {
+    let mut d = base(b, v.session)
+    d.posts = titles(b.store.newest_first())
+    page(b, v, req, 200, d)
+}
+
+// `/admin/new`: the editor, empty.
+fn new_post(b: Blog, req: Request, v: Visit) -> Response {
+    page(b, v, req, 200, base(b, v.session))
+}
+
+// `/admin/posts/:id/edit`: the editor, filled.
+fn edit_post(b: Blog, req: Request, v: Visit) -> Response {
+    match post_named(b, req) {
+        Some(p) => {
+            let mut d = base(b, v.session)
+            d.post = Some(p)
+            d.draft = Draft {
+                title: p.title,
+                body: p.body,
+                name: "",
+                text: "",
+                failed: false,
+                error: "",
+            }
+            page(b, v, req, 200, d)
+        }
+        None => missing(b, v, req)
+    }
+}
+
+// --- The forms ---
+
+// `POST /posts/:id/comments`: adds the comment, then goes to the post's.
+fn add_comment(b: Blog, req: Request, s: Session, fields: Map<String, String>) -> Response {
+    let v = Visit { session: s, fresh: false }
+    let post = match post_named(b, req) {
+        Some(p) => p
+        None => return missing(b, v, req)
+    }
+    let name = cleaned(fields.get("name").unwrap_or(""))
+    let text = cleaned(fields.get("text").unwrap_or(""))
+    if !comment_ok(name, text) {
+        let mut d = base(b, s)
+        d.post = Some(post)
+        d.comments = b.store.comments_on(post.id)
+        d.draft = failed("", "", name, text, "")
+        return page(b, v, req, 400, d)
+    }
+    let mut store = b.store
+    let _ = store.add_comment(post.id, name, text, now())
+    save(b)
+    Response::redirect("/posts/${post.id}#comments")
+}
+
+// `POST /login`: checks the password; then the old session ends, a new one
+// starts for the admin, and the admin's page comes next.
+fn log_in(b: Blog, req: Request, s: Session, fields: Map<String, String>) -> Response {
+    let v = Visit { session: s, fresh: false }
+    let name = fields.get("name").unwrap_or("").trim()
+    let password = fields.get("password").unwrap_or("")
+    if !login_ok(name, password) {
+        let mut d = base(b, s)
+        d.draft = failed("", "", name, "", "")
+        return page(b, v, req, 400, d)
+    }
+    // A name no user has costs the most rounds a user's hash has.
+    let rounds = no_user_rounds(b.users.list, b.iterations)
+    if !password_ok(b.users.user(name), password, rounds) {
+        let mut d = base(b, s)
+        d.draft = failed("", "", name, "", "Wrong name or password.")
+        return page(b, v, req, 401, d)
+    }
+    let mut sessions = b.sessions
+    sessions.end(s.id)
+    let admin = sessions.start(name)
+    Response::redirect("/admin").with_cookie(cookie_for(b, admin))
+}
+
+// `POST /logout`: ends the session and removes its cookie.
+fn log_out(b: Blog, s: Session) -> Response {
+    b.sessions.end(s.id)
+    Response::redirect("/").with_cookie(Cookie::expired("sid"))
+}
+
+// `POST /admin/posts`: adds the post.
+fn create_post(b: Blog, req: Request, s: Session, fields: Map<String, String>) -> Response {
+    let title = cleaned(fields.get("title").unwrap_or(""))
+    let body = cleaned(fields.get("body").unwrap_or(""))
+    if !post_ok(title, body) {
+        let mut d = base(b, s)
+        d.draft = failed(title, body, "", "", "")
+        return page(b, Visit { session: s, fresh: false }, req, 400, d)
+    }
+    let mut store = b.store
+    let _ = store.add_post(title, body, now())
+    save(b)
+    Response::redirect("/admin")
+}
+
+// `POST /admin/posts/:id`: changes the post.
+fn update_post(b: Blog, req: Request, s: Session, fields: Map<String, String>) -> Response {
+    let v = Visit { session: s, fresh: false }
+    let post = match post_named(b, req) {
+        Some(p) => p
+        None => return missing(b, v, req)
+    }
+    let title = cleaned(fields.get("title").unwrap_or(""))
+    let body = cleaned(fields.get("body").unwrap_or(""))
+    if !post_ok(title, body) {
+        let mut d = base(b, s)
+        d.post = Some(post)
+        d.draft = failed(title, body, "", "", "")
+        return page(b, v, req, 400, d)
+    }
+    let mut store = b.store
+    let _ = store.edit_post(post.id, title, body, now())
+    save(b)
+    Response::redirect("/admin")
+}
+
+// `POST /admin/posts/:id/delete`: deletes the post and its comments.
+fn delete_post(b: Blog, req: Request, s: Session) -> Response {
+    let post = match post_named(b, req) {
+        Some(p) => p
+        None => return missing(b, Visit { session: s, fresh: false }, req)
+    }
+    let mut store = b.store
+    let _ = store.delete_post(post.id)
+    save(b)
+    Response::redirect("/admin")
+}
+
+// `POST /admin/comments/:id/delete`: deletes the comment, then goes back to
+// its post.
+fn delete_comment(b: Blog, req: Request, s: Session) -> Response {
+    let mut store = b.store
+    let gone = match number(req.params.get("id").unwrap_or("")) {
+        Some(id) => store.delete_comment(id)
+        None => None
+    }
+    match gone {
+        Some(c) => {
+            save(b)
+            Response::redirect("/posts/${c.post}#comments")
+        }
+        None => missing(b, Visit { session: s, fresh: false }, req)
+    }
+}
+
+// --- The server ---
+
+// The blog's routes, beside the built site. A path that has only a `POST`
+// route answers a `GET` with the not-found page, as any path without a page
+// does, rather than `405`.
+fn server(b: Blog) -> Server {
+    let nowhere = |req: Request| missing(b, visit(b, req), req)
+    Server::new()
+        .get("/", |req: Request| home(b, req))
+        .get("/posts/:id", |req: Request| post_page(b, req))
+        .post(
+            "/posts/:id/comments",
+            |req: Request| checked(
+                b,
+                req,
+                false,
+                |s: Session, f: Map<String, String>| add_comment(b, req, s, f),
+            ),
+        )
+        .get("/posts/:id/comments", nowhere)
+        .get("/login", |req: Request| login_page(b, req))
+        .post(
+            "/login",
+            |req: Request| checked(
+                b,
+                req,
+                false,
+                |s: Session, f: Map<String, String>| log_in(b, req, s, f),
+            ),
+        )
+        .post(
+            "/logout",
+            |req: Request| checked(
+                b,
+                req,
+                false,
+                |s: Session, f: Map<String, String>| log_out(b, s),
+            ),
+        )
+        .get("/logout", nowhere)
+        .get("/admin", |req: Request| for_admin(b, req, |v: Visit| admin_page(b, req, v)))
+        .get("/admin/new", |req: Request| for_admin(b, req, |v: Visit| new_post(b, req, v)))
+        .post(
+            "/admin/posts",
+            |req: Request| checked(
+                b,
+                req,
+                true,
+                |s: Session, f: Map<String, String>| create_post(b, req, s, f),
+            ),
+        )
+        .get("/admin/posts", nowhere)
+        .get(
+            "/admin/posts/:id/edit",
+            |req: Request| for_admin(b, req, |v: Visit| edit_post(b, req, v)),
+        )
+        .post(
+            "/admin/posts/:id",
+            |req: Request| checked(
+                b,
+                req,
+                true,
+                |s: Session, f: Map<String, String>| update_post(b, req, s, f),
+            ),
+        )
+        .get("/admin/posts/:id", nowhere)
+        .post(
+            "/admin/posts/:id/delete",
+            |req: Request| checked(
+                b,
+                req,
+                true,
+                |s: Session, f: Map<String, String>| delete_post(b, req, s),
+            ),
+        )
+        .get("/admin/posts/:id/delete", nowhere)
+        .post(
+            "/admin/comments/:id/delete",
+            |req: Request| checked(
+                b,
+                req,
+                true,
+                |s: Session, f: Map<String, String>| delete_comment(b, req, s),
+            ),
+        )
+        .get("/admin/comments/:id/delete", nowhere)
+        .fallback(
+            |req: Request| match b.dist.file(req) {
+                Some(file) => file
+                None => missing(b, visit(b, req), req)
+            },
+        )
+}
+
+// One line from standard input, less its line end, or `None` when it is not
+// UTF-8.
+async fn read_line() -> Option<String> {
+    let mut got = bytes_from_ints([])
+    let newline = bytes_from_string("\n")
+    let mut more = true
+    while more {
+        match stdin().read(4096).await {
+            Ok(chunk) => {
+                if chunk.len() == 0 { more = false } else { got = got.concat(chunk) }
+                if got.contains(newline) { more = false }
+            }
+            Err(e) => more = false
+        }
+    }
+    let text = match got.to_string() {
+        Some(t) => t
+        None => return None
+    }
+    let line = match text.index_of("\n") {
+        Some(at) => text.slice(0, at)
+        None => text
+    }
+    if line.ends_with("\r") { Some(line.slice(0, line.len() - 1)) } else { Some(line) }
+}
+
+// `--set-password <name>`: the user `name`, with the password read from
+// standard input, in `<data>/users.json`.
+async fn set_password(o: Options, name: String) {
+    if name.trim() == "" || name != name.trim() {
+        eprintln("blog: a user's name is not blank and has no spaces around it")
+        exit(2)
+        return
+    }
+    let password = match read_line().await {
+        Some(p) => p
+        None => {
+            eprintln("blog: the password is not UTF-8")
+            exit(1)
+            return
+        }
+    }
+    if password.len() < 8 {
+        eprintln("blog: a password has at least eight characters")
+        exit(1)
+        return
+    }
+    if password.trim() == "" {
+        eprintln("blog: a password is not blank")
+        exit(1)
+        return
+    }
+    // The hash first, since it takes longest: the file is read after it,
+    // and written straight away.
+    let user = hash_password(name, password, o.iterations)
+    let mut users = match load_users(o.data).await {
+        Ok(u) => u
+        Err(why) => {
+            eprintln("blog: ${why}")
+            exit(1)
+            return
+        }
+    }
+    users.set_user(user)
+    match save_users(o.data, users).await {
+        Ok(_) => {}
+        Err(e) => {
+            eprintln("blog: cannot write ${o.data}/users.json: ${e.message}")
+            exit(1)
+            return
+        }
+    }
+    println("blog: set the password of ${name}")
+}
+
+async fn main() {
+    let o = match options(args()) {
+        Ok(o) => o
+        Err(why) => {
+            eprintln("blog: ${why}")
+            eprintln(USAGE)
+            exit(2)
+            return
+        }
+    }
+    match o.set_password {
+        Some(name) => {
+            set_password(o, name).await
+            return
+        }
+        None => {}
+    }
+    let store = match load_store(o.data).await {
+        Ok(s) => s
+        Err(why) => {
+            eprintln("blog: ${why}")
+            exit(1)
+            return
+        }
+    }
+    let users = match load_users(o.data).await {
+        Ok(u) => u
+        Err(why) => {
+            eprintln("blog: ${why}")
+            exit(1)
+            return
+        }
+    }
+    let key = match load_key(o.data).await {
+        Ok(k) => k
+        Err(why) => {
+            eprintln("blog: ${why}")
+            exit(1)
+            return
+        }
+    }
+    let dist = match Dist::open(o.dist).await {
+        Ok(d) => d
+        Err(e) => {
+            eprintln("blog: ${e.message}")
+            exit(1)
+            return
+        }
+    }
+    let l = match bind(o.address) {
+        Ok(l) => l
+        Err(e) => {
+            eprintln("blog: cannot bind ${o.address}: ${e.message}")
+            exit(1)
+            return
+        }
+    }
+    let port = match l.local_port() {
+        Ok(p) => p
+        Err(e) => {
+            eprintln("blog: ${e.message}")
+            exit(1)
+            return
+        }
+    }
+    println("blog: serving http://${host_of(o.address)}:${port}/")
+    let _ = stdout().flush().await
+    let b = Blog {
+        store: store,
+        users: users,
+        sessions: Sessions::new(key),
+        dist: dist,
+        data: o.data,
+        iterations: o.iterations,
+    }
+    match server(b).listen_on(l).await {
+        Ok(_) => {}
+        Err(e) => {
+            eprintln("blog: ${e.message}")
+            exit(1)
+        }
+    }
+}
````

### Task 7: The browser gate and CI

Spec §1 (the gate), §7.5, and the user's decision 9.

**Files:**
- Create: `tests/browser/blog.mjs`
- Modify: `.github/workflows/ci.yml` (the `browser` job)

- [ ] **Step 1: The test and CI.** `patch_block t7-code`:
  - **`tests/browser/blog.mjs`,** run as `node tests/browser/blog.mjs
    <nova> <work dir> <dir holding node_modules/playwright>`: it copies the
    example into the work directory, emptied first, less `dist/`,
    `target/`, `data/blog.json`, `data/users.json`, each with its `.tmp`,
    and `data/secret`,
    builds it there with `nova build --ssr`, with `NOVA_HOME` beside it, as
    `blog_tests.rs` does, sets the admin's password with `--set-password
    admin --iterations 1000`, the password on standard input, starts the
    server with `--address 127.0.0.1:0`, and reads the port from its first
    line. A page "hydrates in place" when, opened with `app.wasm` held
    back, its root reads `data-state="server"`, each form's submit button is
    there and enabled, and every node under `#app` is kept as a handle; and
    when, once `data-state` reads `hydrated`, each handle but those inside a
    form's `.actions` is still in the page, in order: the submit button is
    the one thing the client may swap (decision 24). Through Playwright, in
    Chromium:
    - `/`, `/posts/1` and `/login` answer `200` and hydrate in place, and
      `/nowhere` and `/posts/99` answer `404` and hydrate in place;
    - on `/posts/1`, the comment form's button is disabled while a field is
      blank, with the field's message under it, and enabled once both are
      filled; the comment posts, and the post's page shows it;
    - a comment with a blank text, sent by the form's `submit()` so the
      client's checks do not stop it, is answered `400` with the post's
      page at `/posts/1/comments`, the path it was posted to, which
      hydrates in place with the name kept and the text's message
      (decision 23);
    - a comment whose text holds a NUL posts, comes back without it, and
      the post's page still hydrates in place (decision 39);
    - a comment whose name and text hold a script that would set
      `window.novaXss` and open a dialog posts too, and neither happens: its
      text shows as written;
    - the comment form, its hidden `csrf` field removed, is answered `403`
      with the page saying the form has expired, and adds nothing;
    - it logs in through the form; `/admin`, `/admin/new` and an edit page
      hydrate in place; it creates a post whose title holds the same script,
      which shows as text on the admin's list, the home page and its own,
      and runs nothing; edits it; and deletes it, after which its page is
      `404`.

    No page logs a console error, a mismatch among them, but the one
    Chromium logs for a page answered `4xx` that the step expects; no page
    throws, and no dialog opens. It prints one line, with the nodes each
    page kept and the client's size, `app.wasm` and `app.js` gzipped,
    printed and not limited, and kills the server at the end, pass or fail.
  - **CI's `browser` job:** one more step, `The blog in Chromium`, after
    `SSR and SSG in Chromium`, runs `blog.mjs` with `target/debug/nova`, a
    work directory in `$RUNNER_TEMP` and the job's Playwright; the job's
    comment names 4.7's gate. The job's build, `cargo build --locked -p
    nova-cli -p nova-runtime`, already leaves the runtime library beside
    `nova`, which the blog's server links.
- [ ] **Step 2: Run the `browser` job, in Chromium,** where Task 0 found
  Playwright: its builds and every script, as CI runs them. As in CI, the
  runtime library beside `nova` must come from the job's own build, and no
  `NOVA_RUNTIME_LIB` may name one: so the library left by earlier builds is
  removed first, and the build puts it back.

  ```sh
  unset NOVA_RUNTIME_LIB
  rm -f target/debug/libnova_runtime.a
  cargo build --locked -p nova-cli -p nova-runtime
  ls target/debug/libnova_runtime.a
  git status --short --ignored --untracked-files=all examples tests > "$P/status-47.txt"
  B="$P/browser-47"
  rm -rf "$B"
  export PLAYWRIGHT_BROWSERS_PATH=/opt/pw-browsers
  target/debug/nova build --target wasm tests/dom/counter.nova -o "$B/counter"
  target/debug/nova build --target wasm tests/dom/ui_counter.nova -o "$B/ui_counter"
  (cd examples/06-counter-spa && ../../target/debug/nova build --target wasm -o "$B/counter-spa")
  node tests/browser/counter.mjs "$B/counter" /opt/node-tools
  node tests/browser/counter.mjs "$B/ui_counter" /opt/node-tools
  node tests/browser/spa.mjs "$B/counter-spa" /opt/node-tools
  node tests/browser/dev.mjs target/debug/nova "$B/dev" /opt/node-tools
  node tests/browser/router.mjs target/debug/nova "$B/router" /opt/node-tools
  node tests/browser/ssr.mjs target/debug/nova "$B/ssr" /opt/node-tools
  node tests/browser/blog.mjs target/debug/nova "$B/blog" /opt/node-tools
  git status --short --ignored --untracked-files=all examples tests | diff "$P/status-47.txt" -
  ```

  Expected:
  - the build ends with `Finished`, `ls` names the library it put back,
    and each of the three builds prints `built <its directory>`;
  - `counter.mjs`, twice: `counter: three clicks, and the count reads 3`;
  - `spa.mjs`: `counter-spa: +, +, +, -, and the count reads 2`;
  - `dev.mjs`, in about 5 seconds: `dev: counts at build <start>.1,
    reloads into .2 on an edit, into .3 after a missed one, shows the
    overlay for the type error of .4 and keeps it over a reload, and counts
    again at .5`, where `<start>` is the server's start in milliseconds,
    such as the replay's `1792643813423`;
  - `router.mjs`, in about 2 seconds: `router: a link to /posts/2 without a
    reload, back home, a reload at /posts/2 that stays at build <start>.1,
    /nowhere not found, and a query read`;
  - `ssr.mjs`, in about 4 seconds: `ssr: / hydrates in place (36 nodes
    kept + 1 joined) and counts, a link to /about without a reload,
    /posts/2 in place (26 + 1), /nowhere 404 in place (28 + 1), and SSG's
    /about/ arrives rendered and hydrates in place (28 + 1); the client is
    41688 bytes gzipped`, as at the base;
  - `blog.mjs`, in about 9 seconds with its build: `blog: every page
    hydrates in place (nodes kept: / 37, /posts/1 48, /login 24, /nowhere
    15, /posts/99 15, /posts/1/comments 59, /admin 53, /admin/new 29,
    /admin/posts/2/edit 30), the comment form's button is disabled while a
    field is blank and the comment posts, a comment that fails a check gets
    400 at /posts/1/comments, a comment holding a NUL comes back without it,
    a script in a comment and in a title stays text, a form without its
    token gets 403, and the admin logs in and creates, edits and deletes a
    post; the client is 49472 bytes gzipped`. Record the client's size in
    the ledger: it is printed, not limited (07 has no limit; 06's is 50 KB);
  - `diff` prints nothing: the builds write outside the tree, and the
    scripts work in copies. In the replay the status itself printed nothing.
- [ ] **Step 3: The checks,** as the conventions list them.
- [ ] **Step 4: Commit** `ci: the blog in Chromium`.

**Patch `t7-code`:**

<!-- patch t7-code -->

````diff
diff --git a/.github/workflows/ci.yml b/.github/workflows/ci.yml
index 296a9854aa4c0f64d8382261674563e4a5ec6eb4..2b865c5cc16d2c4bfdf19327ccf247cb01d642c3 100644
--- a/.github/workflows/ci.yml
+++ b/.github/workflows/ci.yml
@@ -108,7 +108,15 @@ jobs:
   # in `$RUNNER_TEMP`, starts its server on port 0, and checks that a
   # server-rendered page, and a page SSG wrote, served by a static server,
   # hydrate in place: the server's nodes stay, and a click updates them. It
-  # kills the server at the end. Playwright, pinned, is installed here only,
+  # kills the server at the end. And 4.7's gate (spec
+  # docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md
+  # §7.5): `blog.mjs` builds `examples/07-fullstack-blog` with `nova build
+  # --ssr` in `$RUNNER_TEMP`, sets its admin's password, starts its server
+  # on port 0, and checks that every page hydrates in place, that the
+  # comment form's checks and post work, that the admin creates, edits and
+  # deletes a post, that a script in a title or a comment stays text, and
+  # that a form without its CSRF token is refused; it kills the server at
+  # the end. Playwright, pinned, is installed here only,
   # into a scratch directory; nothing in `cargo test` or `nova` installs a
   # package. Ubuntu only: one browser job suffices.
   browser:
@@ -148,6 +156,8 @@ jobs:
           node tests/browser/router.mjs target/debug/nova "$RUNNER_TEMP/router" "$RUNNER_TEMP/pw"
       - name: SSR and SSG in Chromium
         run: node tests/browser/ssr.mjs target/debug/nova "$RUNNER_TEMP/ssr" "$RUNNER_TEMP/pw"
+      - name: The blog in Chromium
+        run: node tests/browser/blog.mjs target/debug/nova "$RUNNER_TEMP/blog" "$RUNNER_TEMP/pw"
 
   fmt:
     name: Rustfmt
diff --git a/tests/browser/blog.mjs b/tests/browser/blog.mjs
new file mode 100644
index 0000000000000000000000000000000000000000..0709918081ad191a90488510665201186c93bc3d
--- /dev/null
+++ b/tests/browser/blog.mjs
@@ -0,0 +1,463 @@
+// The gate of 4.7 in Chromium (spec
+// docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md
+// §7.5), run by CI's `browser` job:
+//
+//     node tests/browser/blog.mjs <nova> <work dir> <dir holding node_modules/playwright>
+//
+// It copies `examples/07-fullstack-blog` into the work directory, emptied
+// first, leaving out what a build or a run there may have left (`dist/`,
+// `target/`, `data/blog.json`, `data/users.json`, each with its `.tmp`,
+// and `data/secret`), and builds it with `nova build --ssr`, with
+// `NOVA_HOME` beside it, as `crates/nova-cli/tests/blog_tests.rs` does.
+// It sets the admin's password through
+// `fullstack-blog-server --set-password admin --iterations 1000`, the
+// password on standard input, and starts the server on
+// `--address 127.0.0.1:0`, reading the port from the line
+// `blog: serving http://127.0.0.1:<port>/`.
+//
+// A page "hydrates in place" when, opened with `app.wasm` held back
+// (`page.route`), its `#root` reads `data-state="server"`, each form's
+// submit button is there and enabled, so the form works without
+// JavaScript, and every node under `#app` is kept as a handle; then, once `app.wasm` loads and the
+// app's `on_mount` sets `data-state` to `hydrated`, after hydration's walk,
+// each handle but those inside a form's `.actions` is still in the page, in
+// the same order. The submit button inside `.actions` is the one thing the
+// client may swap: a form whose fields are blank gets the disabled button
+// in place of the server's enabled one. Through Playwright:
+// 1. `/`, `/posts/1` and `/login` answer `200` and hydrate in place;
+//    `/nowhere` and `/posts/99` answer `404` and hydrate in place;
+// 2. on `/posts/1`, the comment form's button is disabled while a field is
+//    blank, with the field's message under it, and enabled once both are
+//    filled; the comment then posts, and the post's page shows it;
+// 3. a comment with a blank text, sent by the form's `submit()` so the
+//    client's checks do not stop it, is answered `400` with the post's page
+//    at `/posts/1/comments`, the path it was posted to, which hydrates in
+//    place with the name kept and the text's message;
+// 4. a comment whose text holds a NUL posts, and comes back without it,
+//    and the post's page still hydrates in place;
+// 5. a comment whose name and text hold a script that would set
+//    `window.novaXss` and open a dialog posts too: neither happens, and its
+//    text shows as written;
+// 6. the comment form, its hidden `csrf` field removed, is answered `403`
+//    with the page saying the form has expired, and the comment is not
+//    added;
+// 7. it logs in through the form; `/admin`, `/admin/new` and an edit page
+//    hydrate in place; it creates a post whose title holds the same script,
+//    which shows as text on the admin's list, the home page and its own,
+//    and runs nothing; edits it; and deletes it, after which its page is
+//    `404`.
+// No page logs a console error, a hydration mismatch among them, but the
+// one Chromium logs for a document answered `4xx` that the step expects;
+// no page throws, and no dialog opens. The summary line says what it did,
+// the nodes each page kept, and the client's size, `app.wasm` and `app.js`
+// gzipped, printed and not limited. The server is killed at the end, pass
+// or fail. Every wait polls, for 30 seconds at most. Playwright is the
+// job's own install, as for `counter.mjs`.
+
+import { spawn } from "node:child_process";
+import { cp, mkdir, readFile, rm } from "node:fs/promises";
+import { createRequire } from "node:module";
+import { dirname, join, relative, resolve, sep } from "node:path";
+import { createInterface } from "node:readline";
+import { fileURLToPath } from "node:url";
+import { gzipSync } from "node:zlib";
+
+const [nova, work, modules] = process.argv.slice(2).map((p) => resolve(p));
+const { chromium } = createRequire(join(modules, "noop.js"))("playwright");
+const repo = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
+const LIMIT = 30000;
+const PASSWORD = "correct horse battery";
+// A script that would set a global and open a dialog, in a tag the parser
+// runs, and in one whose error handler would.
+const XSS = `<script>window.novaXss = 1; alert("xss")</script><img src=x onerror="window.novaXss = 2; alert('xss')">`;
+// The same, short enough for a comment's name, of 60 characters at most.
+const NAME_XSS = `<img src=x onerror="window.novaXss=3;alert(3)">`;
+const EXE = process.platform === "win32" ? ".exe" : "";
+// The character a browser's parser drops from a page's text.
+const NUL = String.fromCharCode(0);
+
+// Runs `command` in `dir` to its end, `input` on its stdin: its status and
+// its output.
+function run(command, args, dir, env, input = null) {
+  return new Promise((done, failed) => {
+    const child = spawn(command, args, { cwd: dir, env, stdio: ["pipe", "pipe", "pipe"] });
+    let output = "";
+    child.stdout.on("data", (chunk) => {
+      output += chunk;
+    });
+    child.stderr.on("data", (chunk) => {
+      output += chunk;
+    });
+    child.on("error", failed);
+    child.on("exit", (code) => done({ code, output }));
+    child.stdin.end(input ?? "");
+  });
+}
+
+// The server in `dir`: its URL, its stderr so far, and a kill.
+async function startServer(dir, env) {
+  const exe = join(dir, "target", "debug", `fullstack-blog-server${EXE}`);
+  const child = spawn(exe, ["--address", "127.0.0.1:0"], { cwd: dir, env, stdio: ["ignore", "pipe", "pipe"] });
+  let stderr = "";
+  child.stderr.on("data", (chunk) => {
+    stderr += chunk;
+  });
+  const server = { url: null, stderr: () => stderr, kill: () => child.kill() };
+  try {
+    server.url = await new Promise((found, failed) => {
+      const timer = setTimeout(() => failed(new Error("no URL within 30 s")), LIMIT);
+      createInterface({ input: child.stdout }).on("line", (line) => {
+        const url = /^blog: serving (http:\/\/127\.0\.0\.1:\d+\/)$/.exec(line)?.[1];
+        if (url) {
+          clearTimeout(timer);
+          found(url);
+        }
+      });
+      child.on("exit", (code) => failed(new Error(`the server exited with ${code}`)));
+    });
+  } catch (e) {
+    child.kill();
+    throw new Error(`${e.message}; the server's stderr:\n${stderr}`);
+  }
+  return server;
+}
+
+// Polls `test(arg)` in the page until it holds; a navigation between two
+// polls is only a poll that fails.
+async function until(page, test, arg, what) {
+  const deadline = Date.now() + LIMIT;
+  for (;;) {
+    try {
+      if (await page.evaluate(test, arg)) return;
+    } catch {
+      // The page is loading.
+    }
+    if (Date.now() > deadline) {
+      const now = await page.evaluate(() => document.querySelector("h1")?.textContent).catch(() => "?");
+      throw new Error(`timed out waiting for ${what}; the page's heading reads ${JSON.stringify(now)} at ${page.url()}`);
+    }
+    await new Promise((later) => setTimeout(later, 100));
+  }
+}
+
+const state = (page) => page.evaluate(() => document.getElementById("root")?.dataset.state ?? null);
+const heading = (page) => page.evaluate(() => document.querySelector("#app h1")?.textContent ?? null);
+const hydrated = (page, what) =>
+  until(page, () => document.getElementById("root")?.dataset.state === "hydrated", null, `${what} to hydrate`);
+
+// Opens `url`, or runs `navigate`, which leads to it and returns its
+// response, with `app.wasm` held back, checks the server's HTML is there
+// with the heading `expected`, keeps a handle to each node under `#app`,
+// lets `app.wasm` load, waits for hydration, and checks each handle but
+// those inside a form's `.actions` is still in the page, in order. Returns
+// the response's status and how many nodes were kept.
+async function hydratesInPlace(page, url, expected, navigate = () => page.goto(url, { waitUntil: "domcontentloaded" })) {
+  let release;
+  const held = new Promise((go) => {
+    release = go;
+  });
+  await page.route("**/app.wasm", async (route) => {
+    await held;
+    // A check that fails while it is held unroutes it first.
+    await route.continue().catch(() => {});
+  });
+  try {
+    const response = await navigate();
+    if ((await state(page)) !== "server") {
+      throw new Error(`${url}: before app.wasm loads, #root's data-state is ${await state(page)}, not "server"`);
+    }
+    if ((await heading(page)) !== expected) {
+      throw new Error(`${url}: the server's HTML shows ${JSON.stringify(await heading(page))}, not ${expected}`);
+    }
+    // Each form's one submit button, enabled, so the form works without
+    // JavaScript.
+    const buttons = await page.$$eval("form .actions", (all) =>
+      all.map((a) => [...a.querySelectorAll("button")].map((b) => (b.disabled ? "disabled" : "enabled")).join(" ")),
+    );
+    if (buttons.some((b) => b !== "enabled")) {
+      throw new Error(`${url}: before app.wasm loads, the forms' buttons are ${JSON.stringify(buttons)}`);
+    }
+    // Each node but those inside a form's `.actions`, read while they are
+    // all in the page.
+    const kept = await page.evaluateHandle(() =>
+      window.novaNodes().filter((n) => !n.parentNode?.closest?.(".actions")),
+    );
+    release();
+    await hydrated(page, url);
+    const moved = await page.evaluate((nodes) => {
+      const gone = nodes.filter((n) => !n.isConnected || !document.getElementById("app").contains(n));
+      if (gone.length > 0) return `${gone.length} of the server's ${nodes.length} nodes are gone, the first a ${gone[0].nodeName}`;
+      const now = window.novaNodes();
+      const at = nodes.map((n) => now.indexOf(n));
+      const out = at.findIndex((p, i) => i > 0 && p <= at[i - 1]);
+      return out < 0 ? null : `node ${out}, ${nodes[out].nodeName}, moved`;
+    }, kept);
+    if (moved) throw new Error(`${url} did not hydrate in place: ${moved}`);
+    if ((await heading(page)) !== expected) {
+      throw new Error(`${url}: the hydrated page shows ${JSON.stringify(await heading(page))}, not ${expected}`);
+    }
+    const count = await kept.evaluate((nodes) => nodes.length);
+    return { status: response.status(), count };
+  } finally {
+    release();
+    await page.unroute("**/app.wasm");
+  }
+}
+
+// Clicks `selector` and waits for the page it leads to to load and hydrate:
+// the response's status.
+async function submits(page, selector, what) {
+  const [response] = await Promise.all([
+    page.waitForNavigation({ waitUntil: "domcontentloaded", timeout: LIMIT }),
+    page.click(selector),
+  ]);
+  await hydrated(page, what);
+  return response.status();
+}
+
+// The comment form's submit button is disabled, or not.
+const disabled = (page) =>
+  page.evaluate(() => {
+    const buttons = [...document.querySelectorAll("#comment-form .actions button")];
+    return buttons.length === 1 ? buttons[0].disabled : `${buttons.length} buttons`;
+  });
+
+const dir = join(work, "fullstack-blog");
+const env = { ...process.env, NOVA_HOME: join(work, "nova-home"), NO_COLOR: "1" };
+const example = join(repo, "examples", "07-fullstack-blog");
+const left = [
+  "dist",
+  "target",
+  join("data", "blog.json"),
+  join("data", "blog.json.tmp"),
+  join("data", "users.json"),
+  join("data", "users.json.tmp"),
+  join("data", "secret"),
+];
+await rm(work, { recursive: true, force: true });
+await mkdir(work, { recursive: true });
+await cp(example, dir, {
+  recursive: true,
+  filter: (from) => !left.some((l) => relative(example, from) === l || relative(example, from).startsWith(l + sep)),
+});
+
+const build = await run(nova, ["build", "--ssr"], dir, env);
+if (build.code !== 0) throw new Error(`nova build --ssr exited with ${build.code}:\n${build.output}`);
+const exe = join(dir, "target", "debug", `fullstack-blog-server${EXE}`);
+const set = await run(exe, ["--set-password", "admin", "--iterations", "1000"], dir, env, `${PASSWORD}\n`);
+if (set.code !== 0 || set.output.trim() !== "blog: set the password of admin") {
+  throw new Error(`--set-password exited with ${set.code}:\n${set.output}`);
+}
+
+const server = await startServer(dir, env);
+const at = (path) => new URL(path, server.url).href;
+let browser = null;
+try {
+  browser = await chromium.launch();
+  const page = await browser.newPage();
+  const errors = [];
+  const dialogs = [];
+  // The documents a step expects a `4xx` for, whose error Chromium logs.
+  const expected = new Set();
+  page.on("console", (m) => {
+    if (m.type() !== "error") return;
+    if (expected.has(m.location().url) && /status of 4\d\d/.test(m.text())) return;
+    errors.push(`${m.text()} (${m.location().url})`);
+  });
+  page.on("pageerror", (e) => errors.push(String(e)));
+  page.on("dialog", async (d) => {
+    dialogs.push(d.message());
+    await d.dismiss();
+  });
+  const clean = (what) => {
+    if (errors.length > 0) throw new Error(`${what}: the console has errors: ${errors.join("; ")}`);
+    if (dialogs.length > 0) throw new Error(`${what}: a dialog opened: ${dialogs.join("; ")}`);
+  };
+  await page.addInitScript(() => {
+    window.novaNodes = () => {
+      const walker = document.createTreeWalker(document.getElementById("app"), NodeFilter.SHOW_ALL);
+      const nodes = [];
+      while (walker.nextNode()) nodes.push(walker.currentNode);
+      return nodes;
+    };
+  });
+  const inert = async (what) => {
+    if ((await page.evaluate(() => window.novaXss)) !== undefined) throw new Error(`${what}: the script ran`);
+    clean(what);
+  };
+  const kept = [];
+
+  // 1. Every page a visitor sees, in place.
+  for (const [path, title, status] of [
+    ["/", "Posts", 200],
+    ["/posts/1", "Hello from a Nova server", 200],
+    ["/login", "Log in", 200],
+    ["/nowhere", "Not found", 404],
+    ["/posts/99", "Not found", 404],
+  ]) {
+    if (status !== 200) expected.add(at(path));
+    const got = await hydratesInPlace(page, at(path), title);
+    if (got.status !== status) throw new Error(`${path} answered ${got.status}, not ${status}`);
+    kept.push(`${path} ${got.count}`);
+    clean(path);
+  }
+
+  // 2. The comment form's checks, then the comment.
+  await hydratesInPlace(page, at("/posts/1"), "Hello from a Nova server");
+  if ((await disabled(page)) !== true) throw new Error(`a blank comment form's button is ${await disabled(page)}`);
+  await page.fill("#comment-form input[name=name]", "Katherine");
+  if ((await disabled(page)) !== true) throw new Error("the button is enabled with no comment");
+  const messages = () => page.$$eval("#comment-form .message", (ps) => ps.map((p) => p.textContent));
+  if ((await messages()).join("|") !== "|Write a comment.") {
+    throw new Error(`the messages read ${JSON.stringify(await messages())}`);
+  }
+  await page.fill("#comment-form textarea[name=text]", "First paragraph.\n\nSecond paragraph.");
+  if ((await disabled(page)) !== false) throw new Error("the button is disabled with both fields filled");
+  await page.fill("#comment-form input[name=name]", "  ");
+  if ((await disabled(page)) !== true) throw new Error("the button is enabled with a blank name");
+  if ((await messages())[0] !== "Say who you are.") throw new Error(`the messages read ${await messages()}`);
+  await page.fill("#comment-form input[name=name]", "Katherine");
+  const posted = await submits(page, "#comment-form button[type=submit]", "the post after a comment");
+  if (posted !== 200 || new URL(page.url()).pathname !== "/posts/1" || new URL(page.url()).hash !== "#comments") {
+    throw new Error(`the comment led to ${page.url()} with ${posted}`);
+  }
+  const said = await page.$$eval("#comments .comment", (cs) => cs.map((c) => c.innerText));
+  const last = said[said.length - 1] ?? "";
+  if (!last.startsWith("Katherine") || !last.includes("First paragraph.") || !last.includes("Second paragraph.")) {
+    throw new Error(`the comments read ${JSON.stringify(said)}`);
+  }
+  clean("a comment");
+
+  // 3. A comment that fails a check, at the path it was posted to.
+  await page.fill("#comment-form input[name=name]", "Katherine");
+  await page.fill("#comment-form textarea[name=text]", "");
+  expected.add(at("/posts/1/comments"));
+  const failed = await hydratesInPlace(page, at("/posts/1/comments"), "Hello from a Nova server", async () => {
+    const [response] = await Promise.all([
+      page.waitForNavigation({ waitUntil: "domcontentloaded", timeout: LIMIT }),
+      page.evaluate(() => document.getElementById("comment-form").submit()),
+    ]);
+    return response;
+  });
+  if (failed.status !== 400 || new URL(page.url()).pathname !== "/posts/1/comments") {
+    throw new Error(`a comment with a blank text got ${failed.status} at ${page.url()}`);
+  }
+  const name = await page.inputValue("#comment-form input[name=name]");
+  if (name !== "Katherine" || (await messages()).join("|") !== "|Write a comment.") {
+    throw new Error(`the failed comment's form reads ${JSON.stringify(name)}, ${JSON.stringify(await messages())}`);
+  }
+  kept.push(`/posts/1/comments ${failed.count}`);
+  clean("a comment that fails a check");
+
+  // 4. A comment holding a NUL.
+  await page.evaluate((text) => {
+    const field = document.querySelector("#comment-form textarea[name=text]");
+    field.value = text;
+    field.dispatchEvent(new Event("input", { bubbles: true }));
+  }, `one${NUL}two`);
+  await submits(page, "#comment-form button[type=submit]", "the post after a comment with a NUL");
+  await hydratesInPlace(page, at("/posts/1"), "Hello from a Nova server");
+  const nul = await page.$$eval("#comments .comment .body p", (ps) => ps[ps.length - 1].textContent);
+  if (nul !== "onetwo") throw new Error(`the comment with a NUL reads ${JSON.stringify(nul)}`);
+  clean("a comment with a NUL");
+
+  // 5. A comment holding a script.
+  await page.fill("#comment-form input[name=name]", NAME_XSS);
+  await page.fill("#comment-form textarea[name=text]", XSS);
+  await submits(page, "#comment-form button[type=submit]", "the post after a comment with a script");
+  const shown = await page.$$eval("#comments .comment", (cs) => {
+    const c = cs[cs.length - 1];
+    return { who: c.querySelector("strong").textContent, text: c.querySelector(".body p").textContent };
+  });
+  if (shown.who !== NAME_XSS || shown.text !== XSS) {
+    throw new Error(`the comment with a script reads ${JSON.stringify(shown)}`);
+  }
+  await page.waitForTimeout(500);
+  await inert("a comment with a script");
+
+  // 6. A form without its token.
+  const before = await page.$$eval("#comments .comment", (cs) => cs.length);
+  await page.evaluate(() => document.querySelector("#comment-form input[name=csrf]").remove());
+  await page.fill("#comment-form input[name=name]", "Eve");
+  await page.fill("#comment-form textarea[name=text]", "No token");
+  const forged = await submits(page, "#comment-form button[type=submit]", "the refusal");
+  const refusal = await page.evaluate(() => document.getElementById("refused")?.textContent ?? "");
+  if (forged !== 403 || !refusal.includes("expired")) {
+    throw new Error(`a comment without its token got ${forged}: ${JSON.stringify(refusal)}`);
+  }
+  clean("a comment without its token");
+  await hydratesInPlace(page, at("/posts/1"), "Hello from a Nova server");
+  const after = await page.$$eval("#comments .comment", (cs) => cs.length);
+  if (after !== before) throw new Error(`the refused comment was added: ${before} comments became ${after}`);
+
+  // 7. The admin: log in, then create, edit and delete a post.
+  await hydratesInPlace(page, at("/login"), "Log in");
+  await page.fill("#login-form input[name=name]", "admin");
+  await page.fill("#login-form input[name=password]", PASSWORD);
+  const logged = await submits(page, "#login-form button[type=submit]", "the admin's page");
+  if (logged !== 200 || new URL(page.url()).pathname !== "/admin") {
+    throw new Error(`the login led to ${page.url()} with ${logged}`);
+  }
+  for (const [path, title] of [
+    ["/admin", "Admin"],
+    ["/admin/new", "A new post"],
+    ["/admin/posts/2/edit", "Edit a post"],
+  ]) {
+    const got = await hydratesInPlace(page, at(path), title);
+    if (got.status !== 200) throw new Error(`${path} answered ${got.status}`);
+    kept.push(`${path} ${got.count}`);
+    clean(path);
+  }
+  await hydratesInPlace(page, at("/admin"), "Admin");
+  await submits(page, "#new-post", "the editor");
+  const title = `A script: ${XSS}`;
+  await page.fill("#editor input[name=title]", title);
+  await page.fill("#editor textarea[name=body]", "Its first paragraph.\n\nIts second.");
+  await submits(page, "#editor button[type=submit]", "the admin's page after a new post");
+  const listed = await page.$$eval(".row > a:first-child", (as) => as.map((a) => ({ text: a.textContent, href: a.getAttribute("href") })));
+  const made = listed.find((l) => l.text === title);
+  if (!made) throw new Error(`the admin's list reads ${JSON.stringify(listed)}`);
+  await inert("the admin's list");
+  const id = made.href.split("/").pop();
+  await hydratesInPlace(page, at("/"), "Posts");
+  const cards = await page.$$eval(".card h2", (hs) => hs.map((h) => h.textContent));
+  if (cards[0] !== title) throw new Error(`the home page's first post is ${JSON.stringify(cards[0])}`);
+  await inert("the home page");
+  await hydratesInPlace(page, at(made.href), title);
+  await inert("the new post's page");
+
+  await hydratesInPlace(page, at(`/admin/posts/${id}/edit`), "Edit a post");
+  await page.fill("#editor input[name=title]", "An edited post");
+  await submits(page, "#editor button[type=submit]", "the admin's page after an edit");
+  const edited = await page.$$eval(".row > a:first-child", (as) => as.map((a) => a.textContent));
+  if (!edited.includes("An edited post") || edited.includes(title)) {
+    throw new Error(`after the edit the admin's list reads ${JSON.stringify(edited)}`);
+  }
+  await hydratesInPlace(page, at(made.href), "An edited post");
+
+  await hydratesInPlace(page, at("/admin"), "Admin");
+  await submits(page, `form[action="/admin/posts/${id}/delete"] button[type=submit]`, "the admin's page after a delete");
+  const remaining = await page.$$eval(".row > a:first-child", (as) => as.map((a) => a.textContent));
+  if (remaining.includes("An edited post")) throw new Error(`the deleted post is still listed: ${remaining}`);
+  expected.add(at(made.href));
+  const deleted = await hydratesInPlace(page, at(made.href), "Not found");
+  if (deleted.status !== 404) throw new Error(`the deleted post's page answered ${deleted.status}`);
+  clean("the admin's pages");
+
+  // The client's size, printed and not limited, measured as the size test
+  // measures 06's: each file gzipped at the best level.
+  let gzipped = 0;
+  for (const name of ["app.wasm", "app.js"]) {
+    gzipped += gzipSync(await readFile(join(dir, "dist", name)), { level: 9 }).length;
+  }
+
+  console.log(
+    `blog: every page hydrates in place (nodes kept: ${kept.join(", ")}), the comment form's button is disabled while a field is blank and the comment posts, a comment that fails a check gets 400 at /posts/1/comments, a comment holding a NUL comes back without it, a script in a comment and in a title stays text, a form without its token gets 403, and the admin logs in and creates, edits and deletes a post; the client is ${gzipped} bytes gzipped`,
+  );
+} catch (e) {
+  console.error(`the server's stderr:\n${server.stderr()}`);
+  throw e;
+} finally {
+  await browser?.close();
+  server.kill();
+}
````


### Task 8: The records

Spec §9; decisions 14 to 31 and 37 to 44, which ADR 0046 records, with the
user's twelve decisions and the spec's thirteen.

**Files:**
- Create: `docs/adr/0046-the-blog-and-what-std-gains-for-it.md`
- Modify:
  - `nova-spec/00-MASTER-SPEC.md` (Phase 4's gate), `20-STDLIB.md` (§1,
    §5, §6, §8, §19), `30-FRONTEND.md` (§7.4), `60-EXAMPLES.md` (§7, §10)
  - `docs/phase-4-plan.md` (4.7's entry), `CHANGELOG.md`,
    `ARCHITECTURE.md` (`nova-std`'s row)
  - `examples/05-json-api/README.md` (what `Server` routes)

- [ ] **Step 1: The records.** `patch_block t8-code`. Each dated note reads
  "Amended 2026-10-21 (branch `claude/loving-archimedes-frtf5q-47`)".
- [ ] **Step 2: Read them against the code.** Each claim a record makes is
  one this plan's code makes true, and each departure ADR 0046 names is one
  of this plan's decisions:
  - the user's decisions and the spec's §12, against decisions 1 to 13;
  - routes, `405` and the readers, against `std/http`'s `path_pieces`,
    `percent_decoded`, `dispatch`, `params_of`, `form_fields`, `form` and
    `cookies`; cookies and the answers, against `Cookie`, `cookie_line` and
    its rules, `with_cookie`, `html`, `redirect` and the reason phrases;
  - `Dist`, against `open`, `file`, `respond` and `serve`;
  - `rename`, PBKDF2 and hex, against `std/fs`, `std/crypto`,
    `crypto.rs`'s bounds and `std/bytes`; PBKDF2's times, against part A's
    (Checked on);
  - the blog, against `server.nova`'s `checked`, `cookie_for`, `log_in`,
    `save`, `set_password` and `server`, `auth.nova`'s `Sessions`, its
    `start` and `kept`, `no_user_rounds`, `password_ok` and `load_key`,
    `store.nova`'s `Users`, `load_users`, `save_users` and `saved`,
    `data/seed.json`, `forms.nova`'s `sized`,
    `cleaned` and `submit`, `app.nova`'s `Data` and `routes`, `std/ui`'s
    `&#13;` for a lone CR, `Dist`'s doc tests, and `.gitignore`; the
    sessions' times before and after decision 38, the doubling's scan and
    a session's size, against the checks' and the fixes' measurements
    (Checked on);
  - the departures from `60-EXAMPLES.md` §7 and `20-STDLIB.md` §6, the
    consequences and what is left out, against spec §10 and §11 and the
    code; `05-json-api`'s README, against its `main.nova`, which still
    routes by hand over `read_request`;
  - the figures, against the runs: 21,070 bytes for `06-counter-spa`,
    which Task 9 measures by `flate2` at level 9, as the test does; 49,472
    bytes for the blog's client, which Task 7's line printed; 21 tests in
    the blog's `nova test`; and "The tests share one build", against
    `blog_tests.rs`: thirteen tests, nine setting the admin's password at
    1,000 rounds, the timing test another user's at 1,000 and the admin's
    at 100,000, eleven starting a server, one of them posting comments
    while `--set-password` hashes at 600,000 and another running two
    `--set-password` that overlap, and the server's refusal checked on a
    bad key file and a bad `users.json` (decision 31); and, in ADR 0046
    and the README, what two `--set-password` runs at once can lose,
    against `set_password`'s order (decision 37).

  A record that says what the code does not is fixed here, and the ledger
  says so.
- [ ] **Step 3: Run.** `cargo test -p nova-std`. Expected: `2 passed`. Then
  `cargo build --locked --workspace`. Expected: it ends with `Finished`.
- [ ] **Step 4: The checks,** as the conventions list them.
- [ ] **Step 5: Commit** `docs: ADR 0046, and the notes for 4.7`.

**Patch `t8-code`:**

<!-- patch t8-code -->

````diff
diff --git a/ARCHITECTURE.md b/ARCHITECTURE.md
index cff7b021808611e4868a9e8b5e04b8e495479293..cc84a691f1dee52b4ca68a3946cd6bec5fdd4747 100644
--- a/ARCHITECTURE.md
+++ b/ARCHITECTURE.md
@@ -47,7 +47,7 @@ nova-mir            → Mid-level IR
 | `nova-codegen-llvm` | Optimized release backend |
 | `nova-codegen-wasm` | WebAssembly backend: MIR to one module with `wasm-encoder`, structured control flow, and the JS shim (ADR 0039) |
 | `nova-runtime` | GC, async runtime, panic handling |
-| `nova-std` (in `std/`) | The standard library's `.nova` sources, embedded for the compiler; `std/rt` is a WASM build's runtime, in Nova: its allocator, collector and executor (ADRs 0039, 0040); `std/dom`, a WASM build's page, which a program imports (ADR 0041); `std/ui` and `std/ui/html`, reactivity and views, which a program imports, and `std/ui`'s backend, one source per target (ADR 0042); `std/ui/router`, routing by the URL's hash or path, which a program imports (ADR 0044); `std/ui/ssr`, server-side rendering and SSG, which a native program imports (ADR 0045) |
+| `nova-std` (in `std/`) | The standard library's `.nova` sources, embedded for the compiler; `std/rt` is a WASM build's runtime, in Nova: its allocator, collector and executor (ADRs 0039, 0040); `std/dom`, a WASM build's page, which a program imports (ADR 0041); `std/ui` and `std/ui/html`, reactivity and views, which a program imports, and `std/ui`'s backend, one source per target (ADR 0042); `std/ui/router`, routing by the URL's hash or path, which a program imports (ADR 0044); `std/ui/ssr`, server-side rendering and SSG, which a native program imports (ADR 0045), and its `Dist`, the built site a server written by hand serves beside routes of its own (ADR 0046) |
 | `nova-fmt` | The formatter: prints the AST in one fixed layout, keeps every comment, and refuses output that would change the program (ADR 0028); organize imports' block |
 | `nova-lsp` | The language server, `nova lsp`: diagnostics, completion, formatting, hover, definition, references, rename, code actions and semantic tokens over `nova_driver::analyze` (ADR 0029, ADR 0032, ADR 0033) |
 | `nova-test` | A stub, `publish = false`; the test runner is `nova-cli`'s `cmd/test.rs` |
diff --git a/CHANGELOG.md b/CHANGELOG.md
index c09c0e37e07334069faa7a1ae0637e5a92550dc2..658833edeb3fc6ed463c44654cda199c69502050 100644
--- a/CHANGELOG.md
+++ b/CHANGELOG.md
@@ -116,6 +116,40 @@ Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
 - **A CI test of SSR and SSG in Chromium,** `tests/browser/ssr.mjs`: a
   server-rendered page and a statically generated one hydrate without
   making their nodes again.
+- **Routes by method in `std/http`:** `post`, `put`, `delete` and
+  `route(method, pattern, handler)` beside `get`, with patterns whose
+  `:name` pieces the handler finds in `req.params`, as in
+  `.get("/posts/:id", |req: Request| …)`. A path is matched without its
+  query, decoded, as `std/ui/router` reads one. A request whose path a
+  route matches with another method gets `405 Method Not Allowed`, with an
+  `allow` header (ADR 0046).
+- **`Request::query()`, `form()` and `cookies()`** read a request's query,
+  the fields of a form a browser posted, and its cookies, by name.
+- **Cookies and redirects:** `Cookie::new(name, value)`, `HttpOnly` and
+  `SameSite=Lax` unless changed, and `Cookie::expired(name)`;
+  `Response::with_cookie(c)`, one `set-cookie` header per cookie;
+  `Response::html(status, s)`; and `Response::redirect(location)`, a
+  `303 See Other`.
+- **`Dist` in `std/ui/ssr`:** the built site, for a server written by hand
+  that serves it beside routes of its own. `Dist::open(dir)` reads it,
+  `file(req)` answers a request for one of its files, and
+  `respond(status, page)` answers with a page (ADR 0046).
+- **`std/fs::rename`,** which replaces the file at its destination, on
+  Windows too.
+- **`pbkdf2_sha256` and `pbkdf2_sha256_verify`** in `std/crypto`, for
+  storing and checking a password, through `ring`, already a dependency.
+- **`Bytes::to_hex` and `bytes_from_hex`,** on both targets.
+- **`examples/07-fullstack-blog`,** a blog that a visitor reads and
+  comments on, and that its admin writes: a server on `std/http` that
+  renders every page, and a client that hydrates it, with plain HTML forms,
+  signed session cookies, CSRF tokens and PBKDF2 password hashes, and its
+  posts in a JSON file. `nova build --ssr` builds it, and
+  `fullstack-blog-server --set-password <name>` sets its admin's password
+  (ADR 0046).
+- **A CI test of the blog in Chromium,** `tests/browser/blog.mjs`: every
+  page hydrates in place, the admin creates, edits and deletes a post, a
+  script in a title or a comment stays text, and a form without its CSRF
+  token is refused.
 
 ### Changed
 
@@ -130,6 +164,17 @@ Nova uses [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
   `view {` where an expression starts begins a view (ADR 0043).
 - **`examples/06-counter-spa`'s README runs it with `nova dev`**, where it
   built it and served `dist/` with another server.
+- **`Request` has a `params` field and `Response` a `cookies` field,** so
+  a `Request` or `Response` literal must name it (ADR 0046).
+- **`Server::get` takes a pattern, and the query no longer takes part:**
+  `/add` now matches `/add?x=1` and `/add/`, and `/posts/:id` matches
+  `/posts/7`.
+- **A request to a path a route serves with another method gets `405`,**
+  with an `allow` header, where it got `404`, `HEAD` included; and a `GET`
+  of a path only a route of another method matches no longer reaches the
+  `fallback`.
+- **The status line names 301, 302, 303, 401 and 403** by their reason
+  phrases, `303 See Other` where it was `303 Redirection`.
 
 ## [0.3.2] - 2026-10-14
 
diff --git a/docs/adr/0046-the-blog-and-what-std-gains-for-it.md b/docs/adr/0046-the-blog-and-what-std-gains-for-it.md
new file mode 100644
index 0000000000000000000000000000000000000000..ab430c3f523dfbbf1e5c86e50b702277880999d6
--- /dev/null
+++ b/docs/adr/0046-the-blog-and-what-std-gains-for-it.md
@@ -0,0 +1,457 @@
+# ADR 0046 — The blog, and what std gains for it
+
+## Status
+
+Accepted, 2026-10-21 (Phase 4.7, branch
+`claude/loving-archimedes-frtf5q-47`; spec
+`docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md`).
+std grown for the blog; plain HTML forms answered with `303`, and the whole
+page hydrated; PBKDF2 natively through `ring`; the admin's account made by
+the server; comments from anyone; sessions as signed random ids in a table
+in memory; the JSON file written whole, then renamed; a flat `src/`; the
+tests and the gate; plain text bodies; `nova dev` unchanged; and the name
+`fullstack-blog` are the user's decisions of 2026-10-21 (decisions 1 to 12
+below). The spec settled the rest (its §12). What 4.7 built beyond the
+spec, or otherwise than it, is under "What departs from the spec, or fills
+it in".
+
+## Context
+
+After 4.6 a program could render its pages natively and hydrate them in the
+browser, but its server answered `GET`s alone. The phase plan's decision 11
+has 4.7 build `examples/07-fullstack-blog` as `60-EXAMPLES.md` §7 describes
+it, with a JSON file in place of SQLite (ADR 0038): its server a Nova
+program on `std/http`; sessions in cookies signed with `hmac_sha256`,
+`HttpOnly` and `SameSite`; a CSRF token in every form; text from users
+escaped by both renderers by construction; and passwords hashed as 4.7's
+spec decides, "for example PBKDF2 over `hmac_sha256`, written in Nova". Its
+4.7 gate, in the CI job through Playwright: every page works; the admin's
+full CRUD works; a script put into a title or a comment stays inert text;
+and a form without its CSRF token is refused. ADR 0045 left to 4.7 `POST`,
+cookies and path parameters in `std/http`; loading a page's data per
+request, since `render` is synchronous; whether the comments need more than
+whole-page hydration; and, with the backlog, a development mode for SSR.
+The repository 4.7 starts from (spec §2, read at `9ad78cd`) has:
+- a `Server` that serves `GET` alone: `get(path, h)` matches the whole
+  target, query included, so `/add?x=1` misses `/add`; `fallback` answers
+  any other `GET`, and every other method gets `404`; and a handler is
+  `fn(Request) -> Response`, which cannot await;
+- a `Request` with nothing decoded: no reader for a query, a form, a
+  cookie or a parameter;
+- a `Response` whose headers are a `Map`, so a second `set-cookie`
+  replaces the first; no builder for HTML or a redirect; and `303` written
+  `303 Redirection`;
+- `ui::ssr::serve`, which builds its own `Server` with one fallback and no
+  way to add a route, whose `Site.render` sees the location alone, and
+  whose reading of the template and serving of the manifest's files are
+  private; `page`, `page_with` and `document` are public;
+- no `std/fs::rename`, and `std/fs` functions that are `async fn`s which
+  never suspend: a handler cannot await them, but can `spawn` them, and the
+  spawned writes run after the response, in order;
+- `hmac_sha256`, its constant-time verify and `random_bytes` in
+  `std/crypto`, but no PBKDF2: one written in Nova matched RFC 7914's
+  vectors but took about 0.84 s per 100,000 rounds, during which the
+  single-threaded server answers nothing; and no hex or base64;
+- `hydrate_with`, which reads one value of one type for the whole app,
+  once, so a client-side link to another route would render it with the
+  first page's data;
+- `std/ui`, which escapes every text and attribute value and refuses
+  `innerHTML`, `outerHTML` and `srcdoc`, but not a `javascript:` URL in
+  `href`;
+- a nested module path, `import routes::home`, which is E0900.
+
+## Decision
+
+### The user's decisions
+
+1. **std grows what the blog needs.** `std/http` gains routes by method,
+   path parameters, readers for a form and for cookies, several
+   `Set-Cookie` headers on one response, and responses for HTML and for a
+   redirect. `ui::ssr` gives a program the parts `serve` is made of, so a
+   server can put its own routes beside the built site, and each of its
+   pages sees the whole `Request`.
+2. **Forms are plain HTML forms, and every `POST` that succeeds is
+   answered with `303`** (post, redirect, get). Every page works without
+   JavaScript. Hydration adds live checks: a blank or too long field is
+   reported as it is typed, and the submit button is disabled until the
+   form is valid. The whole page hydrates; there are no islands, and
+   `std/ui` gains nothing.
+3. **Passwords are hashed with PBKDF2-HMAC-SHA256,** which `std/crypto`
+   gains natively through `ring`, already in `Cargo.lock`. It runs OWASP's
+   600,000 rounds unless told otherwise, and stores the count with each
+   hash.
+4. **The admin's account is made by the server itself:**
+   `fullstack-blog-server --set-password <name>` reads the password from
+   standard input and writes its hash into the data directory, in
+   `users.json` (below). No password is known from the repository; the
+   tests and the README run the command.
+5. **Anyone may comment,** with a name and a text. Every visitor gets a
+   signed session cookie, logged in or not, which the CSRF token is bound
+   to. The admin can delete a comment.
+6. **A session is a random id, signed, in a table in memory.** The cookie
+   carries the id and its `hmac_sha256` tag, `HttpOnly` and
+   `SameSite=Lax`. The server keeps who each id belongs to and when it
+   ends. Logging out ends it; a restart ends every session. The signing
+   key is made at random the first time the server starts, and kept in the
+   data directory.
+7. **The JSON file is written whole, then renamed into place,** so a crash
+   never leaves half a file: `std/fs` gains `rename`. The seed data is in
+   the repository; the file the server writes is ignored by git.
+8. **`src/` is flat.** A nested module path is E0900, so the modules of
+   `60-EXAMPLES.md` §7's tree sit side by side in `src/`.
+9. **The tests:** `crates/nova-cli/tests/blog_tests.rs` builds 07 with
+   `--ssr` on all three systems and drives its server over HTTP;
+   `nova test` runs the server's own modules' tests, through the package's
+   `src/lib.nova`; and the gate runs in CI's `browser` job through
+   Playwright, in Chromium (`tests/browser/blog.mjs`).
+10. **A post's body is plain text.** A blank line separates paragraphs, and
+    each paragraph is a `<p>` whose text is escaped.
+11. **`nova dev` does not change in 4.7;** a development mode for SSR goes
+    to the backlog. The README builds with `nova build --ssr` and starts
+    the server.
+12. **The package is `fullstack-blog`,** so its server is
+    `target/<profile>/fullstack-blog-server`.
+
+### Decided in the spec (§12)
+
+1. **A route's pattern is matched without the query, piece by piece,
+   decoded, by the router's rules,** and `get` takes patterns too. A
+   pattern's piece is literal, or `:name`, which matches any one piece and
+   binds it; the request's path, its target before any `?`, is split at
+   `/`, its empty pieces dropped and each percent-decoded, as
+   `std/ui/router` splits a route's path, so `/admin/` matches `/admin`.
+   The alternative, a second kind of route, would have left `get` the only
+   one that sees the query.
+2. **`405` with `allow`** when a path's routes have other methods, before
+   the fallback. `dispatch` tries the routes in the order they were added;
+   the first whose method and pattern match answers; otherwise a route
+   whose pattern matches gives `405 Method Not Allowed`, its `allow`
+   header naming the methods of the routes that match, each once, in the
+   order they were added; otherwise a `GET` goes to the `fallback`, when
+   there is one; otherwise `404`. `HEAD` gets no special treatment.
+3. **`Request.params`,** as `20-STDLIB.md` §6 shows it,
+   `req.params.get("id")`, rather than a handler with a second argument.
+   It is empty in a request `parse_request_head` or `read_request` returns.
+4. **`query` and `form` share one decoder,**
+   `application/x-www-form-urlencoded`'s: pairs split at `&`, an empty one
+   skipped, each split at its first `=`, `+` a space, `%XX` decoded as
+   UTF-8, and the last of two names winning, as `std/ui/router`'s
+   `use_query` does. `form` reads the body only when the `content-type`
+   header's media type is that one and the body is UTF-8. `cookies` reads
+   the `cookie` header, and keeps the first of two names, as a browser
+   sends the most specific first; its values are not decoded.
+5. **`Response.cookies`,** a field of its own, which `with_cookie` adds to
+   and `to_bytes` writes, one `set-cookie` line each, after the other
+   headers, rather than a header map that holds several values.
+6. **`Cookie::new`'s defaults are the safe ones:** path `/`, no `Max-Age`,
+   `HttpOnly` and `SameSite=Lax`, and not `Secure`. `Cookie::expired(name)`
+   has the value `""` and `Max-Age=0`.
+7. **`ui::ssr::Dist`,** the parts of `serve`, rather than a `Site` whose
+   `render` takes a `Request`; `serve` keeps its shape for programs that
+   need no more. `Dist::open(dir)` reads the template and the manifest with
+   every check and error `serve` made; `file(req)` answers a request for a
+   file the manifest lists, or returns `None`; `respond(status, page)`
+   returns `document(template, page)` as HTML with `status`. `serve` is
+   rebuilt on `Dist`, and its tests are unchanged.
+8. **Hex in `std/bytes`,** for both targets: `Bytes::to_hex`, lower case,
+   and `bytes_from_hex`, which takes either case. No base64.
+9. **The CSRF token is derived from the session,**
+   `hmac_sha256(key, "csrf:" + id)` in hex, so there is nothing more to
+   store. Each `POST` handler checks it with `hmac_sha256_verify` before it
+   reads anything else, and answers `403` when it is missing or wrong.
+10. **A failed form is rendered again with `400`;** only success
+    redirects.
+11. **A visitor's session lasts a day and the admin's eight hours.**
+    Ended sessions are dropped from the table (when, below); logging in
+    ends the old session and starts a new one, so an id set before the
+    login is worth nothing after it.
+12. **A login with an unknown name costs as much as one with a known
+    name:** it runs the same verify against a fixed salt, and says the
+    same, wrong name or password.
+13. **The forms' checks start after `on_mount`,** so the server's markup is
+    the same with or without JavaScript, and hydration finds what it
+    expects.
+
+## What departs from the spec, or fills it in
+
+- **Percent-decoding is the router's, whole or not at all.** A path's
+  piece, or a query's or form's name or value, is decoded as
+  `std/ui/router`'s `decoded` decodes it, and kept as written when it
+  holds a `%` not followed by two hex digits or its bytes are not UTF-8,
+  but that a query's or form's `+` is still a space: `%zz%41` stays
+  `%zz%41`, where spec §3.2 had only the bad `%` stay as it is. A server's
+  route and a client's so agree on every path.
+- **`pbkdf2_sha256` panics for a length above 65,536** too, the cap
+  `random_bytes` has, where spec §5 had it panic below 1 only. The length
+  is checked before the key is allocated, so no length a program passes
+  can ask for an allocation the process cannot make, or reach the length
+  past which `ring::pbkdf2::derive` unwraps an error. The panics read
+  `<function>: iterations must be from 1 to 4294967295, not <n>` and
+  `pbkdf2_sha256: length must be from 1 to 65536, not <n>`.
+- **`pbkdf2_sha256_verify` takes the start of the right key as the key.**
+  It derives a key of `expected`'s own length, as `ring::pbkdf2::verify`
+  does, and PBKDF2's shorter output is the start of its longer one; an
+  empty `expected` gives `false`. Its doc tells a caller to keep and check
+  the whole key and its length, and the blog refuses a stored hash that is
+  not 32 bytes.
+- **Two cookie rules beyond the spec's:** a `same_site` must be empty or
+  an HTTP token, so it cannot carry a `;` and an attribute of its own; and
+  a value in double quotes, which RFC 6265 allows, is refused, keeping one
+  rule for a value: RFC 6265's cookie-octets, so no space, `"`, `,`, `;`,
+  `\`, control character or anything outside ASCII. A name must be a token,
+  and a path any ASCII character but a control character or `;`. A cookie
+  that breaks a rule is left out of the response, without an error.
+- **The `405`'s body is `method not allowed`,** as `text/plain`, which the
+  spec did not name; and **`Dist::file` does not look at the method,**
+  since `Server` asks its fallback only for a `GET`.
+- **The blog's explicit `GET` routes answer `404` for its paths that have
+  only a `POST` route** (`/posts/:id/comments`, `/logout`, `/admin/posts`,
+  `/admin/posts/:id`, `/admin/posts/:id/delete` and
+  `/admin/comments/:id/delete`), with the not-found page, as spec §6.2's
+  table asks of any other `GET`; without them `std/http` would answer
+  `405`.
+- **The example's `target/` is in `.gitignore`,** which a build in place
+  makes, beside the spec's `dist/`, `data/blog.json` and `data/secret`;
+  and so are `data/blog.json.tmp`, which a write leaves should it stop
+  before the rename, and `data/users.json` and `data/users.json.tmp`
+  (below).
+- **`Data` gains `missing` and `refused`,** and `routes()` maps the paths
+  forms are posted to. `missing` renders the not-found page, for a path
+  with no page and a post that does not exist; `refused` renders a refused
+  form's reason in place of the route's page, with `403`. A form that fails
+  a check is rendered again at the path it was posted to, which is then the
+  browser's location, so `routes()` gives `/posts/:id/comments`,
+  `/admin/posts` and `/admin/posts/:id` their pages, the post's and the
+  editor's, on both sides. A form's values and messages are `Data`'s
+  `draft`, whose `failed` shows the messages from the start and whose
+  `error` holds a message for the whole form, such as a wrong password.
+- **The submit button is two buttons, each under a `show`.** `std/ui`
+  writes every attribute it is given and never removes one, and `disabled`
+  disables a button by being there at all, whatever its value, so one
+  button cannot be both. The server's HTML holds the enabled one, which a
+  page without JavaScript keeps; the client swaps them as the form's
+  checks change.
+- **Stored text is trimmed, holds no NUL, and has its carriage return and
+  line feed pairs made line feeds** (`forms.nova`'s `cleaned`). A
+  browser's parser drops a line feed that begins a `textarea`'s text, so a
+  body stored with one would come back to the editor as another text, and
+  mismatch on every hydration (ADR 0045's consequences); it drops a NUL
+  from a page's text, and makes it U+FFFD in a `textarea` or an attribute,
+  so one comment holding a NUL broke the hydration of its post's page for
+  every visitor, and `std/ui` cannot write a NUL that the parser keeps; and
+  a browser sends a `textarea`'s lines ending in CRLF, where the field's
+  value holds LF. The checks of a post's and a comment's fields measure a
+  value as `cleaned` leaves it, on both sides, so both measure the same
+  text. In Chromium, no other character
+  below U+0020, nor U+007F, broke hydration: not in a post's title and
+  body and a comment's name and text on the post's page, nor in a failed
+  comment's name and text, where they are an attribute's value and a
+  `textarea`'s text; the parser keeps them, form feed included, and
+  `std/ui` writes a lone carriage return as `&#13;`. The login's name is
+  trimmed, and its password is not.
+- **An unknown login name uses the largest iteration count of the users'
+  hashes** that PBKDF2 takes, or, when there is none, `--iterations`,
+  600,000 unless given, which the spec's "the same count" left open, so a
+  name no user has takes as long as the name of the user whose hash has
+  the most rounds. A user whose hash has fewer rounds than another's, such
+  as one set with a lower `--iterations`, still answers sooner, so the time
+  tells that name from one no user has; passwords all set with the same
+  count close it.
+- **`--set-password` refuses, and writes nothing:** with status 2, a name
+  that is blank or has spaces around it, before it reads the password; and
+  with status 1, a password that is not UTF-8, one shorter than eight
+  characters, and one that is blank once trimmed, which the login's check
+  refuses, so the user could never log in with it.
+- **The users have a file of their own,** `<data>/users.json`, where spec
+  §6.3 has the store hold them and §6.5 has `--set-password` write them
+  into `blog.json`. Each file has one writer. `--set-password` hashes the
+  password first, then reads `users.json`, no users when it does not
+  exist, sets the user, writes `users.json.tmp` and renames it over
+  `users.json`; it never reads or writes `blog.json`. The server reads
+  `users.json` once, when it starts, and refuses to start when it cannot
+  read users from it, as `--set-password` refuses then to write it; it
+  never writes it. The server writes `blog.json`, the posts, the comments
+  and the next ids, from its own copy, reading no file. `data/seed.json`
+  has no `users` key, and the store ignores one. Both checks found the one
+  file's two writers at fault. In the first, the server's next write put
+  its own copy of the users over a password `--set-password` had set while
+  it ran, so the old password stayed good, before a restart and after. The
+  fix, each server write keeping the users the file held then, which this
+  replaces, left two faults that the second check found: `--set-password`
+  wrote back the copy it had read before its hash, about two seconds at
+  600,000 rounds, erasing every change the server saved meanwhile, and
+  under steady writes its own user too while it exited 0 (26 of 40 runs at
+  1,000 rounds lost theirs in a probe); and each server write read the
+  file, so a missing `blog.json` wrote the seed's users, none, erasing the
+  admin, and an unreadable one stopped every write, after which a restart
+  refused to start. With one writer per file, neither writes over the
+  other's change: the second check's probes, run again on the new files,
+  lost no comment and no user, and the server wrote a missing `blog.json`,
+  or one holding bad JSON, afresh from its copy. The server still checks
+  logins against the users it read when it started, so the README says to
+  stop the server, run `--set-password`, then start it; a password set
+  while the server runs takes effect at its next start.
+- **Ended sessions are dropped once the table has doubled,** where spec
+  §6.4 drops them each time a session starts: when the table holds at
+  least 64 sessions, and at least twice as many as it kept after the ended
+  ones were last dropped (`Sessions.kept`), starting one drops them first.
+  The table so holds at most twice the sessions live at the last scan, or
+  64 if that is more. At least half the sessions each scan reads have
+  started since the last scan, so the scans cost the same per session on
+  average, however many the table holds; but the request that finds the
+  table doubled scans all of it, in time linear in its size: on a debug
+  runtime, the one that started the 16,385th session took 18.4 ms, and the
+  32,769th's 35.7 ms. Scanning at every start made the cost grow with the
+  table: a `GET /login` without a cookie, on a debug runtime, took 1.59 ms
+  with 200 sessions in the table, 4.91 with 2,200, 7.30 with 4,200, 10.08
+  with 6,200 and 12.78 with 8,200; dropping them once the table has
+  doubled, it took 1.52, 1.60, 1.66, 1.87 and 1.76 ms. A request with a
+  cookie took 1.72 ms in the first run and 1.87 in the second.
+- **`Dist::open`'s doc example is `ignore`,** where spec §7.2 asks a doc
+  test of each new public function: it reads a site `nova build --ssr`
+  wrote into `dist` and serves it, which a doc test has not. The doc tests
+  of `file` and `respond` call `Dist::open` on a site they write, so it is
+  exercised there.
+- **`auth.nova` imports `store`,** for `User`, where spec §6.1 names only
+  `server.nova` and `lib.nova` as importers of the server's own modules.
+  Both are server modules, and the client, `main.nova`, reaches neither.
+- **`lib.nova` is one-line wrappers,** such as `store_from`, `sessions`,
+  `user_with`, `logs_in` and `message_for`, since Nova has no re-exports:
+  `tests/*.nova` import the library as `fullstack_blog`, can name only what
+  it declares, and use what those functions return through its methods and
+  fields.
+- **The session cookie lasts as long as its session,** `Max-Age` 86,400 for
+  a visitor and 28,800 for the admin, so a browser drops it when the server
+  would; logging out sends `Cookie::expired("sid")`.
+- **A refusal's details:** a `POST` with no session, to an admin's path or
+  not, gets `403` with the expired-form page; one with a visitor's session
+  and its right token, to an admin's path, gets `403` with "Only the admin
+  can do that. Log in first."; a `403` page starts no session; and a
+  `POST` that names a post or a comment that does not exist, once its
+  token is checked, gets the not-found page with `404`.
+- **The tests share one build:** `blog_tests.rs` builds the example once
+  per run of the file, and each of its thirteen tests has a data
+  directory of its own. Nine set the admin's password at 1,000 rounds. The
+  test that times logins sets another user's at 1,000 first, then the
+  admin's at 100,000, so that the hashing is most of a login's time and a
+  name no user has must cost the admin's count, not the first user's; it
+  needs the median of five logins with a name no user has and of five
+  with a wrong password each to be at least half the other. Eleven start a
+  server of their own; one of them posts comments while `--set-password`
+  hashes at 600,000 rounds, and another first runs a `--set-password` at
+  1,000 rounds to its end while one at 600,000 hashes, and needs both
+  users in `users.json`. The server's refusal to start is checked on a key
+  file that does not hold 32 bytes in hex, and on a `users.json` that
+  does not hold users. The example's `nova test` runs 21 tests.
+
+## What departs from `60-EXAMPLES.md` §7 and `20-STDLIB.md` §6
+
+- **JSON files in place of SQLite,** by ADR 0038: `data/blog.json`,
+  written by the server, made from `data/seed.json`, which is in the
+  repository, the first time, and `data/users.json`, written by
+  `--set-password`; there is no `std/db`.
+- **A flat `src/`** (decision 8): the tree's `routes/` and `components/`
+  modules, `home`, `post`, `login`, `admin`, `header` and `post_card`, sit
+  in `src/`; `db.nova` is `store.nova`; `main.nova` is the client, and the
+  server's entry is `server.nova`; and `app.nova`, which both share,
+  `forms.nova`, the checks both run, and `lib.nova`, the tests' library,
+  are added. The project also has `index.html` and `data/seed.json`.
+- **The whole post page hydrates,** not only its comments (decision 2),
+  and so does every other page.
+- **Lighthouse is 4.8's,** with 06's; 4.7's gate is the rest of §7's.
+- **`20-STDLIB.md` §6's handlers stay synchronous:** a handler is
+  `fn(Request) -> Response`, since §6's `Handler`, an alias of
+  `async fn(Request) -> Response`, still does not parse (P0001), and the
+  routes return `Server`, not `Self`.
+- **`get` takes a pattern, as every route does, and ignores the query**
+  (§12, decision 1); `req.params.get("id")` returns an `Option<String>`.
+  `use_middleware`, the `Handler` and `Middleware` aliases, and the client
+  are still not shipped; `405` and `fallback` are not in §6.
+
+## Consequences
+
+- **The two new fields break `Request` and `Response` literals:** a
+  program that writes one must name `params` or `cookies`. No release has
+  either; in the repository, std, `examples/05-json-api` and
+  `tests/runtime` were changed for them.
+- **`get` no longer sees the query,** so a program that routed on it
+  changes behaviour; none in the repository did. `/add/` now matches `/add`
+  too.
+- **A request whose path a route matches with another method is `405`,**
+  where it was `404`, `HEAD` included, and a `GET` of a path that only a
+  `POST` route matches no longer reaches the fallback.
+  `examples/03-http-server` now answers `POST /` with `405` and
+  `allow: GET`.
+- **PBKDF2 holds the single-threaded server** for each login, with a
+  known name or not: every other request waits meanwhile. 600,000 rounds
+  took about 2.6 s with the runtime built for debugging (2,564 to 2,638 ms,
+  three runs), and a program built with `nova build --release`, which
+  links the same runtime, took as long; a runtime built for release is
+  expected to take about 0.3 s, as `ring` built optimized took 303 to
+  340 ms in a probe.
+- **A crash between a response and the write it spawned loses that
+  change.** The file is always whole, the old one or the new, but not
+  always the newest.
+- **A restart ends every session,** the admin's included, since the table
+  is in memory; the key in `data/secret` stays.
+- **Ended sessions stay in the table until it has doubled,** so it holds
+  at most twice the sessions live at the last scan, or 64 if that is
+  more: after a burst, the sessions that end stay until the table doubles
+  again. A client that keeps no cookie starts a session with each page it
+  asks for, and a session takes about 658 bytes, so one asking for pages
+  as fast as the server answers, 391 a second in a probe, would grow it by
+  about 22 GB in a day; and a steady stream of them keeps up to two days'
+  worth, the live day and as many ended, where spec risk 5 expected at
+  most a day of visitors.
+- **`--set-password` takes effect when the server next starts,** since the
+  server reads `users.json` only then; the README says to stop the server,
+  run it, then start the server. A server running meanwhile loses nothing
+  to it. Nor does a `--set-password` run that starts and ends while
+  another hashes: each hashes first and reads `users.json` only then, so
+  the slower one reads the faster one's user, as a test checks. Two runs
+  can still lose one's user when their files meet after both hashes: when
+  one reads `users.json` after the other has read it and before the other
+  has renamed its own over it, a window of milliseconds. The README says
+  to run one at a time.
+- **The key and the password hashes are written with the system's default
+  permissions,** since `std/fs` sets no mode; the README says to keep
+  `data/` private.
+- **The login's name is trimmed, not cleaned,** so a NUL in it reaches the
+  page of the failed login; only the visitor who sent it sees that page,
+  and Chromium kept its 26 nodes, with no mismatch.
+- **A field of only U+FEFF, U+2028 or U+2029 is not blank,** since `trim`
+  keeps them; both sides check it alike.
+- **The client is 49,472 bytes gzipped** when `blog.mjs` printed it, built
+  without `--release` and so without `wasm-opt`: just under the 50 KB the
+  phase plan's decision 12 sets for 06, and 07 has no limit. 4.8's
+  Lighthouse run has still to measure the page it loads.
+- **`std/ui` does not refuse a `javascript:` URL** in an `href`. The blog
+  never puts a visitor's text in one: a post's link is built from its id.
+- **A cookie that breaks a rule is dropped silently,** such as a value
+  with a space or a character outside ASCII; hex is safe.
+- **No crate is added,** and `Cargo.lock` does not change.
+
+## What is left out
+
+- Handlers that await, and async file access from a handler.
+- Nested module directories.
+- A development mode for SSR in `nova dev`, which goes to the backlog.
+- `Secure` cookies by default, HTTPS, and a limit on login attempts; the
+  README says what a public deployment adds.
+- More than one admin through the UI; Markdown; images; search; pages of
+  posts.
+- Lighthouse and the size checks, which are 4.8's.
+- Chunked bodies, `HEAD`, compression and environment variables.
+- Refusing `javascript:` URLs, preventing a submit, or `fetch` in shared
+  components, in `std/ui`.
+
+## References
+
+- `docs/phase-4-plan.md` §3, decisions 11 and 12, §4's 4.7 entry, and §5's
+  security rules.
+- The spec named above.
+- ADR 0038, the recorded scope; ADR 0044, the router whose rules a route
+  follows; ADR 0045, SSR and hydration; ADR 0022, the shutdown signals the
+  server inherits.
+- `nova-spec/60-EXAMPLES.md` §7, §9 and §10; `nova-spec/20-STDLIB.md` §5,
+  §6, §8 and §19; `nova-spec/30-FRONTEND.md` §7.4.
diff --git a/docs/phase-4-plan.md b/docs/phase-4-plan.md
index 1821c7cd3a5c667e02d18fac8a8cdf200a9ecda6..c01995825e39559c486cd4bed2b14a8ad3a81126 100644
--- a/docs/phase-4-plan.md
+++ b/docs/phase-4-plan.md
@@ -602,6 +602,31 @@ ADR 0045).
   - a script put into a title or a comment stays inert text;
   - a form without its CSRF token is refused.
 
+**Amended 2026-10-21 (branch `claude/loving-archimedes-frtf5q-47`):** by the
+user's decisions of 2026-10-21, std grows what the blog needs: `std/http`'s
+`Server` routes by method, with `:name` patterns (`req.params`) and `405`;
+`Request` reads a query, a form and cookies; `Response` sets cookies, with
+`Cookie` and `with_cookie`, and builds HTML and `303` redirects; `ui::ssr`
+gains `Dist`, the built site a server written by hand serves beside its
+own routes; `std/fs` gains `rename`; `std/crypto` gains `pbkdf2_sha256` and
+its verify, natively through `ring`, rather than written in Nova; and
+`std/bytes` gains hex. The blog's server, `fullstack-blog-server`, renders
+every page with `page_with` and the client hydrates it whole, with no
+islands; its forms are plain HTML forms, answered with `303` when they
+succeed and checked live once the page has hydrated; every visitor has a
+session, a signed random id in a table in memory, in a cookie `HttpOnly`
+and `SameSite=Lax`, from which each form's CSRF token is derived; anyone
+may comment; passwords are PBKDF2 hashes of 600,000 rounds, set by
+`--set-password` from standard input; the store is a JSON file written
+whole, then renamed into place; `src/` is flat; and `nova dev` does not
+change. The gate is `tests/browser/blog.mjs` in Chromium: every page
+hydrates in place, the comment form's checks and post work, the admin
+creates, edits and deletes a post through the UI, a script in a title or a
+comment stays text, and a form without its token gets `403`;
+`blog_tests.rs` drives the server over HTTP on all three systems
+(`docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md`;
+ADR 0046).
+
 ### 4.8 — The gate and `v0.4.0`
 - **The CI job's Lighthouse run** on both examples, and the size checks
   (decision 12).
diff --git a/examples/05-json-api/README.md b/examples/05-json-api/README.md
index 8f54e3c2dec95b510949cfcba5223050e231ce3e..a4cf595b1f6bc4a9101e2f86b24a7e5ea6ab53b2 100644
--- a/examples/05-json-api/README.md
+++ b/examples/05-json-api/README.md
@@ -5,9 +5,10 @@ A JSON API over `std/http`: list users, create one, fetch one by id.
 ## What this demonstrates
 
 - Routing without a router type: a `match` over `req.method` and a
-  `req.path.split("/")`. `std/http`'s `Server` (see `examples/03-http-server`)
-  routes exact paths and `GET` only, so this example's `POST /users` and its
-  `/users/:id` pattern still route by hand.
+  `req.path.split("/")`, over `read_request` rather than `std/http`'s
+  `Server`. `Server` now routes by method, with `:name` patterns (see
+  `examples/07-fullstack-blog`), but this example keeps the hand-written
+  routing it was written to show.
   **The split
   has to test the resource segment and not only the segment count** --
   `/users/1` splits to `["", "users", "1"]` and so does `/foo/1` to
diff --git a/nova-spec/00-MASTER-SPEC.md b/nova-spec/00-MASTER-SPEC.md
index 638c3d753e9ccf4424ce3d7586ff9043fa0ca96d..85a2d7d5af63e3ff781d883869fe8949ab15fd72 100644
--- a/nova-spec/00-MASTER-SPEC.md
+++ b/nova-spec/00-MASTER-SPEC.md
@@ -701,6 +701,14 @@ Implement std modules in order (each module is a doc in [20-STDLIB.md]):
 
 **Gate:** `examples/06-counter-spa` and `examples/07-fullstack-blog` work in browser, pass Lighthouse 95+.
 
+**Amended 2026-10-21 (branch `claude/loving-archimedes-frtf5q-47`):**
+`examples/07-fullstack-blog` is built in 4.7: a server on `std/http` that
+renders every page with `ui::ssr` beside the built site, and a client that
+hydrates it, with signed session cookies, CSRF tokens and PBKDF2 password
+hashes, by the user's decisions of 2026-10-21 (ADR 0046), and a JSON file in
+place of SQLite (ADR 0038). CI's `browser` job drives it in Chromium; the
+Lighthouse run, for both examples, is 4.8's.
+
 ---
 
 ### Phase 5 — Self-hosting (week 81–104)
diff --git a/nova-spec/20-STDLIB.md b/nova-spec/20-STDLIB.md
index b0bab0a270bc4f41e195a3d3b3a39b508b9dccd9..af9884ba995449d4591a549f744905c0a74cbdfa 100644
--- a/nova-spec/20-STDLIB.md
+++ b/nova-spec/20-STDLIB.md
@@ -77,6 +77,18 @@ static routes' pages. `std/ui` gains `render_hydratable`, `hydrate` and
 `docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
 §3 to §6 and §8 (ADR 0045).
 
+**Amended 2026-10-21 (branch `claude/loving-archimedes-frtf5q-47`):
+`std/ui/ssr`** gains `Dist`, the built site a server written by hand
+serves beside routes of its own: `Dist::open(dir)` reads
+`<dir>/index.html` and `<dir>/manifest.json` once, with every check and
+error `serve` makes on them; `file(req)` answers a request for a file the
+manifest lists, or returns `None` for the program to answer; and
+`respond(status, page)` answers with the page put into the template, as
+`document` puts it, with `status`. `serve` is built on it, and does what
+it did. They are listed in
+`docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md` §4
+(ADR 0046).
+
 ---
 
 ## 2. `std/core` — Foundational Types
@@ -467,6 +479,18 @@ impl Read for File { ... }
 impl Write for File { ... }
 ```
 
+**Amended 2026-10-21 (branch `claude/loving-archimedes-frtf5q-47`):**
+`pub async fn rename(from: String, to: String) -> Result<(), IoError>`
+exists, an `async fn` that never suspends, like its neighbours, over a
+`fs_rename` builtin and Rust's `std::fs::rename`. It renames the file
+`from` to `to`, replacing the file at `to` when there is one, on Windows as
+on Unix, where the replacement is atomic: a file written whole beside its
+target, then renamed over it, leaves the old file or the new one at `to`,
+never half of one. Nothing at `from`, or no directory to hold `to`, is
+`NotFound`
+(`docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md`
+§5; ADR 0046).
+
 ---
 
 ## 6. `std/http` (server + client)
@@ -549,6 +573,39 @@ and path parameters are still not shipped
 (`docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
 §8.3; ADR 0045).
 
+**Amended 2026-10-21 (branch `claude/loving-archimedes-frtf5q-47`):**
+`Server` routes by method: `get`, `post`, `put` and `delete`, each
+`(pattern, handler)`, and `route(method, pattern, handler)`, each returning
+a copy of the server with one more route. A pattern's pieces are literal, or
+`:name`, which matches any one piece; a request's path, its target before
+any `?`, is split at `/`, its empty pieces dropped and each percent-decoded,
+as `std/ui/router` reads a location, so `/posts/:id` matches `/posts/7`,
+`/posts/7/` and `/posts/7?x=1`. `get` takes a pattern too, and the query no
+longer takes part. `dispatch` tries the routes in the order they were added:
+the first whose method and pattern match answers, and its handler finds what
+the pattern took in `req.params`, a new field of `Request`, whose
+`get("id")` returns an `Option`; otherwise, when a route's pattern matches
+with another method, `405 Method Not Allowed`, with an `allow` header naming
+those routes' methods and the body `method not allowed`; otherwise a `GET`
+goes to the `fallback`, whose request has no `params`; otherwise `404`.
+`HEAD` is a method like any other. `Request::query()` and `form()` decode a
+query and a form's body as `application/x-www-form-urlencoded`, the last of
+two names winning, `form` only for that `content-type`; `cookies()` reads
+the `cookie` header, the first of two names winning. `Response` gains
+`cookies`, which `with_cookie(c)` adds to and `to_bytes` writes, one
+`set-cookie` line each, after the other headers. `Cookie::new(name, value)`
+has `Path=/`, `HttpOnly` and `SameSite=Lax`, and `Cookie::expired(name)`
+`Max-Age=0`; a cookie whose name is not a token, or whose value, path or
+`same_site` holds what a cookie cannot carry, is left out.
+`Response::html(status, s)` is `text`'s twin for HTML, and
+`Response::redirect(location)` is `303 See Other`. The reason phrases gain
+301, 302, 303, 401 and 403. A `Request` or `Response` literal must name the
+new field. A handler is still synchronous, `fn(Request) -> Response`;
+`use_middleware`, the `Handler` and `Middleware` aliases, and the client are
+still not shipped
+(`docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md` §3;
+ADR 0046).
+
 ```nova
 module std.http
 
@@ -1663,6 +1720,23 @@ degenerate range returning `min`.
 step toward them. §7's own dated amendment above records what position 12 does
 and does not close.
 
+**Amended 2026-10-21 (branch `claude/loving-archimedes-frtf5q-47`):**
+`pbkdf2_sha256(password, salt, iterations, length) -> Bytes` and
+`pbkdf2_sha256_verify(password, salt, iterations, expected) -> Bool` ship,
+where `password`, `salt` and `expected` are `Bytes`, and `iterations` and
+`length` `Int`s: PBKDF2-HMAC-SHA-256 (RFC 8018) over `ring::pbkdf2`, through
+two runtime intrinsics, so no crate is added. Each panics for an iteration
+count outside 1 to 4,294,967,295, and `pbkdf2_sha256` for a length outside 1
+to 65,536, `random_bytes`' cap. The verify derives a key of `expected`'s
+length and compares in constant time, a property inherited from `ring` and
+not demonstrated by a test, as `hmac_sha256_verify`'s is; so the start of
+the right key passes too, and a caller keeps and checks the whole key and
+its length. It runs on the calling task, and every other task waits: 600,000
+rounds took about 2.6 s with the runtime built for debugging, and `ring`
+built optimized took about 0.3 s
+(`docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md` §5;
+ADR 0046).
+
 ```nova
 module std.crypto
 
@@ -2888,3 +2962,13 @@ impl Eq for Bytes {
   `bytes_from_ints_rejects_a_value_above_the_range`,
   `bytes_from_ints_rejects_a_value_below_the_range` and
   `bytes_reserved_declaration_is_rejected`.
+
+**Amended 2026-10-21 (branch `claude/loving-archimedes-frtf5q-47`):**
+`Bytes::to_hex(self) -> String` writes bytes as hexadecimal, two lower-case
+digits each, and `bytes_from_hex(s: String) -> Option<Bytes>` reads them
+back, taking the digits in either case and returning `None` for an odd
+length or any other character. Both are written in Nova, with no builtin,
+on both targets; `tests/runtime/bytes_hex.nova` runs natively, under
+`NOVA_GC_STRESS=1`, and in the WASM gate
+(`docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md` §5;
+ADR 0046).
diff --git a/nova-spec/30-FRONTEND.md b/nova-spec/30-FRONTEND.md
index debe16a6feb1b427532b90f5e52f6e6d064b0feb..14bf823b0f49f58f53918ad1795c1941e74d7e59 100644
--- a/nova-spec/30-FRONTEND.md
+++ b/nova-spec/30-FRONTEND.md
@@ -410,6 +410,26 @@ for SSR; `HistoryRouter` is
 (`docs/superpowers/specs/2026-10-19-phase-4-6-ssr-hydration-and-ssg-design.md`
 §3 to §7; ADR 0045).
 
+**Amended 2026-10-21 (branch `claude/loving-archimedes-frtf5q-47`):** a
+server written by hand, as `examples/07-fullstack-blog`'s is, builds on
+`ui::ssr`'s parts rather than on `serve`. `Dist::open(dir)` reads the built
+`dist/`'s `index.html` and `manifest.json` once, with `serve`'s checks. The
+program's own `std/http` `Server` routes each page by method and pattern,
+renders it with `page_with(location, data, App)`, and answers with
+`dist.respond(status, page)`, which puts the page into the template as
+`document` does, and to which a route can add a cookie with `with_cookie`;
+it answers a form's `POST` as well, as the blog's does with a `303` redirect
+once the form succeeds. Its `fallback` asks `dist.file(req)` for a file the
+manifest lists before it answers with its own not-found page. So each page
+can choose its status, read the request's cookies and form, and carry the
+data it needs, which the client reads with `hydrate_with`. Since
+`hydrate_with` reads one value for the whole app, once, such a server's
+pages link to each other with plain anchors, each page arriving from the
+server with its own data, rather than with a router's `link`. `serve` is
+rebuilt on `Dist`, and does what it did
+(`docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md` §4
+and §6; ADR 0046).
+
 ---
 
 ## 8. Dev Server (`nova dev`)
diff --git a/nova-spec/60-EXAMPLES.md b/nova-spec/60-EXAMPLES.md
index 628a79fd6fff8ccd3e19947ef03294772aae5a75..b082d69dad7e6867ee4daafdaf86196e7835b90b 100644
--- a/nova-spec/60-EXAMPLES.md
+++ b/nova-spec/60-EXAMPLES.md
@@ -708,6 +708,36 @@ Features required:
 
 **Gate:** All pages work, Lighthouse Performance > 95, full CRUD via admin UI, no XSS via untrusted input.
 
+**Amended 2026-10-21 (branch `claude/loving-archimedes-frtf5q-47`):**
+`examples/07-fullstack-blog`, the package `fullstack-blog`, is built in 4.7
+and differs from the above where the user decided so (ADR 0046). Its posts
+and comments live in memory and are written to `data/blog.json`, a JSON
+file written whole and renamed into place, made from `data/seed.json` the
+first time, and its users are in `data/users.json`, which
+`fullstack-blog-server --set-password` writes the same way: not SQLite
+(ADR 0038). Its `src/` is flat, since a nested module path is E0900:
+`main.nova` is the client, built for WASM, which hydrates each page;
+`server.nova` is the server, a `std/http` `Server` beside the built site,
+which `nova build --ssr` builds into
+`target/<profile>/fullstack-blog-server`; `app.nova`, the pages `home`,
+`post`, `login` and `admin`, the components `header` and `post_card`, and
+`forms`, the checks, are shared; `store`, in place of `db.nova`, and `auth`
+are the server's; and `lib.nova` is the library its `tests/*.nova` reach
+the server's modules through. It also has `index.html` and
+`data/seed.json`, and its README follows §9. Every page hydrates whole, the
+post's included, and every form works without JavaScript; the login is
+checked in the client as it is typed and on the server against a
+PBKDF2-HMAC-SHA256 hash, which `fullstack-blog-server --set-password`
+writes; sessions are signed cookies, `HttpOnly` and `SameSite=Lax`; every
+form carries a CSRF token; and anyone may comment. CI's `browser` job
+drives it in Chromium (`tests/browser/blog.mjs`): every page hydrates in
+place, the admin creates, edits and deletes a post through its pages, a
+script in a title or a comment stays text, and a form without its token is
+refused with `403`. Lighthouse is 4.8's. Its client, built without
+`--release`, is 49,472 bytes gzipped
+(`docs/superpowers/specs/2026-10-21-phase-4-7-fullstack-blog-design.md`
+§6; ADR 0046).
+
 ---
 
 ## 8. Additional Examples (post-Phase 6)
@@ -851,3 +881,16 @@ gate's second clause. The CI job above does not exist in
 live in `crates/nova-cli/tests/run_tests.rs` too, for the first of the two
 reasons above. Whether any example meets this section is a check, not this
 note: `ls -d examples/*/tests/` against the CI job above.
+
+**Amended 2026-10-21 (branch `claude/loving-archimedes-frtf5q-47`):** the
+first of the 2026-10-04 note's two reasons is stale: since Phase 3.3a,
+`nova test` in a project runs every top-level `tests/*.nova` as well as the
+tests of the `src/` files its program and its library reach (`40-TOOLING.md`
+§7). `examples/07-fullstack-blog` is the first example with a `tests/`
+folder: its four files hold 21 tests, which reach the server's modules
+through its `src/lib.nova`, and `run_tests.rs`'s
+`the_blog_example_passes_its_own_tests` runs `nova test` in a copy of it,
+in the ordinary `cargo test` step on all three operating systems. Its server
+is driven end to end over HTTP by `crates/nova-cli/tests/blog_tests.rs`,
+in the same step, and in Chromium by CI's `browser` job. The CI job above
+still does not exist (ADR 0046).
````


### Task 9: Verification, the mutants, the review, the PR

- [ ] **Step 1: The full checks.** First, `df -h /` must show at least 8G
  free; if not, remove scratch directories of earlier sub-phases.

  ```sh
  cargo fmt --all -- --check
  cargo clippy --locked --all-targets --all-features -- -D warnings
  cargo run --locked -q -p nova-cli -- fmt --check std examples tests/wasm tests/dom tests/browser/router-app tests/browser/ssr-app tests/ssr
  cargo run --locked -q -p nova-cli -- fmt --check tests/runtime/bytes_hex.nova tests/runtime/fs_rename.nova tests/runtime/crypto_pbkdf2.nova tests/runtime/crypto_pbkdf2_refused.nova tests/runtime/http_routes.nova tests/runtime/http_request_readers.nova tests/runtime/http_responses.nova tests/runtime/http_server_wire.nova
  cargo build --locked --workspace
  NOVA_REQUIRE_NODE=1 cargo test --locked --workspace --no-fail-fast > "$P/full-47x.txt" 2>&1
  grep '^test result' "$P/full-47x.txt" | awk '{p += $4; f += $6; i += $8} END {print p, f, i}'
  grep ' FAILED$' "$P/full-47x.txt"
  W the_counter_spa_is_under_50_kb_gzipped --nocapture
  ```

  Expected:
  - both formatters print nothing, and clippy ends with `Finished`, with
    no warning or error;
  - the suite, in about 5 minutes: `2419 1 10`, against the baseline's
    `2392 1 10`: 27 new tests:
    - 10 in `run_tests`: `fs_rename_run`, `bytes_hex_run`,
      `crypto_pbkdf2_run`, `crypto_pbkdf2_refuses_its_bad_arguments`,
      `http_routes_run`, `http_request_readers_run`, `http_responses_run`,
      `http_server_wire_run`, `ssr_dist_answers_files_and_pages` and
      `the_blog_example_passes_its_own_tests`;
    - 13 in `blog_tests`, new;
    - 3 in `nova-runtime`'s library:
      `crypto::tests::pbkdf2_matches_rfc_7914s_vectors`,
      `crypto::tests::pbkdf2_verify_accepts_its_key_and_rejects_others` and
      `crypto::tests::pbkdf2_iteration_counts_outside_u32_are_refused`;
    - 1 in `wasm_tests`: `gate::bytes_hex`.

    `doc_test`, `index_complete`, `broken`, `lower_tests` and `nova-std`
    keep their counts, and run more: std's new examples run inside
    `doc_std_runs_stds_doc_tests_inside_a_project_or_not`, the corpus test
    and `broken` take the blog, and `lower_tests`' crypto test holds two
    more stubs. A comparison of the two runs' `test` lines by name may also
    list `io::tests::a_successful_stdout_write_reports_its_byte_count` and
    its `stderr` twin: the runtime's io tests write without a line feed, so
    a run can print their names after a program's `hi` or `bye`, on its
    line. They are the same tests, not new;
  - the one failure is Task 0's IPv6 test;
  - the size: `1 passed`, and it prints `06-counter-spa: 21070 bytes
    gzipped`, by `flate2` at level 9, against 51,200, as at the base: 06
    calls nothing 4.7 adds. Record it in the ledger.
- [ ] **Step 2: The lockfile.** `git diff --numstat c8d6196 -- Cargo.lock`.
  Expected: nothing.
- [ ] **Step 3: Rust 1.78.**

  ```sh
  RUSTFLAGS="-D warnings" CARGO_TARGET_DIR="$P/msrv-47" cargo +1.78 check --locked --workspace
  ```

  Expected: clean, in about a minute (the replay's check took 42 seconds). It
  checks the libraries and binaries, not the tests, as 4.6's did:
  `assert_cmd` 2.2.1's manifest does not parse on 1.78. Then `rm -rf
  "$P/msrv-47"`.
- [ ] **Step 4: The mutants.** One at a time, each edit is made, its tests
  run, and the file restored with `git checkout -- <file>`.
  - Each edit replaces one exact text, which occurs once in its file; "the
    line goes" removes the whole line holding it.
  - Each must fail the tests named. A mutant that passes them is a gap:
    record it in the ledger, and write the missing test before the
    review.
  - At the end, `git status --short` prints nothing.

  Each command runs one test binary with one filter: cargo's filters are
  joined by "or", so a second name would widen the run, not narrow it.
  Each names the tests that fail in its run; the rest of that run passes.
  - A mutant of `std/` needs nothing more: `cargo test` rebuilds the
    embedded std. Nor does a mutant of the blog's sources: `blog_tests.rs`
    and `run_tests.rs` copy the example when they run.
  - A mutant of `crates/nova-runtime`, 6, 7 and 12 to 15, has its command
    build `nova` and the runtime library first, since a debug `nova` links
    the library beside it; once its file is restored, run `cargo build
    --locked -p nova-cli -p nova-runtime` again, so the next mutant runs on
    the true runtime.
  - Sixteen mutants, 75, 76, 83, 87 to 91, 98 to 103, 105 and 106, run
    `the_blog_example_passes_its_own_tests`, whose output also names the
    blog's own case that failed, such as `test
    sessions::a_wrong_tag_is_no_session ... FAILED`.
  - Mutant 107 runs a server that should refuse to start and does not:
    `a_key_file_without_32_bytes_in_hex_is_refused` kills it after 30
    seconds and fails.
  - Mutant 96's texts hold a `\u` escape: make that edit with a script
    (Global Constraints).

  The replay ran all 116 in about 21 minutes, each caught one failing
  through a wrong output, a panic, a wrong status or a wrong exit code,
  never through a build that broke: a run that prints a compile error,
  Rust's or Nova's, or `nova build --ssr failed`, differs from the replay,
  so check the edit.

  1. **`to_hex` writes upper-case digits.** `std/bytes/lib.nova`:
     `let digits = "0123456789abcdef".chars()` becomes `let digits = "0123456789ABCDEF".chars()`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- bytes_hex_run`.
     - Expected: fails: `bytes_hex_run`.
  2. **`bytes_from_hex` takes an odd length.** `std/bytes/lib.nova`:
     `if cs.len() % 2 != 0 { return None }`: the line goes.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- bytes_hex_run`.
     - Expected: fails: `bytes_hex_run`.
  3. **`bytes_from_hex` reads `g` as a digit.** `std/bytes/lib.nova`:
     `if b >= 97 && b <= 102 { return b - 87 }` becomes `if b >= 97 && b <= 103 { return b - 87 }`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- bytes_hex_run`.
     - Expected: fails: `bytes_hex_run`.
  4. **`rename` calls its builtin with its arguments swapped.** `std/fs/lib.nova`:
     `let status = fs_rename(from, to)` becomes `let status = fs_rename(to, from)`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- fs_rename_run`.
     - Expected: fails: `fs_rename_run`.
  5. **`rename`'s error is mapped to the next kind.** `std/fs/lib.nova`:
     replace

     ```
         let status = fs_rename(from, to)
         if status == 0 { return Ok(()) }
         Err(IoError { kind: io_error_kind_of(status), message: fs_last_error_message() })
     ```

     with

     ```
         let status = fs_rename(from, to)
         if status == 0 { return Ok(()) }
         Err(IoError { kind: io_error_kind_of(status + 1), message: fs_last_error_message() })
     ```

     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- fs_rename_run`.
     - Expected: fails: `fs_rename_run`.
  6. **The runtime copies instead of renaming.** `crates/nova-runtime/src/fs.rs`:
     `match std::fs::rename(f, t) {` becomes `match std::fs::copy(f, t).map(|_| ()) {`.
     - Run `CARGO_INCREMENTAL=0 cargo build --locked -p nova-cli -p nova-runtime && CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- fs_rename_run`.
     - Expected: fails: `fs_rename_run`.
  7. **The key is derived with HMAC-SHA-512.** `crates/nova-runtime/src/crypto.rs`:
     replace

     ```
         pbkdf2::derive(
             pbkdf2::PBKDF2_HMAC_SHA256,
     ```

     with

     ```
         pbkdf2::derive(
             pbkdf2::PBKDF2_HMAC_SHA512,
     ```

     - Run `CARGO_INCREMENTAL=0 cargo build --locked -p nova-cli -p nova-runtime && CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- crypto_pbkdf2_run`.
     - Expected: fails: `crypto_pbkdf2_run`.
  8. **The password and the salt are swapped.** `std/crypto/lib.nova`:
     `let status = crypto_pbkdf2_sha256(password, salt, iterations, length)` becomes `let status = crypto_pbkdf2_sha256(salt, password, iterations, length)`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- crypto_pbkdf2_run`.
     - Expected: fails: `crypto_pbkdf2_run`.
  9. **One round, whatever the count.** `std/crypto/lib.nova`:
     `let status = crypto_pbkdf2_sha256(password, salt, iterations, length)` becomes `let status = crypto_pbkdf2_sha256(password, salt, 1, length)`.
     - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- crypto_pbkdf2_run`.
     - Expected: fails: `crypto_pbkdf2_run`.
  10. **32 bytes, whatever the length.** `std/crypto/lib.nova`:
      `let status = crypto_pbkdf2_sha256(password, salt, iterations, length)` becomes `let status = crypto_pbkdf2_sha256(password, salt, iterations, 32)`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- crypto_pbkdf2_run`.
      - Expected: fails: `crypto_pbkdf2_run`.
  11. **`pbkdf2_sha256_verify` says true for a mismatch.** `std/crypto/lib.nova`:
      replace

      ```
          if status < 0 { pbkdf2_refused("pbkdf2_sha256_verify", status, iterations, expected.len()) }
          status == 0
      ```

      with

      ```
          if status < 0 { pbkdf2_refused("pbkdf2_sha256_verify", status, iterations, expected.len()) }
          status >= 0
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- crypto_pbkdf2_run`.
      - Expected: fails: `crypto_pbkdf2_run`.
  12. **An iteration count below 1 is read as 1.** `crates/nova-runtime/src/crypto.rs`:
      `NonZeroU32::new(u32::try_from(iterations).ok()?)` becomes `NonZeroU32::new(u32::try_from(iterations.max(1)).ok()?)`.
      - Run `CARGO_INCREMENTAL=0 cargo build --locked -p nova-cli -p nova-runtime && CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- crypto_pbkdf2_refuses_its_bad_arguments`.
      - Expected: fails: `crypto_pbkdf2_refuses_its_bad_arguments`.
  13. **An iteration count above 4,294,967,295 is read as 1.** `crates/nova-runtime/src/crypto.rs`:
      replace

      ```
          NonZeroU32::new(u32::try_from(iterations).ok()?)
      ```

      with

      ```
          NonZeroU32::new(u32::try_from(if iterations > 4294967295 { 1 } else { iterations }).ok()?)
      ```

      - Run `CARGO_INCREMENTAL=0 cargo build --locked -p nova-cli -p nova-runtime && CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- crypto_pbkdf2_refuses_its_bad_arguments`.
      - Expected: fails: `crypto_pbkdf2_refuses_its_bad_arguments`.
  14. **A length of 0 is taken.** `crates/nova-runtime/src/crypto.rs`:
      `Ok(n) if (1..=MAX_PBKDF2_BYTES).contains(&n) => n,` becomes `Ok(n) if (0..=MAX_PBKDF2_BYTES).contains(&n) => n,`.
      - Run `CARGO_INCREMENTAL=0 cargo build --locked -p nova-cli -p nova-runtime && CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- crypto_pbkdf2_refuses_its_bad_arguments`.
      - Expected: fails: `crypto_pbkdf2_refuses_its_bad_arguments`.
  15. **A length of 65,536 is refused.** `crates/nova-runtime/src/crypto.rs`:
      `Ok(n) if (1..=MAX_PBKDF2_BYTES).contains(&n) => n,` becomes `Ok(n) if (1..MAX_PBKDF2_BYTES).contains(&n) => n,`.
      - Run `CARGO_INCREMENTAL=0 cargo build --locked -p nova-cli -p nova-runtime && CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- crypto_pbkdf2_refuses_its_bad_arguments`.
      - Expected: fails: `crypto_pbkdf2_refuses_its_bad_arguments`.
  16. **`pbkdf2_sha256` does not panic on a refused status.** `std/crypto/lib.nova`:
      the line holding

      ```
          if status != 0 { pbkdf2_refused("pbkdf2_sha256", status, iterations, length) }
      ```

      goes.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- crypto_pbkdf2_refuses_its_bad_arguments`.
      - Expected: fails: `crypto_pbkdf2_refuses_its_bad_arguments`.
  17. **`pbkdf2_sha256_verify` does not panic on a bad count.** `std/crypto/lib.nova`:
      `if status < 0 { pbkdf2_refused("pbkdf2_sha256_verify",` becomes `if status < -5 { pbkdf2_refused("pbkdf2_sha256_verify",`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- crypto_pbkdf2_refuses_its_bad_arguments`.
      - Expected: fails: `crypto_pbkdf2_refuses_its_bad_arguments`.
  18. **`get` registers `HEAD`.** `std/http/lib.nova`:
      `self.route(Get, pattern, handler)` becomes `self.route(Head, pattern, handler)`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_routes_run`.
      - Expected: fails: `http_routes_run`.
  19. **`post` registers `PUT`.** `std/http/lib.nova`:
      `self.route(Post, pattern, handler)` becomes `self.route(Put, pattern, handler)`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_routes_run`.
      - Expected: fails: `http_routes_run`.
  20. **`put` registers `PATCH`.** `std/http/lib.nova`:
      `self.route(Put, pattern, handler)` becomes `self.route(Patch, pattern, handler)`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_routes_run`.
      - Expected: fails: `http_routes_run`.
  21. **`delete` registers `GET`.** `std/http/lib.nova`:
      `self.route(Delete, pattern, handler)` becomes `self.route(Get, pattern, handler)`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_routes_run`.
      - Expected: fails: `http_routes_run`.
  22. **`route` registers `GET`, whatever its method.** `std/http/lib.nova`:
      replace

      ```
              rs.push(Route { method: method, pattern: path_pieces(pattern), handler: handler })
      ```

      with

      ```
              rs.push(Route { method: Get, pattern: path_pieces(pattern), handler: handler })
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_routes_run`.
      - Expected: fails: `http_routes_run`.
  23. **A `:name` piece is bound under `:name`.** `std/http/lib.nova`:
      `let _ = params.insert(p.slice(1, p.len()), pieces[i])` becomes `let _ = params.insert(p, pieces[i])`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_routes_run`.
      - Expected: fails: `http_routes_run`.
  24. **A path's pieces are not decoded.** `std/http/lib.nova`:
      `out[k] = percent_decoded(parts[j])` becomes `out[k] = parts[j]`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_routes_run`.
      - Expected: fails: `http_routes_run`.
  25. **A path's empty pieces are kept.** `std/http/lib.nova`:
      replace

      ```
          let parts = path.split("/")
          let mut n = 0
          let mut i = 0
          while i < parts.len() {
              if parts[i] != "" { n = n + 1 }
              i = i + 1
          }
          let mut out = [""; n]
          let mut k = 0
          let mut j = 0
          while j < parts.len() {
              if parts[j] != "" {
                  out[k] = percent_decoded(parts[j])
                  k = k + 1
              }
              j = j + 1
          }
          out
      ```

      with

      ```
          let parts = path.split("/")
          let mut out = [""; parts.len()]
          let mut j = 0
          while j < parts.len() {
              out[j] = percent_decoded(parts[j])
              j = j + 1
          }
          out
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_routes_run`.
      - Expected: fails: `http_routes_run`.
  26. **The query takes part in matching.** `std/http/lib.nova`:
      replace

      ```
                  Some(at) => target.slice(0, at)
                  None => target
              }
              let pieces = path_pieces(path)
      ```

      with

      ```
                  Some(at) => target
                  None => target
              }
              let pieces = path_pieces(path)
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_routes_run`.
      - Expected: fails: `http_routes_run`.
  27. **The last route added answers first.** `std/http/lib.nova`:
      replace

      ```
              let mut allowed: Vec<String> = Vec::new()
              let mut i = 0
              while i < self.routes.len() {
                  match self.routes.get(i) {
      ```

      with

      ```
              let mut allowed: Vec<String> = Vec::new()
              let mut i = 0
              while i < self.routes.len() {
                  match self.routes.get(self.routes.len() - 1 - i) {
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_routes_run`.
      - Expected: fails: `http_routes_run`.
  28. **No `405`: a path with other methods falls through.** `std/http/lib.nova`:
      `if allowed.len() > 0 {` becomes `if allowed.len() > 99 {`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_routes_run`.
      - Expected: fails: `http_routes_run`.
  29. **`allow` names a method each time a route has it.** `std/http/lib.nova`:
      `if !holds(allowed, name) { allowed.push(name) }` becomes `allowed.push(name)`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_routes_run`.
      - Expected: fails: `http_routes_run`.
  30. **`allow` names the methods in reverse.** `std/http/lib.nova`:
      `names[k] = allowed.get(k).unwrap_or("")` becomes `names[k] = allowed.get(allowed.len() - 1 - k).unwrap_or("")`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_routes_run`.
      - Expected: fails: `http_routes_run`.
  31. **The fallback answers every method.** `std/http/lib.nova`:
      replace

      ```
              match req.method {
                  Get => {
                      match self.unmatched {
      ```

      with

      ```
              match Get {
                  Get => {
                      match self.unmatched {
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_routes_run`.
      - Expected: fails: `http_routes_run`.
  32. **With no route and no fallback, `200`.** `std/http/lib.nova`:
      replace

      ```
                      match self.unmatched {
                          Some(h) => h(req)
                          None => Response::not_found()
      ```

      with

      ```
                      match self.unmatched {
                          Some(h) => h(req)
                          None => Response::text(200, "not found")
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_routes_run`.
      - Expected: fails: `http_routes_run`.
  33. **`query` keeps the `?`.** `std/http/lib.nova`:
      `Some(at) => form_fields(self.path.slice(at + 1, self.path.len()))` becomes `Some(at) => form_fields(self.path.slice(at, self.path.len()))`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_request_readers_run`.
      - Expected: fails: `http_request_readers_run`.
  34. **`form` reads any body.** `std/http/lib.nova`:
      `if media != "application/x-www-form-urlencoded" { return none }`: the line goes.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_request_readers_run`.
      - Expected: fails: `http_request_readers_run`.
  35. **`form`'s media type keeps its parameters.** `std/http/lib.nova`:
      `Some(at) => v.slice(0, at).trim().to_lower()` becomes `Some(at) => v.trim().to_lower()`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_request_readers_run`.
      - Expected: fails: `http_request_readers_run`.
  36. **`+` is not a space.** `std/http/lib.nova`:
      replace

      ```
      fn form_decoded(text: String) -> String { percent_decoded(" ".join(text.split("+"))) }
      ```

      with

      ```
      fn form_decoded(text: String) -> String { percent_decoded(text) }
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_request_readers_run`.
      - Expected: fails: `http_request_readers_run`.
  37. **`%XX` reads its digits the wrong way round.** `std/http/lib.nova`:
      `out[n] = hi * 16 + lo` becomes `out[n] = lo * 16 + hi`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_request_readers_run`.
      - Expected: fails: `http_request_readers_run`.
  38. **An empty part is a field.** `std/http/lib.nova`:
      `if part != "" {` becomes `if true {`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_request_readers_run`.
      - Expected: fails: `http_request_readers_run`.
  39. **A part with no `=` has the value `true`.** `std/http/lib.nova`:
      replace

      ```
                  let mut value = ""
      ```

      with

      ```
                  let mut value = "true"
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_request_readers_run`.
      - Expected: fails: `http_request_readers_run`.
  40. **The first of two names wins.** `std/http/lib.nova`:
      replace

      ```
                  let _ = out.insert(form_decoded(name), form_decoded(value))
      ```

      with

      ```
                  if !out.contains_key(form_decoded(name)) { let _ = out.insert(form_decoded(name), form_decoded(value)) }
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_request_readers_run`.
      - Expected: fails: `http_request_readers_run`.
  41. **The last of two cookies wins.** `std/http/lib.nova`:
      `if !out.contains_key(name) {` becomes `if true {`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_request_readers_run`.
      - Expected: fails: `http_request_readers_run`.
  42. **A cookie's part is not trimmed.** `std/http/lib.nova`:
      `let part = parts[i].trim()` becomes `let part = parts[i]`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_request_readers_run`.
      - Expected: fails: `http_request_readers_run`.
  43. **`Cookie::new` is not `HttpOnly`.** `std/http/lib.nova`:
      `http_only: true,` becomes `http_only: false,`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_responses_run`.
      - Expected: fails: `http_responses_run`.
  44. **`Cookie::expired` lasts a second.** `std/http/lib.nova`:
      `c.max_age = Some(0)` becomes `c.max_age = Some(1)`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_responses_run`.
      - Expected: fails: `http_responses_run`.
  45. **No `Path` written.** `std/http/lib.nova`:
      `if c.path != "" { line = "${line}; Path=${c.path}" }`: the line goes.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_responses_run`.
      - Expected: fails: `http_responses_run`.
  46. **No `Max-Age` written.** `std/http/lib.nova`:
      `Some(n) => line = "${line}; Max-Age=${n}"` becomes `Some(n) => {}`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_responses_run`.
      - Expected: fails: `http_responses_run`.
  47. **No `HttpOnly` written.** `std/http/lib.nova`:
      `if c.http_only { line = "${line}; HttpOnly" }`: the line goes.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_responses_run`.
      - Expected: fails: `http_responses_run`.
  48. **No `SameSite` written.** `std/http/lib.nova`:
      `if c.same_site != "" { line = "${line}; SameSite=${c.same_site}" }`: the line goes.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_responses_run`.
      - Expected: fails: `http_responses_run`.
  49. **No `Secure` written.** `std/http/lib.nova`:
      `if c.secure { line = "${line}; Secure" }`: the line goes.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_responses_run`.
      - Expected: fails: `http_responses_run`.
  50. **Only the first cookie is written.** `std/http/lib.nova`:
      `while c < self.cookies.len() {` becomes `while c < self.cookies.len() && c < 1 {`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_responses_run`.
      - Expected: fails: `http_responses_run`.
  51. **A name that is not a token is written.** `std/http/lib.nova`:
      `if !is_token(c.name) { return None }`: the line goes.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_responses_run`.
      - Expected: fails: `http_responses_run`.
  52. **A value a cookie cannot carry is written.** `std/http/lib.nova`:
      `if !all_chars(c.value, is_cookie_octet) { return None }`: the line goes.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_responses_run`.
      - Expected: fails: `http_responses_run`.
  53. **A path a cookie cannot carry is written.** `std/http/lib.nova`:
      `if !all_chars(c.path, is_path_char) { return None }`: the line goes.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_responses_run`.
      - Expected: fails: `http_responses_run`.
  54. **`html` is `text/plain`.** `std/http/lib.nova`:
      `h.insert("content-type", "text/html; charset=utf-8")` becomes `h.insert("content-type", "text/plain; charset=utf-8")`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_responses_run`.
      - Expected: fails: `http_responses_run`.
  55. **`redirect` is `302`.** `std/http/lib.nova`:
      replace

      ```
              Response { status: 303, headers: h, body: bytes_from_string(""), cookies: [] }
      ```

      with

      ```
              Response { status: 302, headers: h, body: bytes_from_string(""), cookies: [] }
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_responses_run`.
      - Expected: fails: `http_responses_run`.
  56. **`redirect` goes to `/`.** `std/http/lib.nova`:
      `h.insert("location", location)` becomes `h.insert("location", "/")`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_responses_run`.
      - Expected: fails: `http_responses_run`.
  57. **`redirect` has no `content-length`.** `std/http/lib.nova`:
      `h.insert("content-length", "0")`: the line goes.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_responses_run`.
      - Expected: fails: `http_responses_run`.
  58. **`301` has its class's phrase.** `std/http/lib.nova`:
      `if status == 301 { return "Moved Permanently" }`: the line goes.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_responses_run`.
      - Expected: fails: `http_responses_run`.
  59. **`302` has its class's phrase.** `std/http/lib.nova`:
      `if status == 302 { return "Found" }`: the line goes.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_responses_run`.
      - Expected: fails: `http_responses_run`.
  60. **`303` has its class's phrase.** `std/http/lib.nova`:
      `if status == 303 { return "See Other" }`: the line goes.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_responses_run`.
      - Expected: fails: `http_responses_run`.
  61. **`401` has its class's phrase.** `std/http/lib.nova`:
      `if status == 401 { return "Unauthorized" }`: the line goes.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_responses_run`.
      - Expected: fails: `http_responses_run`.
  62. **`403` has its class's phrase.** `std/http/lib.nova`:
      `if status == 403 { return "Forbidden" }`: the line goes.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- http_responses_run`.
      - Expected: fails: `http_responses_run`.
  63. **`Dist::open` takes a template with no root.** `std/ui/ssr/lib.nova`:
      `None => return Err(rootless("ssr", index))` becomes `None => t`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- ssr_dist_answers_files_and_pages`.
      - Expected: fails: `ssr_dist_answers_files_and_pages`.
  64. **`Dist::file` serves a file the manifest does not list.** `std/ui/ssr/lib.nova`:
      `if self.files.get(path).is_none() { return None }`: the line goes.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- ssr_dist_answers_files_and_pages`.
      - Expected: fails: `ssr_dist_answers_files_and_pages`.
  65. **`Dist::file` keeps the path's leading `/`.** `std/ui/ssr/lib.nova`:
      replace

      ```
          if path.starts_with("/") { path.slice(1, path.len()) } else { path }
      ```

      with

      ```
          path
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- ssr_dist_answers_files_and_pages`.
      - Expected: fails: `ssr_dist_answers_files_and_pages`.
  66. **`Dist::file` answers `None` for a listed file it cannot read.** `std/ui/ssr/lib.nova`:
      replace

      ```
              if fs_read("${self.dir}/${path}") != 0 { return Some(Response::not_found()) }
      ```

      with

      ```
              if fs_read("${self.dir}/${path}") != 0 { return None }
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- ssr_dist_answers_files_and_pages`.
      - Expected: fails: `ssr_dist_answers_files_and_pages`.
  67. **`Dist::respond` answers `200`.** `std/ui/ssr/lib.nova`:
      `Response::html(status, document(self.template, page))` becomes `Response::html(200, document(self.template, page))`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- ssr_dist_answers_files_and_pages`.
      - Expected: fails: `ssr_dist_answers_files_and_pages`.
  68. **`Dist::respond` answers `text/plain`.** `std/ui/ssr/lib.nova`:
      `Response::html(status, document(self.template, page))` becomes `Response::text(status, document(self.template, page))`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- ssr_dist_answers_files_and_pages`.
      - Expected: fails: `ssr_dist_answers_files_and_pages`.
  69. **`serve` answers `200` for a path no route matches.** `std/ui/ssr/lib.nova`:
      `let status = if found(s.routes, req.path) { 200 } else { 404 }` becomes `let status = if found(s.routes, req.path) { 200 } else { 200 }`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- ssr_serve_answers_files_and_pages`.
      - Expected: fails: `ssr_serve_answers_files_and_pages`.
  70. **A comment's token is not checked.** `examples/07-fullstack-blog/src/server.nova`:
      replace

      ```
                  "/posts/:id/comments",
                  |req: Request| checked(
                      b,
                      req,
                      false,
                      |s: Session, f: Map<String, String>| add_comment(b, req, s, f),
                  ),
      ```

      with

      ```
                  "/posts/:id/comments",
                  |req: Request| match session_of(b, req) {
                      Some(s) => add_comment(b, req, s, req.form())
                      None => refused(b, req, None, EXPIRED)
                  },
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- a_comment_needs_its_sessions_csrf_token`.
      - Expected: fails: `a_comment_needs_its_sessions_csrf_token`.
  71. **A login's token is not checked.** `examples/07-fullstack-blog/src/server.nova`:
      replace

      ```
                  "/login",
                  |req: Request| checked(
                      b,
                      req,
                      false,
                      |s: Session, f: Map<String, String>| log_in(b, req, s, f),
                  ),
      ```

      with

      ```
                  "/login",
                  |req: Request| match session_of(b, req) {
                      Some(s) => log_in(b, req, s, req.form())
                      None => refused(b, req, None, EXPIRED)
                  },
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- logging_in_takes_the_right_password_and_a_new_session`.
      - Expected: fails: `logging_in_takes_the_right_password_and_a_new_session`.
  72. **A logout's token is not checked.** `examples/07-fullstack-blog/src/server.nova`:
      replace

      ```
                  "/logout",
                  |req: Request| checked(
                      b,
                      req,
                      false,
                      |s: Session, f: Map<String, String>| log_out(b, s),
                  ),
      ```

      with

      ```
                  "/logout",
                  |req: Request| match session_of(b, req) {
                      Some(s) => log_out(b, s)
                      None => refused(b, req, None, EXPIRED)
                  },
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- logging_out_ends_the_session`.
      - Expected: fails: `logging_out_ends_the_session`.
  73. **An admin form's token is not checked.** `examples/07-fullstack-blog/src/server.nova`:
      replace

      ```
          if !b.sessions.token_ok(s, fields.get("csrf").unwrap_or("")) {
      ```

      with

      ```
          if !admin && !b.sessions.token_ok(s, fields.get("csrf").unwrap_or("")) {
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- the_admin_pages_need_the_admin`.
      - Expected: fails: `the_admin_pages_need_the_admin`.
  74. **A CSRF token is not bound to its session.** `examples/07-fullstack-blog/src/auth.nova`:
      replace

      ```
              hmac_sha256(self.key, bytes_from_string("csrf:${s.id}")).to_hex()
          }

          /// Whether `token` is `s`'s CSRF token, compared in constant time.
          pub fn token_ok(self, s: Session, token: String) -> Bool {
              match bytes_from_hex(token) {
                  Some(t) => hmac_sha256_verify(self.key, bytes_from_string("csrf:${s.id}"), t)
      ```

      with

      ```
              hmac_sha256(self.key, bytes_from_string("csrf:")).to_hex()
          }

          /// Whether `token` is `s`'s CSRF token, compared in constant time.
          pub fn token_ok(self, s: Session, token: String) -> Bool {
              match bytes_from_hex(token) {
                  Some(t) => hmac_sha256_verify(self.key, bytes_from_string("csrf:"), t)
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- a_comment_needs_its_sessions_csrf_token`.
      - Expected: fails: `a_comment_needs_its_sessions_csrf_token`.
  75. **A cookie's tag is not checked.** `examples/07-fullstack-blog/src/auth.nova`:
      the line holding

      ```
              if !hmac_sha256_verify(self.key, bytes_from_string(id), tag) { return None }
      ```

      goes.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- the_blog_example_passes_its_own_tests`.
      - Expected: fails: `the_blog_example_passes_its_own_tests`, whose output names the blog's own failing case `sessions::a_wrong_tag_is_no_session`.
  76. **A session never ends.** `examples/07-fullstack-blog/src/auth.nova`:
      replace

      ```
          pub fn ended(self, s: Session) -> Bool { s.started.elapsed().as_millis() >= s.life * 1000 }
      ```

      with

      ```
          pub fn ended(self, s: Session) -> Bool { false }
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- the_blog_example_passes_its_own_tests`.
      - Expected: fails: `the_blog_example_passes_its_own_tests`, whose output names the blog's own failing cases `sessions::an_ended_or_unknown_session_is_no_session` and `sessions::ended_sessions_are_dropped_once_the_table_has_doubled`.
  77. **Logging in keeps the session's id.** `examples/07-fullstack-blog/src/server.nova`:
      replace

      ```
          let mut sessions = b.sessions
          sessions.end(s.id)
          let admin = sessions.start(name)
      ```

      with

      ```
          let admin = Session { id: s.id, user: name, started: s.started, life: b.sessions.admin_life }
          let mut table = b.sessions.table
          let _ = table.insert(admin.id, admin)
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- logging_in_takes_the_right_password_and_a_new_session`.
      - Expected: fails: `logging_in_takes_the_right_password_and_a_new_session`.
  78. **Logging out leaves the session.** `examples/07-fullstack-blog/src/server.nova`:
      replace

      ```
          b.sessions.end(s.id)
          Response::redirect("/").with_cookie(Cookie::expired("sid"))
      ```

      with

      ```
          Response::redirect("/").with_cookie(Cookie::expired("sid"))
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- logging_out_ends_the_session`.
      - Expected: fails: `logging_out_ends_the_session`.
  79. **The session cookie is not `HttpOnly`.** `examples/07-fullstack-blog/src/server.nova`:
      replace

      ```
          c.max_age = Some(s.life)
          c
      }
      ```

      with

      ```
          c.max_age = Some(s.life)
          c.http_only = false
          c
      }
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- every_page_answers_with_its_status`.
      - Expected: fails: `every_page_answers_with_its_status`.
  80. **The session cookie is `SameSite=None`.** `examples/07-fullstack-blog/src/server.nova`:
      replace

      ```
          c.max_age = Some(s.life)
          c
      }
      ```

      with

      ```
          c.max_age = Some(s.life)
          c.same_site = "None"
          c
      }
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- every_page_answers_with_its_status`.
      - Expected: fails: `every_page_answers_with_its_status`.
  81. **An admin page is shown without the admin.** `examples/07-fullstack-blog/src/server.nova`:
      `None => Response::redirect("/login")` becomes `None => then(visit(b, req))`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- the_admin_pages_need_the_admin`.
      - Expected: fails: `the_admin_pages_need_the_admin`.
  82. **An admin form is taken without the admin.** `examples/07-fullstack-blog/src/server.nova`:
      the line holding

      ```
          if admin && s.user == "" { return refused(b, req, Some(s), NOT_ADMIN) }
      ```

      goes.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- the_admin_pages_need_the_admin`.
      - Expected: fails: `the_admin_pages_need_the_admin`.
  83. **A stored hash of any length is checked.** `examples/07-fullstack-blog/src/auth.nova`:
      `if key.len() != 32 { return false }`: the line goes.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- the_blog_example_passes_its_own_tests`.
      - Expected: fails: `the_blog_example_passes_its_own_tests`, whose output names the blog's own failing case `passwords::a_hash_cut_short_never_matches`.
  84. **A name no user has costs nothing.** `examples/07-fullstack-blog/src/auth.nova`:
      the line holding

      ```
                  let _ = pbkdf2_sha256_verify(given, no_salt(), rounds, bytes_from_ints([0; 32]))
      ```

      goes.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- a_name_no_user_has_takes_as_long_as_a_wrong_password`.
      - Expected: fails: `a_name_no_user_has_takes_as_long_as_a_wrong_password`.
  85. **`--set-password` takes five characters.** `examples/07-fullstack-blog/src/server.nova`:
      `if password.len() < 8 {` becomes `if password.len() < 5 {`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- set_password_writes_the_admin`.
      - Expected: fails: `set_password_writes_the_admin`.
  86. **A file is never renamed into place.** `examples/07-fullstack-blog/src/store.nova`:
      `rename(tmp, "${dir}/${name}").await` becomes `Ok(())`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- the_posts_survive_a_restart`.
      - Expected: fails: `the_posts_survive_a_restart`.
  87. **Deleting a post keeps its comments.** `examples/07-fullstack-blog/src/store.nova`:
      `if c.post != id { cs.push(c) }` becomes `cs.push(c)`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- the_blog_example_passes_its_own_tests`.
      - Expected: fails: `the_blog_example_passes_its_own_tests`, whose output names the blog's own failing case `store::edits_and_deletes`.
  88. **A title of 201 characters passes.** `examples/07-fullstack-blog/src/forms.nova`:
      `const TITLE_MAX: Int = 200` becomes `const TITLE_MAX: Int = 201`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- the_blog_example_passes_its_own_tests`.
      - Expected: fails: `the_blog_example_passes_its_own_tests`, whose output names the blog's own failing case `forms::a_title_has_1_to_200_characters`.
  89. **A blank line does not separate paragraphs.** `examples/07-fullstack-blog/src/post_card.nova`:
      `if line.trim() == "" {` becomes `if false {`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- the_blog_example_passes_its_own_tests`.
      - Expected: fails: `the_blog_example_passes_its_own_tests`, whose output names the blog's own failing case `forms::a_blank_line_separates_paragraphs`.
  90. **A stored field is not trimmed.** `examples/07-fullstack-blog/src/forms.nova`:
      `"\n".join(kept.split("\r\n")).trim()` becomes `"\n".join(kept.split("\r\n"))`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- the_blog_example_passes_its_own_tests`.
      - Expected: fails: `the_blog_example_passes_its_own_tests`, whose output names the blog's own failing cases `forms::a_form_passes_when_each_field_does`, `forms::a_title_has_1_to_200_characters`, `forms::each_field_has_its_limits`, `forms::the_server_keeps_line_feeds_and_no_spaces_around` and `forms::the_server_keeps_no_nul`.
  91. **A stored field keeps its CRLFs.** `examples/07-fullstack-blog/src/forms.nova`:
      `"\n".join(kept.split("\r\n")).trim()` becomes `kept.trim()`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- the_blog_example_passes_its_own_tests`.
      - Expected: fails: `the_blog_example_passes_its_own_tests`, whose output names the blog's own failing cases `forms::the_server_keeps_line_feeds_and_no_spaces_around` and `forms::the_server_keeps_no_nul`.
  92. **A `GET` of a `POST`-only path is `405`.** `examples/07-fullstack-blog/src/server.nova`:
      `.get("/posts/:id/comments", nowhere)`: the line goes.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- every_page_answers_with_its_status`.
      - Expected: fails: `every_page_answers_with_its_status`.
  93. **The not-found page is `200`.** `examples/07-fullstack-blog/src/server.nova`:
      `page(b, v, req, 404, d)` becomes `page(b, v, req, 200, d)`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- every_page_answers_with_its_status`.
      - Expected: fails: `every_page_answers_with_its_status`.
  94. **`<` in a text node is not escaped.** `std/ui/lib.nova`:
      `parts.push("&lt;")` becomes `parts.push("<")`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- visitor_text_comes_back_escaped`.
      - Expected: fails: `visitor_text_comes_back_escaped`.
  95. **`"` in an attribute is not escaped.** `std/ui/lib.nova`:
      `parts.push("&quot;")` becomes `parts.push("\"")`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- visitor_text_comes_back_escaped`.
      - Expected: fails: `visitor_text_comes_back_escaped`.
  96. **`<` in the page's data is not escaped.** `std/ui/ssr/lib.nova`:
      `"\\u003c"` becomes `"<"`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- visitor_text_comes_back_escaped`.
      - Expected: fails: `visitor_text_comes_back_escaped`.
  97. **`--set-password` writes `blog.json`, from the store it read before hashing.** `examples/07-fullstack-blog/src/server.nova`:
      replace

      ```
          let user = hash_password(name, password, o.iterations)
      ```

      with

      ```
          let stale = load_store(o.data).await.unwrap()
          let user = hash_password(name, password, o.iterations)
          saved(o.data, stale).await
      ```

      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- set_password_writes_the_admin`.
      - Expected: fails: `set_password_writes_the_admin`.
  98. **Starting a session always drops the ended ones.** `examples/07-fullstack-blog/src/auth.nova`:
      `if table.len() >= FEWEST_TO_DROP && table.len() >= self.kept * 2 {` becomes `if true {`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- the_blog_example_passes_its_own_tests`.
      - Expected: fails: `the_blog_example_passes_its_own_tests`, whose output names the blog's own failing case `sessions::ended_sessions_are_dropped_once_the_table_has_doubled`.
  99. **The ended sessions are never dropped.** `examples/07-fullstack-blog/src/auth.nova`:
      `if table.len() >= FEWEST_TO_DROP && table.len() >= self.kept * 2 {` becomes `if false {`.
      - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- the_blog_example_passes_its_own_tests`.
      - Expected: fails: `the_blog_example_passes_its_own_tests`, whose output names the blog's own failing case `sessions::ended_sessions_are_dropped_once_the_table_has_doubled`.
  100. **No floor: the ended sessions are dropped whenever the table has doubled.** `examples/07-fullstack-blog/src/auth.nova`:
       `if table.len() >= FEWEST_TO_DROP && table.len() >= self.kept * 2 {` becomes `if table.len() >= self.kept * 2 {`.
       - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- the_blog_example_passes_its_own_tests`.
       - Expected: fails: `the_blog_example_passes_its_own_tests`, whose output names the blog's own failing case `sessions::ended_sessions_are_dropped_once_the_table_has_doubled`.
  101. **No doubling: the ended sessions are dropped whenever the table holds 64.** `examples/07-fullstack-blog/src/auth.nova`:
       `if table.len() >= FEWEST_TO_DROP && table.len() >= self.kept * 2 {` becomes `if table.len() >= FEWEST_TO_DROP {`.
       - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- the_blog_example_passes_its_own_tests`.
       - Expected: fails: `the_blog_example_passes_its_own_tests`, whose output names the blog's own failing case `sessions::ended_sessions_are_dropped_once_the_table_has_doubled`.
  102. **`kept` is not updated when the ended sessions are dropped.** `examples/07-fullstack-blog/src/auth.nova`:
       `self.kept = table.len()`: the line goes.
       - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- the_blog_example_passes_its_own_tests`.
       - Expected: fails: `the_blog_example_passes_its_own_tests`, whose output names the blog's own failing case `sessions::ended_sessions_are_dropped_once_the_table_has_doubled`.
  103. **A stored field keeps its NULs.** `examples/07-fullstack-blog/src/forms.nova`:
       `let kept = "".join(text.split("\0"))` becomes `let kept = text`.
       - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- the_blog_example_passes_its_own_tests`.
       - Expected: fails: `the_blog_example_passes_its_own_tests`, whose output names the blog's own failing case `forms::the_server_keeps_no_nul`.
  104. **`--set-password` takes a blank password.** `examples/07-fullstack-blog/src/server.nova`:
       `if password.trim() == "" {` becomes `if password == "" {`.
       - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- set_password_writes_the_admin`.
       - Expected: fails: `set_password_writes_the_admin`.
  105. **A name no user has costs the first user's rounds.** `examples/07-fullstack-blog/src/auth.nova`:
       `if count_ok(n) && n > most { most = n }` becomes `if count_ok(n) && most == 0 { most = n }`.
       - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- the_blog_example_passes_its_own_tests`.
       - Expected: fails: `the_blog_example_passes_its_own_tests`, whose output names the blog's own failing case `passwords::a_name_no_user_has_costs_the_most_rounds_a_user_has`.
  106. **A name no user has costs a count PBKDF2 refuses.** `examples/07-fullstack-blog/src/auth.nova`:
       `if count_ok(n) && n > most { most = n }` becomes `if n > most { most = n }`.
       - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test run_tests -- the_blog_example_passes_its_own_tests`.
       - Expected: fails: `the_blog_example_passes_its_own_tests`, whose output names the blog's own failing case `passwords::a_name_no_user_has_costs_the_most_rounds_a_user_has`.
  107. **A key file of any length is taken.** `examples/07-fullstack-blog/src/auth.nova`:
       the line holding

       ```
               if key.len() != 32 { return Err("${path} does not hold 32 bytes in hex") }
       ```

       goes.
       - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- a_key_file_without_32_bytes_in_hex_is_refused`.
       - Expected: fails: `a_key_file_without_32_bytes_in_hex_is_refused`.
  108. **`--set-password` takes a blank name.** `examples/07-fullstack-blog/src/server.nova`:
       `if name.trim() == "" || name != name.trim() {` becomes `if name != name.trim() {`.
       - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- set_password_writes_the_admin`.
       - Expected: fails: `set_password_writes_the_admin`.
  109. **`--set-password` takes a name with spaces around it.** `examples/07-fullstack-blog/src/server.nova`:
       `if name.trim() == "" || name != name.trim() {` becomes `if name.trim() == "" {`.
       - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- set_password_writes_the_admin`.
       - Expected: fails: `set_password_writes_the_admin`.
  110. **`--set-password` refuses a bad name with status 1.** `examples/07-fullstack-blog/src/server.nova`:
       replace

       ```
               eprintln("blog: a user's name is not blank and has no spaces around it")
               exit(2)
       ```

       with

       ```
               eprintln("blog: a user's name is not blank and has no spaces around it")
               exit(1)
       ```

       - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- set_password_writes_the_admin`.
       - Expected: fails: `set_password_writes_the_admin`.
  111. **`--set-password` refuses a password that is not UTF-8 with status 2.** `examples/07-fullstack-blog/src/server.nova`:
       replace

       ```
                   eprintln("blog: the password is not UTF-8")
                   exit(1)
       ```

       with

       ```
                   eprintln("blog: the password is not UTF-8")
                   exit(2)
       ```

       - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- set_password_writes_the_admin`.
       - Expected: fails: `set_password_writes_the_admin`.
  112. **The server writes `users.json` with each change.** `examples/07-fullstack-blog/src/server.nova`:
       replace

       ```
       fn save(b: Blog) { let _ = spawn(saved(b.data, b.store)) }
       ```

       with

       ```
       fn save(b: Blog) {
           let _ = spawn(saved(b.data, b.store))
           let _ = spawn(save_users(b.data, b.users))
       }
       ```

       - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- a_password_set_while_the_server_runs_loses_nothing`.
       - Expected: fails: `a_password_set_while_the_server_runs_loses_nothing`.
  113. **`--set-password` reads `users.json` before it hashes.** `examples/07-fullstack-blog/src/server.nova`:
       replace

       ```
           let user = hash_password(name, password, o.iterations)
           let mut users = match load_users(o.data).await {
               Ok(u) => u
               Err(why) => {
                   eprintln("blog: ${why}")
                   exit(1)
                   return
               }
           }
       ```

       with

       ```
           let mut users = match load_users(o.data).await {
               Ok(u) => u
               Err(why) => {
                   eprintln("blog: ${why}")
                   exit(1)
                   return
               }
           }
           let user = hash_password(name, password, o.iterations)
       ```

       - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- overlapping_set_password_runs_keep_both_users`.
       - Expected: fails: `overlapping_set_password_runs_keep_both_users`.
  114. **The server does not read `users.json` when it starts.** `examples/07-fullstack-blog/src/server.nova`:
       replace

       ```
           let users = match load_users(o.data).await {
               Ok(u) => u
               Err(why) => {
                   eprintln("blog: ${why}")
                   exit(1)
                   return
               }
           }
           let key = match load_key(o.data).await {
       ```

       with

       ```
           let users = Users::new()
           let key = match load_key(o.data).await {
       ```

       - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- logging_in_takes_the_right_password_and_a_new_session`.
       - Expected: fails: `logging_in_takes_the_right_password_and_a_new_session`.
  115. **A `users.json` that does not hold users is taken as none.** `examples/07-fullstack-blog/src/store.nova`:
       replace

       ```
               Ok(u) => Ok(u)
               Err(why) => Err("${path}: ${why}")
       ```

       with

       ```
               Ok(u) => Ok(u)
               Err(why) => Ok(Users::new())
       ```

       - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- set_password_writes_the_admin`.
       - Expected: fails: `set_password_writes_the_admin`.
  116. **A login with a name no user has costs the first user's rounds.** `examples/07-fullstack-blog/src/server.nova`:
       replace

       ```
           let rounds = no_user_rounds(b.users.list, b.iterations)
       ```

       with

       ```
           let rounds = match b.users.list.get(0) {
               Some(u) => u.iterations
               None => b.iterations
           }
       ```

       - Run `CARGO_INCREMENTAL=0 cargo test -p nova-cli --test blog_tests -- a_name_no_user_has_takes_as_long_as_a_wrong_password`.
       - Expected: fails: `a_name_no_user_has_takes_as_long_as_a_wrong_password`.

- [ ] **Step 5: The review's inputs.** `06-counter-spa`'s size and the full
  run's figures, as Step 1 measured them, the blog's client size, as Task
  7's line printed it, and the mutants' outcome go in the ledger.
- [ ] **Step 6: One fresh read-only review, then one fix pass** (the
  user's workflow).
  - The reviewer reads `git diff $BASE..HEAD` (Task 0) against the spec,
    ADR 0046 and this plan, and is told to hunt for:
    - a route that matches what the router would not, or misses what it
      would: a piece decoded partly, an empty piece kept, a query taking
      part, a `:name` binding the wrong piece; a `405` whose `allow` repeats
      or misorders a method, a method other than `GET` reaching the
      fallback, a `GET` of a `POST`-only path of the blog reaching `405`
      (decisions 1, 2, 14 and 21);
    - a reader that decodes what it should not, or not what it should:
      `+`, `%XX`, a half that does not decode, an empty part, a part with no
      `=`; `form` reading a body whose media type is not the form's, or
      that is not UTF-8; the wrong one of two names kept (decision 4);
    - a cookie written that breaks a rule, or a header or an attribute
      injected through a name, a value, a path or a `same_site`; a cookie
      lost or reordered; `with_cookie` changing the response it was called
      on (decisions 5, 6, 17 and 20);
    - `Dist` answering otherwise than `serve` did: a file served that the
      manifest does not list, by any spelling, a listed file's status or
      type, a template's root not emptied, an error in other words
      (decisions 7 and 19);
    - PBKDF2's arguments swapped or a bound off by one; a length allocated
      before it is checked; `verify`'s acceptance of a key's start reaching
      the blog, which must refuse a hash that is not 32 bytes; a name no
      user has answered sooner than a wrong password, or than the name of
      the user whose hash has the most rounds; a `--set-password` that
      writes what it should refuse, or refuses with another status
      (decisions 12, 15, 16, 26, 27, 40 and 41);
    - a `POST` handler that reads a field, or changes anything, before its
      token is checked, or that takes another session's token; an admin's
      page or form reached without the admin's session; a session whose tag
      is not checked, that never ends, that keeps its id at login, or that
      outlives a logout; a cookie without `HttpOnly` or `SameSite=Lax`, or
      that outlives its session; a refusal page that starts a session; a
      table whose ended sessions are scanned at every start, or never
      dropped, or a live one dropped (decisions 9, 11, 28, 29, 35 and 38);
    - a visitor's text reaching the page unescaped, on either renderer or
      in the page's data, or reaching an `href`, a `location` or a cookie;
    - the store's file left half written, a write lost but for the one
      after a crash, writes out of order, a deleted post's comments kept;
      a file with two writers: the server writing `users.json` or reading
      a file to write `blog.json`, or `--set-password` writing `blog.json`
      or reading `users.json` long before it writes it; a `users.json` that
      does not hold users taken as none (decision 37);
    - a page whose hydration mismatches, or a form that needs JavaScript:
      the server's button and messages, the checks' start in `on_mount`, a
      stored text that comes back to a `textarea` otherwise, a NUL or
      another control character in a visitor's text, the two sides
      measuring a field differently, a refused or failed form's page at its
      `POST` path (decisions 13, 23, 24, 25, 39 and 44);
    - a test that could pass without the code it names, as the two the
      prototype's gap fix closed did, and the sessions' test did for the
      floor of 64 before `3e49a31` (mutant 100) and nothing did for when
      `--set-password` reads `users.json` before `64064e8` (mutant 113),
      and the timing test's margin and the overlapping runs' (decisions 35
      to 38);
    - Windows in `blog_tests.rs`: the `.exe`, CRLF in the copied files, the
      server killed and waited for before its files go, `rename` over a
      file and the locks Windows takes;
    - a record that says what the code does not, MSRV 1.78, and tests that
      touch the internet, use port 3000, or use temp directories without
      fixed names.
  - Each finding is fixed in one commit, `4.7: the review's fixes`, or
    answered in the ledger with the reason it stands.
  - After the fixes, Step 1 runs again.
- [ ] **Step 7: The PR.**
  - Push: `git push -u origin claude/loving-archimedes-frtf5q-47`.
  - Open the PR to `main` with the repository's template headings, if it
    has one. The body names no local path and no person.
  - It holds:
    - what 4.7 adds;
    - the gate's evidence: the blog in Chromium through `blog.mjs`, every
      page hydrating in place, the comment form's checks and post, a script
      in a title and in a comment staying text, a form without its token
      refused with `403`, and the admin's create, edit and delete;
      `blog_tests.rs` and the blog's `nova test` on all three systems; the
      sizes, `06-counter-spa`'s unchanged and the blog's client's printed;
    - the user's twelve decisions, and this plan's decisions, numbered as
      here, with 14 to 30 and 36 to 44 marked as differing from the spec;
    - that `Cargo.lock` is unchanged;
    - PBKDF2's measured times, the fact-check's nine findings and what
      came of each, and what stands (Checked on);
    - the mutants, the two gaps the first run found, the tests that closed
      them, the gaps of mutants 100 and 113 and the tests that closed
      them, and the review's findings and what came of each.
  - Read CI once it ends. Expected: green on all three systems, and the
    `browser` job green; the test job runs the WASM tests under Node 22
    with `NOVA_REQUIRE_NODE=1`.
- [ ] **Step 8: The merge,** only on the user's word.
  - By rebase, pinned to the PR's head commit. Never auto-merge.
  - Then `git fetch origin main` and `git diff <head> origin/main
    --stat`, which prints nothing: the merged tree is the PR's.
