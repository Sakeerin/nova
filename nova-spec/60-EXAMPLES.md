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
