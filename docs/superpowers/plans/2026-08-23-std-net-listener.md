# `std/net` Listener Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give `std/net` a server side — `TcpListener`, `bind`, `local_port`, `accept` — so Phase 2 position 10 (`std/http`) has a transport to build on.

**Architecture:** Three new intrinsics in `crates/nova-runtime/src/net.rs`. Two of them (`net_listen`, `net_local_port`) cannot suspend, so they take `nova_rt_net_close`'s plain status-word shape and get no poll function. One (`net_accept`) does suspend, so it gets exactly one `extern "C-unwind"` poll function, level-triggered and tag-free. Listeners share the existing socket table via a two-variant enum, which means `close` is reused unchanged and needs no fourth intrinsic.

**Tech Stack:** Rust (`std::net::TcpListener`, no new dependency), Nova (`std/net/lib.nova`), the existing `select`/`WSAPoll` poller.

**Spec:** `docs/superpowers/specs/2026-08-23-std-net-listener-design.md`

## Global Constraints

- `cargo build --locked --workspace` **before** `cargo test`. `--no-fail-fast`.
- **Sum every `test result:` line across all 44 targets. Never pipe cargo output through `head` or `tail`.** Baseline: **1057 passed / 0 failed / 8 ignored**.
- Clippy `--all-targets -- -D warnings` on ubuntu **and** windows; `cargo fmt --all -- --check`.
- **MSRV 1.78: no `reason = "..."` in any lint attribute.**
- The ignored GC tests stay ignored and untouched. The count is **8 unconditional attributes** (six in `gc.rs`, two in `task.rs`) plus **one conditional** — `#[cfg_attr(target_os = "linux", ignore = ...)]` on `extern_ffi_run` — so the runtime count is 8 on Windows/macOS and **9 on Linux**, and CI's advisory `--ignored` step is red on Linux **by design**.
- The poll ABI is **frozen**: `PollFn = unsafe extern "C-unwind" fn(*mut u8, *mut u8) -> i64`, `POLL_PENDING = 0`, `POLL_READY = 1`, `task_ctx` always null.
- **No panic may cross a generated poll boundary.**
- `std/net/lib.nova` is `include_str!`'d, so editing it forces a full workspace rebuild.
- Commit messages to a UTF-8 file applied with `git commit -F`, **never a heredoc**. Each body ends exactly `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.
- **Cite no SHA that is not already an ancestor of `main`.** `3901db5` is. The spec commit is branch-local — do not cite it in any file.
- **Byte-scan every file you write**: no byte below 0x20 outside tab/CR/LF, no `0x7f`, valid UTF-8, and **zero** occurrences of a backslash-`u` escape followed by four hex digits in tracked markdown. Write code points as `U+XXXX`.

## THE BUILD-BREAKING HAZARD, READ BEFORE WRITING ANY RUST

`no_net_intrinsic_can_panic` (`crates/nova-runtime/src/net.rs:2683`) **fails the build** on `.borrow_mut()`, `.borrow()`, `unwrap()`, `.expect(`, `panic!` or `format!` appearing anywhere in the **production half** of `net.rs` — **including inside a doc comment**. It is a grep, so the error names the pattern, not your line. Use the existing non-panicking helpers:

- `with_fd`-family accessors (they use `try_borrow_mut` and `abort_with`, never `borrow_mut`)
- `closed_fd_error()` (`net.rs:185`) → `fail(&std::io::Error::other("socket is not open"))`
- `fail(&e)` for a real `std::io::Error`
- `crate::task::abort_with(...)` for the genuinely impossible

Test code below `#[cfg(test)]` may use `.expect(...)` freely — the 12 existing test call sites do.

## File Structure

| File | Responsibility | Task |
|---|---|---|
| `crates/nova-runtime/src/net.rs` | the `Sock` enum, kind-checked accessors, three intrinsics, one poll function, Rust unit tests | 1, 2, 3 |
| `crates/nova-resolver/src/lib.rs` | `Builtin` variants, name table, `STD_ONLY` 66 → 69 | 1, 2, 3 |
| `crates/nova-typeck/src/check.rs` | group arm, `builtin_signature`, `#[cfg(test)]` description table | 1, 2, 3 |
| `crates/nova-mir/src/lib.rs` | `RtFunc` variants, symbol names, MIR signatures | 1, 2, 3 |
| `crates/nova-mir/src/lower.rs` | lowering | 1, 2, 3 |
| `crates/nova-codegen-cranelift/src/lib.rs` | nothing to edit — its guard test verifies `symbols()` | 1, 2, 3 |
| `std/net/lib.nova` | `TcpListener`, `bind`, `local_port`, `accept`, `close` | 1, 2, 3 |
| `tests/runtime/net_listener_accept.nova` + `.stdout` | the end-to-end fixture | 4 |
| `crates/nova-cli/tests/run_tests.rs` | the fixture's `#[test]` — **registration is not automatic** | 4 |
| `nova-spec/20-STDLIB.md`, `docs/adr/0013-io-poller.md`, `CHANGELOG.md` | records | 5 |

## The 12-site intrinsic checklist

Each of the three intrinsics pays this. ADR 0018 §3 states the counting rule; `str_to_float` is the most recent worked example and the best anchor to read at every site. **7 of the 12 are compiler-forced, and reaching 7 requires `--all-targets`** — a plain `cargo check --workspace` finds 6 and reports success, because the typeck description table sits under `#[cfg(test)]`.

1. `crates/nova-resolver/src/lib.rs` — `Builtin` enum variant
2. `crates/nova-resolver/src/lib.rs` — doc comment on that variant
3. `crates/nova-resolver/src/lib.rs` — name table (`fn name`)
4. `crates/nova-resolver/src/lib.rs` — `STD_ONLY` array **and its declared length**
5. `crates/nova-typeck/src/check.rs` — std-only group arm
6. `crates/nova-typeck/src/check.rs` — `builtin_signature`
7. `crates/nova-typeck/src/check.rs` — `#[cfg(test)]` description table
8. `crates/nova-mir/src/lib.rs` — `RtFunc` enum variant
9. `crates/nova-mir/src/lib.rs` — `RtFunc::name()` symbol string
10. `crates/nova-mir/src/lib.rs` — MIR signature
11. `crates/nova-mir/src/lower.rs` — the lowering arm
12. `crates/nova-runtime/src/lib.rs` — `symbols()` registration

**Site 12 is the one that survives every compiler to JIT link time.** Its guard is `every_rt_func_symbol_is_registered_with_the_jit` in `crates/nova-codegen-cranelift/src/lib.rs`. Omit it and the workspace compiles clean, then dies inside cranelift-jit.

---

## Task 1: The `Sock` enum, `net_listen`, and `bind`

**Files:**
- Modify: `crates/nova-runtime/src/net.rs:102-127` (table and `with_fd`), plus the 3 production call sites at `:541`, `:689`, `:722` and the 12 test call sites the compiler will name
- Modify: the 12 checklist sites for `net_listen`
- Modify: `std/net/lib.nova`
- Test: `crates/nova-runtime/src/net.rs` `#[cfg(test)]`

**Interfaces:**
- Produces: `nova_rt_net_listen(addr: *const NovaStr) -> i64`; Nova `pub record TcpListener { fd: Int }` and `pub fn bind(addr: String) -> Result<TcpListener, IoError>`; Rust helpers `with_stream`, `with_listener`, `wrong_kind_error`

- [ ] **Step 1: Write the failing Rust unit tests**

Put these in `net.rs`'s existing `#[cfg(test)] mod tests`.

```rust
#[test]
fn net_listen_binds_an_ephemeral_port_and_registers_a_listener() {
    let addr = nova_str("127.0.0.1:0");
    let status = unsafe { nova_rt_net_listen(&addr) };
    assert_eq!(status, 0, "binding loopback port 0 must succeed");
    let fd = taken_i64();
    assert!(fd > 0, "fd must be a real handle, got {fd}");
    assert!(is_listener(fd), "fd must be registered as a listener");
    unsafe { nova_rt_net_close(fd) };
}

#[test]
fn net_listen_reports_an_error_for_an_unbindable_address() {
    // Port 1 on a non-local address cannot be bound.
    let addr = nova_str("192.0.2.1:1");
    let status = unsafe { nova_rt_net_listen(&addr) };
    assert_ne!(status, 0, "binding an unroutable address must fail");
}

#[test]
fn reading_a_listener_fd_is_a_wrong_kind_error_not_a_closed_one() {
    let addr = nova_str("127.0.0.1:0");
    assert_eq!(unsafe { nova_rt_net_listen(&addr) }, 0);
    let fd = taken_i64();
    let status = unsafe { nova_rt_net_read(fd, 16) };
    assert_ne!(status, 0, "read on a listener must not succeed");
    unsafe { nova_rt_net_close(fd) };
}
```

`nova_str` and `taken_i64` are helpers the existing tests already use for building a `NovaStr` and decoding `Slot::Buffer`; locate them by name rather than by line and reuse them. If either does not exist under those names, find the equivalent the existing `connect` tests use and match it.

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cargo test -p nova-runtime net_listen
```

Expected: FAIL — `nova_rt_net_listen` not found.

- [ ] **Step 3: Introduce the `Sock` enum and split the accessor**

Replace `net.rs:102-127`'s table and `with_fd`. **`remove_socket` needs no change** — it removes by key regardless of variant, which is exactly why `close` is reused and there is no fourth intrinsic.

```rust
/// A registered handle: either a connected stream or a listening socket.
///
/// One table rather than two, so there is one `NEXT_FD` space, one closedness
/// invariant and one `close`. The payoff is that a kind mismatch is an
/// explicit error: a read against a listener fd reports wrong-kind rather
/// than reporting "closed", which is the wrong-and-plausible answer a second
/// table would give.
enum Sock {
    Stream(TcpStream),
    Listener(TcpListener),
}

/// Run `f` against the stream behind `fd`. `None` covers three cases the
/// caller reports identically: absent (closed, stale or forged) and
/// wrong-kind.
fn with_stream<R>(fd: i64, f: impl FnOnce(&mut TcpStream) -> R) -> Option<R> {
    SOCKETS.with(|sockets| {
        let Ok(mut sockets) = sockets.try_borrow_mut() else {
            crate::task::abort_with("nova_rt_net: handle table is already borrowed")
        };
        match sockets.get_mut(&fd) {
            Some(Sock::Stream(s)) => Some(f(s)),
            _ => None,
        }
    })
}

/// Run `f` against the listener behind `fd`. Same three-into-one collapse as
/// [`with_stream`].
fn with_listener<R>(fd: i64, f: impl FnOnce(&mut TcpListener) -> R) -> Option<R> {
    SOCKETS.with(|sockets| {
        let Ok(mut sockets) = sockets.try_borrow_mut() else {
            crate::task::abort_with("nova_rt_net: handle table is already borrowed")
        };
        match sockets.get_mut(&fd) {
            Some(Sock::Listener(l)) => Some(f(l)),
            _ => None,
        }
    })
}
```

Change `SOCKETS`'s type to `RefCell<HashMap<i64, Sock>>` and `register_new_socket` to take a `Sock`. Add a sibling `register_new_listener` **or** have the one function take `Sock` — either is fine, pick one and be consistent.

**Do not add a distinct wrong-kind status code.** Both wrong-kind and absent reach `closed_fd_error()`, and the spec records that collapse deliberately: `IoErrorKind` has no variant for it and adding one is a wire-contract change across `fs.rs` and `std/io`, out of scope here.

- [ ] **Step 4: Update the three production call sites**

The compiler will name them. They are `net.rs:541` (`take_error`), `:689` (`Read::read`), `:722` (`Write::write`) — each becomes `with_stream(...)` unchanged otherwise. The 12 test call sites also become `with_stream`; they use `.expect("fd must be open")`, which is permitted under `#[cfg(test)]`.

- [ ] **Step 5: Write `nova_rt_net_listen`**

Model it on `nova_rt_net_close` (`net.rs:199-202`) — a plain status word, no future, no poll function.

```rust
/// Bind and listen on `addr`, registering a non-blocking listener.
///
/// Non-suspending: binding and listening do not block, so this is an ordinary
/// status word like [`nova_rt_net_close`] rather than a future constructor.
/// On success the new fd is stashed via `Slot::Buffer` exactly as
/// `connect`'s is, so the Nova side decodes it with the same `decode_count`.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_net_listen(addr: *const NovaStr) -> i64 {
    let addr = as_str(addr);
    let listener = match TcpListener::bind(addr) {
        Ok(l) => l,
        Err(e) => return fail(&e),
    };
    if let Err(e) = listener.set_nonblocking(true) {
        return fail(&e);
    }
    let fd = register_new_socket(Sock::Listener(listener));
    stash_i64(fd);
    OK
}
```

`set_nonblocking(true)` is load-bearing, not hygiene: without it `accept` blocks the whole executor thread instead of parking, and Task 3's mutation step proves it.

- [ ] **Step 6: Add the 12 checklist sites for `net_listen`**

Walk the checklist above with `str_to_float` open as the anchor. The typeck signature is one `String` parameter returning `Int`.

- [ ] **Step 7: Add the Nova surface**

In `std/net/lib.nova`, after the `TcpStream` block:

```nova
// A listening socket. Server side of `std/net`, added because Phase 2
// position 10 (`std/http`) has no transport without it.
pub record TcpListener { fd: Int }

// Bind and listen on `addr` ("host:port"). Pass port 0 to let the kernel
// choose, then read it back with `local_port`.
//
// **A plain `fn`, not an `async fn`: this cannot suspend.** Binding and
// listening never block, so `net_listen` is an ordinary status word and
// there is no future to await. Forcing `.await` here would misstate the
// cost. Note the deliberate exception at `TcpListener::close` below.
//
// An address already in use arrives as `IoError { kind: Other }`, not a
// distinct kind: `IoErrorKind` has eight variants and `AddrInUse` is not
// among them, so the most likely failure of this function is discriminable
// only by `message`, whose text is the operating system's own. Adding a
// variant is a wire-contract change across the runtime's `fail` and
// `std/io`'s `io_error_kind_of` together, and is deliberately not done here.
pub fn bind(addr: String) -> Result<TcpListener, IoError> {
    let status = net_listen(addr)
    if status == 0 {
        return Ok(TcpListener { fd: decode_count(fs_take_bytes()) })
    }
    Err(IoError { kind: io_error_kind_of(status), message: fs_last_error_message() })
}

impl TcpListener {
    // Stops accepting and releases the port. Idempotent, for the same reason
    // `TcpStream::close` is: Nova has no move checking, so `self` by value
    // cannot prevent a second call.
    //
    // **An `async fn` even though it cannot suspend** -- the one deliberate
    // exception to the rule stated at `bind` above. It calls the same
    // `net_close` a stream does, with no `.await`, because the runtime's
    // table removes an entry by key regardless of which kind it holds. It is
    // declared `async` to match the shipped `TcpStream::close`: a server
    // closes both handles in sequence, and a reader should not have to
    // remember which of two identically named socket methods needs `.await`.
    pub async fn close(self) -> Result<(), IoError> {
        let status = net_close(self.fd)
        if status == 0 {
            return Ok(())
        }
        Err(IoError { kind: io_error_kind_of(status), message: fs_last_error_message() })
    }
}
```

- [ ] **Step 8: Build, then run the whole suite**

```bash
cargo build --locked --workspace
```

```bash
cargo test --workspace --no-fail-fast
```

Sum **every** `test result:` line across all 44 targets. Expected: **1060 passed / 0 failed / 8 ignored** — the baseline 1057 plus this task's three new Rust tests. Report the real numbers whatever they are.

- [ ] **Step 9: Clippy, fmt, and the byte scan**

```bash
cargo clippy --all-targets --workspace -- -D warnings
```

```bash
cargo fmt --all -- --check
```

If the build fails naming a grep pattern rather than a line, that is `no_net_intrinsic_can_panic` — find the `unwrap`/`expect`/`format!`/`borrow_mut` you added in the production half, including in a doc comment.

- [ ] **Step 10: Commit**

Write the message to a UTF-8 file and apply it with `git commit -F`. Never a heredoc.

---

## Task 2: `net_local_port` and `local_port`

**Files:**
- Modify: `crates/nova-runtime/src/net.rs`, the 12 checklist sites, `std/net/lib.nova`
- Test: `crates/nova-runtime/src/net.rs` `#[cfg(test)]`

**Interfaces:**
- Consumes: `Sock`, `with_listener`, `register_new_socket`, `stash_i64`, `closed_fd_error` from Task 1
- Produces: `nova_rt_net_local_port(fd: i64) -> i64`; Nova `pub fn local_port(self) -> Result<Int, IoError>` on `TcpListener`

- [ ] **Step 1: Write the failing Rust unit tests**

```rust
#[test]
fn net_local_port_reports_the_kernel_assigned_port() {
    let addr = nova_str("127.0.0.1:0");
    assert_eq!(unsafe { nova_rt_net_listen(&addr) }, 0);
    let fd = taken_i64();
    let status = unsafe { nova_rt_net_local_port(fd) };
    assert_eq!(status, 0, "a bound listener must report its port");
    let port = taken_i64();
    assert!(
        port > 0 && port < 65536,
        "port must be in range, got {port}"
    );
    unsafe { nova_rt_net_close(fd) };
}

#[test]
fn net_local_port_on_a_stream_fd_is_an_error() {
    let (fd, _keepalive) = connected_stream_for_test();
    let status = unsafe { nova_rt_net_local_port(fd) };
    assert_ne!(status, 0, "a stream is the wrong kind for local_port");
    unsafe { nova_rt_net_close(fd) };
}

#[test]
fn net_local_port_on_an_absent_fd_is_an_error() {
    let status = unsafe { nova_rt_net_local_port(999_999) };
    assert_ne!(status, 0, "an unregistered fd must not report a port");
}
```

`connected_stream_for_test` stands for whatever helper the existing tests use to get a live stream fd plus a keepalive for the far end — locate it by reading a neighbouring `read`/`write` test and reuse it rather than writing a new one. **Keep the far end alive** for the test's duration; dropping it is the `dead_addr()` mistake in miniature.

- [ ] **Step 2: Run the tests to verify they fail**

```bash
cargo test -p nova-runtime net_local_port
```

Expected: FAIL — `nova_rt_net_local_port` not found.

- [ ] **Step 3: Write the intrinsic**

```rust
/// Report the port `fd`'s listener is bound to.
///
/// Non-suspending. The port travels in `Slot::Buffer` rather than in the
/// return value because the status word is already spoken for: it *is* the
/// error kind, so a port returned there would make 0 ambiguous and every
/// non-zero port indistinguishable from a failure.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_net_local_port(fd: i64) -> i64 {
    match with_listener(fd, |l| l.local_addr()) {
        Some(Ok(addr)) => {
            stash_i64(i64::from(addr.port()));
            OK
        }
        Some(Err(e)) => fail(&e),
        None => closed_fd_error(),
    }
}
```

`i64::from(addr.port())` rather than a cast — `port()` is a `u16` and the conversion is infallible, so no `as` is needed and clippy stays quiet.

- [ ] **Step 4: Run the tests to verify they pass**

```bash
cargo test -p nova-runtime net_local_port
```

Expected: PASS, 3 tests.

- [ ] **Step 5: Add the 12 checklist sites**

Typeck signature: one `Int` parameter returning `Int`.

- [ ] **Step 6: Add the Nova method**

Inside the existing `impl TcpListener` block from Task 1:

```nova
    // The port this listener is bound to, which is how a caller learns the
    // kernel's choice after binding port 0.
    //
    // **A plain `fn`, not an `async fn`: this cannot suspend** -- the same
    // rule `bind` states, and the same one `close` above deliberately
    // excepts.
    pub fn local_port(self) -> Result<Int, IoError> {
        let status = net_local_port(self.fd)
        if status == 0 {
            return Ok(decode_count(fs_take_bytes()))
        }
        Err(IoError { kind: io_error_kind_of(status), message: fs_last_error_message() })
    }
```

- [ ] **Step 7: Build, run the whole suite, clippy, fmt**

Expected total: **1063 passed / 0 failed / 8 ignored** across 44 targets. Sum every line; never pipe through `head`/`tail`.

- [ ] **Step 8: Commit**

---

## Task 3: `net_accept`, `poll_accept`, and `accept`

The only suspending intrinsic, so the only one with a poll function.

**Files:**
- Modify: `crates/nova-runtime/src/net.rs`, the 12 checklist sites, `std/net/lib.nova`
- Test: `crates/nova-runtime/src/net.rs` `#[cfg(test)]`

**Interfaces:**
- Consumes: everything from Tasks 1 and 2
- Produces: `nova_rt_net_accept(fd: i64) -> *mut u8` (a future) and its `poll_accept`; Nova `pub async fn accept(self) -> Result<TcpStream, IoError>`

- [ ] **Step 1: Read the worked example first**

Read `poll_read_timeout` at `crates/nova-runtime/src/net.rs:942-958` and the `net_read` future constructor. **Level-triggered and tag-free is mandatory, not stylistic**: the poll ABI is frozen, `task_ctx` is always null, and no wake path records *why* a task woke, so every poll must re-derive its own state from the socket rather than from a stored tag.

- [ ] **Step 2: Write the failing Rust unit test**

```rust
#[test]
fn accept_parks_until_a_client_connects_then_yields_a_stream_fd() {
    let addr = nova_str("127.0.0.1:0");
    assert_eq!(unsafe { nova_rt_net_listen(&addr) }, 0);
    let listener_fd = taken_i64();
    assert_eq!(unsafe { nova_rt_net_local_port(listener_fd) }, 0);
    let port = taken_i64();

    // First poll must park: nothing has connected yet.
    let fut = unsafe { nova_rt_net_accept(listener_fd) };
    assert_eq!(poll_once_for_test(fut), POLL_PENDING, "must park with no client");

    // Connect from the far side, then poll again.
    let client = std::net::TcpStream::connect(("127.0.0.1", port as u16))
        .expect("loopback connect must succeed");
    let mut ready = POLL_PENDING;
    for _ in 0..100 {
        ready = poll_once_for_test(fut);
        if ready == POLL_READY {
            break;
        }
        crate::poll::wait_for_test();
    }
    assert_eq!(ready, POLL_READY, "must settle once a client has connected");
    let accepted_fd = taken_i64();
    assert!(accepted_fd > 0 && accepted_fd != listener_fd);

    drop(client);
    unsafe { nova_rt_net_close(accepted_fd) };
    unsafe { nova_rt_net_close(listener_fd) };
}
```

`poll_once_for_test` and `wait_for_test` stand for whatever the existing suspending-intrinsic tests use to drive one poll and to advance the poller — **locate them by reading a `poll_read`/`poll_connect` test and reuse those**. Do not invent new harness functions if equivalents exist. `port as u16` is inside `#[cfg(test)]`, where a cast is fine; the production side uses `i64::from` instead.

- [ ] **Step 3: Run it to verify it fails**

```bash
cargo test -p nova-runtime accept_parks_until
```

Expected: FAIL — `nova_rt_net_accept` not found.

- [ ] **Step 4: Write the poll function and the future constructor**

```rust
/// One non-blocking `accept` attempt, parking on read-readiness if none is
/// pending.
///
/// Level-triggered and tag-free: it re-attempts the accept on every poll and
/// derives everything from the listener, because the frozen poll ABI carries
/// no waker and records no reason for a wake.
///
/// Accept-readiness is read-readiness on both `select` and `WSAPoll`, so this
/// stages `Interest::Read` and the poller needs no new variant.
unsafe extern "C-unwind" fn poll_accept(state: *mut u8, _task_ctx: *mut u8) -> i64 {
    let fd = state_fd(state);
    let outcome = with_listener(fd, |l| l.accept());
    match outcome {
        Some(Ok((stream, _peer))) => {
            if let Err(e) = stream.set_nonblocking(true) {
                settle(state, fail(&e));
                return POLL_READY;
            }
            let accepted = register_new_socket(Sock::Stream(stream));
            stash_i64(accepted);
            settle(state, OK);
            POLL_READY
        }
        Some(Err(e)) if e.kind() == std::io::ErrorKind::WouldBlock => {
            match with_listener(fd, |l| raw_socket_of_listener(l)) {
                Some(socket) => {
                    crate::task::stage_io_park(socket, Interest::Read, None);
                    POLL_PENDING
                }
                None => {
                    settle(state, closed_fd_error());
                    POLL_READY
                }
            }
        }
        Some(Err(e)) => {
            settle(state, fail(&e));
            POLL_READY
        }
        None => {
            settle(state, closed_fd_error());
            POLL_READY
        }
    }
}
```

`state_fd`, `settle` and `raw_socket_of` stand for the existing helpers of that shape — read `poll_read`'s body and use exactly what it uses, adding a `raw_socket_of_listener` sibling if `raw_socket_of` is typed to `TcpStream`. **The accepted stream must be set non-blocking**; it does not inherit that from the listener on every platform.

Write the future constructor `nova_rt_net_accept(fd: i64) -> *mut u8` by copying `net_read`'s constructor and substituting `poll_accept`. It stores only the listener fd in its state.

- [ ] **Step 5: Run the test to verify it passes**

```bash
cargo test -p nova-runtime accept_parks_until
```

Expected: PASS.

- [ ] **Step 6: Add the 12 checklist sites**

Typeck signature: one `Int` parameter returning `Future<Int>`.

- [ ] **Step 7: Add the Nova method**

```nova
    // Waits for the next incoming connection, suspending the calling task
    // rather than blocking the executor.
    //
    // The returned `TcpStream` is indistinguishable from a connected one, so
    // every `Read`, `Write`, `read_timeout` and `close` above works on it
    // unchanged.
    //
    // **One socket wait per task.** Staging two I/O waits in a single poll
    // aborts the process, so a task that is parked in `accept` cannot also be
    // reading a connection: a server needs at least two tasks, and there is
    // no `select` to avoid that.
    pub async fn accept(self) -> Result<TcpStream, IoError> {
        let status = net_accept(self.fd).await
        if status == 0 {
            return Ok(TcpStream { fd: decode_count(fs_take_bytes()) })
        }
        Err(IoError { kind: io_error_kind_of(status), message: fs_last_error_message() })
    }
```

- [ ] **Step 8: Run the three mutations and report which tests fail BY NAME**

Mutate to a plausible wrong predicate, never to a constant. **Verify each mutant's behaviour rather than the shipped code's** — a mutation that leaves a loop unsatisfiable *hangs* rather than fails, so watch for a hang and treat it as a result, not a stall. Revert each mutation before the next.

1. **Invert the `WouldBlock` branch** so it settles instead of parking: change `e.kind() == std::io::ErrorKind::WouldBlock` to `e.kind() != std::io::ErrorKind::WouldBlock`. Expect the accept test to fail.
2. **Drop `set_nonblocking(true)` in `nova_rt_net_listen`.** Expect a hang or a failure; say which, and say whether any test catches it. If none does, **record that plainly** rather than claiming coverage.
3. **Return the wrong fd from `nova_rt_net_listen`**: `stash_i64(fd + 1)`. Expect `net_local_port` and accept tests to fail.

**Do not claim a test is the only one catching a mutation without counting.** Four such claims were measured false across five increments on this project. Run the whole suite for each mutation and count.

- [ ] **Step 9: Build, run the whole suite, clippy, fmt**

Expected: **1064 passed / 0 failed / 8 ignored** across 44 targets.

- [ ] **Step 10: Commit**

---

## Task 4: The end-to-end fixture

**Files:**
- Create: `tests/runtime/net_listener_accept.nova`, `tests/runtime/net_listener_accept.stdout`
- Modify: `crates/nova-cli/tests/run_tests.rs`

**Interfaces:**
- Consumes: `bind`, `local_port`, `accept`, `close` from Tasks 1-3

- [ ] **Step 1: Write the fixture**

```nova
// WHAT THIS PINS: a real connection, end to end, through the listener added
// this increment -- `bind` on port 0, `local_port` to learn the kernel's
// choice, `accept` parking until a client arrives, then one round trip.
//
// PORT 0 IS REQUIRED, NOT PREFERRED. A fixed port collides when test
// binaries run concurrently, which is exactly the shape of the unfixed
// `dead_addr()` flake in the runtime's own tests: it binds `127.0.0.1:0` and
// then DROPS the listener, so a concurrent bind can steal the port. This
// fixture holds its listener for the listener's whole life and so cannot
// reproduce that.
//
// NO CHANNEL, deliberately. `recv` on an empty channel spins unconditionally,
// so a consumer waiting on a socket-parked producer could never be woken.
// None is needed: `bind` is synchronous, so the port is known before anything
// parks and travels to the client task as a plain argument.
//
// NO ASSERTION ON ELAPSED TIME. Ordering is what proves parking here, not
// duration.
async fn client(port: Int) {
    let s = connect("127.0.0.1:${port}")
    match s {
        Ok(stream) => {
            let _ = stream.write(bytes_from_ints([104, 105])).await
            let echoed = stream.read(16).await
            match echoed {
                Ok(b) => print("client got ${b.len()} bytes")
                Err(e) => print("client read error ${e.message}")
            }
            let _ = stream.close().await
        }
        Err(e) => print("client connect error ${e.message}")
    }
}

fn main() {
    let listener = bind("127.0.0.1:0")
    match listener {
        Ok(l) => {
            let port = l.local_port()
            match port {
                Ok(p) => {
                    print("bound")
                    let h = spawn(client(p))
                    let accepted = l.accept().await
                    match accepted {
                        Ok(conn) => {
                            let got = conn.read(16).await
                            match got {
                                Ok(b) => {
                                    print("server got ${b.len()} bytes")
                                    let _ = conn.write(b).await
                                }
                                Err(e) => print("server read error ${e.message}")
                            }
                            let _ = conn.close().await
                        }
                        Err(e) => print("accept error ${e.message}")
                    }
                    h.join().await
                    let _ = l.close().await
                    print("done")
                }
                Err(e) => print("local_port error ${e.message}")
            }
        }
        Err(e) => print("bind error ${e.message}")
    }
}
```

**This fixture is a sketch of the required shape, not guaranteed-compiling Nova.** `main` cannot be `async` in this project unless existing fixtures show otherwise — read `tests/runtime/net_interleave.nova` and copy its exact entry-point and `spawn`/`join` idiom, including how it reaches `block_on`. Match that file rather than this sketch wherever the two disagree, and say in your report which parts you changed and why.

- [ ] **Step 2: Register the fixture — NOT AUTOMATIC**

Add an explicit `#[test]` in `crates/nova-cli/tests/run_tests.rs`, copying the shape of a neighbouring `*_run` test. **A fixture registered nowhere runs zero tests and reports nothing.** An earlier plan on this project omitted this and three fixtures would have silently never executed.

- [ ] **Step 3: Capture the golden, then verify it is right rather than merely produced**

Run the fixture, write its output to `tests/runtime/net_listener_accept.stdout`, then **read the golden and confirm each line is what the program should print** — not just what it did print. A golden captured from a wrong implementation is a wrong golden that passes forever.

- [ ] **Step 4: Run the fixture's test, then the whole suite**

```bash
cargo test -p nova-cli --test run_tests net_listener_accept
```

Then the full suite. Expected: **1065 passed / 0 failed / 8 ignored** across 44 targets.

- [ ] **Step 5: Run it 5 times in isolation to check for flakiness**

```bash
cargo test -p nova-cli --test run_tests net_listener_accept
```

Five consecutive passes. This fixture binds a real port, so flakiness here is the thing most likely to annoy every future increment. If it flakes, diagnose the mechanism before proceeding — do not retry and move on.

- [ ] **Step 6: Byte-scan both new files, then commit**

---

## Task 5: Records

**Files:**
- Modify: `nova-spec/20-STDLIB.md` (§16, and §1's index line), `docs/adr/0013-io-poller.md`, `CHANGELOG.md`

- [ ] **Step 1: Amend `20-STDLIB.md` §16**

House style: `**AMENDED <date> (branch \`<branch>\`):**`. The sentence at `:1509-1512` — "`bind`/`accept`/`TcpListener` (a server side), UDP, and Unix sockets are all named in §1's module-index line for `std/net`, but none of the three is built by this section; each remains a future increment's to add" — is now **two-thirds false**. Say so in place. The amendment must also carry:

- The four new signatures, verbatim.
- That §1's index line (`20-STDLIB.md:16`, `std/net          TCP/UDP/Unix sockets`) names UDP and Unix sockets but **not** the server side, which is only implied by "TCP" — so the sentence being amended overstated what §1 says.
- The plain-`fn` rule for `bind`/`local_port` **and** its one deliberate exception at `close`, with the reason. A principle with a silent exception is worse than either.
- Both error collapses: `AddrInUse` is not among `IoErrorKind`'s eight variants so an in-use port reports `Other`, and wrong-kind access reports `Other` for the same reason.
- That concurrency is **not** proven by this increment: no fixture parks two sockets at once, and there is no `select`.

- [ ] **Step 2: Amend `docs/adr/0013-io-poller.md`**

A dated amendment, **not a new ADR** — what is being accepted is a property of the poller's own design. Record that a listener makes the `FD_SETSIZE` skip path reachable for the first time: on Unix an fd at or above the ceiling is skipped and never watched again, and because only `read_timeout` stages a deadline that task is never woken and never errors. Quote the poller's own comment — it already calls that path and the non-`EINTR` error path "still reasoned, not measured" — and state that this increment does not change it.

- [ ] **Step 3: Add the CHANGELOG entry**

`[Unreleased]`, `### Added`. Lines **at most 78 columns**. Name the three intrinsics, the four Nova functions, and what is not closed.

- [ ] **Step 4: Confirm no new ADR is needed and nothing claims Phase 2 is done**

This finishes position 9 in order, so ADR 0014's out-of-order index does not apply. Grep the branch's added lines for any claim that Phase 2 is complete or that position 10 is now unblocked-and-started — positions 10 and 12, `examples/05-json-api` and `docs/benchmarks/` are all still absent, and the tag stays `v0.2.0-alpha.1`.

- [ ] **Step 5: Byte-scan every changed file, run the whole suite, commit**

Records-only, so expected: **1065 passed / 0 failed / 8 ignored**, unchanged from Task 4.

---

## Self-review notes

Run against the spec after writing, and recorded here rather than dropped:

- **Spec coverage.** §3's three intrinsics → Tasks 1-3. §4's enum table → Task 1 Step 3. §5's four signatures → Tasks 1, 2, 3. §6's two collapses → Task 1 Step 3 and Task 5 Step 1. §7's four inherited constraints → recorded at Task 3 Step 7's doc comment and Task 5 Step 2. §8's fixture, unit tests and mutations → Tasks 1-4. §9's records → Task 5. **No spec section is unclaimed.**
- **One thing the spec asks for that no task can close**: §11's "compile the new signatures before later tasks depend on them". Task 1 Step 8 is the earliest build, so a signature error surfaces there — but the *Nova* surface is only exercised end to end in Task 4. That is a real gap and it is deliberate: the alternative is a throwaway fixture in Task 1 that Task 4 replaces. Task 1's implementer should still run one `nova run` against a two-line scratch file outside `tests/` to prove `bind` compiles and links before Task 2 builds on it.
- **Test-count arithmetic.** 1057 baseline, +3 (Task 1), +3 (Task 2), +1 (Task 3), +1 (Task 4) = **1065**. Each task states its own expected total so a mismatch is caught where it happens rather than at the end.
- **Helper names are marked as stand-ins.** `nova_str`, `taken_i64`, `poll_once_for_test`, `wait_for_test`, `state_fd`, `settle`, `raw_socket_of`, `connected_stream_for_test` are named as the shapes to locate and reuse, not invented APIs. Every step that uses one says to read a neighbouring test or poll function and match it. This is the one place the plan cannot be literal without reading ~400 more lines of `net.rs`, and inventing plausible-but-wrong helper names would be worse than saying so.
