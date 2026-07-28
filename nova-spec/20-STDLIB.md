# 20 — Standard Library Specification

> Phase: 2 (core), 4 (frontend)
> Location: `std/` (written in Nova, with FFI to `nova-runtime` Rust crate)

---

## 1. Module Index (v1.0)

```
std/core         primitives, Option, Result, traits
std/fmt          formatting (Display, Debug, write)
std/io           I/O abstractions
std/fs           filesystem
std/net          TCP/UDP/Unix sockets
std/http         HTTP client + server
std/json         JSON parse/serialize (codec-based)
std/crypto       hashing, AEAD, random
std/time         Instant, Duration, sleep
std/log          structured logging
std/test         test framework
std/collections  Vec, Map, Set, Queue
std/strings      Unicode-aware string ops
std/regex        PCRE-compatible
std/process      spawn, env, args
std/sync         Mutex, RwLock, channel, atomic
std/task         async runtime primitives
```

---

## 2. `std/core` — Foundational Types

```nova
module std.core

// === Result === //
pub type Result<T, E> =
  | Ok(T)
  | Err(E)

impl<T, E> Result<T, E> {
    pub fn is_ok(self) -> Bool { match self { Ok(_) => true, Err(_) => false } }
    pub fn is_err(self) -> Bool { !self.is_ok() }
    pub fn map<U>(self, f: fn(T) -> U) -> Result<U, E> { ... }
    pub fn map_err<F>(self, f: fn(E) -> F) -> Result<T, F> { ... }
    pub fn and_then<U>(self, f: fn(T) -> Result<U, E>) -> Result<U, E> { ... }
    pub fn unwrap(self) -> T { ... }   // panics if Err
    pub fn unwrap_or(self, default: T) -> T { ... }
}

// === Option === //
pub type Option<T> =
  | Some(T)
  | None

impl<T> Option<T> {
    pub fn is_some(self) -> Bool { ... }
    pub fn is_none(self) -> Bool { ... }
    pub fn map<U>(self, f: fn(T) -> U) -> Option<U> { ... }
    pub fn unwrap_or(self, default: T) -> T { ... }
    pub fn ok_or<E>(self, err: E) -> Result<T, E> { ... }
}

// === Foundational Traits === //
pub trait Display {
    fn fmt(self) -> String
}

pub trait Debug {
    fn dbg(self) -> String
}

pub trait Eq {
    fn eq(self, other: Self) -> Bool
    fn ne(self, other: Self) -> Bool { !self.eq(other) }
}

pub trait Ord: Eq {
    fn cmp(self, other: Self) -> Ordering
}

pub type Ordering = | Less | Equal | Greater

pub trait Clone { fn clone(self) -> Self }
pub trait Copy: Clone {}      // marker
pub trait Default { fn default() -> Self }

pub trait Hash {
    fn hash<H: Hasher>(self, h: H)
}

pub trait Iterator {
    type Item
    fn next(self) -> Option<Self::Item>   // `::`, not `.` — see docs/adr/0006
    // The receiver becomes `mut self` once the mutable-receiver rule covers
    // trait methods (ADR 0005 §1's migration path); `next` is the first such
    // method and is what forces that gap closed.
    // default methods: map, filter, collect, fold, ...
}

// === Primitives have inherent impls === //
// Int, Float, Bool, Char, String all impl: Display, Debug, Eq, Clone, Hash
```

---

## 3. `std/fmt`

```nova
module std.fmt

pub fn print(s: String) { ... }
pub fn println(s: String) { ... }
pub fn eprint(s: String) { ... }
pub fn eprintln(s: String) { ... }

// Format builder for Display impls
pub record Formatter { ... }

// String interpolation desugars to call:
pub fn format(parts: [FormatPart]) -> String { ... }

pub type FormatPart = | Lit(String) | Val(String)
```

---

## 4. `std/io`

```nova
module std.io

pub trait Read {
    async fn read(self, buf: &mut [u8]) -> Result<Int, IoError>
    async fn read_to_end(self, buf: &mut [u8]) -> Result<Int, IoError> { /* default */ }
}

pub trait Write {
    async fn write(self, buf: &[u8]) -> Result<Int, IoError>
    async fn flush(self) -> Result<(), IoError>
}

pub record IoError {
    pub kind: IoErrorKind
    pub message: String
}

pub type IoErrorKind =
    | NotFound
    | PermissionDenied
    | ConnectionRefused
    | TimedOut
    | Interrupted
    | Other

pub fn stdin() -> impl Read
pub fn stdout() -> impl Write
pub fn stderr() -> impl Write
```

---

## 5. `std/fs`

```nova
module std.fs

pub async fn read(path: String) -> Result<[u8], IoError>
pub async fn read_to_string(path: String) -> Result<String, IoError>
pub async fn write(path: String, content: [u8]) -> Result<(), IoError>
pub async fn write_string(path: String, content: String) -> Result<(), IoError>
pub async fn exists(path: String) -> Bool
pub async fn create_dir(path: String) -> Result<(), IoError>
pub async fn create_dir_all(path: String) -> Result<(), IoError>
pub async fn remove_file(path: String) -> Result<(), IoError>
pub async fn remove_dir_all(path: String) -> Result<(), IoError>
pub async fn read_dir(path: String) -> Result<[DirEntry], IoError>

pub record DirEntry {
    pub name: String
    pub path: String
    pub is_file: Bool
    pub is_dir: Bool
}

pub async fn open(path: String, options: OpenOptions) -> Result<File, IoError>

pub record File { /* opaque */ }
impl Read for File { ... }
impl Write for File { ... }
```

---

## 6. `std/http` (server + client)

```nova
module std.http

// === Client === //
pub async fn get(url: String) -> Result<Response, HttpError>
pub async fn post(url: String, body: [u8]) -> Result<Response, HttpError>

pub record Request {
    pub method: Method
    pub url: String
    pub headers: Headers
    pub body: [u8]
}

pub record Response {
    pub status: Int
    pub headers: Headers
    pub body: [u8]
}

impl Response {
    pub fn json<T: FromJson>(self) -> Result<T, JsonError> { ... }
    pub fn text(self) -> Result<String, IoError> { ... }
    pub fn bytes(self) -> [u8] { self.body }
}

pub type Method = | Get | Post | Put | Delete | Patch | Head | Options

// === Server === //
pub record Server {
    /* opaque */
}

impl Server {
    pub fn new() -> Server
    pub fn get(self, path: String, handler: Handler) -> Self
    pub fn post(self, path: String, handler: Handler) -> Self
    pub fn put(self, path: String, handler: Handler) -> Self
    pub fn delete(self, path: String, handler: Handler) -> Self
    pub fn route(self, method: Method, path: String, handler: Handler) -> Self
    pub fn use_middleware(self, mw: Middleware) -> Self
    pub async fn listen(self, addr: String) -> Result<(), IoError>
}

// Handler is async fn(Request) -> Response (or Result)
pub type Handler = async fn(Request) -> Response

pub type Middleware = async fn(Request, Next) -> Response

// Path params: app.get("/users/:id", |req| { req.params.get("id") })

pub record HttpError { ... }
```

---

## 7. `std/json` (codec-based, type-safe)

```nova
module std.json

pub trait ToJson { fn to_json(self) -> JsonValue }
pub trait FromJson { fn from_json(v: JsonValue) -> Result<Self, JsonError> }

pub type JsonValue =
    | Null
    | Bool(Bool)
    | Number(Float)
    | String(String)
    | Array([JsonValue])
    | Object(Map<String, JsonValue>)

pub fn parse(s: String) -> Result<JsonValue, JsonError>
pub fn stringify(v: JsonValue) -> String
pub fn stringify_pretty(v: JsonValue, indent: Int) -> String

// Auto-derive
@derive(ToJson, FromJson)
record User {
    id: Int
    name: String
    email: String
}

let u = User { id: 1, name: "Deen", email: "x@y.z" }
let s = json.stringify(u.to_json())  // {"id":1,"name":"Deen","email":"x@y.z"}
let u2 = User::from_json(json.parse(s)?)?
```

`@derive` for ToJson/FromJson is implemented as a compiler builtin (Phase 2).

---

## 8. `std/crypto`

```nova
module std.crypto

// Hashes
pub fn sha256(data: [u8]) -> [u8; 32]
pub fn sha512(data: [u8]) -> [u8; 64]
pub fn blake3(data: [u8]) -> [u8; 32]

// HMAC
pub fn hmac_sha256(key: [u8], data: [u8]) -> [u8; 32]

// AEAD
pub record Aead { /* opaque */ }
impl Aead {
    pub fn aes_gcm_256(key: [u8; 32]) -> Result<Aead, CryptoError>
    pub fn chacha20_poly1305(key: [u8; 32]) -> Result<Aead, CryptoError>
    pub fn encrypt(self, nonce: [u8; 12], aad: [u8], plaintext: [u8]) -> [u8]
    pub fn decrypt(self, nonce: [u8; 12], aad: [u8], ciphertext: [u8]) -> Result<[u8], CryptoError>
}

// Random
pub fn random_bytes(n: Int) -> [u8]
pub fn random_int(min: Int, max: Int) -> Int
```

Backed by `ring` in nova-runtime.

---

## 9. `std/time`

```nova
module std.time

pub record Instant { /* opaque */ }
impl Instant {
    pub fn now() -> Instant
    pub fn elapsed(self) -> Duration
    pub fn duration_since(self, earlier: Instant) -> Duration
}

pub record Duration { /* opaque */ }
impl Duration {
    pub fn from_secs(s: Int) -> Duration
    pub fn from_millis(ms: Int) -> Duration
    pub fn from_micros(us: Int) -> Duration
    pub fn as_secs(self) -> Int
    pub fn as_millis(self) -> Int
}

pub async fn sleep(d: Duration)
pub async fn timeout<T>(d: Duration, fut: Future<T>) -> Result<T, TimeoutError>
```

---

## 10. `std/log`

```nova
module std.log

pub fn trace(msg: String)
pub fn debug(msg: String)
pub fn info(msg: String)
pub fn warn(msg: String)
pub fn error(msg: String)

pub fn init() { /* default logger to stderr, format: JSON or human based on TTY */ }
pub fn init_with(config: LogConfig) { ... }

pub record LogConfig {
    pub level: LogLevel
    pub format: LogFormat
    pub output: LogOutput
}

pub type LogLevel = | Trace | Debug | Info | Warn | Error
pub type LogFormat = | Human | Json
pub type LogOutput = | Stderr | Stdout | File(String)
```

---

## 11. `std/test`

```nova
module std.test

@test
fn add_works() {
    assert_eq(1 + 1, 2)
}

@test(should_panic)
fn divide_by_zero_panics() {
    let _ = 1 / 0
}

@bench
fn bench_fib() {
    fib(20)
}

pub fn assert(cond: Bool, msg: String) { if !cond { panic!(msg) } }
pub fn assert_eq<T: Eq + Debug>(a: T, b: T) { ... }
pub fn assert_ne<T: Eq + Debug>(a: T, b: T) { ... }
pub fn assert_throws<F: fn() -> T, T>(f: F, expected: String) { ... }
```

`nova test` discovers all `@test` functions across the package and runs them in parallel.

---

## 12. `std/collections`

```nova
module std.collections

// Growable array
pub record Vec<T> { /* opaque */ }
impl<T> Vec<T> {
    pub fn new() -> Vec<T>
    pub fn with_capacity(n: Int) -> Vec<T>
    pub fn push(self, value: T)
    pub fn pop(self) -> Option<T>
    pub fn len(self) -> Int
    pub fn get(self, index: Int) -> Option<&T>
    pub fn iter(self) -> impl Iterator<Item = &T>
    // ... etc
}

// Hash map
pub record Map<K, V> { /* opaque */ }
impl<K: Hash + Eq, V> Map<K, V> {
    pub fn new() -> Map<K, V>
    pub fn insert(self, key: K, value: V) -> Option<V>
    pub fn get(self, key: K) -> Option<&V>
    pub fn remove(self, key: K) -> Option<V>
    pub fn contains_key(self, key: K) -> Bool
    pub fn len(self) -> Int
    pub fn iter(self) -> impl Iterator<Item = (&K, &V)>
    // ... etc
}

pub record Set<T> { /* opaque */ }
impl<T: Hash + Eq> Set<T> {
    pub fn new() -> Set<T>
    // ... etc
}

pub record Queue<T> { /* opaque */ }
pub record Deque<T> { /* opaque */ }
```

`Map`'s and `Set`'s `Hash + Eq` requirement is written on the **`impl` block**,
not on the record's own type parameters. A trait bound on a *record* (or sum)
type parameter is rejected with `E0900`: bounds are discharged at
monomorphization, which walks function and impl instances, so a bound in that
position would enforce nothing. Writing it on the `impl` is not a workaround but
the enforced spelling — it is what `std/collections` does.

The enforcement is per method instantiation, not on the type itself. Nova has
no field privacy, so a record literal reaches the type without going through
any method: `let m: Map<NotHashable, Int> = Map { len: 0, used: 0, keys: [],
vals: [], state: [] }` compiles and runs. What the bound on the `impl` actually
rejects, with `E0013` at monomorphization, is instantiating a *method* —
`Map::new()`, `insert`, `get`, … — with a key that is not `Hash + Eq`.

---

## 13. `std/sync` & `std/task`

```nova
module std.sync

pub record Mutex<T> { /* opaque */ }
impl<T> Mutex<T> {
    pub fn new(value: T) -> Mutex<T>
    pub async fn lock(self) -> MutexGuard<T>
}

pub record Channel<T> { /* opaque */ }
pub fn channel<T>(buffer: Int) -> (Sender<T>, Receiver<T>)

module std.task

pub fn spawn<T>(fut: Future<T>) -> JoinHandle<T>
pub fn spawn_blocking<T>(f: fn() -> T) -> JoinHandle<T>

pub record JoinHandle<T> { ... }
impl<T> JoinHandle<T> {
    pub async fn join(self) -> T
    pub fn cancel(self)
}
```

---

## 14. Implementation Strategy

For each module:
1. Define API in Nova (`std/<module>/lib.nova`)
2. Implement runtime hooks in `crates/nova-runtime/src/<module>.rs` exposed as `extern "C"`
3. Nova code calls runtime via `extern "nova-rt"` declarations
4. Write tests in `std/<module>/tests/`

Order (from `00-MASTER-SPEC.md` Phase 2):
core → fmt/io → collections → strings → fs → time/log → task → sync → net → http → json → crypto → test

---

## 15. Documentation

- Every public item has `///` doc comment
- Examples in doc comments are tested by `nova test --doc`
- `nova doc` generates a static HTML site like rustdoc
