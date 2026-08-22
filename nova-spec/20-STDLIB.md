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

**AMENDED 2026-08-16 (branch `io-poller-std-net`): the paragraph above is now
stale for one of its four names.** `std/net` gained a dedicated section, §16,
appended after §15 rather than inserted in module-index order (see §16's own
opening note for why). `std/bytes`, `std/strings`, `std/regex` and
`std/process` still have no dedicated numbered section below.

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

**AMENDED 2026-08-19 (branch `std-fmt`): this section's four print functions
and `Formatter` remain unshipped as declared, but position 2 in
`00-MASTER-SPEC.md` §3's build order is now closed rather than skipped a
third time** (see `docs/adr/0014-stdlib-build-order-deviations.md` and
`docs/adr/0015-std-fmt-scope.md`). `std/fmt` shipped as an eleventh
`STD_MODULES` entry (`STD_MODULES` 10 → 11) carrying four methods the code
block above never named: `Int::pad(width)`, `String::pad_left(width)`,
`String::pad_right(width)`, and `Float::fixed(places)`, backed by one new
runtime intrinsic, `float_fixed` (`Builtin::STD_ONLY` 64 → 65). These close
the module's one genuinely missing capability — Nova has no `Float`→`Int`
conversion at all, so fixed-place decimal rendering was inexpressible in the
language, not merely absent from the stdlib — while padding had already been
proven expressible and hand-rolled once, in `std/time`'s now-deleted
`pad2`/`pad3`. The four print functions above (`print`/`println`/`eprint`/
`eprintln`) remain what ADR 0014 already recorded them as: compiler builtins
(`Builtin::Print`/`Println`/`EPrint`/`EPrintln`), untouched by this
increment.

**`format(parts: [FormatPart]) -> String` and `Formatter` are not shipping,
and neither is pending work — both are specifications the compiler has
overtaken, not gaps still to fill.** `FormatPart`'s `Val(String)` arm means
`format`'s parts arrive **already stringified**, so `format` is
concatenation; the compiler already lowers string interpolation to
concatenation directly, and routing it through this Nova function instead
would allocate an array to do the identical work, via a compiler change,
with nothing visible to any user — a pessimization, not a feature.
`Formatter`'s body is elided above and described only as a "Format builder
for Display impls" — there is no specified behaviour there to implement.
`Display` is `fn fmt(self) -> String` (`std/core/lib.nova:98`) and returns
a whole string in one call regardless, so any builder reachable from it is
a longer, slower spelling of interpolation: a `mut self` accumulator (ADR
0005 §1 — ten uses in shipped `std/collections`) is buildable inside `fmt`
without changing `Display`'s shape at all, but each append copies the
string accumulated so far, so it is quadratic where interpolation is a
single compiler lowering. Neither item is deferred: `format` is a
pessimization of work the compiler already does, and `Formatter` has no
specification to implement in the first place.

**Separately, and not a `std/fmt` deviation:** the `module std.fmt` line
that opens the code block above does not parse, and this is true of every
section in this document — no std module source declares a `module` line at
all, measured across `grep -n "^module " std/*/lib.nova`, now twelve
`lib.nova` files (the eleven `STD_MODULES` entries plus `std/test`, held out
of that array and seeded only under `nova test`), all returning zero hits.
§10's own 2026-08-19 amendment already recorded this same fact at ten
`STD_MODULES` entries plus `std/test` (eleven total); `std/fmt` joining
`STD_MODULES` this increment moves that count to twelve — a shift in the
count, not in the underlying fact, and §10's paragraph is not itself wrong,
only now one behind. A reader checking only this section would otherwise
conclude `std/fmt` alone fails to conform; it does not stand out, and no
section here does.

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
// built (2026-08-14, both amendments below). See
// docs/adr/0011-io-error-kinds.md for the §5 deviation this left, since
// closed.
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
//
// AMENDED 2026-08-16 (branch `io-poller-std-net`): after this increment,
// **`std/fs` suspends nowhere and `std/net` suspends everywhere** -- two
// different wrapper shapes are both live in this stdlib at once, and a
// reader who has only met one of them should know the other exists.
// `std/fs`'s `async fn`s (§5) call a plain, status-returning intrinsic with
// no `.await` inside them at all, so the underlying operation always runs to
// completion inside the first poll -- not merely because no poller existed
// yet, but because a regular file is not readiness-pollable on any of this
// project's three CI platforms the way a socket is; that is what a
// completion-based interface (IOCP, `io_uring`) exists for, and building one
// is a subsystem of its own, out of scope here (`docs/adr/0013-io-poller.md`).
// `std/net`'s wrappers (§16) instead call a *future-constructing* intrinsic
// and `.await` it -- `crates/nova-runtime/src/net.rs`'s `connect`, `read`,
// `write` and `read_timeout` are Rust-built futures with their own poll
// function, parked through the executor's new third wake source (socket
// readiness, `crates/nova-runtime/src/poll.rs`) exactly as `sleep` parks on
// a deadline -- so every one of them can genuinely suspend the calling task
// and let a sibling run, which `tests/runtime/net_interleave.nova` pins end
// to end. This is an honest consequence of files not being
// readiness-pollable, not an inconsistency to reconcile. Separately,
// `TimedOut` and `ConnectionRefused` below -- both already listed since the
// 2026-08-11 amendment, with no producer -- gain their first ones in
// `std/net`'s `connect` and `read_timeout`; see
// `crates/nova-runtime/src/fs.rs`'s own pinned-kinds comment for exactly
// which fixture pins which of the eight.
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

// AMENDED 2026-08-14, reworded 2026-08-15 (branch `file-open-openoptions`):
// `OpenOptions` is `open`'s parameter type below and has been since this
// section first named it, but this document never declared it as a type
// until now. Declared here with no `pub` on any field -- Nova has no field
// privacy at all (`File` below is the identical case), so marking a field
// `pub` or not changes nothing about who can read or construct one; the
// declaration below simply doesn't. (The 2026-08-15 reword replaced a
// clause that grounded the missing `pub` in `std/fs/lib.nova`'s current
// bytes with the language rule stated above. The declaration below was not
// touched by that reword, but it is not from 2026-08-14 either: it was
// added on 2026-08-15 (`509834e`), the 2026-08-14 amendment above having
// named the type without ever declaring it.)
pub record OpenOptions {
    read: Bool
    write: Bool
    append: Bool
    truncate: Bool
    create: Bool
    create_new: Bool
}

// AMENDED 2026-08-15 (branch `file-open-openoptions`): the two `impl`
// blocks below are added to the declared surface. The prose further down
// has named `impl Default` and the three constructors as part of what
// `std/fs` ships since this section's 2026-08-14 note, but the fence
// showed only the record -- so the section described a surface it did not
// declare. Bodies are elided (`{ ... }`) in this section's own style, the
// way `File`'s trait impls below are. This closes a gap between the prose
// and the fence, not a gap in what the language allows: Nova has no field
// privacy, so a record literal could always build an `OpenOptions`.
impl Default for OpenOptions { ... }

impl OpenOptions {
    pub fn reading() -> OpenOptions { ... }
    pub fn writing() -> OpenOptions { ... }
    pub fn appending() -> OpenOptions { ... }
}

// `impl Default` sets every flag false; that value alone is not a legal
// `open` argument (`std::fs::OpenOptions` requires at least one of
// read/write/append), so it exists as a base for field assignment, not for
// direct use. Three named constructors cover the common cases instead:
// `reading()`, `writing()` (write + create + truncate) and `appending()`
// (append + create). There is no chainable builder: a receiver-mutating
// method cannot be called on a temporary (`E0060`, measured), so
// `OpenOptions::reading().with_write()` does not compile in this language --
// an exotic combination starts from `OpenOptions::default()` and assigns
// fields on a `let mut` binding instead.
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

**AMENDED 2026-08-17 (branch `std-time`): `Instant` and `Duration` ship as
declared above** — `timeout`/`TimeoutError` followed in a later increment,
recorded in the 2026-08-18 amendment below — **and the `/* opaque */`
markers state an intent this language cannot enforce.** Both records are
`{ nanos: Int }` — nanoseconds since a single process-monotonic origin for
`Instant`, a nanosecond count for `Duration` — with no status-code boundary
at all, since neither performs I/O and `Instant::now()` is infallible.
`/* opaque */` above is aspirational, not enforced: Nova has no field
privacy (`check_record_literal` in `nova-typeck` never consults a field's
`pub` marker), so `Duration { nanos: -1_000_000_000 }` compiles from any
module and reaches `sleep` having bypassed every saturating constructor —
the identical situation `std/net`'s `TcpStream { fd: Int }` (§16) and
`std/fs`'s `File { fd: Int }` (§5) already document for their own records;
`/* opaque */` is not currently expressible in this language, for any
record. Separately, the "at least"
contract `sleep` already promises is platform-asymmetric in its
granularity, but not because the wait itself always rounds up: `select_timeout`
(Unix) and `wsapoll_timeout_ms` (Windows) truncate a remaining duration to
whole microseconds or whole milliseconds the ordinary way, and only lift a
result that truncated all the way to zero back up to one unit — never down
to zero, not always up. A 1.5µs remainder becomes 1µs; a 1.5ms remainder
becomes 1ms on Windows — both rounded down, just not past the point where a
real wait would collapse into a busy spin. What still makes "at least" true
is `task.rs`'s `wake_due`, which wakes a parked task only once its deadline
is `<=` the clock reading it is checked against: a platform wait that
returns early because its own timeout truncated short reports "nothing
ready," and the drive loop simply re-polls and re-parks on the same
deadline rather than waking the sleeper ahead of schedule
(`crates/nova-runtime/src/poll.rs`'s `select_timeout` and
`wsapoll_timeout_ms`; `crates/nova-runtime/src/task.rs`'s `wake_due`).

**AMENDED 2026-08-18 (branch `timeout-combinator`): `timeout<T>` and
`TimeoutError` now ship too, closing §9 entirely.** Delivered over one new
builtin, `task_timeout_future`, and one hand-written `PollFn`,
`poll_timeout`, which polls the inner future before checking the deadline,
so a future that already completed is never reported as timed out. Getting
here needed the executor to widen: a deadline may now accompany any wait
and two deadlines merge to the earlier by `min` (`Wait::Task` grew a
`deadline: Option<Instant>` field), and `poll_sleep` became level-triggered
like `poll_join` instead of edge-triggered, so a wake merged from an
unrelated deadline cannot make it fabricate a completion. **`timeout`
abandons its inner future; it does not cancel it** — the poll ABI has no
cancellation hook, so on timeout the inner is simply never polled again.
That costs nothing for `sleep` (GC reclaims its state), `join` (the joined
task runs on independently), or `read`/`write` (the caller still holds the
`TcpStream`), but it costs a socket for `connect`: `start_connect` registers
the socket in the poller's table and only `finish_connect` removes it, so a
`connect` abandoned mid-attempt leaves an entry nothing can reach or close,
leaking it until process exit — the same standing
`docs/adr/0012-file-descriptor-lifecycle.md` already accepts for any
unclosed descriptor. Design:
`docs/superpowers/specs/2026-08-18-timeout-combinator-design.md`.

**AMENDED 2026-08-19 (branch `std-log-core`): a new record, `SystemTime`,
ships in `std/time` — an addition to this section, not a correction of the
code block above, which still declares only `Instant` and `Duration`
correctly.** `SystemTime { nanos: Int }` counts nanoseconds since the
**Unix** epoch, read by a new intrinsic, `nova_rt_time_now_epoch_nanos`,
deliberately distinct from `crate::time::now_nanos()`/`epoch()`, which are
process-relative and answer a different question. It is a **separate
type** from `Instant`, not a method on it: `Instant`'s whole contract is
that it is monotonic and comparable by subtraction within one process, and
a wall clock is neither — it can jump backwards when NTP corrects it. Two
types that cannot be confused for one another is the point. `SystemTime::
to_iso8601()` renders fixed-width ISO-8601 to milliseconds
(`2026-08-19T02:40:13.123Z`), computed entirely in Nova over Hinnant's
civil-from-days algorithm. **UTC only, and permanently so**: `00-MASTER-
SPEC.md` §6's Rust crate list is FINAL and carries no date/time crate, so
there is no timezone database this stdlib could consult even if a local
offset were wanted, and it is not — a local-time rendering would be a guess
that is wrong twice a year in every DST zone, with no way to get it right.
This **discharges the wall-clock deferral** this section's own design
recorded (`docs/superpowers/specs/2026-08-17-std-time-design.md` §1): *"§9
specifies a monotonic `Instant` only. `std/log` will eventually want a
timestamp, which is a wall clock; adding one now is speculation, so it
waits for the increment that needs it."* `std/log` (§10) is that
increment, and the timestamp it needed is `SystemTime`.

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

**AMENDED 2026-08-19 (branch `std-log-core`): shipped, with one shape
deviation from the block above and two variants deferred rather than
missing.** The five level functions are **associated functions on an
empty record, `Log`** (`Log::info("...")`, `Log::error("...")`, and so
on) rather than the top-level `pub fn`s shown above. Nova has no import
statements and no qualified paths — every std module's public names are
glob-imported into every other module — and `import_std_module`
(`crates/nova-resolver/src/lib.rs:1305-1311`) resolves a name collision
between a std module and the importing module **silently, in the
importing module's favour**: "leaving any name a module already defines
or imports untouched." A top-level `pub fn error` would therefore make
`std/log`'s own `error` unreachable in any module that defined its own,
with no diagnostic anywhere — a logging call resolving to the wrong
function is a worse failure than one that fails to compile.
`std/strings/lib.nova:248-252` already declined this same trade for
`join`, for the identical reason, stated in the source: "a top-level `pub
fn` is glob-imported into every module and would take the name `join`
from all user code." `Log` itself is an ordinary, empty, glob-imported
record — Nova has no free-standing namespaces — so `RESERVED_TYPE_NAMES`
stays at 7 and `STD_MODULES` goes 9 → 10.

`LogFormat::Json`, `LogOutput::File(String)`, and the TTY detection that
would choose between `Human` and `Json` automatically are a **named next
increment, not gaps left by this one**: `LogConfig` already carries a
`format` field and `LogOutput` already has two of its eventual three
variants, specifically so that increment adds only variants, not fields —
adding a *variant* later breaks only exhaustive matches, where adding a
*field* to `LogConfig` would break every existing construction site.
`serde_json` is already on the FINAL crate list (`00-MASTER-SPEC.md` §6),
which is what makes that increment's escaping a choice rather than a
hand-rolling exercise. No TTY-detection facility (`isatty`, `is_terminal`,
`GetConsoleMode`) exists anywhere in `crates/` today.

**Every level function returns nothing and cannot fail.** `std/io`'s
`Write` trait is entirely `async`, and `.await`ing a log call would make
logging impossible from any synchronous function, including
`Display::fmt` and a panic path — so logging is built instead over the
existing synchronous `println`/`eprintln` builtins, which already write
without a `Result`. A logger has nowhere left to report a write failure on
stderr; propagating one would only move the same unanswerable question to
every call site.

**Separately, and not particular to this section:** the `module std.log`
line that opens this section's code block above does not parse — `grep -n
"^module " std/*/lib.nova` returns zero hits across all eleven std module
sources the glob matches (the ten in `STD_MODULES` plus `std/test`, which
is deliberately held out of it and seeded only under `nova test`), none of
which declares a `module` line at all. This is not a
`std/log` deviation; it is a pre-existing gap in this document that
predates this increment and reaches every one of `nova-spec`'s dotted
`module std.x` headers — `module std.core` (§2, :47), `std.fmt` (§3,
:146), `std.io` (§4, :219), `std.fs` (§5, :332), `std.http` (§6, :430),
`std.json` (§7, :488), `std.crypto` (§8), `std.time` (§9), `std.log`
(§10, above), `std.test` (§11), `std.collections` (§12), `std.sync` and
`std.task` (§13), and `std.net` (§16) — thirteen numbered sections in
all. A reader who checks only this section would otherwise conclude
`std/log` alone fails to conform; it does not stand out, and no section
here does. Recorded rather than fixed: correcting thirteen headers is a
documentation pass of its own and out of scope for a records-only task.

(The four line numbers above for `std.io`, `std.fs`, `std.http` and
`std.json` were re-measured and corrected as part of the `std-fmt`
branch's own edits, which inserted new text into §3 above them. This is
not the same case as the thirteen `module` headers just above, or as the
cross-file `.rs` citations elsewhere in this document that this project's
convention treats as a grep hint rather than a promise: these four are
intra-file pointers, in the very file the branch edited, that were exact
before the edit, so keeping them exact after it is the editor's job and
mechanically checkable, not a matter of judgment.)

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

**AMENDED 2026-08-20 (branch `std-sync-mutex`): position 8 above is now
partially closed.** `Mutex<T> { locked: Bool, value: T }` and
`MutexGuard<T> { owner: Mutex<T>, released: Bool }` shipped as part of
`std/sync` (`STD_MODULES` 11 → 12): `Mutex::new`, private `Mutex::take`,
`Mutex::try_lock -> Option<MutexGuard<T>>`, `async fn Mutex::lock ->
MutexGuard<T>`, `MutexGuard::get`, `MutexGuard::set`,
`MutexGuard::release` — see `docs/adr/0016-std-sync-partial-close.md` for
the decision and its consequences. `channel<T>`, `spawn_blocking`, and
`JoinHandle::cancel`, the other three items in the code block above, did
not ship.

**Two members of that surface were added on 2026-08-21, after this
increment's own review, and are recorded here because a reader has no
other way to tell them from the original draft.** `MutexGuard`'s
`released: Bool` is what makes `release`'s documented idempotence true
rather than merely stated: `release` was an unconditional
`self.owner.locked = false` on a guard that carried no released-state, so
a second `release` on a guard whose mutex had since been reacquired freed
a lock another task was holding. `MutexGuard::set(mut self, v: T)` is the
write half of the pair `std/collections`' `Vec` already ships (`get` at
`std/collections/lib.nova:49`, `set` at `:53`): `get` returns the value,
so for a `T` with no assignable interior — `Int`, `Bool`, `Float`, `Char`,
`String` — the guard was a read-only view, which is why
`sync_mutex_two_tasks_serialise.nova` wraps its counter in a
`Counter { n: Int }` record. `set` is a no-op on a released guard, for the
same reason `release` is. Both are pure Nova: zero new intrinsics,
`Builtin::STD_ONLY` still 65. Each use of the flag has its own fixture:
`tests/runtime/sync_mutex_stale_guard_cannot_steal.nova` for `release`,
`sync_mutex_stale_guard_cannot_write.nova` for `set`.

**Neither flag reaches tampering, and the root of that is one fact: Nova
enforces no field privacy.** `released` is an ordinary readable and
writable field, so it closes mistakes rather than attacks, by two measured
routes of unequal reach. *Forgery:* `MutexGuard { owner: m, released:
false }` is ordinary legal code and its `release` frees a lock a real
guard holds — measured; it needs a `Mutex<T>` to name. *Resurrection:*
writing `g.released = false` on a guard that was genuinely obtained and
genuinely released makes its next `release` free a lock a different task
now holds — also measured, and **strictly weaker**, because it needs
nothing but the stale guard. An earlier version of this amendment named
forgery alone, which understated the reach; corrected 2026-08-21. Both are
the unenforceable shape ADR 0012 records for `File { fd: 9999 }`, with one
difference: a forged `fd` safely misses a table lookup, whereas a forged
or resurrected guard writes straight to the live mutex. `get` is
deliberately *not* guarded, and not for want of the state — it returns a
`T`, `T` is generic, so an early return has no value to hand back;
`Option<T>` would change the signature; and a Nova panic aborts rather
than unwinds across a poll boundary. A stale read also cannot corrupt the
mutex, where a stale `set` or `release` writes to it.

**Release is explicit, and this is not a shortcut a future increment should
undo.** `Drop` appears in three spec files —
`12-TYPESYSTEM.md:192`, `13-RUNTIME.md:96`, `14-CODEGEN.md:24` — and is
implemented in none of them: no handling in `nova-typeck`, `nova-resolver`
or `nova-mir`, and no `trait Drop` in `std/core`. Releasing a guard is pure
Nova (`self.owner.locked = false`), so RAII here would need the *language*
feature, and no runtime mechanism substitutes for it. **ADR 0012 is cited
only as precedent for the response, not as the reason.** An earlier
version of this project's own design doc for this increment claimed ADR
0012 shows the collector "offers no per-object hook" — **ADR 0012 says the
opposite**, in bold: "The collector has a per-object notification hook,"
and goes on to warn that a reader who finds it will reasonably ask why
`File` does not register with it. ADR 0012's actual argument is that the
hook reports the *freed object's own address*, never a field value read out
of it, which tells an `fd`-keyed table nothing — an argument about a
runtime-managed handle, and it does not transfer to a `MutexGuard`, whose
release never touches the collector at all. What does transfer is the
*shape* of the decision: explicit, idempotent release (`release` is a
no-op on a second call) over a collector-based backstop, accepting a
uniform documented leak — a guard never released leaves its mutex locked
for the life of the process, the same trade ADR 0012 made for an unclosed
`File`.

**Of those other three items, two remain deferrals with named blockers,
not oversights**, and neither is a matter of merely not having gotten to
it yet. **The third, `channel<T>`, shipped on 2026-08-21 (branch
`std-sync-channel`) — see this section's own amendment of that date below,
and `docs/adr/0017-std-sync-channel-shape.md`.** Be precise about which
half of this paragraph went stale. Its *premise* is still true:
`channel<T>(buffer: Int) -> (Sender<T>, Receiver<T>)` does name a tuple,
Nova still has no tuple types, and that signature still produces
`error[E0900]: tuple types are not supported yet` today. What is false is
the *conclusion* drawn from it — that the channel is therefore a deferral.
The signature moved instead: `channel` now returns `Channel<T>`, the
record this section's original code block declares immediately above the
function and never returns. **The blocker was not lifted; it was routed
around.** Nothing in this paragraph should be read as saying `channel` is
unbuilt. The two that do remain are unchanged by that
increment, and both are `std/task` items rather than `std/sync` ones.
`spawn_blocking<T>(f: fn() -> T) ->
JoinHandle<T>` needs a thread pool to run `f` on — `nova-runtime`'s
`task.rs` opens with "A single-threaded cooperative executor," and the only
`std::thread::spawn` anywhere in the runtime is a test helper in `poll.rs`,
not a pool a std module could dispatch onto. `JoinHandle::cancel(self)`
needs a mechanism this runtime does not have: nothing unwinds and the poll
ABI has no interrupt hook, so there is no way to stop a task mid-flight.
**That is a deferral with a named blocker, not a decision against
cancellation** — `13-RUNTIME.md` §4.4 specifies structured cancellation and
`docs/adr/0009-async-execution-model.md` files "No cancellation" as an
**open residual gap** (`:365`), naming "a future `JoinHandle` drop or
cancellation" as the natural fix point (`:405`). What ADR 0009 settles is
narrower: because the poll ABI has no cancellation hook, `timeout<T>`
**abandons** its inner future rather than cancelling it (`:328-329`).
Building the hook is its own increment. **An earlier version of this
sentence said `cancel` "would contradict the abandonment, not cancellation
contract that ADR 0009 already settled"; ADR 0009 settles no such thing
about `cancel`, and this is corrected here (2026-08-21).**

**Separately, and not a `std/sync` deviation:** the `module std.sync` line
that opens the code block above does not parse, and this is true of every
section in this document — no std module source declares a `module` line
at all. `std/sync` also introduces a second, related fact worth recording
here for the first time: `$std.test` is not simply another entry held out
of `STD_MODULES`, the way this note has described it before — it has its
own named constant, `STD_TEST_MODULE`
(`crates/nova-resolver/src/lib.rs:1293`), separate from the `STD_MODULES`
array precisely so that array's length does not depend on whether `nova
test` is the subcommand running. Counting both: 12 `STD_MODULES` entries
plus `STD_TEST_MODULE` is **thirteen** `lib.nova` files on disk, up from
twelve before this increment. Two earlier amendments record the same fact
at their own dates, and they do **not** record the same number: §3's
2026-08-19 amendment records **twelve** total (eleven `STD_MODULES`
entries plus `std/test`), and §10's earlier 2026-08-19 amendment records
**eleven** (ten plus `std/test`), correct as of its own date and written
before `std/fmt` joined the array — §3's own text says so explicitly.
`std/sync` joining `STD_MODULES` this increment moves the count to
thirteen. That is a shift in the count, not in the underlying fact:
neither earlier paragraph is wrong, each is simply one or two behind,
exactly as §10's was when `std/fmt` landed. A reader checking only this section
would otherwise conclude `std/sync` alone fails to conform; it does not
stand out, and no section here does.

**The concurrency control this module buys is real, and so is its cost.**
Contention is handled by yielding and retrying
(`while !self.take() { yield_now().await }`) rather than by parking on a
dedicated wait condition: no executor change, no fourth `Wait` variant
alongside `Deadline`/`Task`/`Io` (`crates/nova-runtime/src/task.rs:162`),
no arm added to `wake_due`'s `retain` match, whose existing arms end in a
wildcard rather than an exhaustive list. The cost is that a waiter spinning
on `try_lock` stays *runnable* the whole time it waits, so `report_deadlock`
— which only ever sees tasks that are parked — cannot see it: a mutex that
is locked and never released does not deadlock visibly, it spins forever
instead. This `Mutex` is also **not re-entrant** (a task that calls `lock`
while already holding the same mutex spins against itself, forever) and
makes **no fairness guarantee** (which waiter acquires the lock next, when
several are spinning, is unspecified).

**AMENDED 2026-08-21 (branch `std-sync-channel`): the bounded channel
shipped, and it does not have the signature this section writes.** The
paragraph above listing it among the deferrals is corrected in place at
its own location; this is the record of what was actually built. Shipped
in `std/sync/lib.nova`, pure Nova, **zero new intrinsics**
(`Builtin::STD_ONLY` stays at 65) and no `STD_MODULES` change (still 12 —
`std/sync` joined the array last increment):

```nova
pub record Channel<T> { ring: Ring<T>, cap: Int, closed: Bool }
pub record Sender<T> { ch: Channel<T> }
pub record Receiver<T> { ch: Channel<T> }

pub fn channel<T>(buffer: Int) -> Channel<T>

impl<T> Channel<T> {
    pub fn sender(self) -> Sender<T>
    pub fn receiver(self) -> Receiver<T>
}
impl<T> Sender<T> {
    pub fn try_send(mut self, v: T) -> Bool
    pub async fn send(mut self, v: T) -> Bool
    pub fn close(mut self)
}
impl<T> Receiver<T> {
    pub fn try_recv(mut self) -> Option<T>
    pub async fn recv(mut self) -> Option<T>
}
```

**The `mut self` receivers are part of the surface, not decoration.** Per
`docs/adr/0005-mutable-receivers-and-one-shot-hash.md`, `mut` is a
permission on the *binding*, so a `mut self` method can only be called
through a binding that is itself `mut`. Every call site therefore reads
`let mut tx = ch.sender()`, and `let tx = ch.sender()` followed by
`tx.try_send(1)` does not compile — measured:
``error[E0060]: `Sender_T.try_send` mutates its receiver, but `tx` is
immutable``. `sender()` and `receiver()` take a
plain `self` and `channel` is a free function, so the annotated `let ch:
Channel<Int> = channel(2)` needs no `mut` — only the two handles do. An
earlier version of this amendment omitted every receiver, which made the
block read as though `let tx = ch.sender()` were enough; §13's own style
carries receivers (`pub async fn lock(self) -> MutexGuard<T>`) and this
now matches it.

**The deviation is confined to the return type, and it moved *to* a type
this section already declares.** The specified
`-> (Sender<T>, Receiver<T>)` cannot be compiled: tuples are
`error[E0900]: tuple types are not supported yet`
(`crates/nova-typeck/src/check.rs:2505`), whose own note says the feature
"arrives in a later milestone". So `channel` returns `Channel<T>` — the
record declared on the line above it in the code block at the top of this
section, which that block then never returns, never gives an `impl`, and
never mentions again — and the pair is reached through `ch.sender()` and
`ch.receiver()`. `Sender<T>` and `Receiver<T>` are named in the specified
signature but declared nowhere in that block, and are declared here for
the first time. All three of the channel type names that block uses are
therefore built; `Ring<T>` (private) is the only name in the shipped
module that the block does not have. **Every claim in this paragraph about
what §13 "never" does is scoped to that original code block, not to §13
as a whole** — this amendment is itself part of §13 now, and it returns
`Channel<T>`, declares all three types, and names `Ring<T>`.
**Reverting to the tuple when Nova gains tuples is optional, not owed** —
the accessor form gives the pair an identity that can carry future
accessors without changing `channel`'s signature, which a tuple return
cannot. See `docs/adr/0017-std-sync-channel-shape.md`.

**Semantics, and what pins each.** The `channel_*` fixtures in
`tests/runtime/` cover the clauses below. Two of them were added only
after this amendment was first written, because an earlier version of this
sentence claimed each clause was "pinned by its own fixture" when two were
pinned by nothing: the `buffer < 1` clamp, which no test exercised (no
`channel(0)` or negative argument existed in the repo at all), and the
async `send`'s refusal on a closed channel, which no test called.
`channel_clamps_buffer_below_one` and `channel_send_refuses_when_closed`
close both. A third fixture, `channel_enqueue_wraps_after_dequeue`, was
added on 2026-08-22 for a gap of a different kind, found by review rather
than by writing a record: the ring's *enqueue* modulo was executed by no
test in a state where it wraps, and no record had claimed it covered **or**
recorded it as missing. `docs/adr/0017-std-sync-channel-shape.md` carries
the measured detector counts. `try_send` returns `false`
when the channel is **full or closed**; the async `send` returns `false`
**only** when closed, waiting out a full channel. `try_recv` returns
`None` when **empty**; the async `recv` returns `None` **only when closed
and drained**, never merely empty — every iteration tries the dequeue
before reading `closed`, so buffered values drain before a close is
reported, and a consumer loop therefore has a termination condition at
all. `close` is idempotent and leaves buffered values readable, so a
producer may close the instant it stops producing. `buffer < 1` **clamps
to 1** rather than panicking: a zero-capacity rendezvous channel needs a
handoff protocol yield-and-retry cannot express without a second wait
state, and clamping follows `Int::pad`'s precedent
(`std/fmt/lib.nova:30`, `if len >= width { return s }`; `:27` is that
function's signature) of an early return over a panic.

**Every call site must annotate, and this is forced rather than chosen.**
`let ch: Channel<Int> = channel(2)` is the only available call form: `T`
appears only in the return type, so no argument infers it, and Nova has
no turbofish — `channel<Int>(2)` is not a call but
`error[P0001]: chained comparison operators are not allowed (use
parentheses)`, the parser reading `channel < Int > (2)`. Both obvious
escapes were refused and the reasons are recorded in ADR 0017: a seed
parameter would make `T` inferable but promotes an implementation detail
(`[fill; n]` needs a value of `T`) to permanent API, and `Channel::new`
has the identical inference problem while moving further from this
section's `channel` free function.

**The field-privacy hazards are documented, not prevented, and ranked by
reach the way `MutexGuard`'s are above** — the same standing that guard has
and `File { fd: 9999 }` has under ADR 0012, except that here the
enforcement is *unavailable* rather than merely absent: a record field's
`vis` is dropped at AST→HIR lowering
(`crates/nova-hir/src/lib.rs:918-923`), so `check_field_set`
(`crates/nova-typeck/src/check.rs:6214-6272`) has no visibility left to
check. *Forgery grants nothing:* `Sender { ch: c }` is ordinary legal code,
so the producer/consumer split carries intent and not enforcement — but the
literal needs a `Channel<T>` to name and `sender()`/`receiver()` are `pub`
and take a plain `self`, so whoever can name the channel already holds both
handles legitimately. *Reopening is the strongest route:*
`ch.closed = false`, reachable from any legitimately-held handle, destroys
the one property that gives a consumer loop a termination condition — that
`recv`'s `None` means closed **and** drained — and `close` only ever sets
the flag true, so no API path reaches it. *Writes through the `pub` ring:*
`ch.ring.head` and `ch.ring.len` break the `head >= 0`, `len >= 0`,
`cap >= 1` invariant the truncating `%` depends on, without ever naming the
non-`pub` `Ring<T>`. An earlier version of this amendment named forgery
alone — the identical understatement corrected for `MutexGuard` above on
2026-08-21; corrected here 2026-08-22. *Invisible spin:* contention is
handled by yielding and retrying, exactly as `Mutex` above, with the same
consequence — a waiter stays **runnable**, and `report_deadlock`
(`crates/nova-runtime/src/task.rs:1243`) is reached from one place only
(`:1007`) and only when the ready queue is empty, so it cannot see a
channel nobody drains. Such a channel does not deadlock visibly; it spins
forever. This was not merely predicted but **observed**: a mutation making
`recv` answer `None` on an empty-but-open channel truncated its fixture's
output and then hung indefinitely, costing a suite timeout. No fairness
guarantee is made either.

**`RwLock` and `atomic` remain, and three documents disagree about
whether they were ever specified.** §1's module index (`:27`) names four
things for `std/sync` — `Mutex, RwLock, channel, atomic`;
`00-MASTER-SPEC.md:238`'s position 8 names three, omitting `RwLock`; and
**this section specifies two**, `Mutex` and `Channel`, and is the only one
of the three that gives any signature at all. So the accurate claim is
that `channel` **completes this section's own specification of
`std/sync`** — not that it closes position 8, which stays partial.
`RwLock` and `atomic` are named only in index lines that no section
specifies: neither has a signature, a semantic, or a section anywhere in
`nova-spec/`, so neither is a matter of not having gotten to it. The two
`std/task`/position-7 items above, `spawn_blocking` and
`JoinHandle::cancel`, are unaffected by this increment and unchanged.

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

---

## 16. `std/net`

**Numbered out of the module-index order above** (§1 lists `std/net`
immediately after `std/fs`/`std/bytes`), the same way `std/bytes`,
`std/strings`, `std/regex` and `std/process` have no dedicated section at
all. Appended here rather than inserted between §5 and §6 so no existing
numbered section — several of which are cross-referenced by number
elsewhere in this repository — has to be renumbered.

```nova
module std.net

pub record TcpStream { /* opaque */ }

pub async fn connect(addr: String) -> Result<TcpStream, IoError>

impl TcpStream {
    pub async fn close(self) -> Result<(), IoError>
    pub async fn read_timeout(self, max: Int, ms: Int) -> Result<Bytes, IoError>
}

impl Read for TcpStream { ... }
impl Write for TcpStream { ... }
```

`addr` is `"host:port"`, resolved through the identical mechanism
`std::net::TcpStream::connect` itself uses, taking the first resolved
address for a multi-address name (`crates/nova-runtime/src/net.rs`'s own
`resolve_addr`). `TcpStream` is written `{ /* opaque */ }` for the same
reason `File` is in §5: nothing outside this module should rely on its
shape, but it is not opaque to the language itself — it ships as
`TcpStream { fd: Int }`, an `Int` key into a runtime-owned table of open
sockets, not an OS socket handle. Nova has no field privacy, so
`TcpStream { fd: 9999 }`, naming no connection this module ever opened, is
ordinary, legal code; `close` is idempotent and any other operation on a
closed, stale, or forged handle is an ordinary `IoError { kind: Other }`,
never a panic — absence from the socket table is what closedness *is*,
identically to `File` in §5.

`read_timeout` has no `std/fs` analogue (nothing there takes a deadline), so
it ships as a second inherent method beside `close` rather than folded into
`Read`, which has no room for a third argument. It reports `TimedOut` if
`ms` milliseconds pass with nothing to read first; otherwise it behaves
exactly like `Read::read` below it, including EOF/short-read semantics — an
**empty** result is end of stream, and a **short** one is not.

`impl Read for TcpStream` and `impl Write for TcpStream` reuse §4's traits
unchanged — `read`/`write`/`flush`, the `Future<T>`-returning spelling those
trait methods already require. `Write::write` may write fewer bytes than
given — one non-blocking attempt, not a `write_all` loop — identically to
§4's own contract for the trait, and deliberately unlike `std/fs`'s
top-level, count-less, write-all `write` function in §5 (`File`'s own
`impl Write`, sharing this same trait, already carries the same
short-write contract `TcpStream` does here). `flush` always returns
`Ok(())` immediately, with no runtime call at all: one `write` call against
a `TcpStream` is already one unbuffered syscall, so there is no userspace
write buffer for a flush to push anything out of — the same reasoning
`std::io::Write for std::net::TcpStream` in Rust's own standard library
rests on.

**`connect`, `read`, `write`, and `read_timeout` genuinely suspend the
calling task rather than blocking the whole executor**, per §4's 2026-08-16
amendment and `docs/adr/0013-io-poller.md` — the property this module exists
to add. `close` and `flush` do not: `close` calls a plain status-returning
intrinsic with no `.await`, and `flush` never calls a runtime intrinsic at
all, for the reasons given above. `bind`/`accept`/`TcpListener` (a server
side), UDP, and Unix sockets are all named in §1's module-index line for
`std/net`, but none of the three is built by this section; each remains a
future increment's to add.
