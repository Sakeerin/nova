# 60 — Reference Examples

> Phase: parallel with each phase
> Location: `examples/`

These are the canonical programs that gate phase completion. Implement each as a full Nova project with `nova.toml`, `src/`, `tests/`, and `README.md`.

---

## 1. `01-hello-world` (Phase 1 gate)

**Goal:** Smallest working program.

`src/main.nova`:
```nova
import std/fmt { println }

fn main() {
    println("Hello, World!")
}
```

`README.md` covers: install, `nova run`, expected output.

**Gate:** `nova run` outputs `Hello, World!\n`

---

## 2. `02-fibonacci` (Phase 1 gate)

**Goal:** Recursion + arithmetic + CLI args + pattern match.

`src/main.nova`:
```nova
import std/fmt { println }
import std/process { args }

fn fib(n: Int) -> Int {
    match n {
        0 => 0,
        1 => 1,
        n => fib(n - 1) + fib(n - 2),
    }
}

fn main() {
    let argv = args()
    let n = argv.get(1)
        .and_then(|s| s.parse::<Int>().ok())
        .unwrap_or(10)
    println("fib(${n}) = ${fib(n)}")
}
```

**Gate:** `nova run -- 20` outputs `fib(20) = 6765`

---

## 3. `03-http-server` (Phase 2 gate)

**Goal:** stdlib http server works end-to-end.

`src/main.nova`:
```nova
import std/http
import std/log

async fn main() {
    log.init()

    let app = http.Server.new()
        .get("/", |_| http.Response.text("Hello from Nova!"))
        .get("/health", |_| http.Response.json({ "status": "ok" }))

    log.info("listening on :3000")
    app.listen("0.0.0.0:3000").await.unwrap()
}
```

**Gate:** `curl http://localhost:3000/` returns `Hello from Nova!`. Process exits cleanly on SIGTERM.

**Recorded 2026-09-01 (branch `std-http-parsing`), not fixed here: this
example's number and name have drifted from what `examples/` holds.** Both
this section and `00-MASTER-SPEC.md` §2's tree name `03-http-server` — the
drift is in two spec files, not one — while `examples/` on disk holds
`03-producer-consumer` instead. Measured directly (`ls examples/`), not
recalled.

---

## 4. `04-todo-cli` (Phase 2 gate)

**Goal:** filesystem + collections + JSON + async.

`src/main.nova`:
```nova
import std/fs
import std/json
import std/process { args, exit }
import std/fmt { println, eprintln }

@derive(ToJson, FromJson, Clone)
record Todo {
    id: Int
    title: String
    done: Bool
}

const DB_PATH = "todos.json"

async fn load_todos() -> [Todo] {
    if !fs.exists(DB_PATH).await { return [] }
    let bytes = fs.read(DB_PATH).await.unwrap_or([])
    let s = String::from_utf8(bytes).unwrap_or("[]")
    json.parse(s)
        .and_then(|v| Vec::<Todo>::from_json(v))
        .unwrap_or([])
}

async fn save_todos(todos: [Todo]) {
    let s = json.stringify_pretty(todos.to_json(), 2)
    fs.write_string(DB_PATH, s).await.unwrap()
}

async fn main() {
    let argv = args()
    match argv.get(1).map(|s| s.as_str()) {
        Some("add") => {
            let title = argv.get(2).unwrap_or("untitled".to_string())
            let mut todos = load_todos().await
            let id = todos.iter().map(|t| t.id).max().unwrap_or(0) + 1
            todos.push(Todo { id, title, done: false })
            save_todos(todos).await
            println("added: ${id}")
        }
        Some("list") => {
            for todo in load_todos().await {
                let mark = if todo.done { "[x]" } else { "[ ]" }
                println("${mark} ${todo.id}: ${todo.title}")
            }
        }
        Some("done") => {
            let id = argv.get(2).and_then(|s| s.parse::<Int>().ok()).unwrap_or(0)
            let mut todos = load_todos().await
            for todo in &mut todos {
                if todo.id == id { todo.done = true }
            }
            save_todos(todos).await
        }
        _ => {
            eprintln("usage: todo {add <title> | list | done <id>}")
            exit(1)
        }
    }
}
```

**Gate:** Full CLI cycle works (add → list → done → list).

---

## 5. `05-json-api` (Phase 2 gate — benchmark)

**Goal:** Combine HTTP + JSON + state for benchmarking vs Bun.

**AMENDED 2026-09-03 (branch `phase-2-gate-benchmark`): this listing is
written in a Nova that does not exist, and its own benchmark methodology
does not run on this project's development host.** Measurably absent from
the language, each needed by the listing below: `@derive` (used here as
`@derive(ToJson, FromJson, Clone)`) is not implemented —
`nova-spec/20-STDLIB.md:548` calls it "a compiler builtin (Phase 2)",
deferred rather than shipped; `Map` has `keys()` and no `values()`
(`std/collections/lib.nova`), and the listing below calls `users.values()`;
the language has no String-to-number conversion reachable from user code —
`req.params.get("id").parse::<Int>()` below needs one, and the one
conversion of that shape that exists, `str_to_float`, is `STD_ONLY`,
reachable only from within `std/` modules, not from a program shaped like
this one; the `Handler` type this listing's `http.Server.get`/`.post` need
is `P0001` — `nova-spec/20-STDLIB.md:504`'s own
`pub type Handler = async fn(Request) -> Response` does not parse; and
there is no `?` operator and no turbofish, both used twice below
(`req.body_json::<User>()?`, `req.params.get("id").parse::<Int>()?`). Its
`BENCHMARK.md` destination below is unsatisfiable until this example
exists — `examples/` holds `01-hello-world`, `02-fibonacci` and
`03-producer-consumer`, no `05-json-api` — so the measured number this gate
asks for lives in `docs/benchmarks/` instead. Its own benchmark
methodology, `wrk -t8 -c200 -d30s`, does not run on this project's Windows
development host either: of `wrk`, `oha`, `bombardier`, `hey`, `ab`, `k6`
and `vegeta`, none is installed there, and `wrk` itself is POSIX-only.
`crates/nova-bench-http` and `docs/benchmarks/` exist because of that
gap — see `docs/benchmarks/README.md` for the procedure used instead and
`docs/benchmarks/http-fixed-response.md` for the one number it has
produced. The listing below stays as written, as the aspiration it always
was.

**AMENDED 2026-09-10 (branch `examples-05-json-api`): the example now exists,
and the gate this section names is still NOT met.** `examples/05-json-api/`
holds `src/main.nova`, a `README.md` and a `BENCHMARK.md`. It serves the same
three routes the listing below describes — `GET /users`, `POST /users` and
`GET /users/:id` — written in the language that exists rather than the one the
listing assumes: routing is a `match` over `req.method` and
`req.path.split("/")`, `impl FromJson for User` is written out by hand, and
response bodies are interpolated strings rather than `Map`-backed
`JsonValue`s, because `Map` iteration order is seeded per process and a test
over JSON text would flake between runs. **The decision the amendment above
records — that the listing below stays as written, as the aspiration it always
was — is not reopened here.** Each substitution and the diagnostic that
established it are in
`docs/superpowers/specs/2026-09-10-examples-05-json-api-design.md` section 3.

**The `BENCHMARK.md` sentence in the amendment above has fallen in both
halves.** `examples/05-json-api` exists, so `examples/` no longer holds only
the three folders that sentence lists; and `examples/05-json-api/BENCHMARK.md`
exists and carries the figure, so the measured number this gate asks for no
longer lives in `docs/benchmarks/` instead. `docs/benchmarks/` keeps its own
separate figure for `std/http`'s read-and-parse path, which is a different
subject and not comparable to this one — `BENCHMARK.md` says why, and
`docs/benchmarks/` remains the destination `00-MASTER-SPEC.md` section 3 asks
for.

**Measured 2026-09-10, and the gate is NOT met.** Against `/users`, the
endpoint this section's own methodology names, at a ten-user collection:
**455.5 req/sec** over a 494-byte body, no errors. `00-MASTER-SPEC.md` section
3's Phase 2 gate asks for 10k+, so this is short by a factor of roughly
twenty-two, and no record may read this figure as the gate being reached. Two
other collection sizes were taken beside it — empty at 3100.6 and twenty users
at 232.1 — and cost is linear in response bytes at roughly four microseconds
each; a quadratic-accumulation hypothesis was tested and refuted. Backend,
runtime profile, generator settings and route all belong to the figure and are
recorded beside it in `examples/05-json-api/BENCHMARK.md`, which is what to
read before citing it. **This section's own criterion, a ratio of at least 1.0
against Bun, is still unmeasured** — but Bun 1.3.0 is installed on this
project's development host, so that half is measurable rather than blocked,
which is a weaker thing to inherit than the `wrk` gap the amendment above
records.

**Correction to the amendment above: a String-to-number conversion IS
reachable from user code, and this increment did not add it.** That
amendment's evidence is sound as far as it goes — `str_to_float` is
`Builtin::STD_ONLY`, and so is `char_to_int`, both verified inside that
array's real bounds in `crates/nova-resolver/src/lib.rs` — but the conclusion
does not follow from it. `std/json` exposes `pub fn parse` and
`pub trait FromJson` with an `impl FromJson for Int`, so `parse("42")` followed
by `Int::from_json(v)` yields `42` from ordinary user code. That is what
`examples/05-json-api`'s `path_id` does, and
`json_api_example_serves_its_routes` in `crates/nova-cli/tests/run_tests.rs`
drives it both ways, through `GET /users/1` and through `GET /users/zz`.
**That route existed before this increment and went unnoticed; no credit for
adding it belongs here.** A hand-rolled digit walk over `String::chars()` is a
second route — more code, and it repeats the range check `Int::from_json`
already makes — so neither should be called *the* route. What the amendment
above has right, narrower than what it wrote: `parse::<Int>()` as the listing
spells it does not exist, and neither does turbofish.

**On struct update syntax this amendment corrects nothing, because the
amendment above claims nothing about it.** Checked against that amendment's
own text rather than inherited from a summary of it: the features it names as
measurably absent, each needed by the listing, are `@derive`, `Map::values()`,
a String-to-number conversion, the `Handler` type alias, `?` and turbofish.
Struct update syntax is not among them, and it works — the listing below writes
`User { id, ..user }`, and `tests/runtime/records.nova` executes
`Point { x: 100, ..q }` against a golden `r = (100, 24)`, so the overridden
field and the inherited one are both driven by a fixture that runs. If a later
increment amends that list, re-read it rather than this sentence. Two tracked
records do get it wrong, and each is corrected where it sits rather than here:
`CHANGELOG.md`'s `[0.2.0-alpha.2]` prose names struct update syntax among what
the language lacks, corrected there under `[Unreleased]`; and this increment's
own design document says *this* amendment listed it, corrected at both of its
sites. That is the population searched — tracked files, both spellings
(hyphenated and not), flattened first because this repo's prose wraps — and not
a claim that no other record says it.

**What the amendment above still has right, and this increment left open.**
`@derive` is not implemented — an unknown attribute is `E0082`, and the message
lists what the resolver's `KNOWN_ATTRIBUTES` holds — `test` alone at this
amendment's date, so read that constant in `crates/nova-resolver/src/lib.rs`
rather than this sentence. `Map` has `keys()` and no `values()`
(`std/collections/lib.nova`), so this example walks
ids ascending from 1 instead of iterating values. The `Handler` type alias
still does not parse, so there is nothing for `Server.get`/`.post` to be built
on. There is no `?` operator. None of those moved here.

`src/main.nova`:
```nova
import std/http
import std/json
import std/sync { Mutex }
import std/collections { Map }
import std/log

@derive(ToJson, FromJson, Clone)
record User {
    id: Int
    name: String
    email: String
}

record AppState {
    users: Mutex<Map<Int, User>>
    next_id: Mutex<Int>
}

async fn main() {
    log.init()

    let state = AppState {
        users: Mutex.new(Map.new()),
        next_id: Mutex.new(1),
    }

    let app = http.Server.new()
        .get("/users", async |_| {
            let users = state.users.lock().await
            http.Response.json(users.values().to_json())
        })
        .post("/users", async |req| {
            let user = req.body_json::<User>()?
            let id = {
                let mut next = state.next_id.lock().await
                let id = *next
                *next += 1
                id
            }
            let with_id = User { id, ..user }
            state.users.lock().await.insert(id, with_id.clone())
            http.Response.json(with_id).status(201)
        })
        .get("/users/:id", async |req| {
            let id = req.params.get("id").parse::<Int>()?
            match state.users.lock().await.get(id) {
                Some(u) => http.Response.json(u),
                None => http.Response.status(404).text("not found"),
            }
        })

    log.info("listening on :3000")
    app.listen("0.0.0.0:3000").await.unwrap()
}
```

**Gate:** Benchmark vs Bun on same hardware shows ≥ 1.0x req/sec ratio. Document numbers in `examples/05-json-api/BENCHMARK.md`.

Benchmark methodology:
- `wrk -t8 -c200 -d30s http://localhost:3000/users`
- Same hardware, same kernel tuning
- Cold and warm runs
- Record p50, p95, p99 latency + req/sec

---

## 6. `06-counter-spa` (Phase 4 gate)

**Goal:** Smallest WASM frontend.

`src/main.nova`:
```nova
import nova/ui
import nova/ui/html { div, button, text }

component Counter() {
    let count = signal(0)

    view {
        div(class: "counter") {
            text("Count: ${count.value}")
            button(on_click: || count.update(|n| n + 1)) {
                text("+")
            }
            button(on_click: || count.update(|n| n - 1)) {
                text("-")
            }
        }
    }
}

fn main() {
    ui.mount(Counter, "#app")
}
```

`index.html`:
```html
<!DOCTYPE html>
<html>
<head><meta charset="utf-8"><title>Counter</title></head>
<body>
  <div id="app"></div>
  <script type="module" src="/app.js"></script>
</body>
</html>
```

**Gate:** `nova dev` opens browser, counter responds to clicks. Bundle size < 50 KB gzipped.

---

## 7. `07-fullstack-blog` (Phase 4 gate)

**Goal:** End-to-end full-stack: SSR + hydration + DB + auth.

Structure:
```
07-fullstack-blog/
├── nova.toml
├── src/
│   ├── main.nova          # entry point
│   ├── routes/
│   │   ├── home.nova
│   │   ├── post.nova
│   │   ├── login.nova
│   │   └── admin.nova
│   ├── components/
│   │   ├── header.nova
│   │   └── post_card.nova
│   ├── db.nova            # SQLite via std/db
│   └── auth.nova
├── public/
│   └── styles.css
├── tests/
└── README.md
```

Features required:
- Home page lists posts (SSR)
- Post detail page (SSR + hydration for comments)
- Login form (client-side validation, server-side auth)
- Admin: create/edit/delete posts (protected by session)
- SQLite for storage
- Cookie-based sessions
- CSRF protection

**Gate:** All pages work, Lighthouse Performance > 95, full CRUD via admin UI, no XSS via untrusted input.

---

## 8. Additional Examples (post-Phase 6)

These extend the canonical set after 1.0 release:

| Folder | Demonstrates |
|---|---|
| `08-todomvc/` | Canonical TodoMVC for direct comparison with frameworks |
| `09-realworld-app/` | RealWorld spec implementation (auth, CRUD, routing, comments) |
| `10-game-2048/` | WASM perf showcase, animation |
| `11-cli-todo-tui/` | Terminal UI app (via std/tui added in v1.1) |
| `12-grpc-server/` | gRPC service (via std/grpc in v1.2) |
| `13-graphql-api/` | GraphQL server example |
| `14-microservice/` | Production microservice template with health, metrics, tracing |
| `15-static-blog/` | SSG-only output (Markdown → HTML at build time) |

---

## 9. Per-example README Template

Every example folder must have a `README.md` that follows this template:

```markdown
# <example name>

<one-line description>

## What this demonstrates
- Feature 1
- Feature 2

## Run it
\`\`\`bash
cd examples/<name>
nova run
\`\`\`

## Expected output
\`\`\`
...
\`\`\`

## Notes
<Anything tricky, links to relevant spec sections>
```

**Recorded 2026-09-01 (branch `std-http-parsing`), not fixed here: no
example on disk has this README yet.** Checked directly against every entry
under `examples/` rather than assumed from one — `01-hello-world`,
`02-fibonacci` and `03-producer-consumer` each lack a `README.md` entirely.
An earlier draft of the `std/http` design work named only the third of
these, which was true of it and misleading about the other two.

**AMENDED 2026-09-10 (branch `examples-05-json-api`): one example on disk now
follows this template, and the rest still do not.**
`examples/05-json-api/README.md` follows it — name, one-line description, What
this demonstrates, Run it, Expected output, Notes. **The population changed,
not the check.** The 2026-09-01 record above was measured against every entry
under `examples/` at its own date and was right about all of them; writing one
README added a fourth entry rather than correcting a wrong reading of the three
it names, and `01-hello-world`, `02-fibonacci` and `03-producer-consumer` still
have no `README.md` at all. Nothing here brought them into line, and a reader
should not take this note as saying otherwise. The durable check is
`ls examples/*/README.md` against `ls -d examples/*/`, not either paragraph.

**That 2026-09-01 sentence is also this project's clearest instance of the
line-oriented sweep hazard, and it belongs here rather than only in a ledger.**
Measured against this file as it stood before this amendment:
`grep -c 'no example on disk'` returned **0**, because "no" ends one line of
that sentence and "example" begins the next. Flattened — line endings collapsed
to spaces — the phrase was there exactly once. So a line-oriented sweep of this
very file reported the claim absent while the claim was present and stale.

**Writing that measurement down changed it, which is the other half of the
lesson.** The paragraph above quotes the phrase on a single line, so the same
`grep -c` returns **1** against this file now, and it lands on that paragraph
rather than on the sentence the paragraph is about — re-measured after the edit,
not predicted. A count taken of a document and then written into that document
stops being a count of it. Two things to carry: flatten before concluding a
wrapped claim is absent, since a `grep` miss over wrapped prose is not evidence
the claim is not there; and re-measure rather than trusting a figure recorded
inside the file it counts.

---

## 10. Test Coverage per Example

Every example must include:
- `tests/` folder with at least one integration test
- A line in the workspace CI matrix that runs `nova test` in that directory
- A line that runs the example end-to-end and asserts expected behavior

CI snippet (in `.github/workflows/ci.yml`):
```yaml
examples:
  runs-on: ubuntu-latest
  strategy:
    matrix:
      example:
        - 01-hello-world
        - 02-fibonacci
        - 03-http-server
        - 04-todo-cli
        - 05-json-api
        - 06-counter-spa
        - 07-fullstack-blog
  steps:
    - uses: actions/checkout@v4
    - run: cargo build --release -p nova-cli
    - run: ./target/release/nova test
      working-directory: examples/${{ matrix.example }}
```
