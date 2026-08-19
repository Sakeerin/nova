# `std/log` core — design

**Status:** approved 2026-08-19. Base: `main` == `origin/main` == `64e2be6`, 470 commits, 0 merge commits, 1009 tests (8 deliberately ignored), clean tree, seven CI checks green, tagged `v0.2.0-alpha.1`.

**Every `file:line` citation below is relative to that baseline**, and several have since drifted — this increment added four builtins to `crates/nova-resolver/src/lib.rs`, which moved everything under them. `import_std_module`'s doc comment was `:1281` and is now `:1305`; the four print builtins were `:571-574` and are now `:586-589`. Both were correct when written, which is precisely why the citation style is the problem rather than the numbers: **a line number in a design doc is a claim that expires, and this one expired against the increment's own edits.** Locate by content, not by line, and treat any number here as a hint about where to grep.

**Goal.** Give Nova a logger: `nova-spec/20-STDLIB.md` §10 minus its JSON format, file output and TTY detection, plus the wall clock §10 needs and §9 deliberately does not have.

**Approach in one line.** The runtime contributes a clock reading and a three-field configuration cell; every formatting, padding and filtering decision is Nova code, and the writing reuses the `println`/`eprintln` builtins that already exist.

---

## 1. Scope

### In

- A new embedded module `std/log` with five level functions, `init`, `init_with`, `LogConfig`, and level filtering.
- The `Human` line format, and `Stderr`/`Stdout` outputs.
- Wall-clock time in `std/time`: one new intrinsic and a `SystemTime` record that can render ISO-8601 UTC.
- An ADR recording why the build order is departed from again (§2).

### Out, deliberately — increment B

- **`LogFormat::Json`.** Needs string escaping, which is worth doing once and doing properly. `serde_json` is in `00-MASTER-SPEC.md` §6's FINAL crate list, so the escaping does not have to be hand-rolled — but choosing between escaping in Nova and escaping in the runtime is its own decision, and it is not on the critical path to a working logger.
- **`LogOutput::File(String)`.** A logger holding a file across calls is a resource-lifetime question, and this repo already has a considered answer for that shape in ADR 0012 (`std/fs`'s explicit `close`, absence-from-table *is* closedness). Reusing it needs a sync file-write path, which no intrinsic currently offers — every `std/fs` write is `async fn` through the `Write` trait. That is a new runtime surface and belongs with the increment that can test it.
- **TTY auto-detection.** §10 says `init()` picks "JSON or human based on TTY". Nothing in the tree detects a terminal — `isatty`, `is_terminal` and `GetConsoleMode` appear nowhere in `crates/`, measured — so this is a new platform shim with a Windows arm and a Unix arm. It is also **only** useful once `Json` exists, since with one format there is nothing to choose between. It follows `Json`, not this increment.

### Out permanently, not deferred

- **Local time and timezones.** Every log line is **UTC**. `00-MASTER-SPEC.md` §6's crate list is FINAL and contains no date/time crate — no `chrono`, no `time`, no `jiff` — so there is no timezone database to consult. A hand-rolled local-time offset would be a guess that is wrong twice a year in every DST zone. UTC is not a limitation being worked around here; it is the correct answer for a log timestamp and it is the only one this dependency set can state truthfully.

---

## 2. Why `std/log`, and the build-order deviation

`00-MASTER-SPEC.md` §3 lists Phase 2's modules "in order" and calls the build order strict. By that order the earliest incomplete entry is **position 2, `std/fmt`** — `std/io` shipped, and `std/fmt` has no module at all: its four print functions are compiler builtins (`Builtin::Print`, `Println`, `EPrint`, `EPrintln`, `crates/nova-resolver/src/lib.rs:571-574`) and `Formatter` does not exist. This increment takes position 6's `std/log` instead, and that needs recording rather than assuming.

**Why `std/log` first.** §10 is fully specified: five level functions, `init`, `init_with`, and three sum types with named variants. Every dependency exists — `std/io`'s streams, `std/time`'s clock, `std/strings`' `repeat`. And `std/time`'s own design deferred wall-clock time as speculation with a named condition: *"`std/log` will eventually want a timestamp, which is a wall clock; adding one now is speculation, so it waits for the increment that needs it."* This is that increment, so the deferral discharges exactly as written.

**Why not `std/fmt` yet.** What position 2 still needs is thinner than its position suggests and murkier than §10. `Display` already exists (`std/core/lib.nova:98`, `pub trait Display { fn fmt(self) -> String }`) with impls for `Int`/`Float`/`Bool`/`Char`/`String`, and string interpolation already calls it. So two thirds of `std/fmt` would be **replacing working mechanisms**: moving four functioning builtins behind a module, and re-pointing interpolation's desugaring at a Nova-level `format(parts: [FormatPart]) -> String`. The remaining third, `pub record Formatter { ... }`, has its body elided in the spec — there is no specified behaviour to build, so a `std/fmt` increment would begin by designing what §3 left blank. That is a real increment and it deserves its own scoping conversation; it is not a thing to sweep in ahead of a fully-specified module.

**The deviation gets an ADR**, not a footnote, because `00-MASTER-SPEC.md` **§7 item 5** requires one ("ADR written for any decision deviating from this spec") and because this is the *second* time position 2 has been passed over — the first was Phase 2.1, recorded in `docs/superpowers/specs/2026-07-25-phase-2-1-std-core-design.md`, which deferred `std/fmt` + `std/io` behind async. An undocumented skip that happens twice stops looking like a decision and starts looking like an oversight.

---

## 3. Why `Log::info` and not `info`

**Nova has no import statements and no qualified paths.** Every std module's public names are glob-imported into every other module — `tests/runtime/timeout_ok.nova` calls `sleep`, `Duration::from_millis` and `timeout` with no import line of any kind. So a top-level `pub fn` in a std module takes that name in every Nova program that will ever be written.

**The collision is silent, and its direction is documented.** `import_std_module` (`crates/nova-resolver/src/lib.rs:1281-1286`) binds one std module's names into every other module's scope, *"leaving any name a module already defines or imports untouched (a user — or another std module's own — item shadows a std one)"*. So a program with its own `error` does not break: it silently keeps its own, and `std/log`'s `error` becomes **unreachable with no diagnostic**. A logging call that resolves to the wrong function is a worse failure than one that fails to compile.

§10 writes the five levels as top-level `pub fn trace/debug/info/warn/error(msg: String)`. Those are five of the most collision-prone identifiers in general programming, and `error` is the worst of them. The current glob-imported namespace already holds `read`, `write`, `open`, `exists`, `connect` and `assert`, so this project has tolerated some risk here — but it has also explicitly refused it once, and left the reason in the source. `std/strings/lib.nova:249-251` makes `join` a method on `String` rather than a free function, saying: *"a top-level `pub fn` is glob-imported into every module and would take the name `join` from all user code."*

**So the five levels become associated functions on an empty record**, reached as `Log::info("...")`. This deviates from §10's *shape* while delivering its full capability, follows a precedent already set and documented in this stdlib, and costs one identifier (`Log`) instead of five. The mechanism is measured, not assumed: `pub record Marker {}` parses and empty records already ship (`std/io`'s `Stdin`/`Stdout`/`Stderr`), and no-receiver associated functions already work (`Instant::now()`, `Vec::new()`, `OpenOptions::reading()`).

`Log` is an ordinary record, so `RESERVED_TYPE_NAMES` stays **7**. `STD_MODULES` goes **9 → 10**.

---

## 4. The wall clock

`std/time` gains one intrinsic and one record.

```rust
#[no_mangle]
pub extern "C-unwind" fn nova_rt_time_now_epoch_nanos() -> i64
```

`SystemTime::now().duration_since(UNIX_EPOCH)` narrowed to `i64` nanoseconds, **saturating at `i64::MAX`** rather than wrapping, and returning `0` if the system clock is set before 1970 (`duration_since` errors there; the clamp keeps a nonsensical clock from producing a nonsensical date rather than a negative one that the calendar math would then have to interpret).

```nova
pub record SystemTime { nanos: Int }

impl SystemTime {
    pub fn now() -> SystemTime
    pub fn to_iso8601(self) -> String
}
```

**This is an addition to `nova-spec` §9, which specifies a monotonic `Instant` only.** It is deliberately a **separate type** from `Instant` rather than a method on it: `Instant`'s whole contract is that it is monotonic and comparable by subtraction within one process, and a wall clock is neither — it can jump backwards when NTP corrects it. Two types that cannot be confused for one another is the point.

**`i64` epoch nanoseconds saturate in 2262.** Recorded, not guarded: the same bound `Duration` already carries, and no arithmetic here can be made correct past it anyway.

**Note the two clocks now in `std/time` read from different origins.** `Instant` counts from `crate::time::epoch()`, a `OnceLock` captured at first read; `SystemTime` counts from the Unix epoch. `crate::time::now_nanos()` is the existing shared helper for the former and **must not** be reused for the latter — they are different quantities that happen to share a unit and a width, which is precisely the shape of mistake `SLEEP_SLOT_NANOS` → `SLEEP_SLOT_DEADLINE_NANOS` was renamed to prevent.

---

## 5. ISO-8601, computed in Nova

`to_iso8601()` renders `2026-08-19T02:40:13.123Z` — date, `T`, time to milliseconds, `Z`. Fixed width, so log lines align without padding logic in the caller.

Split first, by integer division on the nanosecond count:

| Quantity | From |
|---|---|
| `days` | `nanos / 86_400_000_000_000` |
| `nanos_of_day` | `nanos % 86_400_000_000_000` |
| `hour` | `nanos_of_day / 3_600_000_000_000` |
| `minute` | `nanos_of_day / 60_000_000_000 % 60` |
| `second` | `nanos_of_day / 1_000_000_000 % 60` |
| `milli` | `nanos_of_day / 1_000_000 % 1_000` |

Then **civil-from-days** turns `days` into year/month/day. The algorithm is Howard Hinnant's, shifting the era so that leap-day handling falls out of integer arithmetic with no conditional branches and no table:

```
z      = days + 719_468
era    = z / 146_097
doe    = z - era * 146_097                       // [0, 146096]
yoe    = (doe - doe/1460 + doe/36524 - doe/146096) / 365
y      = yoe + era * 400
doy    = doe - (365*yoe + yoe/4 - yoe/100)       // [0, 365]
mp     = (5*doy + 2) / 153                       // [0, 11], March-based
day    = doy - (153*mp + 2)/5 + 1                // [1, 31]
month  = if mp < 10 { mp + 3 } else { mp - 9 }
year   = if month <= 2 { y + 1 } else { y }
```

**Exact for every date in range, with no floating point.** Because it is Nova rather than Rust, every step is reachable from a Nova fixture, which is the property that made `std/time`'s arithmetic cheap to trust.

Zero-padding uses `String::repeat` (`std/strings/lib.nova:300`) over the digit count, as two small helpers, `pad2` and `pad3`. **Padding is where a plausible-looking implementation silently produces `2026-8-9T2:40:13.7Z`**, so §10's fixtures pin single-digit months, days, hours, minutes, seconds and a sub-100 millisecond value explicitly.

**Range assumption, stated because the formatter depends on it:** `year` is rendered unpadded, so a four-digit year is assumed. Given the clock's clamp at 0 and its saturation in 2262, every value this can be handed renders as four digits.

---

## 6. The `std/log` surface

`std/log/lib.nova`, complete:

```nova
module std.log

pub type LogLevel  = | Trace | Debug | Info | Warn | Error
pub type LogFormat = | Human
pub type LogOutput = | Stderr | Stdout

pub record LogConfig {
    pub level: LogLevel
    pub format: LogFormat
    pub output: LogOutput
}

pub record Log {}

impl LogLevel {
    pub fn to_int(self) -> Int
    pub fn label(self) -> String
}

impl Log {
    pub fn init()
    pub fn init_with(config: LogConfig)

    pub fn trace(msg: String)
    pub fn debug(msg: String)
    pub fn info(msg: String)
    pub fn warn(msg: String)
    pub fn error(msg: String)
}
```

`impl` on a sum type is **measured, not assumed**: `std/core/lib.nova:11` is `impl<T> Option<T> { ... }` and carries `is_some`, `map` and `and_then`.

| Function | Definition |
|---|---|
| `LogLevel::to_int` | `Trace` 0, `Debug` 1, `Info` 2, `Warn` 3, `Error` 4 |
| `LogLevel::label` | `"TRACE"`, `"DEBUG"`, `"INFO"`, `"WARN"`, `"ERROR"` |
| `Log::init()` | `init_with(LogConfig { level: Info, format: Human, output: Stderr })` |
| `Log::init_with(c)` | `log_set_config(c.level.to_int(), to_stderr_flag(c.output))` (below) |
| `Log::<level>(msg)` | emit at that level (below) |

Every level function is the same three steps:

```nova
pub fn info(msg: String) { emit(Info, msg) }

fn emit(level: LogLevel, msg: String) {
    if level.to_int() < log_config_level() { return }
    let line = "${SystemTime::now().to_iso8601()} ${level.label()} ${msg}"
    if log_config_to_stderr() { eprintln(line) } else { println(line) }
}
```

**`init_with` must `match` on the output rather than compare it**, because Nova has no equality on sum types — `c.output == Stderr` fails to compile with `error[E0013]: equality operators are not defined for \`LogOutput\` (operator traits arrive later in Phase 1)`, measured against a two-variant probe at `64e2be6`. So:

```nova
fn to_stderr_flag(out: LogOutput) -> Int {
    match out { Stderr => 1, Stdout => 0 }
}
```

That also means **`LogLevel` filtering cannot compare variants directly**, which is what `to_int` is for — it is not a convenience, it is the only mechanism available. `c.format` is deliberately unread this increment: `LogFormat` has one variant, so there is nothing to branch on, and the field exists only so that adding `Json` later does not change `LogConfig`'s shape (§6, above).

`emit` is a **private** top-level `fn` — no `pub`, so `import_std_module` never binds it into another module's scope and it takes no name from user code. The threshold is compared **before** the timestamp is read and the line is built, so a filtered-out call costs one intrinsic and no allocation.

**The line format is `<iso8601> <LEVEL> <message>`**, single-space separated, with the level label unpadded. A padded label would align columns but makes the format's width depend on the longest variant name, which is the kind of coupling that breaks silently when a level is added.

**Why `LogConfig` carries all three fields now**, even though `LogFormat` and `LogOutput` have one and two variants: adding a *field* later breaks every construction site, while adding a *variant* later breaks only exhaustive matches. Users construct `LogConfig`; std matches on the variants. So the field set is fixed now and the variant sets grow additively in increment B — `format: Human` written today keeps compiling when `Json` arrives.

**Writing reuses the existing builtins.** `println` and `eprintln` are `Builtin::Println` and `Builtin::EPrintln` (`crates/nova-resolver/src/lib.rs:571-574`), synchronous `extern "C"` calls. This matters beyond convenience: **`std/io`'s `Write` trait is entirely async** — `fn write(self, buf: Bytes) -> Future<Result<Int, IoError>>`, and `Stdout`/`Stderr` implement it that way (`std/io/lib.nova:179`, `:207`) — while §10's log functions are synchronous. Routing the logger through `Write` would make every log call `.await`, which would make logging impossible from any synchronous function, including `Display::fmt` and a panic path. Reusing `println`/`eprintln` keeps the logger synchronous **and adds no new sync-write precedent**, because that is exactly the path `println` has always taken.

**Log calls therefore cannot fail and return nothing**, matching §10's signatures. A write error on stderr is unreportable by a logger anyway — there is nowhere left to report it.

---

## 7. Configuration state, and the auto-initialize rule

Nova has no mutable global state: top-level bindings are `const NAME: Type = value`. So the configuration lives in the runtime, in the shape `file.rs`'s open-file table and `task.rs`'s `CURRENT` already establish.

```rust
// crates/nova-runtime/src/log.rs
struct Config { level: i64, to_stderr: bool }
// thread-local Cell<Option<Config>>
```

**`None` means "not yet initialized", and the getters resolve it to the default.** That is the whole auto-initialize rule: the first read installs `Config { level: 2 /* Info */, to_stderr: true }` and returns it, so `Log::info("x")` works in a program that never calls `init()`. `Log::init()` is then not a prerequisite but an override spelled explicitly, exactly equal to `init_with` of the default, and a later `init_with` overwrites — last writer wins.

**Three intrinsics, and deliberately not two.**

```rust
pub extern "C-unwind" fn nova_rt_log_config_level() -> i64
pub extern "C-unwind" fn nova_rt_log_config_to_stderr() -> i64   // 0 or 1
pub extern "C-unwind" fn nova_rt_log_set_config(level: i64, to_stderr: i64)
```

Two separate getters rather than one packed integer (`level * 2 + to_stderr`). Packing would save one builtin and reintroduce **the exact hazard this project has now hit twice**: an `i64` whose *meaning* changed while its type did not, which the compiler cannot catch — `SLEEP_SLOT_MS` → `SLEEP_SLOT_NANOS`, then `SLEEP_SLOT_NANOS` → `SLEEP_SLOT_DEADLINE_NANOS`, each a rename done specifically because the type stayed put. One builtin is a cheaper price than a third instance.

`to_stderr` crosses the boundary as `i64` rather than a Rust `bool`, matching every other intrinsic in this crate; the Nova side is typed `Bool` and the conversion is one comparison.

**This grows in increment B**, and that is expected: `format` adds a getter, and `File(String)` adds an output path, so `log_set_config` gains parameters. Four intrinsics today, `STD_ONLY` **60 → 64**.

**Thread-local, not global**, because the executor is single-threaded and every other piece of runtime state here already is. If Nova ever grows real threads, a per-thread logger configuration is the wrong answer and this becomes an ADR-worthy change — noted now so that it is a decision then, rather than a discovery.

---

## 8. Edge cases, each with a stated answer

| Case | Answer |
|---|---|
| Log before `init` | Emits at the default (§7). Not dropped, not a warning. |
| `init_with` after logging | Reconfigures; last writer wins. |
| Clock before 1970 | Intrinsic clamps to `0`, rendering `1970-01-01T00:00:00.000Z`. |
| Clock past 2262 | `i64` saturates; the timestamp stops advancing. Recorded, not guarded. |
| Empty message | Emitted as an empty field: `<ts> <LEVEL> ` with a trailing space. Not special-cased. |
| Message containing a newline | Emitted verbatim, so one call can produce two lines. `Human` is not a machine format; `Json` in increment B is where escaping belongs. |
| Filtered-out call | Returns after one intrinsic. No clock read, no allocation. |
| `Error` level with threshold `Error` | Emits. The comparison is `<` on `to_int`, so the threshold is inclusive. |

---

## 9. Testing

**Runtime unit tests** (`crates/nova-runtime/src/log.rs`): the default is installed on first read; `set` then `get` round-trips; `set` twice keeps the second.

**Nova fixtures**, each registered with an explicit `#[test]` in `crates/nova-cli/tests/run_tests.rs` — **registration is not automatic**, and a fixture without one runs zero tests:

- `log_default_level` — `Log::info` and `Log::error` emit, `Log::debug` and `Log::trace` do not, with no `init` call anywhere.
- `log_init_with_threshold` — `init_with` at `Warn` silences `Info`, admits `Error`.
- `log_reconfigure_after_logging` — log, then `init_with` at a different level, then log again.
- `log_stdout_output` — `output: Stdout` puts the line on stdout, which the golden distinguishes from stderr.
- `log_level_labels` — all five labels, one line each, at threshold `Trace`.

Every log-line golden must **normalize the timestamp**, since it is a live clock: replace the ISO-8601 prefix before comparing, the same treatment `tests/runtime/nova_test.stdout` already applies to a Windows NTSTATUS. What is asserted is the *shape* of the timestamp plus the exact level and message.

**ISO-8601 is tested separately and exactly**, against a fixed `SystemTime { nanos: ... }` rather than the clock, so the assertions are on known values:

| Input | Expects | Pins |
|---|---|---|
| `0` | `1970-01-01T00:00:00.000Z` | the epoch itself |
| `951_782_400_000_000_000` | `2000-02-29T00:00:00.000Z` | a leap day in a leap century |
| `1_709_164_800_000_000_000` | `2024-02-29T00:00:00.000Z` | an ordinary leap day |
| `4_107_542_400_000_000_000` | `2100-03-01T00:00:00.000Z` | 2100 is **not** a leap year |
| `1_767_225_599_999_000_000` | `2025-12-31T23:59:59.999Z` | year boundary, all fields at maximum |
| `1_756_598_463_007_000_000` | `2025-08-31T00:01:03.007Z` | single-digit minute and second, sub-100 millis |

The last row exists specifically to fail an unpadded formatter, and the 2100 row exists specifically to fail a leap-year rule that tests only divisibility by 4.

**Mutations to run and report**, each with the test that must fail:

| Mutation | Caught by |
|---|---|
| `pad2` returns its input unpadded | the single-digit ISO-8601 row |
| `<` becomes `<=` in `emit`'s threshold check | `log_default_level` (`Info` at threshold `Info` stops emitting) |
| the getter returns a hardcoded default instead of reading the cell | `log_reconfigure_after_logging` |
| `Warn` and `Error` swap in `to_int` | `log_init_with_threshold` |
| `to_iso8601` uses `days / 365` instead of civil-from-days | every ISO row except the epoch |
| `eprintln`/`println` swapped in `emit` | `log_stdout_output` |

The threshold mutation deserves note: `<` vs `<=` is a one-character change that a test at any *other* level pair cannot catch, which is why `log_default_level` asserts on `Info` *at* the `Info` threshold specifically.

---

## 10. Records

- **CHANGELOG** `[Unreleased]`: Added for `std/log` and `SystemTime`; nothing under Changed, since this increment removes and renames nothing.
- **`nova-spec/20-STDLIB.md` §9**: a dated amendment for `SystemTime`, discharging the wall-clock deferral the `std/time` design recorded and naming this increment as the consumer it named.
- **`nova-spec/20-STDLIB.md` §10**: a dated amendment recording what shipped and what did not — the `Log::` shape with its reason, and `Json`/`File`/TTY as increment B rather than as gaps.
- **A new ADR, the build-order deviation.** Why position 6 was taken before position 2, what `std/fmt` still actually needs, and that this is the second such skip. No other ADR: nothing here changes the execution model, the resource model or the GC.
- **`docs/superpowers/specs/2026-08-17-std-time-design.md` §1**: its wall-clock deferral now has an answer and should point here, the same way its `timeout<T>` deferral was pointed at the combinator spec.

---

## 11. Measured language and codebase facts this design rests on

Each of these was checked against the tree at `64e2be6` rather than recalled:

- **No import statements, no qualified paths.** `tests/runtime/timeout_ok.nova` uses `sleep`, `Duration::from_millis` and `timeout` with no import line.
- **Std-vs-user collisions are silent, and the user wins.** `import_std_module`, `crates/nova-resolver/src/lib.rs:1281-1286`.
- **A std module's own item colliding with its own pre-seeded builtin is `E0002`, not a silent shadow** — a different path from the glob import, documented at `crates/nova-resolver/src/lib.rs:483-496`. This is why `emit` must not be named after any builtin.
- **`impl` works on sum types.** `impl<T> Option<T>`, `std/core/lib.nova:11`.
- **Empty records parse and already ship.** `std/io`'s `Stdin`/`Stdout`/`Stderr`, `:136`/`:153`/`:184`.
- **`eprint`/`eprintln` are builtins.** `Builtin::EPrint`, `Builtin::EPrintln`, `crates/nova-resolver/src/lib.rs:573-574`.
- **`std/io`'s `Write` is async throughout.** `std/io/lib.nova:116`, `:118`, `:179`, `:207`.
- **`String::repeat` exists.** `std/strings/lib.nova:300`.
- **There is no `==` on sum types.** A probe of `x == A` over `pub type Sel = | A | B` returns `error[E0013]: equality operators are not defined for \`Sel\` (operator traits arrive later in Phase 1)`. `Eq` is a trait with impls for `Int`/`Float`/`Bool`/`Char`/`String` only (`std/core/lib.nova:485-491`), and no std module compares a sum-type value anywhere. Every variant test in this design is therefore a `match`.
- **A bare `return` works in a function returning nothing.** Probed directly: `fn maybe(n: Int) { if n < 1 { return } println("ran") }` compiles and runs, printing nothing for `0` and `ran` for `5`. No std module does this, so it was measured rather than inferred — `emit`'s early exit depends on it.
- **`Display` already exists**, so `std/fmt` is thinner than its position suggests. `std/core/lib.nova:98`.
- **No date/time crate is permitted.** `00-MASTER-SPEC.md` §6, FINAL: `tokio`, `hyper`, `ring`, `serde`, `serde_json`, `toml`, `tracing`, `tracing-subscriber`. `serde_json`'s presence is what makes increment B's escaping a choice rather than a hand-rolling exercise.
- **No TTY detection anywhere.** `isatty`, `is_terminal` and `GetConsoleMode` appear nowhere in `crates/`.
- **The three counts, as declared:** `STD_ONLY: [Builtin; 60]` (`:669`), `RESERVED_TYPE_NAMES: [&str; 7]` (`:765`), `STD_MODULES: [(&str, &str); 9]` (`:1236`). This increment takes them to **64**, **7** and **10**.
