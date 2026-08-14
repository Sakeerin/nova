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
std/bytes        immutable byte buffers
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

**AMENDED 2026-08-12 (branch `byte-type`):** `std/bytes` shipped as a `STD_MODULES` entry this branch
(`docs/superpowers/specs/2026-08-12-byte-type-design.md`) but this index still listed only the six
prior entries — added above. Like `std/strings`, `std/regex`, `std/process` and `std/net`, it has no
dedicated numbered section below yet.

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
    fn next(mut self) -> Option<Self::Item>   // `::`, not `.` — see docs/adr/0006
    // `mut self`, not `self`: the mutable-receiver rule now covers trait
    // methods (ADR 0005 §1's migration path is complete), and `next` is the
    // first such method — it is what forced that gap closed. Load-bearing, not
    // stylistic: with plain `self` on both sides, `VecIter::next`'s body does
    // not compile (`E0060`, cannot assign to a field of immutable self). The
    // consequence for callers is that an iterator must be held in a `mut`
    // binding or arrive as a `mut` parameter.
    //
    // `next` is the ONLY `mut self` method here, and the note above applies to
    // it alone. The six defaults below take plain `self` — see ADR 0007 §2 for
    // why, and for what that gives up.

    // Adapters: lazy, nothing is consumed until a consumer runs.
    fn map<U>(self, f: fn(Self::Item) -> U) -> MapIter<Self, U>
    fn filter(self, keep: fn(Self::Item) -> Bool) -> FilterIter<Self>

    // Consumers: each drives the iterator to exhaustion, or short-circuits.
    fn fold<A>(self, init: A, f: fn(A, Self::Item) -> A) -> A
    fn count(self) -> Int
    fn any(self, p: fn(Self::Item) -> Bool) -> Bool
    fn collect(self) -> Vec<Self::Item>
}

// The adapters `map` and `filter` return, in std/core beside the trait. Each
// carries a bound on its type parameter purely so the field type may name
// `I::Item` — a resolution scope, not a constraint (ADR 0007 §1).
pub record MapIter<I: Iterator, U> { it: I, f: fn(I::Item) -> U }
pub record FilterIter<I: Iterator> { it: I, keep: fn(I::Item) -> Bool }

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

// AMENDED 2026-08-12 (branch `byte-type`): the `&mut [u8]`/`&[u8]` buffer
// parameters below are buffer-FILLING, which needs references -- `&Int` is
// `E0900`, measured, and Nova does not have them. This is not merely
// unimplemented: the byte-type design spec settles Nova's byte I/O as
// buffer-RETURNING instead, so references are off this roadmap permanently
// (docs/superpowers/specs/2026-08-12-byte-type-design.md §1, §6 -- nothing
// in the remaining increments needs them). `Bytes` (a scanned `{len, ptr}`
// header over a GC leaf buffer, `std/bytes`) is the concrete buffer type:
// `std/fs`'s `read`/`write` below in §5 already ship against it --
// `Result<Bytes, IoError>` and `content: Bytes`, not the `[u8]` shown there
// -- and `open`/`File`/these two traits do too, now that all three are
// built (2026-08-14, both amendments below). See docs/adr/0011-io-error-
// kinds.md for the §5 deviation this left, since closed.
//
// AMENDED 2026-08-14 (branch `read-write-stdio`): neither trait below
// compiles exactly as declared, and neither do `stdin`/`stdout`/`stderr`'s
// return types further down. `async fn` in a trait *declaration* is
// `E0900`, measured directly, so `Read` and `Write` ship instead as
// `fn ... -> Future<T>`: calling an `async fn` without `.await` produces its
// `Future` without running it, so a plain, non-`async` `fn` can still
// return one, unawaited. Separately, `impl Trait` in return position does
// not parse at all (`P0001`), so `stdin`/`stdout`/`stderr` return the
// concrete, fieldless records `Stdin`/`Stdout`/`Stderr` instead of `impl
// Read`/`impl Write` -- each has a plain, lowercase-named `fn` constructor,
// not the trait-typed return shown below. See `std/io/lib.nova` for the
// shipped signatures and docs/adr/0011-io-error-kinds.md decision 2 for the
// deviation this narrowed at the time, to `open`/`File` only -- closed since,
// by the amendment §5 below records.
//
// AMENDED 2026-08-14 (branch `file-open-openoptions`): two clauses the note
// above left unstated, both true of what it shipped. First, `Write::write`'s
// `-> Result<Int, IoError>` implies a count without ever saying a *short*
// write is legal -- it is: `write` is plain `write`, not `write_all`, so a
// caller that needs every byte of `buf` sent must loop on the returned count
// itself (stated on the Nova side by `std/io/lib.nova`'s own doc comment on
// `Write::write`). Second, `nova_rt_io_stdin_read` allocates its caller's
// `max` eagerly, before any read happens, so a generous *ceiling* is charged
// in full even when the real read returns far fewer bytes or fails outright
// -- `read(max)` expresses an upper bound, not a request for exactly that
// many bytes, and paying for the bound up front was not stated anywhere a
// caller would read it before this note. `crates/nova-runtime/src/file.rs`'s
// `nova_rt_file_read` repeats the identical eager-allocation shape for
// `File::read` below, once `File` existed to have one.
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

// AMENDED 2026-08-11 (branch `std-fs-strings`): `AlreadyExists` and
// `InvalidData` added to the list below, which originally had six variants
// and was network-flavoured; on a filesystem, a `create_dir` on an existing
// path and a non-UTF-8 `read_to_string` would both otherwise collapse into
// `Other`, forcing user code to string-match the platform-specific `message`
// to tell them apart. See docs/adr/0011-io-error-kinds.md.
pub type IoErrorKind =
    | NotFound
    | PermissionDenied
    | AlreadyExists
    | InvalidData
    | Interrupted
    | TimedOut
    | ConnectionRefused
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

// AMENDED 2026-08-14 (branch `file-open-openoptions`): `OpenOptions` is
// `open`'s parameter type above and has been since this section first named
// it, but this document never defined it as a type until now -- the design
// spec this increment implements
// (docs/superpowers/specs/2026-08-14-file-open-and-openoptions-design.md,
// §2's record definition, and §1's own words: "This spec is the first
// document to define it") already had; this note is this document's own
// catch-up to that, not a claim that no document anywhere ever defined it.
// It ships as a record of six `Bool` flags, in the order `open` forwards
// them to the runtime: read, write, append, truncate, create, create_new.
// `impl Default` sets every flag false; that value alone is not a legal
// `open` argument
// (`std::fs::OpenOptions` requires at least one of read/write/append), so it
// exists as a base for field assignment, not for direct use. Three named
// constructors cover the common cases instead: `reading()`, `writing()`
// (write + create + truncate) and `appending()` (append + create). There is
// no chainable builder: a receiver-mutating method cannot be called on a
// temporary (`E0060`, measured), so `OpenOptions::reading().with_write()`
// does not compile in this language -- an exotic combination starts from
// `OpenOptions::default()` and assigns fields on a `let mut` binding instead.
//
// `File` below is written `{ /* opaque */ }` because nothing outside
// `std/fs` should rely on its shape, but it is not opaque to the language
// itself: it ships as `File { fd: Int }`, an `Int` key into a runtime-owned
// table of open OS handles (`crates/nova-runtime/src/file.rs`), not an OS
// file descriptor number. Nova has no destructors. The code block below
// shows no `close` at all -- it is inherent (`pub async fn close(self) ->
// Result<(), IoError>`), not a method of `Read` or `Write`, so it does not
// appear in either `impl` -- and it is the only release mechanism: a `File`
// that is never closed leaks its descriptor for the life of the process, on
// every platform, deliberately (docs/adr/0012-file-descriptor-lifecycle.md).
// `close` is idempotent, and any other operation on a closed, stale, or forged
// handle (Nova has no field privacy, so `File { fd: 9999 }`, naming no file
// this module ever opened, is ordinary, legal code) is an ordinary
// `IoError { kind: Other }`, never a panic: absence from the handle table is
// what closedness *is*, so a closed, a stale, and a forged handle all get
// identical treatment. See docs/adr/0011-io-error-kinds.md decision 2, whose
// deviation `open`/`File` were the last item of, now closed.
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
fn out_of_bounds_panics() {
    let xs = [1, 2, 3]
    let _ = xs[9]
}

@bench
fn bench_fib() {
    fib(20)
}

pub fn assert(cond: Bool, msg: String) { if !cond { panic!(msg) } }
pub fn assert_eq<T: Eq + Debug>(a: T, b: T) { ... }
pub fn assert_ne<T: Eq + Debug>(a: T, b: T) { ... }
```

**`should_panic` needs a *checked* panic — integer division by zero is not
one.** An earlier revision of this example used `let _ = 1 / 0`. Measured on
this compiler, that is a hard trap (an illegal-instruction trap, with no
message and nothing written to stderr, that never reaches `abort()` at all),
not a call into the runtime's panic path. `@test(should_panic)` passes only when the test process both exits
nonzero *and* stderr contains a `nova: panic:` line; a division by zero
satisfies the first and not the second, so under this rule it **fails**
`should_panic` rather than passing it. Array-index-out-of-bounds panics
through a checked runtime path instead (`nova_rt_check_bounds`) and does
satisfy `should_panic`, which is why it replaces the earlier example above.
See `docs/adr/0008-attributes-and-test-isolation.md` §2 for the full
classification rule and the measurement behind it.

`assert_throws` is **not implemented, and cannot be, under this compiler's
test isolation.** The runtime has no unwinding anywhere — a panic calls
`std::process::abort()` directly (`nova_rt_panic_str`) — so there is no way
to catch one, inspect its value, and resume the calling test.
`@test(should_panic)` is the only supported way to assert that a test
panics; there is no supported way to assert *what* it panics with. See
`docs/adr/0008-attributes-and-test-isolation.md` §2.

`nova test` discovers `@test` functions by walking the entry file's
(`src/main.nova`) `import` graph — the same set of modules an ordinary build
of that entry point would pull in — **not** by scanning the package for
every file that defines one. A `@test` function that lives in a module
nothing imports is therefore never discovered: it does not run, is not
counted, and `nova test` still reports `test result: ok` having executed
zero of it (`docs/adr/0008-attributes-and-test-isolation.md` §1 records this
as a known gap). Each discovered test runs to completion, in its own
process, before starting the next — **not in parallel**. Isolation, not
speed, is why: a panic aborts its process with no
unwinding, so a runner sharing one process across tests (concurrently or
not) could not survive one failing test to report on the rest.

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
