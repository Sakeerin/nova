# Phase 6.1: the playground

- **Date:** 2026-11-28. **Status:** approved by the user (2026-11-28), as
  drafted.
- **The user's decisions** of 2026-11-28, B1 to B12, are quoted in §11,
  with the spec's own.
- **Branch:** `claude/loving-archimedes-frtf5q-61`, from `main` at
  `9142d541` (6.0, #153). Nothing is pushed without the user's word.
- **The records' rule:** every record this pull request writes names
  `main`'s commits only, by 8 characters; pull request numbers are
  allowed; no local or scratch path, no person's name or email, no model
  identifier. The spike is "the spike of 2026-11-26, its code not
  committed"; the user is "the user".
- **Inputs:** `docs/phase-6-plan.md` §1's gate item 5 (`:63-68`), §3
  decision 12 (`:364-378`), §4's 6.1 (`:440-461`) and its 6.0 note
  (`:427-438`); ADR 0062 decisions 3 and 12
  (`docs/adr/0062-phase-6-scope.md:114-139`, `:186-191`) and its figures
  (`:193-228`); the research of 2026-11-28 at `9142d541`; the spike;
  ADRs 0039 and 0040; the 5.5a and 6.0 specs.

## 1. What 6.1 delivers

The master spec's Phase 6 item 5, as ADR 0062 decision 3 has it: S0's
front end and WASM backend, built for `wasm32-unknown-unknown`, compile
one source text in a browser worker, std embedded, into diagnostics or
`app.wasm` and `app.js`; the program runs in a fresh sandboxed iframe.
1. **`crates/nova-playground`,** unpublished: a safe `compile` on every
   target, a C ABI on wasm32 (§3.1); the shadow stack set at link time, a
   `[profile.playground]` and `wasm-opt` (§3.3, §3.4).
2. **`nova-codegen-wasm`:** the caller's thread on wasm32, the spike's one
   patch (§3.2); `init({ print })` in the shim (§3.5).
3. **The page,** `tools/playground/`, a build output 6.8 deploys (§3.6 to
   §3.8). **CI:** a Playground job, the target on 1.78 (§4); gate item 5's
   tests (§5). **ADR 0063** and the notes (§8). No new external crate, npm
   package at run time, release asset or error code.

## 2. The starting point

Read at `9142d541`; nothing built.

- **The front end reads through `Sources`**
  (`crates/nova-driver/src/analyze.rs:24-42`), but `nova-driver` cannot be
  the entry: it depends on all three backends and the native runtime
  (`crates/nova-driver/Cargo.toml:18-36`), and its front end is private
  (`FrontendContext`, `crates/nova-driver/src/lib.rs:1382-1402`) and
  renders to stderr (`:1460-1481`, `crates/nova-diagnostics/src/render.rs:16`).
  `build_program_wasm` (`lib.rs:865-892`, the driver's) compiles and makes the shim in
  memory (`:866-878`), then writes four files.
- **std is embedded** (`std/lib.rs:11-74`); `CompileTarget::std_modules`
  gives WASM 16 modules (`crates/nova-resolver/src/lib.rs:2867-2874`,
  `:2892-2909`). **Diagnostics render to text** (`render.rs:72`).
- **Codegen's thread:** `compile_module` runs `compile` on a scoped thread
  of 256 MB (`crates/nova-codegen-wasm/src/lib.rs:61-63`, `:196-208`); on
  wasm32 every compile fails at run time, "starting the compiling thread:
  operation not supported on this platform" (the spike).
- **The shim** (`crates/nova-codegen-wasm/src/shim.rs:186-226`): `write`
  goes to Node's streams, else to the console a line at a time
  (`shim/core.js:34-58`); `init` takes `options.wasm` as bytes
  (`shim/run.js:2-4`) and resolves when no task is left or the program
  ends (`:113-127`); `fail` writes `nova: stack overflow` or `nova: trap:
  …` for the program's own end (`:70-91`).
- **CI:** 19 jobs, cargo always `--locked` but `cargo fmt`
  (`.github/workflows/ci.yml:12-22`). The Browser job: `ubuntu-24.04`
  (`:281-286`), Node 22, `playwright@1.56.1`, `lighthouse@13.5.0` and
  `binaryen@132.0.0` in `$RUNNER_TEMP/pw` (`:306-309`), `wasm-opt` on
  `PATH` in one step (`:328-343`). The MSRV job: `RUSTUP_TOOLCHAIN:
  "1.78.0"` (`:420-425`), `dtolnay/rust-toolchain@1.78` with no input
  (`:429`), `cargo check --locked --workspace` under `RUSTFLAGS: -D
  warnings` (`:467-469`). No workflow names `wasm32`.
- **The workspace:** members `["crates/*", "std"]` (`Cargo.toml:1-3`); no
  `[profile.*]`; `Cargo.lock` at `version = 3`, 254 packages.
  `UNPUBLISHED` holds three (`crates/nova-cli/tests/release_manifests.rs:23`),
  with 20 published (`:160-173`) and an exact `=0.5.0` pin on every normal
  dependency between members (`:220-237`; dev-dependencies unchecked).
- **The spike of 2026-11-26, its code not committed:** about 60 lines of
  `nova-driver` copied behind a throwaway C ABI; figures in ADR 0062
  (`:209-223`); 1.78, `wasm-opt`, CI and `std/dom` unmeasured (`:225-228`).

## 3. The design, part by part

### 3.1 The crate `nova-playground` (B1)

**Files:** `crates/nova-playground/Cargo.toml`, `build.rs` (§3.3),
`src/lib.rs` (the API, the front end), `src/abi.rs` (the C ABI), `tests/`
(§5). **The manifest:** the workspace's version, edition, `rust-version`,
licence and links; `publish = false` with a comment, as
`crates/nova-bundler/Cargo.toml:10-13`; `[lib] crate-type = ["cdylib",
"rlib"]`, `bench = false`; `nova-ast`, `nova-codegen-wasm`,
`nova-diagnostics`, `nova-hir`, `nova-lexer`, `nova-mir`, `nova-parser`,
`nova-resolver` and `nova-typeck` as `{ workspace = true }`, the exact
pins `release_manifests.rs` asks (`nova-std` comes through the resolver);
the dev-dependency `nova-driver = { path = "../nova-driver" }`. Not in
`[workspace.dependencies]`, the published crates' (`release_manifests.rs:183-218`).
**`cdylib` and `rlib`:** on wasm32 the `cdylib` is the module with its
exports; the `rlib` lets host tests call `compile`. Host builds also link
an export-less shared library, seconds a system, as `nova-runtime` builds
two kinds (`crates/nova-runtime/Cargo.toml:24`); `rlib` alone with `cargo
rustc --crate-type cdylib` in CI would make local and CI builds differ.

**The API, on every target:**

```rust
/// The name the source is compiled under, as diagnostics show it.
pub const FILE_NAME: &str = "main.nova";
/// What [`compile`] gives.
pub enum Outcome {
    /// `app.wasm`, `app.js`, whether it calls std's `extern "js"`
    /// functions (`std/dom`, `std/ui`), and its warnings, rendered.
    Compiled { wasm: Vec<u8>, js: String, page: bool, warnings: String },
    /// The diagnostics as `nova` renders them (or a limit's), the errors.
    Failed { text: String, errors: usize },
    /// The compiler's own error, a bug, in `nova`'s words.
    Internal { text: String },
}
/// Compile `source` as `nova build --target wasm` compiles a loose
/// `main.nova` that imports no file, with std embedded.
pub fn compile(source: &str) -> Outcome;
```

`page` is true when MIR imports one of `std/dom`'s functions, an extern
of `ExternAbi::Js` named `dom_…` (`crates/nova-mir/src/lib.rs:85-91`);
`std/rt`'s own `host_*` externs, which every collecting program reaches
(`std/rt/lib.nova:1477-1479`, `:1562-1565`), do not count. A `Limit` is
`Failed`, `Error: ` and its words, as `nova` prints it; any other codegen
or shim error is `Internal`, with `build_program_wasm`'s context
(`crates/nova-driver/src/lib.rs:870-878`).

**The C ABI, `src/abi.rs`, behind `#[cfg(target_arch = "wasm32")]`:**

| Export | Signature | What it does |
|---|---|---|
| `alloc` | `(len: usize) -> *mut u8` | a buffer for the source's UTF-8 |
| `dealloc` | `(ptr: *mut u8, len: usize)` | frees it |
| `compile` | `(ptr: *const u8, len: usize) -> *const u32` | nine words: status; `app.wasm`'s, `app.js`'s and the text's pointer and length; the error count; `page` |
| `panic_info` | `() -> *const u32` | two words a panic hook fills: the message's pointer and length |

Status 0 is compiled (the text holds warnings), 1 failed, 2 internal; the
buffers live until the next `compile`; the module imports nothing. A
panic traps after the hook, so JS takes `panic_info`'s address at
instantiation and reads the message from memory after a trap, never
calling the dead instance. All `unsafe` is in `abi.rs`, allowed against
the crate's `#![deny(unsafe_code)]`, for 6.6's inventory
(`docs/phase-6-plan.md:553-556`); 1.78's forms (`#[no_mangle]`).

**The front end, copied:** about 60 lines of `nova-driver`'s private
logic, each copy's doc comment naming its original:

| Step | Copied from (`crates/nova-driver/src/`) |
|---|---|
| lex and parse, their diagnostics | `program.rs:437-447`, in `load_program` |
| `@test` functions dropped | `strip_test_functions`, `lib.rs:1697-1714`, a loose root's case |
| std's files registered | `register_std`, `lib.rs:747-758` |
| resolve, check, the entry's `main`, `std/rt`'s functions | `FrontendContext::check`, `lib.rs:1579-1615` |
| MIR | `lower_to_mir_for`, `lib.rs:1349-1361`; `mir_options`, `:998-1003` |
| codegen and the shim | `build_program_wasm`, `lib.rs:866-878` |
| verbatim | `NOT_THE_ENTRY_MAIN` `:967`, `runtime_impls` `:978-995`, `keep_the_entry_main` `:1009-1025` |
| errors counted, diagnostics kept | `FrontendContext::render`, `lib.rs:1460-1481`, rendered once by `render_to_string` |

Each stage stops on an error, as `check` does; `nova-driver` does not
change; `the_same_as_nova_driver.rs` holds the copy (§5). A split of the
driver would publish a 21st crate; a feature would put `cfg` in 25 places.

**`Cargo.lock`:** one block, `nova-playground` 0.5.0 with its ten member
dependencies (the nine and `nova-driver`), between `nova-parser`'s and
`nova-pm`'s (`Cargo.lock:1171-1183`); no other line, 255 packages;
`version = 3` expected to stay, as the prototype's diff shows (§6).
**`UNPUBLISHED`** becomes `[&str; 4]` with `nova-playground`
(`release_manifests.rs:23`), the test's name and the module's comment
saying four (`:161`, `:5`); the 20, `publish-crates.sh` and the package
gate stay. `ARCHITECTURE.md`'s table gains a row (`:34-58`).

### 3.2 Codegen's thread on wasm32

At the start of `compile_module` (`crates/nova-codegen-wasm/src/lib.rs:198`):

```rust
    // wasm32 has no threads to start (the playground, ADR 0063): compile on
    // the caller's stack, which the playground's build sets.
    if cfg!(target_arch = "wasm32") {
        return compile(mir);
    }
```

**Natively nothing changes:** `cfg!` is a constant `false`, so the thread,
its stack and the panic's resume run as before. `cfg!`, not `#[cfg]`:
both branches type-check everywhere, and `STACK` stays used on wasm32,
where `-D warnings` would refuse a dead constant. The backend's tests and
the WASM suite, unchanged, hold the native path; every compile of the
Node and Chromium tests the wasm32 one.

### 3.3 The stack

`crates/nova-playground/build.rs` prints
`cargo:rustc-link-arg-cdylib=-zstack-size=4194304` when
`CARGO_CFG_TARGET_ARCH` is `wasm32`, else nothing, so no host linker sees
it. A `RUSTFLAGS` (the MSRV job sets one) cannot drop it, as it drops
`.cargo/config.toml`'s `rustflags`, and a local build equals CI's.
**4 MiB:** Rust's shadow stack sits first in memory, 1 MiB by default. In
the spike, at depth 128, 768 KiB passed and 512 KiB trapped on 20 of 42
probes, while V8 used 350 to 450 KB of Node's 984 KB: the shadow stack
needs about 1.1 to 2.2 times V8's use. At 4 MiB V8 runs out first, so an
overflow is one kind of error (§3.7), for memory alone (a fresh instance
from 2.25 MB to about 5.25 MB); 8 MiB if the prototype sees an
out-of-bounds trap first. **Held by** the Node test: a fresh instance's
memory is over 4 MiB.

### 3.4 The profile and `wasm-opt` (B4)

In the root `Cargo.toml`, after `[workspace.dependencies]`, with a comment
naming this spec and saying only `--profile playground` uses it:
`[profile.playground]` with `inherits = "release"`, `opt-level = "z"`,
`lto = true`, `codegen-units = 1` and `strip = true`. No other profile
changes (there is none; a per-package override cannot set `lto`); custom
profiles and `strip` predate 1.78; no line starts with `version = `, which
the release scripts read (`release.yml:101`). The spike here: 1,534,479
bytes raw, 519,967 gzipped, hello world's first compile 126 ms, ready 60
ms after navigation, depth 128 surviving in a worker.

**`wasm-opt`,** binaryen 132.0.0: expected `-Oz --enable-bulk-memory
--enable-multivalue --enable-nontrapping-float-to-int
--enable-reference-types`, stable's wasm32 features beyond binaryen's
defaults (`crates/nova-cli/src/bundle.rs:288-302`). Unknown until the
prototype: binaryen refuses a feature it has not enabled, and `strip` may
drop the features section. If `wasm-opt` breaks the module or slows the
compile, the page ships cargo's output, within the limit already, and the
user is told. **The fallback (B4):** if the depth cases trap in CI's
Chromium worker at `z`, or its long-body limits fall well below the
spike's, opt-level 3 with LTO, one codegen unit and `strip` (666,364
bytes gzipped), and the user is told.

### 3.5 The shim's print hook

`init({ wasm, print })`, `print(fd, text)` taking each write to standard
output (1) and error (2) whole:
- **`shim/core.js`:** `let print = null;` beside `pending` (`:36`);
  `write` (`:37-48`) begins `if (print !== null) { print(fd, s); return;
  }`, before Node's streams, so a Node host with the hook gets the writes;
  the comment at `:34-35` names it.
- **`shim/run.js`:** `init` (`:116`) first sets `print = options.print ??
  null;`; its comment (`:113-115`) names the option.

**Without `print`** every byte goes where it went (`node app.js`, the
console's whole lines, `flush`). No test pins `write`'s text;
`wasm_tests.rs:620-622` reads `export default async function init(`,
which stays; none passes `print` (`tests/dom/counter.mjs:5`). The WASM
suite, `tests/dom/` and the Browser job are unchanged.

### 3.6 The page (B5 to B10)

**Files,** in `tools/playground/`, beside `tools/vscode-nova/`
(`release.yml`'s paths name only that and `tools/install/`, `:26-27`):

| File | What it holds |
|---|---|
| `index.html` | the examples' `<select>`, Run, Stop, a status line, the editor, the output pane, the program's frame; its own CSP `<meta>` |
| `style.css` | the look, light and dark |
| `page.js` | the worker, Run, Stop, the limits, the iframe, §3.7's messages |
| `compiler.js`, `worker.js` | the compiler's driver (load, compile, classify a throw, re-instantiate), shared with the Node test; the worker over it |
| `runner.js` | the runner's text, put in each iframe's `srcdoc` |
| `examples.txt` | the fixed list (§3.8): name, label, source path, expected output |
| `examples/` | `type_error.nova` and `.txt`, `hello.stdout`, `producer_consumer.stdout` |
| `build.mjs` | `node tools/playground/build.mjs <compiler.wasm> <out> [--wasm-opt]` |

**The build,** on Node's built-in modules, runs `wasm-opt` when asked,
copies the files and the module as `nova_playground.wasm`, and writes
`examples.json` from `examples.txt`, each text read from its path, so the
sources live once. It prints the raw, optimised and gzipped sizes (zlib
level 9, as `tests/browser/spa.mjs` measures) and fails at 5,000,000
bytes gzipped (B12). Every URL is relative, for 6.8's `/nova/` base.

**The editor (B6):** a `<textarea>`, Tab inserting four spaces, Ctrl or
Cmd with Enter running, opening on hello world. Nothing is stored or read
from the URL: no share links (B7).

**The compiler (B9):** `worker.js`, a module worker of the page's origin,
`WebAssembly.compile`s the fetched bytes once (whatever the MIME type) and
instantiates; it posts `ready` with its time, and answers `{ id, source
}` with `compiled` (`wasm`, `js`, `page`, `warnings`, `ms`), `failed`
(`text`, `errors`), `too-long` or `internal` (§3.7). After a throw it
instantiates the cached module afresh (1.4 ms in the spike). A compile
past 30 s, or Stop during one, makes the page replace the worker.

**The program (B5, B10):** each Run removes the last iframe and makes a
fresh `<iframe sandbox="allow-scripts">`, no `allow-same-origin`, so its
origin is opaque, its `srcdoc` a CSP `<meta>`, `<div id="app">` and the
runner. The runner posts `ready`; the page posts `{ js, wasm, page }`;
the runner loads `app.js` and calls `init({ wasm, print })`, posting each
write, then `end` when `init` resolves or `error` if it rejects. The page
takes only messages whose `source` is that iframe's window and shows them
by `textContent`, never as HTML, standard error styled apart. With `page`
set the program runs on the iframe's thread, the frame shown, so a
`std/dom` program draws there, its listeners live until Stop or the next
Run; a console program runs in a module worker the runner starts inside
the iframe, if Chromium allows one, else on its thread, the frame hidden.

**The CSP:** the iframe's allows its runner (a per-Run nonce), `app.js`
as the prototype finds it loads in an opaque origin (a `blob:` module, or
inline) and `'wasm-unsafe-eval'`; `default-src 'none'`, `connect-src
'none'`: `fetch` fails, nothing reaches the network. A `srcdoc` also obeys
the page's policy, which allows the page's own files and the runner. The
exact strings are the prototype's.

**The limits and Stop:** a Run not at `end` 10 s after it started is
stopped; Stop works at any time; both remove the iframe and its workers,
and say so. A program whose `init` resolved (a `std/dom` program waiting
on clicks) is not timed. **No npm package:** the browser alone.

### 3.7 Long function bodies (Q1)

**How it surfaces:** the structurer recurses once per sequential join or
loop (`crates/nova-codegen-wasm/src/lib.rs:61-63`). In the spike every V8 overflow was `RangeError:
Maximum call stack size exceeded`, out of `compile`; the shadow stack's,
below 1 MiB only, `RuntimeError: memory access out of bounds`. A worker
compiled 1,500 `if`s in a row and 600 `while` loops, overflowing at 2,000
and 800. **Caught in the worker** by `compiler.js`, which drops the
instance: a `RangeError` matching `/call stack/`, or that `RuntimeError`
with no panic message, is `too-long`; any other throw, one with a panic
message, or status 2 is `internal`.

**The page's messages:**
- *Too long:* "The playground's compiler ran out of stack on this
  program: one of its functions is probably too long for the compiler in
  the browser. Splitting it into smaller functions should help, and `nova
  build --target wasm` compiles it as it is on your own machine."
- *Internal:* "The compiler failed on this program: an internal error, a
  bug in Nova. Please report it with the program. The compiler said:"
  and the panic's message or the error.
- *Diagnostics:* the text, then "could not compile due to N previous
  error(s)", as `nova` ends it (`crates/nova-cli/src/cmd/run.rs:167`).
- *The limits:* "Stopped: the program was still running after 10
  seconds."; "Stopped."; "The compiler took more than 30 seconds and was
  restarted."

The program's own overflow stays the shim's `nova: stack overflow`
(`shim/run.js:74-76`). **No new error code:** JS, not the compiler, says
it; the playground is outside the promise (`docs/phase-6-plan.md:314-317`);
none fits: E0900 is "not supported yet", E0902 an extern unresolved at run
time (`crates/nova-driver/src/lib.rs:815-816`), P0002 nesting past 128
(`crates/nova-parser/src/error.rs:52-60`), and codegen's limits are uncoded
(`crates/nova-codegen-wasm/src/lib.rs:48-59`). 6.7's index needs no entry;
the iterative structurer stays on the backlog.

### 3.8 The fixed list (B8)

Each uses only WASM's std (`crates/nova-resolver/src/lib.rs:2892-2909`);
`examples.txt` names each source where it lives:

| Name | Source | Expected | Why it fits WASM's std |
|---|---|---|---|
| hello | `examples/01-hello-world/src/main.nova` | `Hello, World!` (`run_tests.rs:27-33`) | core's `println` |
| producer-consumer | `examples/03-producer-consumer/src/main.nova` | `got 1` to `got 5`, `done` (`run_tests.rs:1340-1346`) | `task`, `sync`, `time`; `async fn main` on the event loop (ADR 0040) |
| traits | `tests/runtime/traits.nova` | its `.stdout` | core; WASM gate (`wasm_tests.rs:2013`) |
| json_round_trip | `tests/runtime/json_round_trip.nova` | its `.stdout` | `std/json`; gate `:2036` |
| collections | `tests/runtime/collections.nova` | its `.stdout` | `std/collections`; gate `:1997` |
| iterator | `tests/runtime/iterator.nova` | its `.stdout` | core's iterators; gate `:2009` |
| counter | `examples/06-counter-spa/src/main.nova` | `Count: 2` after `+` three times, `-` once (`tests/browser/spa.mjs:92-94`) | `std/ui`, `std/dom`; the Browser job builds it (`ci.yml:305`) |
| type_error | `tools/playground/examples/type_error.nova` (new) | its `.txt` | E0010 on both targets |

`hello.stdout` and `producer_consumer.stdout` copy `run_tests.rs`'s
strings; `type_error.nova` is new (the spike's: a `String` bound to an
`Int` call), its text `nova run`'s, the file named `main.nova`. Not fit:
`02-fibonacci`, `04-todo-cli` (`std/process`, `std/fs`), `03-http-server`,
`05-json-api` (`std/http`), `05-geometry`, `07-fullstack-blog`.

## 4. CI

**The Playground job (B3),** `name: Playground`, after the Browser job,
on `ubuntu-24.04`, the Browser job's image (Playwright 1.56.1 needs it,
`ci.yml:283-286`). Its steps:
1. `actions/checkout@v4`; `dtolnay/rust-toolchain@stable` with `targets:
   wasm32-unknown-unknown` (B2), as `release.yml:61-63` uses the input
   (not `rust-toolchain.toml`, which every local rustup would follow),
   and `components: clippy`; `Swatinem/rust-cache@v2`; Node 22.
2. `cargo build --locked -p nova-playground --profile playground --target
   wasm32-unknown-unknown`.
3. `cargo clippy --locked -p nova-playground --target
   wasm32-unknown-unknown -- -D warnings`: the Clippy job's hosts never
   compile `abi.rs` (as `ci.yml:368-378` says of `cfg(windows)`). Neither
   command takes `--all-targets`, which would build the dev-dependency
   `nova-driver`, and the native runtime, for wasm32.
4. `npm install --no-save --prefix "$RUNNER_TEMP/pw" playwright@1.56.1
   binaryen@132.0.0`, then Chromium, as `ci.yml:306-309`.
5. The page (`id: page`), binaryen on `PATH` for this step alone:
   `wasm-opt --version`; `node tools/playground/build.mjs
   target/wasm32-unknown-unknown/playground/nova_playground.wasm
   "$RUNNER_TEMP/playground" --wasm-opt`: three sizes printed, failing at
   5,000,000 bytes gzipped.
6. `node tests/playground/node.mjs "$RUNNER_TEMP/playground"`.
7. `node tests/browser/playground.mjs "$RUNNER_TEMP/playground"
   "$RUNNER_TEMP/pw"`: prints the ready time, hello world's first compile
   and the depth summary; fails at 2 s or on a trap at 128.
8. `actions/upload-artifact@v4`, `name: playground`, the built page, `if:
   ${{ !cancelled() && steps.page.outcome == 'success' }}` (B11), for the
   user to try with any static server; no release carries it.

**The MSRV job (B2):** `targets: wasm32-unknown-unknown` on
`dtolnay/rust-toolchain@1.78` (`:429`), and after the workspace check a
step `cargo check --locked -p nova-playground --target
wasm32-unknown-unknown`, the same `RUSTFLAGS: -D warnings`: the only place
1.78 sees the wasm32 code. The guard stays. 1.78 emits an MVP-style
module where stable enables six features; the playground ships stable's
build, and 1.78 only checks. If 1.78 cannot, the user is asked.

**CI goes from 19 jobs to 20.** Test, Selfhost, Clippy, Installed nova
and Packaged nova build the new member for their host; the Test job runs
its host tests on three systems (`NOVA_REQUIRE_NODE=1`, `ci.yml:73-78`);
the package gate still packages 20 crates. **Workflows on the pull
request:** CI, all 20 jobs (`ci.yml:3-6`); Release, since `Cargo.toml`
and `Cargo.lock` change (`release.yml:16-27`): its four Build jobs, the
VS Code extension, std's site and Prepare, Publish skipped (`:378`), none
building the new crate. Benchmarks run on `main` after the merge
(`benchmarks.yml:3-5`).

**Time (estimated):** the Playground job 5 to 8 minutes cold (the spike's
wasm32 build 20 to 27 s locally; Playwright's install a minute), in
parallel, so the wall time should hold (#153's run 45954286014: 19 jobs
in about 12 minutes); MSRV about a minute more; host builds seconds more.

## 5. Testing

Test first: each test seen to fail before its code. No test touches the
internet (the page is served from 127.0.0.1, port 0); fixed temporary
names, `nova-playground-test-<name>`; no host test reads `$NOVA_HOME`
(loose files only), and one that comes to need it sets it to a fresh
directory; 1.78's forms, no `unwrap()` outside tests, rustdoc throughout.

1. **Unit tests** in `src/lib.rs`: hello compiles (`\0asm`, `page`
   false); a type error fails with one error naming `main.nova`; warnings
   travel with a success; an `@test` function is dropped; `page` is true
   for the counter; an empty source fails. The Node test drives `abi.rs`.
2. **The host equality test,** `tests/the_same_as_nova_driver.rs`, in
   `cargo test --workspace` on three systems, each case on a thread of
   `nova_driver::PIPELINE_STACK` (`crates/nova-driver/src/lib.rs:794`):
   every program of `examples.txt`, and inline sources with an `@test`
   function, an import of a missing module, a lex error and a parse
   error, written as `main.nova` in a fresh fixed directory and built by
   `build_program_wasm`. A success's `app.wasm` and `app.js` equal
   `compile`'s byte for byte; a failure's error count equals
   `Outcome::Failed`'s, and its text `render_to_string` over
   `nova_driver::analyze`'s diagnostics (the CLI's `load_program`,
   `analyze.rs:128`), the file named `main.nova`; type_error's text equals
   `type_error.txt`; `examples.txt` holds eight entries, each path there.
3. **The print hook,** `tests/print_hook.rs`, with the `NOVA_REQUIRE_NODE`
   pattern of `wasm_tests.rs:563-582`: `print("a")`, `println("b")`,
   `eprintln("c")`, `print("d")`, compiled by `compile` on the host and run
   by Node with `init({ wasm, print })`, give the hook `[1,"a"]`,
   `[1,"b\n"]`, `[2,"c\n"]`, `[1,"d"]` in order and Node's streams
   nothing; without `print`, `node app.js` prints as before.
4. **Codegen's `cfg`:** natively, the backend's tests and the WASM suite,
   unchanged; on wasm32, every compile of 6 and 7. **5. The manifests:**
   `release_manifests.rs`, four unpublished.
6. **The Node test,** `tests/playground/node.mjs <page>`, on the shipped
   module through `compiler.js`: a fresh instance's memory over 4 MiB;
   each console example run by `init({ wasm, print })` from a `data:`
   import, against its text, standard error empty; `page` set for the
   counter alone; type_error's text and count; the depth cases of
   `tests/playground/depth.mjs` (5.1b's 26 kinds of
   `crates/nova-parser/tests/depth.rs` and 16 that type-check, written
   afresh): at 128 no throw and each typed kind's output (nested `fn`
   items E0900, as natively), at 129 P0002; 10,000 `if`s in a row
   `too-long`, then hello on the next instance; 10,000 `while` loops on
   the locals limit. A script, not a Rust wrapper: the module exists only
   in this job, which runs it as the Browser job runs its scripts.
7. **The Chromium test,** `tests/browser/playground.mjs <page> <modules>`,
   like `counter.mjs` (its server, `:30-46`), on the real page: the ready
   time; hello world's first compile, read from the status line's data
   attribute, printed, under 2,000 ms (B12); each console example's pane;
   type_error's diagnostics and closing line; the counter clicked to
   `Count: 2` in its iframe; the depth cases through the editor, in the
   worker, where headroom is thin; 10,000 `if`s showing the too-long
   message, then hello; a loop stopped by Stop, then by the 10 s limit,
   the page answering throughout; a `std/dom` fetch to the test server
   failing, the server seeing no request; every request one of the page's
   files, none to another host.

**The limits (B12):** size in the build (§3.6), time and depth in 7,
depth in 6 too. **Mutants,** once in the prototype, each caught: no
`@test` strip in the copy (2); the hook ignoring `fd` (3); `build.rs`
without its argument (6); the classification reversed (6, 7);
`allow-same-origin`, or no `connect-src 'none'` (7); no time limit (7).

## 6. The prototype

A local branch from the spec's commit, as 5.5a's. It finds out:
1. **1.78 on wasm32,** under `-D warnings`: locally only if the user
   allows the target on the local 1.78 toolchain (6.0's word was for the
   spike), else on the draft pull request.
2. **The build:** both crate types under `z` with LTO; **`wasm-opt`:**
   binaryen 132.0.0's flags, the module valid after it, its size, its
   effect on compile time and depth; on the draft pull
   request, or locally if the user allows binaryen in scratch space.
   **Chromium in CI:** the first compile, the ready time, depth and the
   long-body limits in the worker.
3. **The iframe:** a sandboxed `srcdoc` iframe running the program under
   the CSP; `app.js` as a `blob:` module or inline; a module worker inside
   it; whether Chromium runs it off the page's thread (Stop during a
   loop). With neither a worker nor another thread, the user is asked.
4. **Headroom** at 4 MiB and `z` with LTO: which overflow comes first;
   depth 128 under Node at reduced `--stack-size`, as the spike measured;
   B4's fallback if thin.
5. **`Cargo.lock`:** one block, `version = 3`; if a newer cargo writes 4,
   the local 1.78 cargo (default 3) adds the block, or the user is asked.
6. **CI's times** (§4), and §5's mutants.

**A draft pull request,** on the user's word for the push, measures CI
only, closed unmerged, as 5.5a's #144, before the plan.

## 7. The cut

One pull request, "Phase 6.1: the playground", in tasks, test first,
merged by rebase on the user's word:
1. `docs: the Phase 6.1 spec is approved`; 2. `Plan Phase 6.1: the
   playground, and the spec's dated amendment`;
3. `nova-codegen-wasm: compile on the caller's thread on wasm32, and
   init's print hook` (§3.2, §3.5);
4. `nova-playground: S0's front end and WASM backend, for the browser`
   (§3.1, §3.3, the profile, `Cargo.lock`, `UNPUBLISHED`,
   `ARCHITECTURE.md`; §5's 1 to 3 and 5);
5. `playground: the page, its build and the fixed list` (§3.6 to §3.8);
6. `ci: the Playground job, and wasm32 in the MSRV job` (§4; §5's 6, 7);
7. `6.1: the review's fixes`; 8. `docs: ADR 0063, the playground, and the
   notes for 6.1` (§8, with CI's figures).

About 1,200 to 1,700 lines of Rust, JS, HTML and CI with tests (the
spike's entry was 320), at or above the plan's 800 to 1,500.

## 8. Records

- **ADR 0063, the playground:** the crate (unpublished, `cdylib` and
  `rlib`, the C ABI with no `wasm-bindgen` and no import, the copy and its
  test); codegen's `cfg`; the shadow stack; the profile and `wasm-opt`'s
  flags; the print hook in every `app.js`; the page (worker, opaque-origin
  iframe, CSP, limits, Stop, `std/dom` in the frame, textarea, no share
  links, no npm); the long-body message, no code; the fixed list and
  tests; the CI job and the 1.78 check; CI's figures against ADR 0062's
  limits (sizes raw, optimised, gzipped; ready time; hello world's first
  compile; depth and the long-body limits in the worker); no release
  asset; 6.8 deploys.
- **`CHANGELOG.md`'s `[Unreleased]`,** after 6.0's note, `### Added`:
  **the playground**, a page compiling Nova in the browser with `nova`'s
  own front end and WASM backend, built for WebAssembly by the unpublished
  `nova-playground`, running the program in a sandboxed frame; a CI
  artifact until 6.8 puts it on the site; CI's size and time. **`init`
  takes `print(fd, text)`,** every write whole; without it nothing
  changes.
- **The plan's 6.1 entry,** a dated note after `:461`, as 6.0's: what was
  built, CI's figures, the long body caught, the page an artifact until
  6.8; "This plan's `path:line`s stay `4fe11d55`'s."
- **Dated notes:** the master spec, after Phase 6's 6.0 note
  (`nova-spec/00-MASTER-SPEC.md:944-956`), item 5 built, ADR 0063;
  `nova-spec/14-CODEGEN.md` §8, after `:356`, the caller's thread on
  wasm32 and `init`'s `print`; `agent.md` §5, under the crates' table.
  ADR 0063 cites this spec and its plan.

## 9. Risks

1. **Headroom in the worker:** 1.1 to 1.4 times at 128, estimated
   locally; `wasm-opt` and CI's Chromium may cut it. The Chromium test
   fails on a trap; B4's fallback, then the user.
2. **The iframe's thread:** where a browser runs a sandboxed `srcdoc`
   iframe on the page's thread, a `std/dom` program that never returns
   freezes the tab before the limit fires; CI has Chromium alone. Console
   programs in a worker inside the iframe escape it (§6 item 3).
3. **1.78 on wasm32,** unchecked: a failure needs a fix or a ruling.
   **`wasm-opt`:** flags, validity, a slower compile (cargo's output is
   within the limit). **The copy drifting:** the equality test fails.
4. **What a `std/dom` program can do:** run JS through `dom_set_property`
   (`shim.rs:107`) and navigate its own frame; the opaque origin,
   `connect-src 'none'` and no `allow-same-origin`, popups or top
   navigation keep the account's other sites out of reach
   (`docs/phase-6-plan.md:720-723`); 6.6's threat model takes it up.
5. **CI time;** Chromium's timing (2 s against 126 to 197 ms);
   `Cargo.lock`'s format (§6 item 5). **Unmeasured:** phones, slow
   machines, a real network, the history router in `about:srcdoc`.

## 10. Not in 6.1

The site, its Pages deploy, a release asset (6.8); share links, an editor
library, `fetch` from programs (B7, B6, B10); N built for WASM and an
iterative structurer (the backlog); std's checked items kept across
compiles (6.0's ruling); several files or packages; a server compile;
offline caching; any change to `nova-driver`, the promise or the codes.

## 11. Decisions

The user's, of 2026-11-28, quoted as given: each option's name in Thai
ends "(แนะนำ)", "recommended"; the text after it says what was chosen.

- **B1** "crate ใหม่ ไม่ publish (แนะนำ)": `crates/nova-playground`,
  `publish = false`, its `Cargo.lock` block by the user's word,
  `UNPUBLISHED` three to four, a host test against `build_program_wasm`.
- **B2** "Playground job + MSRV 1.78 (แนะนำ)": the target in a new job and
  in the MSRV job, checking the crate for wasm32 on 1.78. **B3** "job ใหม่
  "Playground" (แนะนำ)": 20 CI jobs.
- **B4** "[profile.playground] z+LTO (แนะนำ)": inheriting release, `z`,
  LTO, one codegen unit, `strip`; opt-level 3 with LTO if the worker's
  stack proves thin, the user told.
- **B5** "ใน iframe + ตัวอย่าง dom 1 ตัว (แนะนำ)": a fresh sandboxed iframe
  per Run (`allow-scripts`, opaque origin, `srcdoc`, CSP); console output
  to the pane, `std/dom` drawn in the iframe; Stop and the limit remove it.
- **B6** "<textarea> ไม่มี dependency (แนะนำ)": no npm package. **B7**
  "ไม่มี (แนะนำ)": no share links.
- **B8** "ชุดเล็ก ~6 ตัว (แนะนำ)": hello world, producer-consumer, traits,
  json_round_trip, collections, iterator, the counter (dom), type_error.
- **B9** "ใน worker (แนะนำ)": the compiler in a worker. **B10** "ปิด (CSP
  connect-src 'none') (แนะนำ)": no network for programs. **B11** "ไม่ใส่
  (แนะนำ)": a CI artifact only; `release.yml` unchanged.
- **B12** "assert ใน CI (แนะนำ)": the job fails at 5 MB gzipped, at 2 s
  for hello world, or on a trap at depth 128, and prints the figures.

**Made in this spec, each (recommended, not asked):**

1. `nova-playground`, `cdylib` and `rlib`, a safe `compile` everywhere,
   the C ABI on wasm32 alone: four exports, nine words, no import, the
   panic read without a call (§3.1).
2. The front end copied, each original named, `nova-driver` unchanged;
   failures compared through `nova_driver::analyze` (§3.1, §5).
3. `cfg!`, not `#[cfg]` (§3.2); 4 MiB of shadow stack from `build.rs`, 8
   MiB if the prototype asks (§3.3); `wasm-opt -Oz` with stable's
   features, measured (§3.4).
4. The page in `tools/playground/`, one Node build script, `examples.txt`
   naming the sources where they live; the counter is
   `examples/06-counter-spa` (§3.6, §3.8).
5. 10 s for a Run until `init` resolves, 30 s for a compile; console
   programs in a worker inside the iframe if Chromium allows; `page` from
   MIR's `dom_…` externs; output by `textContent`; the page's own CSP
   (§3.6). The messages as drafted, with no code (§3.7).
6. 5 MB as 5,000,000 bytes, zlib level 9; hello world's first compile in
   the worker the timed figure (§3.6, §5). Clippy for wasm32 in the
   Playground job; the artifact when the page built (§4).
7. The Node test a script; §5's mutants; `#![deny(unsafe_code)]` but
   `abi.rs`; ADR 0063, §8's notes; one pull request (§7).

## 12. Gate

6.1 is done when its pull request is merged by rebase on the user's word,
CI green on it and on `main`'s merged commit, and:
- **both tests pass in CI** (gate item 5): Node and Chromium compile and
  run hello world and the fixed list against their expected output, show
  type_error's diagnostics and the long-body message, a compile after it;
- **size and compile time recorded in ADR 0063, within ADR 0062's limits**
  (under 5 MB gzipped; hello world under 2 s in Chromium on CI) **or as
  the user rules;** every depth case at 128 survived in the worker;
- **the WASM suite and the Selfhost job unchanged:** none of their files
  edited, all passing, `selfhost.rs` within its lines;
- the host tests pass on three systems; the MSRV job checks the crate for
  wasm32 on 1.78, or the user has ruled; fmt, Clippy (three hosts and
  wasm32) and the package gate's 20 crates pass; `Cargo.lock` gains one
  block; the Release run is green, Publish skipped;
- ADR 0063 and §8's notes are merged. Then 6.2, if not under way, waits
  for the user's word.
