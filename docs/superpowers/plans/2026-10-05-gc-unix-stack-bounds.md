# Garbage Collection on Linux and macOS Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make Nova's collector run on glibc Linux and macOS, not only on Windows, and prove a long-running server's live set stays bounded on all three CI operating systems.

**Architecture:**
- **Stack bounds.** `gc.rs`'s `stack_base()` asks the calling thread for its stack top: `pthread_getattr_np` plus `pthread_attr_getstack` on glibc Linux, `pthread_get_stackaddr_np` on macOS. Windows keeps `GetCurrentThreadStackLimits`. Any other platform still returns `None`, now announced once per thread under `NOVA_GC_DEBUG`.
- **Register spill.** On GCC and Clang, `gc_stack.c` adds `__builtin_unwind_init()` and a scan from a deeper frame, because glibc's and Apple's `setjmp` scramble some saved registers. MSVC's path is unchanged.
- **Tests.** The Windows-only GC tests run everywhere. Two new tests push 3,000 connections through `examples/03-http-server`. CI's advisory step gains `--no-fail-fast`, and clippy gains a macOS leg.

**Tech Stack:** Rust (`nova-runtime`, `libc` 0.2.189, the `cc` build of `gc_stack.c`), C (GCC, Clang and MSVC), libtest end-to-end tests in `crates/nova-cli/tests/run_tests.rs`, GitHub Actions, and Docker (`rust:1-slim`) for local Linux runs.

**Spec:** `docs/superpowers/specs/2026-10-05-gc-unix-stack-bounds-design.md` (commits `de0ce87` and `57bd423`), approved by the user on 2026-10-05. Read it before Task 1. It is the authority this plan argues from.

## Global Constraints

- **Which platforms get stack bounds** (spec §3.2):
  - glibc Linux, `#[cfg(all(target_os = "linux", target_env = "gnu"))]`;
  - macOS, `#[cfg(target_os = "macos")]`;
  - Windows, unchanged.

  Every other platform returns `None` from `stack_base()`.
- **No new dependency.** `libc` is already a `[target.'cfg(unix)'.dependencies]` dependency of `nova-runtime`. `Cargo.lock` must not change: check it with `git diff --exit-code Cargo.lock`.
- **`gc_stack.c`'s MSVC (`#else`) branch** is today's function, byte for byte (spec §3.3).
- **The no-bounds line** is exactly `nova-gc: no stack bounds on this platform; collection is disabled and every allocation leaks`. It is printed once per thread, and only under `NOVA_GC_DEBUG` (spec §3.4).
- **The eight GC scan tests stay `#[ignore]`d** (ADR 0010): `gc.rs`'s `mod registry` (6) and `task.rs`'s `mod root_registration` (2). Only their `#[cfg(windows)]` gates go (spec §3.5, §5.2).
- **The leak test** (spec §5.3):
  - 3,000 requests, each on a new connection that the client closes;
  - at least 10 collections;
  - every collection under 1,000 live objects;
  - run under `nova run` only.

  This plan adds an eight-client variant with a bound of 2,000 (Review Focus 1).
- **The register spill has no discriminating test** (spec §5.4). No task may claim otherwise.
- **Linux runs locally in Docker** (`rust:1-slim`, Rust 1.99.0, user-approved 2026-10-05). macOS runs only in CI (spec §1, §5.4).
- **Toolchain and CI.** Edition 2021, MSRV 1.78. CI runs `cargo test --locked --workspace --all-features --no-fail-fast` and `cargo clippy --locked --all-targets --all-features -- -D warnings`. Both must pass on every leg.
- **Records** (spec §6):
  - ADR bodies stay unchanged; ADRs and specs get dated notes;
  - source comments are rewritten in place;
  - the dated plans and specs under `docs/superpowers/` stay as they are, except for one note on this branch's own spec (Task 4).
- **Dated notes say `2026-10-05`.** If you execute on a later day, change that date in every note you write, and leave the dates that refer to past events as they are.

## Review Focus

The five conditions most likely to bite a user, most likely first, each with the test that pins it:

1. **A server under concurrent connections** keeps more tasks alive at once. Its live set must sit higher but stay flat, not grow with requests served → `http_server_example_keeps_a_bounded_live_set_under_concurrent_clients` (Task 1). The spec left this uncovered (§5.7, item 3). The bound comes from a Linux measurement taken while writing this plan, on the spike build `9aad29f`: 8 clients gave 61–544 live objects, flat across 22 collections. 1, 4 and 16 clients gave 39–104, 88–294 and 77–1,052.
2. **A collection on a thread other than the main one** must scan that thread's own stack → `stack_base_is_the_calling_threads_own` (Task 2). `nova run` and a built executable run the program on the main thread, so `gc_reclaims_garbage` and the leak tests cover the main thread's path.
3. **A pointer held only in a callee-saved register when the collector runs** must keep its object alive. **No test can pin this**: no test can force a pointer to live only in such a register. It rests on §3.3's reasoning and on the 17 `NOVA_GC_STRESS` tests, which collect on every allocation, passing on Linux (Task 2) and on CI's macOS. Task 2's report must say so.
4. **glibc's main thread without `/proc`** (some containers and chroots) finds no bounds and leaks. **No test can pin this**: every runner, the Docker container included, has `/proc`. The no-bounds line names the condition. Task 2's report must say so.
5. **macOS's conservative over-retention** must stay bounded, not grow → both leak tests, on CI's `macos-latest` (Task 1's tests, checked in the PR's CI).

## File Structure

| File | Task | Responsibility |
|---|---|---|
| `crates/nova-cli/tests/run_tests.rs` | 1, 2, 4 | The live-count parser, the 03 harness's `spawn_with_env`, the two leak tests, `gc_reclaims_garbage` lifted off Windows, three stress-test comments |
| `crates/nova-runtime/src/gc.rs` | 2, 3 | `stack_base()` per platform; the no-bounds line; two stack tests; `collect_for_test` and `mod registry` lifted off Windows; comments |
| `crates/nova-runtime/src/gc_stack.c` | 2 | The GCC/Clang register spill; the MSVC branch unchanged |
| `crates/nova-runtime/build.rs` | 2 | Its doc comment, for the new spill |
| `docs/adr/0024-gc-stack-bounds-on-unix.md` (new) | 2 | The decision record |
| `crates/nova-runtime/src/task.rs`, `crates/nova-runtime/src/fs.rs` | 3 | `mod root_registration` lifted off Windows; comments |
| `.github/workflows/ci.yml` | 3 | The advisory step's `--no-fail-fast`, the macOS clippy leg, both comments |
| Records (Task 4 lists them) | 4 | Dated notes, source comments, the CHANGELOG, the sweep |

## Conventions for every task

- **Working directory.** `D:\Projects\nona\nova`, which is Git Bash `/d/Projects/nona/nova`. The Bash tool resets its directory after each call, so write `cd /d/Projects/nona/nova && …` in one command.
- **Line endings.** The working tree is CRLF (`core.autocrlf=true`). The Edit tool is fine. A script that matches multi-line text must convert its anchors to the file's own newline; the scripts below do.
- **Write scripts and commit messages with the Write tool, never a Bash heredoc.** The Bash tool here turns `\\` into `\`, which once put raw carriage returns into a Rust string while this plan was being written. Put them in `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/` and run them with `python -X utf8 <path>`. Rename each edit script to `<name>.applied` once it has run.
- **Stale runtime library.** `nova build` in tests links `target/debug/nova_runtime.lib`. Run `cargo build -p nova-runtime` before any `nova-cli` test run that follows a runtime change.
- **Chain a commit and what follows it with `&&`, never `;`.** A failed format check must stop the commit and every step after it.
- **Long output** goes to a file under `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/`. Read its tail.
- **Port 3000 must be free for the Windows runs of the 03 tests.** That means every `http_server_example_*` test, and so every full Windows suite. Check before each such run with `netstat -ano | grep -E "[:.]3000 .*LISTENING"`; it prints nothing when the port is free. **The user's own servers sometimes hold it.** On 2026-10-05 it was QuoteFlow's `next start --hostname 127.0.0.1` (node). Never stop such a process: stop and ask the user. The Linux container has its own port 3000 and needs no check.
- **TIME_WAIT on Windows.** Each leak test leaves up to 3,000 client ports in TIME_WAIT, out of Windows' 16,384, for minutes. Do not run the two leak tests more than twice within five minutes on Windows. A `connect` failing with `os error 10048` means wait, not a bug.
- **Linux runs** use the harness Task 1 creates: `bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh <cargo arguments>`.
  - It exports the checkout's tracked files, uncommitted edits included, as an LF tarball (`git stash create`, then `git archive`). It then runs `cargo build -p nova-runtime` followed by `cargo <arguments>` in a throwaway `rust:1-slim` container.
  - **Untracked files are not exported:** commit or `git add` a new file first.
  - Docker Desktop must be running: `docker version` shows a `Server:` section. Start it with PowerShell `Start-Process "C:\Program Files\Docker\Docker\Docker Desktop.exe"`.
- **Mutants run only on committed work.** Each mutant script refuses to start if its target file has uncommitted changes. Each is undone with `git checkout -- <file>`, which then restores the committed version and nothing else. Never commit a mutant.
- **Counting a full run.** Every full-suite step below is followed by this, with `<FILE>` replaced:
  ```bash
  sed -E 's/\x1b\[[0-9;]*m//g' <FILE> | grep -E "^test result:" | awk '{p+=$4; f+=$6; i+=$8; n++} END {print n" result lines: "p" passed, "f" failed, "i" ignored"}'
  ```

---

### Task 1: The 03 server leak tests, and the Linux harness

**Files:**
- Create: `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh` and `in_container.sh`. These are outside the repository, not committed.
- Modify: `crates/nova-cli/tests/run_tests.rs`:
  - a new parser before `gc_reclaims_garbage` (`:1625`);
  - `gc_reclaims_garbage`'s loop (`:1663–1671`);
  - `Http03Server::spawn` (`:10242`);
  - two helpers and two tests after `http_server_example_fails_fast_when_the_port_is_taken` (`:10764–10784`).

**Interfaces:**
- Consumes: the 03 harness as it is: `lock_port_3000()`, `assert_port_3000_free()`, `Http03Server::{wait_until_accepting, pid, wait_for_exit, streams, kill_and_panic}`, `Http03Conn::{open, send, response}`, `http03_response`, `send_termination_request`, `describe_status`.
- Produces, for Task 2 and the final report:
  - `fn gc_live_object_counts(stderr: &str) -> Vec<u64>`;
  - `Http03Server::spawn_with_env(env: &[(&str, &str)]) -> Http03Server`;
  - `fn http03_stop_for_live_counts(server: &mut Http03Server) -> Vec<u64>`;
  - `fn assert_live_set_bounded(live: &[u64], requests: usize, bound: u64)`;
  - the tests `http_server_example_keeps_a_bounded_live_set` and `http_server_example_keeps_a_bounded_live_set_under_concurrent_clients`;
  - the Linux harness;
  - the **Linux baseline** counts, written to the ledger.

- [ ] **Step 1: Write the Linux harness**

Write this with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh`:

```bash
#!/bin/bash
# Host side (Git Bash): export the checkout's tracked files, uncommitted edits
# included, as an LF tarball, and run `cargo "$@"` on it in a Linux container.
# Untracked files are not exported.
set -euo pipefail
L=/c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux
cd /d/Projects/nona/nova
H=$(git stash create)
git -c core.autocrlf=false archive --format=tar -o "$L/src.tar" "${H:-HEAD}"
git rev-parse --short "${H:-HEAD}" > "$L/src.rev"
MSYS_NO_PATHCONV=1 docker run --rm \
  -v "C:/Users/SAKEER~1/AppData/Local/Temp/gcm/linux:/probe:ro" \
  -v nova-cargo-registry:/usr/local/cargo/registry \
  -v nova-rustup:/usr/local/rustup \
  -v nova-linux-target:/work/target \
  rust:1-slim bash /probe/in_container.sh "$@"
```

And this to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/in_container.sh`:

```bash
#!/bin/bash
# Container side: unpack /probe/src.tar and run `cargo "$@"` on Linux. Build
# output lives in the named volume at /work/target, never in the Windows
# checkout's target/. RUSTUP_TOOLCHAIN overrides rust-toolchain.toml's
# `stable`, which rustup would otherwise try to download.
set -euo pipefail
export RUSTUP_TOOLCHAIN=1.99.0-x86_64-unknown-linux-gnu
export CARGO_TARGET_DIR=/work/target
rustup component add clippy rustfmt 2>&1 | tail -1
rm -rf /work/nova && mkdir -p /work/nova
tar -xf /probe/src.tar -C /work/nova
cd /work/nova
echo "linux: source $(cat /probe/src.rev), $(rustc --version)"
cargo build --locked -p nova-runtime 2>&1 | tail -1
cargo "$@"
```

Run: `python -X utf8 -c "import sys; [print(p, open(p,'rb').read().count(b'\r')) for p in sys.argv[1:]]" C:/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh C:/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/in_container.sh && docker version --format "{{.Server.Version}}"`
Expected:
- `0` carriage returns in each file;
- then a server version such as `28.3.2`.

If Docker answers with no server, start Docker Desktop (Conventions) and re-run.

- [ ] **Step 2: Take the Linux baseline, before any change**

Run in the background, because it builds the whole workspace:

```bash
bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked --workspace --all-features --no-fail-fast > /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/baseline.txt 2>&1
```

Then count it as in Conventions, with `<FILE>` = `/c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/baseline.txt`.

Expected: about 50 result lines, compared with PR #97's ubuntu CI run (`1211 passed, 0 failed, 1 ignored`). The container is not a CI runner: it runs as root, from a fresh clone. Read every `FAILED` test in the file and judge whether the environment explains it. Then write the baseline to the ledger, as `Linux baseline: <n> result lines, <p> passed, <f> failed, <i> ignored; failures: <names or none>`. Every later Linux expectation in this plan is relative to that line.

- [ ] **Step 3: Add the live-count parser, and use it in `gc_reclaims_garbage`**

In `crates/nova-cli/tests/run_tests.rs`, insert this immediately before the line `/// The GC reclaims garbage: a loop allocating far more than the heap threshold`:

```rust
/// The live-object count of every collection in a `NOVA_GC_DEBUG` log, in
/// order. Each collection logs
/// `nova-gc: collection C freed F bytes, L objects live (B bytes)`, and this
/// returns the `L`s.
fn gc_live_object_counts(stderr: &str) -> Vec<u64> {
    stderr
        .lines()
        .filter(|line| line.starts_with("nova-gc: collection"))
        .filter_map(|line| {
            let rest = &line[line.find(" bytes, ")? + " bytes, ".len()..];
            rest.split(' ').next()?.parse().ok()
        })
        .collect()
}

```

Then replace this, in `gc_reclaims_garbage`:

```rust
    let mut max_live = 0u64;
    for line in stderr.lines() {
        if let Some(i) = line.find(" bytes, ") {
            let rest = &line[i + " bytes, ".len()..];
            if let Some(n) = rest.split(' ').next().and_then(|s| s.parse::<u64>().ok()) {
                max_live = max_live.max(n);
            }
        }
    }
```

with:

```rust
    let max_live = gc_live_object_counts(&stderr)
        .into_iter()
        .max()
        .unwrap_or(0);
```

- [ ] **Step 4: Let the 03 harness set environment variables**

Replace this, at the start of `impl Http03Server`:

```rust
    fn spawn() -> Self {
        use std::io::Read;
        use std::process::Stdio;
        let mut cmd = std::process::Command::new(assert_cmd::cargo::cargo_bin("nova"));
        cmd.arg("run")
            .arg(repo_root().join("examples/03-http-server/src/main.nova"))
            .stdin(Stdio::null())
```

with:

```rust
    fn spawn() -> Self {
        Self::spawn_with_env(&[])
    }

    /// As [`Http03Server::spawn`], with `env` set in the child's environment.
    fn spawn_with_env(env: &[(&str, &str)]) -> Self {
        use std::io::Read;
        use std::process::Stdio;
        let mut cmd = std::process::Command::new(assert_cmd::cargo::cargo_bin("nova"));
        cmd.arg("run")
            .arg(repo_root().join("examples/03-http-server/src/main.nova"))
            .envs(env.iter().copied())
            .stdin(Stdio::null())
```

- [ ] **Step 5: Write the two leak tests and their helpers**

Insert this immediately before the line that reads:

```rust
/// A server that never calls `Server::listen` installs no handler, so a
```

The insertion:

```rust
/// Stops the example with a termination request and returns the live-object
/// count of every collection it logged. The server must have been started
/// with `NOVA_GC_DEBUG=1`, and must exit 0: a crash under collection is a
/// failure here, not a pass.
fn http03_stop_for_live_counts(server: &mut Http03Server) -> Vec<u64> {
    use std::time::Duration;
    if let Err(e) = send_termination_request(server.pid()) {
        server.kill_and_panic(&format!("could not deliver the signal: {e}"));
    }
    let status = match server.wait_for_exit(Duration::from_secs(10)) {
        Some(s) => s,
        None => server.kill_and_panic("the example did not exit within 10 s of the signal"),
    };
    let (out, err) = server.streams();
    assert_eq!(
        status.code(),
        Some(0),
        "{}; stdout={out:?} stderr={err:?}",
        describe_status(&status)
    );
    gc_live_object_counts(&err)
}

/// Fails unless at least 10 collections ran across `requests`, so the bound
/// is checked across the whole run rather than only at its start, and unless
/// every one of them kept fewer than `bound` objects live.
fn assert_live_set_bounded(live: &[u64], requests: usize, bound: u64) {
    assert!(
        live.len() >= 10,
        "expected at least 10 collections over {requests} requests, saw {}: {live:?}",
        live.len()
    );
    let max = live.iter().copied().max().unwrap_or(0);
    assert!(
        max < bound,
        "the live set must stay bounded: a collection reported {max} live objects, \
         the bound is {bound} (all {} collections: {live:?})",
        live.len()
    );
}

/// Phase 2's "server-side apps work", measured on the collector: 3,000
/// requests, each on a connection of its own, leave a bounded live set. Every
/// connection is a task the server spawns and the executor releases when it
/// finishes, so a root that outlived its task would leave at least one more
/// object live per connection. The design measured 38-109 live objects on
/// Windows and on Linux
/// (docs/superpowers/specs/2026-10-05-gc-unix-stack-bounds-design.md §2.4);
/// the bound is about 10x that, and a leak of one object per connection
/// crosses it after about 1,000 connections.
#[test]
fn http_server_example_keeps_a_bounded_live_set() {
    const REQUESTS: usize = 3_000;
    let _port = lock_port_3000();
    assert_port_3000_free();
    let mut server = Http03Server::spawn_with_env(&[("NOVA_GC_DEBUG", "1")]);
    server.wait_until_accepting();
    for i in 0..REQUESTS {
        let mut conn = Http03Conn::open();
        conn.send("GET / HTTP/1.1\r\nhost: x\r\n\r\n");
        let r = http03_response(&mut server, &mut conn, &format!("request {i}"));
        assert_eq!(r.status, 200, "request {i}");
    }
    let live = http03_stop_for_live_counts(&mut server);
    assert_live_set_bounded(&live, REQUESTS, 1_000);
}

/// The same property with eight clients at once, 375 connections each: more
/// tasks are alive at a time, so the live set sits higher, but it must still
/// stay flat. Measured on Linux while planning this test: 61-544 live objects
/// across 22 collections, flat
/// (docs/superpowers/plans/2026-10-05-gc-unix-stack-bounds.md, Review Focus
/// 1). A leak of one object per connection would add 3,000.
#[test]
fn http_server_example_keeps_a_bounded_live_set_under_concurrent_clients() {
    const CLIENTS: usize = 8;
    const PER_CLIENT: usize = 375;
    let _port = lock_port_3000();
    assert_port_3000_free();
    let mut server = Http03Server::spawn_with_env(&[("NOVA_GC_DEBUG", "1")]);
    server.wait_until_accepting();
    let clients: Vec<_> = (0..CLIENTS)
        .map(|c| {
            std::thread::spawn(move || -> Result<(), String> {
                for i in 0..PER_CLIENT {
                    let mut conn = Http03Conn::open();
                    conn.send("GET / HTTP/1.1\r\nhost: x\r\n\r\n");
                    let r = conn
                        .response()
                        .map_err(|e| format!("client {c}, request {i}: {e}"))?;
                    if r.status != 200 {
                        return Err(format!("client {c}, request {i}: status {}", r.status));
                    }
                }
                Ok(())
            })
        })
        .collect();
    for client in clients {
        let outcome = client
            .join()
            .unwrap_or_else(|_| Err("a client thread panicked".to_string()));
        if let Err(e) = outcome {
            server.kill_and_panic(&e);
        }
    }
    let live = http03_stop_for_live_counts(&mut server);
    assert_live_set_bounded(&live, CLIENTS * PER_CLIENT, 2_000);
}

```

- [ ] **Step 6: Run them on Windows, where the collector already works**

Check port 3000 first (Conventions). Then run: `cd /d/Projects/nona/nova && cargo build -p nova-runtime 2>&1 | tail -1 && cargo test --locked -p nova-cli --test run_tests -- http_server_example_keeps_a_bounded_live_set gc_reclaims_garbage 2>&1 | tail -8`
Expected: `test result: ok. 3 passed; 0 failed`. That is `gc_reclaims_garbage` and the two leak tests; the filter matches both leak tests by substring. They pass before anything else changes, because Windows already collects. Steps 9 and 10 are where they are shown to fail.

- [ ] **Step 7: Format, lint, and commit**

Write this message with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t1_commit.txt`:

```text
test: bound the 03 server's live set under the collector

Two end-to-end tests push 3,000 connections through
examples/03-http-server with NOVA_GC_DEBUG set: one client at a time,
then eight at once. They require at least 10 collections, and every
collection under 1,000 and 2,000 live objects respectively. They pass
on Windows, which already collects, and fail on Linux and macOS until
the collector runs there.

The live-count parser moves out of gc_reclaims_garbage into a helper
both use, and Http03Server::spawn_with_env lets a test set the child's
environment.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && cargo fmt --all && cargo fmt --all -- --check && cargo clippy --locked --all-targets --all-features -- -D warnings 2>&1 | tail -2 && git add crates/nova-cli/tests/run_tests.rs && git commit -q -F C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t1_commit.txt && git log --oneline -1`
Expected:
- clippy ends with `Finished` and no warning;
- one new commit, `test: bound the 03 server's live set under the collector`.

- [ ] **Step 8: Watch both leak tests fail on Linux**

Run: `bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked -p nova-cli --test run_tests -- http_server_example_keeps_a_bounded_live_set > /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/t1_red.txt 2>&1; grep -E "^test |panicked|expected at least|test result" /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/t1_red.txt | head -12`
Expected:
- both tests `FAILED`, each with `expected at least 10 collections over 3000 requests, saw 0: []`;
- `test result: FAILED. 0 passed; 2 failed`.

The server served every request and exited 0, but never collected, because `stack_base()` returns `None` on Linux. Any other failure is not this RED: investigate it before going on.

- [ ] **Step 9: Mutant (a): no collection on Windows**

Write this with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t1_mutant_a.py`:

```python
# MUTANT, never committed: Windows' stack_base() returns None, so collect()
# never marks. Undo with `git checkout -- crates/nova-runtime/src/gc.rs`.
import subprocess, sys
ROOT = "D:/Projects/nona/nova/"
F = "crates/nova-runtime/src/gc.rs"
if subprocess.run(["git", "-C", ROOT, "diff", "--quiet", "--", F]).returncode != 0:
    sys.exit("ABORT: %s has uncommitted changes; commit them first" % F)
raw = open(ROOT + F, encoding="utf-8", newline="").read()
nl = "\r\n" if "\r\n" in raw else "\n"
old = "    (high > low).then_some(high)" + nl
if raw.count(old) != 1:
    sys.exit("ABORT: anchor matched %d times" % raw.count(old))
raw = raw.replace(old, "    (high > low).then_some(high).and(None)" + nl)
open(ROOT + F, "w", encoding="utf-8", newline="").write(raw)
print("mutant (a) applied")
```

Run (port 3000 checked first): `cd /d/Projects/nona/nova && python -X utf8 C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t1_mutant_a.py && cargo test --locked -p nova-cli --test run_tests -- http_server_example_keeps_a_bounded_live_set gc_reclaims_garbage > /c/Users/SAKEER~1/AppData/Local/Temp/gcm/t1_mutant_a.txt 2>&1; git checkout -- crates/nova-runtime/src/gc.rs && git status --short && grep -E "^test |expected at least|test result" /c/Users/SAKEER~1/AppData/Local/Temp/gcm/t1_mutant_a.txt | head -10`
Expected:
- `mutant (a) applied`;
- an empty `git status`;
- `test result: FAILED. 0 passed; 3 failed`, with these messages:
  - both leak tests: `expected at least 10 collections over 3000 requests, saw 0`;
  - `gc_reclaims_garbage`: `expected at least one collection`.

Each must fail on the assertion named here. A crash, or a failure elsewhere, proves nothing about that assertion.

- [ ] **Step 10: Mutant (b): finished tasks stay rooted**

Write this with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t1_mutant_b.py`:

```python
# MUTANT, never committed: release_internal() keeps every finished task's
# GC root, so connection tasks pile up. Undo with
# `git checkout -- crates/nova-runtime/src/task.rs`.
import subprocess, sys
ROOT = "D:/Projects/nona/nova/"
F = "crates/nova-runtime/src/task.rs"
if subprocess.run(["git", "-C", ROOT, "diff", "--quiet", "--", F]).returncode != 0:
    sys.exit("ABORT: %s has uncommitted changes; commit them first" % F)
raw = open(ROOT + F, encoding="utf-8", newline="").read()
nl = "\r\n" if "\r\n" in raw else "\n"
old = nl.join([
    "    // orders its own `gc::remove_root` after releasing it.",
    "    if let Some(state) = state {",
    "        gc::remove_root(state);",
    "    }",
]) + nl
if raw.count(old) != 1:
    sys.exit("ABORT: anchor matched %d times" % raw.count(old))
new = old.replace("gc::remove_root(state);", "let _ = state;")
raw = raw.replace(old, new)
open(ROOT + F, "w", encoding="utf-8", newline="").write(raw)
print("mutant (b) applied")
```

Run (port 3000 checked first, and at least two minutes after step 9): `cd /d/Projects/nona/nova && python -X utf8 C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t1_mutant_b.py && cargo test --locked -p nova-cli --test run_tests -- http_server_example_keeps_a_bounded_live_set gc_reclaims_garbage > /c/Users/SAKEER~1/AppData/Local/Temp/gcm/t1_mutant_b.txt 2>&1; git checkout -- crates/nova-runtime/src/task.rs && git status --short && grep -E "^test |must stay bounded|test result" /c/Users/SAKEER~1/AppData/Local/Temp/gcm/t1_mutant_b.txt | head -10`
Expected:
- `mutant (b) applied`;
- an empty `git status`;
- `gc_reclaims_garbage` `ok`, because its program spawns no task;
- both leak tests `FAILED` with `the live set must stay bounded: a collection reported N live objects`, where N is at least the bound, 1000 and 2000 respectively;
- `test result: FAILED. 1 passed; 2 failed`.

This is the path the 03 server's connection tasks take. `poll_one` releases every finished task's root through `release_internal` (`task.rs:824`). The only exception is `block_on`'s root task, which keeps its root until `take_output_internal` (spec §5.4).

Rename both mutant scripts to `.applied`. The commit from step 7 is the task's last; there is nothing new to commit.

---

### Task 2: Collection on Linux and macOS

**Files:**
- Modify: `crates/nova-runtime/src/gc.rs`:
  - module doc `:15–16`, `:51–53`;
  - `collect()` `:409–416`;
  - `nova_gc_scan_range`'s doc `:567–568`;
  - `stack_base` `:604–609`;
  - two tests inserted before `:969`.
- Modify: `crates/nova-runtime/src/gc_stack.c` (whole file), and `crates/nova-runtime/build.rs` `:1–4`.
- Modify: `crates/nova-cli/tests/run_tests.rs`, `gc_reclaims_garbage`'s doc and gate (`:1628–1630`).
- Create: `docs/adr/0024-gc-stack-bounds-on-unix.md`.

**Interfaces:**
- Consumes: Task 1's tests and the Linux harness.
- Produces: `fn stack_base() -> Option<usize>` with four `cfg` arms (Windows, glibc Linux, macOS, and the fallback), plus the tests `stack_base_lies_above_the_current_frame` and `stack_base_is_the_calling_threads_own`. `gc_reclaims_garbage` runs on every OS. Task 3 relies on collection running on Linux.

- [ ] **Step 1: Write the two stack tests, and lift `gc_reclaims_garbage` off Windows**

In `crates/nova-runtime/src/gc.rs`, insert this immediately before the line that reads:

```rust
    /// Tests exercising the real, stack-scanning `collect()` (every test
```

The insertion:

```rust
    /// On every platform that implements stack bounds, the top lies above
    /// this frame, and within a plausible stack size of it.
    #[cfg(any(
        windows,
        all(target_os = "linux", target_env = "gnu"),
        target_os = "macos"
    ))]
    #[test]
    fn stack_base_lies_above_the_current_frame() {
        let local = 0u8;
        let here = std::ptr::addr_of!(local) as usize;
        let base = stack_base().expect("this platform implements stack bounds");
        assert!(
            base > here,
            "top {base:#x} must lie above this frame {here:#x}"
        );
        assert!(
            base - here < 1 << 30,
            "top {base:#x} is implausibly far above {here:#x}"
        );
    }

    /// The top is the calling thread's own: on a thread spawned with a
    /// 256 KiB stack, it lies less than 1 MiB above a local in that thread.
    /// A version that returned another thread's top would fail, because
    /// thread stacks are separate mappings far more than 1 MiB apart.
    #[cfg(any(
        windows,
        all(target_os = "linux", target_env = "gnu"),
        target_os = "macos"
    ))]
    #[test]
    fn stack_base_is_the_calling_threads_own() {
        const STACK: usize = 256 * 1024;
        let (base, here) = std::thread::Builder::new()
            .stack_size(STACK)
            .spawn(|| {
                let local = 0u8;
                (stack_base(), std::ptr::addr_of!(local) as usize)
            })
            .expect("spawn a thread")
            .join()
            .expect("join it");
        let base = base.expect("this platform implements stack bounds");
        assert!(base > here, "top {base:#x} must lie above {here:#x}");
        assert!(
            base - here < 4 * STACK,
            "top {base:#x} is not this thread's: {here:#x}"
        );
    }

```

In `crates/nova-cli/tests/run_tests.rs`, replace:

```rust
/// Windows-only: precise stack bounds (and thus collection) are currently
/// implemented there.
#[cfg(windows)]
#[test]
fn gc_reclaims_garbage() {
```

with:

```rust
/// It runs wherever `gc::stack_base` has an implementation, which includes
/// every CI operating system (`docs/adr/0024-gc-stack-bounds-on-unix.md`).
#[test]
fn gc_reclaims_garbage() {
```

- [ ] **Step 2: Watch them fail on Linux**

Run: `bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked -p nova-runtime --lib gc::tests::stack_base > /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/t2_red_unit.txt 2>&1; grep -E "^test |panicked|implements stack bounds|test result" /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/t2_red_unit.txt | head -10`
Expected: both stack tests `FAILED`, each panicking with `this platform implements stack bounds`, and `test result: FAILED. 0 passed; 2 failed`.

Run: `bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked -p nova-cli --test run_tests -- gc_reclaims_garbage http_server_example_keeps_a_bounded_live_set > /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/t2_red_e2e.txt 2>&1; grep -E "^test |expected at least|test result" /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/t2_red_e2e.txt | head -10`
Expected:
- `gc_reclaims_garbage` `FAILED` with `expected at least one collection`;
- both leak tests `FAILED` with `saw 0`;
- `0 passed; 3 failed`.

On Windows the two stack tests already pass, since Windows implements stack bounds: `cd /d/Projects/nona/nova && cargo test --locked -p nova-runtime --lib gc::tests::stack_base 2>&1 | tail -3` shows `2 passed`. That is expected and is not the RED; the RED is on Linux.

- [ ] **Step 3: Implement `stack_base()` for glibc Linux and macOS**

In `crates/nova-runtime/src/gc.rs`, replace:

```rust
#[cfg(not(windows))]
fn stack_base() -> Option<usize> {
    // Precise stack bounds for non-Windows platforms are a follow-up; until
    // then collection is skipped there.
    None
}
```

with:

```rust
/// The calling thread's stack top on glibc Linux, from `pthread_getattr_np`:
/// its lowest address plus its size. On the main thread glibc reads
/// `/proc/self/maps` for this, and fails without `/proc`.
#[cfg(all(target_os = "linux", target_env = "gnu"))]
fn stack_base() -> Option<usize> {
    // SAFETY: `pthread_getattr_np` fills `attr` for the calling thread, and
    // `pthread_attr_getstack` reads the stack's lowest address and size from
    // it. `attr` is destroyed before returning.
    unsafe {
        let mut attr: libc::pthread_attr_t = std::mem::zeroed();
        if libc::pthread_getattr_np(libc::pthread_self(), &mut attr) != 0 {
            return None;
        }
        let mut addr: *mut libc::c_void = std::ptr::null_mut();
        let mut size: libc::size_t = 0;
        let rc = libc::pthread_attr_getstack(&attr, &mut addr, &mut size);
        libc::pthread_attr_destroy(&mut attr);
        if rc != 0 || addr.is_null() || size == 0 {
            return None;
        }
        Some(addr as usize + size)
    }
}

/// The calling thread's stack top on macOS, which `pthread_get_stackaddr_np`
/// reports directly.
#[cfg(target_os = "macos")]
fn stack_base() -> Option<usize> {
    // SAFETY: reads the calling thread's own stack top.
    let top = unsafe { libc::pthread_get_stackaddr_np(libc::pthread_self()) };
    (!top.is_null()).then_some(top as usize)
}

/// No stack bounds on this platform: `collect()` skips collection, and every
/// allocation lives until the process exits.
#[cfg(not(any(
    windows,
    all(target_os = "linux", target_env = "gnu"),
    target_os = "macos"
)))]
fn stack_base() -> Option<usize> {
    None
}
```

- [ ] **Step 4: Announce a platform without bounds, once per thread**

In `collect()`, replace:

```rust
    // Capture the stack base once; give up (leak) on unsupported platforms.
    let base = HEAP.with(|h| {
        let mut h = h.borrow_mut();
        if h.base == 0 {
            h.base = stack_base().unwrap_or(usize::MAX);
        }
        h.base
    });
```

with:

```rust
    // Capture the stack base once; give up (leak) on unsupported platforms,
    // and say so once per thread under `NOVA_GC_DEBUG`. `eprintln!` allocates
    // only through the system allocator, never this heap.
    let base = HEAP.with(|h| {
        let mut h = h.borrow_mut();
        if h.base == 0 {
            h.base = stack_base().unwrap_or_else(|| {
                if debug() {
                    eprintln!(
                        "nova-gc: no stack bounds on this platform; \
                         collection is disabled and every allocation leaks"
                    );
                }
                usize::MAX
            });
        }
        h.base
    });
```

No CI platform reaches this branch, so no test executes the line (spec §8).

- [ ] **Step 5: The register spill**

Replace the whole of `crates/nova-runtime/src/gc_stack.c`, using the Write tool, with:

```c
/* Register flush for the conservative GC.
 *
 * A heap root can be held only in a callee-saved register at the point of
 * collection. This shim writes the callee-saved registers into its own stack
 * frame, then scans from below that frame up to `stack_base` (the thread's
 * stack top), which covers the saved registers and every caller frame.
 * Caller-saved registers need no flushing: the C ABI treats them as clobbered
 * by the call into the runtime, so the compiled Nova code has already spilled
 * any live root out of them before calling the allocator.
 * `nova_gc_scan_range` is implemented in Rust (`gc.rs`).
 *
 * GCC and Clang: `setjmp` alone is not enough. glibc's x86-64 `setjmp` stores
 * the saved `rbp`, `rsp` and return address scrambled, and Apple's arm64
 * `setjmp` the frame pointer, link register and stack pointer, so a root held
 * only in one of those would not look like an address in the saved copy.
 * `__builtin_unwind_init()` makes this function save every callee-saved
 * register in its own frame instead. The scan runs in a separate non-inlined
 * function, so it starts below this frame and covers all of it. The empty
 * `asm` after the call reads `regs`, so the call cannot become a tail call
 * that pops this frame first. `setjmp` stays as a second copy: that is the
 * shape CI first ran (draft PR #98). No test can force a root into a
 * callee-saved register, so none discriminates this; see
 * `docs/adr/0024-gc-stack-bounds-on-unix.md`.
 *
 * MSVC: `setjmp` into `regs`, then a scan from `regs` up.
 */
#include <setjmp.h>

extern void nova_gc_scan_range(void *lo, void *hi);

#if defined(__GNUC__) || defined(__clang__)

static __attribute__((noinline)) void nova_gc_scan_from_below(void *stack_base) {
    void *volatile marker = 0;
    nova_gc_scan_range((void *)&marker, stack_base);
}

void nova_gc_collect_roots(void *stack_base) {
    jmp_buf regs;
    __builtin_unwind_init();
    setjmp(regs);
    nova_gc_scan_from_below(stack_base);
    __asm__ __volatile__("" : : "r"(&regs) : "memory");
}

#else

void nova_gc_collect_roots(void *stack_base) {
    jmp_buf regs;
    setjmp(regs);
    nova_gc_scan_range((void *)&regs, stack_base);
}

#endif
```

Check that the MSVC branch is today's function byte for byte. Run: `cd /d/Projects/nona/nova && git show HEAD:crates/nova-runtime/src/gc_stack.c | tr -d '\r' | sed -n '/^void nova_gc_collect_roots/,/^}/p' > /c/Users/SAKEER~1/AppData/Local/Temp/gcm/msvc_before.txt && tr -d '\r' < crates/nova-runtime/src/gc_stack.c | sed -n '/^#else/,/^#endif/p' | sed -n '/^void nova_gc_collect_roots/,/^}/p' > /c/Users/SAKEER~1/AppData/Local/Temp/gcm/msvc_after.txt && diff /c/Users/SAKEER~1/AppData/Local/Temp/gcm/msvc_before.txt /c/Users/SAKEER~1/AppData/Local/Temp/gcm/msvc_after.txt && echo MSVC-UNCHANGED`
Expected: `MSVC-UNCHANGED`.

- [ ] **Step 6: The comments this step makes stale**

In `crates/nova-runtime/src/gc.rs`, replace:

```rust
//! - **callee-saved registers**, flushed onto the stack by the `setjmp` shim in
//!   `gc_stack.c` (caller-saved registers hold no live root at a call boundary);
```

with:

```rust
//! - **callee-saved registers**, flushed onto the stack by the register-spill
//!   shim in `gc_stack.c` (caller-saved registers hold no live root at a call
//!   boundary);
```

Replace:

```rust
//! Precise stack bounds are currently only implemented on Windows; on other
//! platforms collection is skipped (allocations leak, as before — never
//! unsafe).
```

with:

```rust
//! The stack scan needs the calling thread's stack top, which `stack_base`
//! finds on Windows, glibc Linux and macOS
//! (`docs/adr/0024-gc-stack-bounds-on-unix.md`). On any other platform
//! collection is skipped: allocations leak until exit, which is never unsafe,
//! and `NOVA_GC_DEBUG` says so once per thread.
```

Replace:

```rust
/// Push every aligned machine word in `[lo, hi)` as a candidate root. Called
/// from the `setjmp` shim with the register buffer and stack range.
```

with:

```rust
/// Push every aligned machine word in `[lo, hi)` as a candidate root. Called
/// by `gc_stack.c`'s register-spill shim, with a range that starts below the
/// spilled registers and ends at the stack top.
```

In `crates/nova-runtime/build.rs`, replace:

```rust
//! Compiles the small C shim that flushes callee-saved registers onto the
//! stack (via `setjmp`) so the conservative GC's stack scan can see roots held
//! only in registers. Doing this in C avoids depending on architecture-specific
//! inline assembly; `setjmp` is portable. The resulting static archive is
```

with:

```rust
//! Compiles the small C shim that flushes callee-saved registers onto the
//! stack so the conservative GC's stack scan can see roots held only in
//! registers: `__builtin_unwind_init` plus `setjmp` on GCC and Clang, `setjmp`
//! on MSVC (`src/gc_stack.c` says why). Doing this in C avoids depending on
//! architecture-specific inline assembly. The resulting static archive is
```

- [ ] **Step 7: Watch them pass on Linux, then the whole suite there**

Run: `bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked -p nova-runtime --lib gc::tests::stack_base 2>&1 | tail -3 && bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked -p nova-cli --test run_tests -- gc_reclaims_garbage http_server_example_keeps_a_bounded_live_set 2>&1 | tail -3`
Expected: `2 passed` and then `3 passed`.

Then run in the background: `bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked --workspace --all-features --no-fail-fast > /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/t2_full.txt 2>&1`. Count it as in Conventions. Expected:
- passed = the Linux baseline + 5: the two stack tests, `gc_reclaims_garbage`, and the two leak tests;
- failed = the baseline's failed, and no new name among them;
- ignored = the baseline's.

Then check each of the 17 stress tests by name:

```bash
for t in gate_async_tasks_under_gc_stress interpolation_nary_under_gc_stress a_spawned_tasks_string_output_survives_release_at_completion a_recycled_state_address_does_not_resolve_a_never_spawned_future std_core_under_gc_stress collections_under_gc_stress strings_under_gc_stress a_vec_iterator_keeps_its_backing_storage_alive_under_gc_stress assoc_types_under_gc_stress iterator_under_gc_stress nova_test_under_gc_stress fs_read_dir_under_gc_stress bytes_api_under_gc_stress fs_bytes_roundtrip_under_gc_stress json_round_trip_under_gc_stress json_stringify_escapes_under_gc_stress json_parse_strings_under_gc_stress; do grep -qE "^test $t \.\.\. ok" /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/t2_full.txt || echo "NOT OK: $t"; done; echo STRESS-CHECKED
```

Expected: only `STRESS-CHECKED`. These 17 now collect on every allocation on Linux, so a root the spill missed would show up here as a crash or as wrong output.

- [ ] **Step 8: Windows, unchanged in behaviour**

Check port 3000 (Conventions). Run in the background: `cd /d/Projects/nona/nova && cargo build -p nova-runtime 2>&1 | tail -1 && cargo test --locked --workspace --all-features --no-fail-fast > /c/Users/SAKEER~1/AppData/Local/Temp/gcm/t2_win_full.txt 2>&1`. Count it as in Conventions.
Expected: `1222 passed, 0 failed, 8 ignored`. That is PR #97's CI count of 1218, plus the two leak tests from Task 1 and the two stack tests. If a `0xC0000005` crash appears in a `*_build_standalone` or `nova test` product, read `docs/adr/0008-attributes-and-test-isolation.md` §4 first: it is a known flake family.

- [ ] **Step 9: Lint the macOS version through a scratch crate**

No macOS host is available, and the `cc` build of `gc_stack.c` cannot cross-compile for macOS here. So the macOS function is linted on its own.

Write this with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t2_macos_check.py`:

```python
# Copies gc.rs's macOS stack_base, verbatim, into a scratch crate and prints
# the command that type-checks and lints it for an Apple target.
import os, re, sys
ROOT = "D:/Projects/nona/nova/"
OUT = "C:/Users/SAKEER~1/AppData/Local/Temp/gcm/macos_check/"
src = open(ROOT + "crates/nova-runtime/src/gc.rs", encoding="utf-8").read().replace("\r\n", "\n")
m = re.search(r'#\[cfg\(target_os = "macos"\)\]\nfn stack_base\(\) -> Option<usize> \{\n.*?\n\}\n', src, re.S)
if not m:
    sys.exit("ABORT: the macOS stack_base was not found")
os.makedirs(OUT + "src", exist_ok=True)
open(OUT + "Cargo.toml", "w").write(
    '[package]\nname = "macos_check"\nversion = "0.0.0"\nedition = "2021"\n\n'
    '[dependencies]\nlibc = "=0.2.189"\n')
open(OUT + "src/lib.rs", "w").write(
    m.group(0) + "\npub fn stack_top() -> Option<usize> {\n    stack_base()\n}\n")
print("wrote", OUT, "(%d bytes of stack_base)" % len(m.group(0)))
```

Run: `python -X utf8 C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t2_macos_check.py && cd /c/Users/SAKEER~1/AppData/Local/Temp/gcm/macos_check && cargo clippy --offline --target x86_64-apple-darwin -- -D warnings 2>&1 | tail -3`
Expected: a `wrote` line, then clippy `Finished` with no warning.

This script edits nothing in the repository, so it keeps its name. Task 5 runs it again.

**Ruling:** the spec (§5.5) names `aarch64-apple-darwin`. This step uses `x86_64-apple-darwin`, which is already installed, so nothing is downloaded. `libc` 0.2.189 declares `pthread_get_stackaddr_np` for every Apple target. CI's new macOS clippy leg (Task 3) lints the real crate on arm64. Ledger this ruling.

- [ ] **Step 10: ADR 0024**

Write this with the Write tool to `docs/adr/0024-gc-stack-bounds-on-unix.md`:

````markdown
# ADR 0024 — The collector finds its stack on glibc Linux and macOS too

## Status

Accepted (2026-10-05). Branch `gc-unix-stack-bounds`
(`docs/superpowers/specs/2026-10-05-gc-unix-stack-bounds-design.md`).

## Context

`crates/nova-runtime/src/gc.rs` is a conservative mark-and-sweep collector.
Its roots include every word between its own frame and the calling thread's
stack top. Until this decision only Windows could find that top
(`GetCurrentThreadStackLimits`). Everywhere else `stack_base()` returned `None`,
`collect()` returned before marking anything, and every allocation lived until
the process exited. A server on Linux or macOS grew with every request it
served, against Phase 2's goal that server-side apps work.

A throwaway CI spike, draft PR #98 (closed unmerged, run 37269987533), turned
collection on for Linux and macOS. The whole blocking suite passed on ubuntu
and macOS, including all 17 `NOVA_GC_STRESS` tests. Those collect on every
allocation, so a missed root would have shown up as a crash or as wrong output.

Two facts constrain any design:

- **A root held only in a callee-saved register is invisible to a stack scan**
  unless the register is written to the stack first.
- **glibc's and Apple's `setjmp` scramble some of the registers they save.**
  glibc's x86-64 `setjmp` stores the saved `rbp`, `rsp` and return address
  mangled; Apple's arm64 `setjmp`, the frame pointer, link register and stack
  pointer. A `setjmp` buffer alone is not a reliable register spill there.

## Decision

1. **Ask the calling thread for its own stack top.** `stack_base()` is still
   cached per thread, in `HEAP.base`:
   - glibc Linux: `pthread_getattr_np` on `pthread_self()`, then
     `pthread_attr_getstack`. The top is the lowest address plus the size.
   - macOS: `pthread_get_stackaddr_np`, which reports the top directly.
   - Windows: `GetCurrentThreadStackLimits`, unchanged.
   - Every other platform: `None`, as before.
2. **On GCC and Clang, spill registers with `__builtin_unwind_init()`.**
   `gc_stack.c`'s `nova_gc_collect_roots` scans from a separate non-inlined
   frame below its own. An empty `asm` that reads the `setjmp` buffer keeps the
   call from becoming a tail call. `setjmp` stays as a second copy, which is the
   shape the spike ran. MSVC's path is unchanged.
3. **A platform without stack bounds says so.** Under `NOVA_GC_DEBUG`, the first
   collection on each such thread prints `nova-gc: no stack bounds on this
   platform; collection is disabled and every allocation leaks`.

## Alternatives considered

- **Record the stack top where a program starts.** A Rust entry wrapper would
  store the address of one of its own locals, then call the Nova program. It
  needs no platform API, so collection would run everywhere. Declined:
  - every way a program starts would need the wrapper: a built executable, the
    JIT under `nova run`, and every runtime unit test on its libtest thread;
  - the base must be recorded in a caller frame, because a frame that records
    its own base can have locals placed above the marker;
  - nothing has measured it, while CI ran this decision.
- **A crate for stack bounds.** The one known here, `stacker`, exposes only the
  space remaining down to the stack's low end. The scan needs the high end.
- **Dropping `setjmp` on the GCC/Clang path.** `__builtin_unwind_init` is enough
  on its own. Declined only because it would ship a shape no CI run has tested.

## Consequences

- **Collection runs on all three CI platforms.** Measured during the design on
  `examples/03-http-server`: 3,000 single-request connections leave 38–109 live
  objects on Windows and on Linux alike, flat. Two tests keep that bounded:
  `http_server_example_keeps_a_bounded_live_set`, and a variant with eight
  concurrent clients.
- **The register spill has no discriminating test.** No test can force a
  pointer to live only in a callee-saved register, so removing
  `__builtin_unwind_init` would probably fail no test. The evidence is the
  stress suite passing with collection live on Linux and macOS.
- **Any other platform still leaks**, as does glibc's main thread where `/proc`
  is unavailable, since glibc reads `/proc/self/maps` for it. The no-bounds line
  names the condition. No CI runner reaches it.
- **ADR 0010's eight GC scan tests are no longer `#[cfg(windows)]`.** They stay
  `#[ignore]`d, and CI's advisory step now runs them on every OS. That step
  gained `--no-fail-fast`, because on ubuntu it had stopped at the first failing
  test binary, before `nova-runtime`. In the spike, on macOS, three of the four
  `is_none()` tests failed, each by over-retention, the direction ADR 0010
  describes.
- **CI's clippy gains a macOS leg.** `gc.rs` now has `#[cfg(target_os =
  "macos")]` code, which no other leg compiles.
- **ADR 0012's second reason no longer holds**: collection runs off Windows.
  Its decision stands on its first reason.

## References

- Spec: `docs/superpowers/specs/2026-10-05-gc-unix-stack-bounds-design.md`
- Spike: https://github.com/Sakeerin/nova/pull/98 (closed unmerged; branch
  `spike-ci-gc-unix`, commit `9aad29f`)
- `crates/nova-runtime/src/gc.rs`: `stack_base`, `collect`, `nova_gc_scan_range`
- `crates/nova-runtime/src/gc_stack.c`: the register spill
- `docs/adr/0002-phase1-leaking-allocator.md`,
  `docs/adr/0010-conservative-scan-root-test-gating.md`,
  `docs/adr/0012-file-descriptor-lifecycle.md`
````

- [ ] **Step 11: Format, lint on both OSes, and commit**

Run: `cd /d/Projects/nona/nova && cargo fmt --all && cargo fmt --all -- --check && cargo clippy --locked --all-targets --all-features -- -D warnings 2>&1 | tail -2 && bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh clippy --locked --all-targets --all-features -- -D warnings 2>&1 | tail -3 && git diff --exit-code Cargo.lock && echo LOCK-UNCHANGED`
Expected:
- Windows clippy `Finished` with no warning;
- Linux clippy `Finished` with no warning;
- `LOCK-UNCHANGED`.

Write this message with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t2_commit.txt`:

```text
gc: collect on glibc Linux and macOS, not only on Windows

stack_base() returned None everywhere but Windows, so collect() never
marked and every allocation leaked there. It now asks the calling
thread: pthread_getattr_np plus pthread_attr_getstack on glibc Linux,
pthread_get_stackaddr_np on macOS. Any other platform still skips
collection, and under NOVA_GC_DEBUG now says so once per thread.

On GCC and Clang the register spill adds __builtin_unwind_init and a
scan from a deeper frame, because glibc's and Apple's setjmp scramble
some of the registers they save. MSVC's path is unchanged. No test can
discriminate the spill; the 17 NOVA_GC_STRESS tests pass with
collection live on Linux.

Two stack tests, and gc_reclaims_garbage now runs on every OS. The 03
leak tests from the previous commit pass on Linux. ADR 0024 records
the decision.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && git add crates/nova-runtime/src/gc.rs crates/nova-runtime/src/gc_stack.c crates/nova-runtime/build.rs crates/nova-cli/tests/run_tests.rs docs/adr/0024-gc-stack-bounds-on-unix.md && git commit -q -F C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t2_commit.txt && git log --oneline -1`
Expected: one new commit, `gc: collect on glibc Linux and macOS, not only on Windows`.

The task's report must state Review Focus 3 and 4 plainly: no test covers the register spill or a missing `/proc`.

---

### Task 3: The GC scan tests on every OS, and CI

**Files:**
- Modify: `crates/nova-runtime/src/gc.rs`:
  - `:321–326`, `:359–368` (`collect_for_test`), `:378–381`, `:711–713`, `:917–922`;
  - the `mod registry` doc and gate, `:975–1027`;
  - `:1039–1042`, `:1113–1117`, `:1341`.
- Modify: `crates/nova-runtime/src/task.rs` `:281–283`, and `:5342–5350` plus `:5374` (`mod root_registration`'s doc and gate).
- Modify: `crates/nova-runtime/src/fs.rs` `:384–389`.
- Modify: `.github/workflows/ci.yml` `:66–72` (the advisory step) and `:84–99` (clippy).

**Interfaces:**
- Consumes: Task 2's `stack_base()`, which is what makes these tests meaningful off Windows.
- Produces: the eight `#[ignore]`d GC scan tests compiled on every OS; CI's advisory step with `--no-fail-fast`; a third clippy leg.

- [ ] **Step 1: Record what Linux runs today**

Run: `bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked -p nova-runtime --lib -- --ignored 2>&1 | grep -E "^running|test result" | tail -3`
Expected: `running 0 tests`. Off Windows the eight are compiled out today, so `--ignored` finds none.

- [ ] **Step 2: Lift the three Windows gates, and rewrite the comments that justified them**

In `crates/nova-runtime/src/gc.rs`, make these replacements.

`root_count`'s doc. Replace:

```rust
/// accidental stack root. Reading the registry directly is deterministic and
/// platform-independent, where a `collect()`-based assertion is neither.
```

with:

```rust
/// accidental stack root. Reading the registry directly is deterministic and
/// works on every platform; a `collect()`-based assertion is neither, since
/// the scan can over-retain and does not run where `stack_base` has no
/// implementation.
```

`collect_for_test`. Replace:

```rust
/// `#[cfg(windows)]` matches its only callers: `task.rs`'s
/// `root_registration` tests, themselves gated to Windows because
/// `stack_base` below only has a real implementation there (off Windows,
/// `collect()` returns before marking anything, so those assertions would
/// pass or fail for the wrong reason rather than exercising this at all).
/// Without this gate the function has no caller at all off Windows and
/// reads as dead code there under `-D warnings`.
#[cfg(test)]
#[cfg(windows)]
#[inline(always)]
```

with:

```rust
/// Its only callers are `task.rs`'s `root_registration` tests, which are
/// compiled on every platform and run under `--ignored`. Where `stack_base`
/// has no implementation, `collect()` returns before marking anything, so
/// their assertions would pass or fail for the wrong reason; no CI platform
/// is such a platform (`docs/adr/0024-gc-stack-bounds-on-unix.md`).
#[cfg(test)]
#[inline(always)]
```

`sweep_with_roots_for_test`'s doc. Replace:

```rust
/// That one runs the real cycle, so what survives depends on the conservative
/// stack scan -- which has an implementation on Windows alone, and which
```

with:

```rust
/// That one runs the real cycle, so what survives depends on the conservative
/// stack scan -- which runs only where `stack_base` has an implementation, and which
```

`the_crossing_allocation_collects_before_taking_its_slot`'s doc. Replace:

```rust
    /// takes its slot: the count it leaves is its own slot alone. Holds off
    /// Windows too, where `collect()` resets the count and returns.
```

with:

```rust
    /// takes its slot: the count it leaves is its own slot alone. Holds on a
    /// platform without stack bounds too, where `collect()` resets the count
    /// and returns.
```

The comment above `registering_the_same_address_twice_requires_removing_it_twice`. Replace:

```rust
    // -- unlike the tests in `mod registry` below -- they run on every
    // platform, including the two (Linux, macOS) where `collect()` itself is
    // a no-op (see `mod registry`'s doc comment).
```

with:

```rust
    // -- unlike the `#[ignore]`d tests in `mod registry` below -- they are
    // deterministic, and they hold even where `collect()` is a no-op for want
    // of stack bounds (see `mod registry`'s doc comment).
```

`mod registry`'s doc, first paragraph. Replace:

```rust
    /// **Gated to Windows.** `stack_base` (Windows implementation at `:419`;
    /// the `#[cfg(not(windows))]` stub returning `None` is at `:432`) only
    /// implements precise stack bounds on Windows; off it, `collect()`
    /// (`:264`, early return at `:273-275`) sets `alloc_since_gc = 0` and
    /// returns *before* even looking at `PINNED` -- no scan, no mark, no
    /// sweep, on any platform this collector doesn't yet support. Off
    /// Windows, every `is_none()` assertion in this module (checking that
    /// something was swept) would fail outright, and every `is_some()`
    /// assertion (checking that something survived) would pass vacuously --
    /// identically to what `add_root` being `{}` would produce, which is the
    /// one thing a test in this file must never do. This is derived by
    /// inspection of `stack_base`/`collect()` above, not run on Linux or
    /// macOS; it does not need to be, since the mechanism (`collect()` never
    /// reaching `PINNED`) applies uniformly to every test below regardless of
    /// which assertion it makes. Not hypothetical either way:
    /// `.github/workflows/ci.yml` runs `cargo test --workspace --all-features`
    /// on `ubuntu-latest`, `windows-latest`, and `macos-latest`, so this would
    /// land red (and green for the wrong reason) on two of three CI jobs.
```

with:

```rust
    /// **Not gated to a platform.** Until 2026-10-05 this module was
    /// `#[cfg(windows)]`, because only Windows had a `stack_base`
    /// implementation; glibc Linux and macOS have one now
    /// (`docs/adr/0024-gc-stack-bounds-on-unix.md`). Where `stack_base` has
    /// none, `collect()` sets `alloc_since_gc = 0` and returns *before* even
    /// looking at `PINNED` -- no scan, no mark, no sweep -- so every
    /// `is_none()` assertion in this module (checking that something was
    /// swept) would fail outright, and every `is_some()` assertion (checking
    /// that something survived) would pass vacuously, identically to what
    /// `add_root` being `{}` would produce, which is the one thing a test in
    /// this file must never do. No CI platform is such a platform.
```

Its second paragraph. Replace:

```rust
    /// **Unconditionally `#[ignore]`d, in debug and release alike** --
    /// independently of the Windows gate above, and no longer only under
    /// `--release` as this comment used to say. `collect()`'s conservative
```

with:

```rust
    /// **Unconditionally `#[ignore]`d, in debug and release alike** -- no
    /// longer only under `--release` as this comment used to say.
    /// `collect()`'s conservative
```

Replace:

```rust
    /// scanned range's low end (so *not* `&regs`, the register-flush buffer
    /// the `setjmp` shim seeds at the start of its own frame, which sits at
    /// that low end). Mechanism identified, not fixed. The full account --
```

with:

```rust
    /// scanned range's low end (so *not* the registers `gc_stack.c` spills at
    /// that low end). Mechanism identified, not fixed. The full account --
```

Replace:

```rust
    /// same ungated-canary pattern a Critical review finding flagged, and the
    /// same pairing principle the Windows gate above exists for. Each test's
```

with:

```rust
    /// same ungated-canary pattern a Critical review finding flagged. Each test's
```

The gate itself. Replace:

```rust
    #[cfg(windows)]
    mod registry {
```

with:

```rust
    mod registry {
```

`hide`'s doc. Replace:

```rust
        /// scanner looks: a stack slot, or a callee-saved register the
        /// `setjmp` shim flushes to one. A plain `usize` copy of the address
```

with:

```rust
        /// scanner looks: a stack slot, or a callee-saved register
        /// `gc_stack.c` spills to one. A plain `usize` copy of the address
```

The hazard comment in the first registry test. Replace:

```rust
            //    a stack slot, or a callee-saved register the setjmp shim flushes
            //    to one. A plain copy, bit-identical to the object's address, is
```

with:

```rust
            //    a stack slot, or a callee-saved register `gc_stack.c` spills
            //    to one. A plain copy, bit-identical to the object's address, is
```

The pairing comment near `:1341`. Replace:

```rust
            // would be exactly the pattern this file's Windows gate exists to
```

with:

```rust
            // would be exactly the pattern this file's pairing rule exists to
```

In `crates/nova-runtime/src/task.rs`, replace:

```rust
/// `poll_one`. Gated `#[cfg(test)]` rather than `#[cfg(windows)]`: its callers
/// are `fs.rs`'s tests, which run on every platform, so unlike
/// `gc::collect_for_test` this cannot read as dead code off Windows.
```

with:

```rust
/// `poll_one`. Gated `#[cfg(test)]` only: its callers are `fs.rs`'s tests,
/// which run on every platform.
```

Replace:

```rust
    /// Windows-only, matching `gc.rs`'s own `mod registry` precedent, and for
    /// the identical reason: both tests below call `gc::collect_for_test`,
    /// which runs the real, stack-scanning `collect()`. `stack_base()`
    /// (`gc.rs`) only has a real implementation on Windows; elsewhere it
    /// returns `None`, so `collect()` returns before marking anything and an
    /// `is_some()` assertion would pass vacuously while an `is_none()`
    /// assertion would fail outright. `.github/workflows/ci.yml` runs
    /// ubuntu, windows and macos, so leaving this ungated would land red on
    /// two of three jobs and green on the third for the wrong reason.
```

with:

```rust
    /// Compiled on every platform, like `gc.rs`'s own `mod registry`, and for
    /// the reason both were `#[cfg(windows)]` until 2026-10-05: both tests
    /// below call `gc::collect_for_test`, which runs the real, stack-scanning
    /// `collect()`, and that needs `stack_base()` (`gc.rs`). It has glibc
    /// Linux and macOS versions now (`docs/adr/0024-gc-stack-bounds-on-unix.md`).
    /// Where it has none, `collect()` returns before marking anything, so an
    /// `is_some()` assertion would pass vacuously while an `is_none()`
    /// assertion would fail outright.
```

Replace:

```rust
    #[cfg(windows)]
    mod root_registration {
```

with:

```rust
    mod root_registration {
```

**Ruling:** `task.rs:2895–2897` stays as it is. The spec's list (§6) includes it, but it is a conditional, "wherever `gc.rs`'s `stack_base` has no implementation", and stays true after this change. Ledger this ruling.

In `crates/nova-runtime/src/fs.rs`, replace:

```rust
/// `#[cfg(test)]` only, not also `#[cfg(windows)]`: every caller of this
/// function is an ordinary, cross-platform `task.rs` test, the same
/// reasoning `set_current_for_test`'s own doc comment gives for itself --
/// contrast `gc::collect_for_test`, whose *only* callers genuinely are
/// `#[cfg(windows)]`, which is why that one carries the platform gate and
/// this one must not.
```

with:

```rust
/// `#[cfg(test)]` only: every caller of this function is an ordinary,
/// cross-platform `task.rs` test, the same reasoning
/// `set_current_for_test`'s own doc comment gives for itself.
```

- [ ] **Step 3: The eight now run on Linux**

Run: `bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked -p nova-runtime --lib -- --ignored > /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/t3_ignored.txt 2>&1; grep -E "^running|^test |test result" /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/t3_ignored.txt | tail -12`
Expected: `running 8 tests`, naming the six `gc::tests::registry::*` and the two `task::tests::root_registration::*` tests. The four tests asserting a root survived should pass:
- `a_registered_root_survives_a_collection_with_no_stack_reference`;
- `a_registered_root_keeps_its_transitive_children_alive`;
- `the_registry_survives_more_than_one_collection`;
- `a_spawned_tasks_state_is_registered_as_a_gc_root`.

Any of the four asserting something was swept may fail, but only with a "survived" message. That is ADR 0010's over-retention, and these tests stay `#[ignore]`d for it. Record each result in the ledger. A failure of any other kind is a finding: investigate it before going on.

Run: `bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked -p nova-runtime --lib 2>&1 | tail -3`
Expected: `0 failed`, and 8 more `ignored` than before this task.

- [ ] **Step 4: CI**

In `.github/workflows/ci.yml`, replace:

```yaml
      # All eight are `#[cfg(windows)]`, so this filters to zero tests on the
      # ubuntu and macos legs. That is intentional rather than a matrix
      # exclusion: the day `gc::stack_base` grows a non-Windows implementation,
      # this step starts covering them everywhere with no CI change.
      - name: cargo test (ignored GC scan tests; advisory, may flake)
        continue-on-error: true
        run: cargo test --locked --workspace --all-features -- --ignored
```

with:

```yaml
      # The eight run on all three legs. They were `#[cfg(windows)]` until
      # 2026-10-05, when `gc::stack_base` gained glibc Linux and macOS versions
      # (docs/adr/0024-gc-stack-bounds-on-unix.md). `--no-fail-fast` is what
      # lets them run on ubuntu: without it cargo stops after the first test
      # binary with a failure, and `extern_ffi_run` (ignored on Linux only,
      # issue #3) fails in `run_tests`, long before `nova-runtime`'s binary.
      # The job stays green either way, so count this step's `test result:`
      # lines, not its outcome.
      - name: cargo test (ignored GC scan tests; advisory, may flake)
        continue-on-error: true
        run: cargo test --locked --workspace --all-features --no-fail-fast -- --ignored
```

Replace:

```yaml
  # Two legs, because a one-OS clippy is blind in both directions: an
  # ubuntu-only run never compiles the 33 `#[cfg(windows)]` regions across 7
  # files, and anything used only on Windows lints as dead there. Both halves
  # have bitten this project -- two of the five defects PR #2 fixed were
  # Windows-only code reading as dead on ubuntu, and three lints in
  # `net.rs`'s Unix arm reached CI on PR #8 because no local host runs Linux.
  #
  # No macos leg: the tree has no `target_os = "macos"` or `target_vendor =
  # "apple"` cfg at all, so macos would lint exactly what ubuntu does.
  clippy:
    name: Clippy (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest]
```

with:

```yaml
  # Three legs, because a one-OS clippy is blind in both directions: an
  # ubuntu-only run never compiles a `#[cfg(windows)]` region
  # (`git grep -n '#\[cfg(windows)\]' -- '*.rs'` lists them), and anything
  # used only on Windows lints as dead there. Both halves have bitten this
  # project -- two of the five defects PR #2 fixed were Windows-only code
  # reading as dead on ubuntu, and three lints in `net.rs`'s Unix arm reached
  # CI on PR #8 because no local host runs Linux.
  #
  # The macos leg exists because `gc.rs`'s `stack_base` has a
  # `#[cfg(target_os = "macos")]` version that no other leg compiles
  # (2026-10-05, docs/adr/0024-gc-stack-bounds-on-unix.md).
  clippy:
    name: Clippy (${{ matrix.os }})
    runs-on: ${{ matrix.os }}
    strategy:
      fail-fast: false
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]
```

Run: `cd /d/Projects/nona/nova && python -X utf8 -c "import yaml" 2>/dev/null && python -X utf8 -c "import yaml,sys; d=yaml.safe_load(open('.github/workflows/ci.yml')); print(d['jobs']['clippy']['strategy']['matrix']['os']); print([s.get('run') for s in d['jobs']['test']['steps'] if 'ignored' in str(s.get('name'))])" || grep -n -E "macos-latest\]|no-fail-fast -- --ignored" .github/workflows/ci.yml`
Expected: either the parsed matrix `['ubuntu-latest', 'windows-latest', 'macos-latest']` and the advisory command with `--no-fail-fast`, or, without PyYAML, the same two lines found by `grep`.

- [ ] **Step 5: Lint on both OSes, and commit**

Run: `cd /d/Projects/nona/nova && cargo fmt --all && cargo fmt --all -- --check && cargo clippy --locked --all-targets --all-features -- -D warnings 2>&1 | tail -2 && bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh clippy --locked --all-targets --all-features -- -D warnings 2>&1 | tail -3 && cargo test --locked -p nova-runtime --lib 2>&1 | tail -3`
Expected:
- both clippy runs `Finished` with no warning. On Linux, `collect_for_test` now has callers and is no longer dead code.
- Windows `nova-runtime` lib `0 failed`, with its ignored count unchanged: the eight were already compiled there.

Write this message with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t3_commit.txt`:

```text
gc: compile the GC scan tests on every OS; CI runs and lints them

The eight #[ignore]d GC scan tests in gc.rs's mod registry and
task.rs's mod root_registration, and gc::collect_for_test, were
#[cfg(windows)] because only Windows had stack bounds. They compile
everywhere now and stay ignored (ADR 0010); every comment that
justified the gate is rewritten.

CI's advisory --ignored step gains --no-fail-fast. Without it, ubuntu
stopped at run_tests, the first test binary with a failure (through
extern_ffi_run, issue #3), before nova-runtime's binary ever ran.
Clippy gains a macos-latest leg, because gc.rs now has macOS-only code.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && git add crates/nova-runtime/src/gc.rs crates/nova-runtime/src/task.rs crates/nova-runtime/src/fs.rs .github/workflows/ci.yml && git commit -q -F C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t3_commit.txt && git log --oneline -1`
Expected: one new commit, `gc: compile the GC scan tests on every OS; CI runs and lints them`.

---

### Task 4: Records: dated notes, source comments, the CHANGELOG, and the sweep

**Files:** (all modify)
- `docs/adr/0002-phase1-leaking-allocator.md`, `0009-async-execution-model.md`, `0010-conservative-scan-root-test-gating.md`, `0012-file-descriptor-lifecycle.md` (four notes), `0013-io-poller.md` (two), `0018-std-json-scope-and-build-order.md`, `0020-size-class-page-heap.md`
- `nova-spec/13-RUNTIME.md` (§3.1), `nova-spec/20-STDLIB.md` (`:766`), `docs/phase-2-plan.md` (`:35`, `:335`, `:348`)
- `crates/nova-runtime/src/crypto.rs` (`:79–82`), `std/json/lib.nova` (`:204–206`), `std/collections/lib.nova` (`:419`), `tests/runtime/recycled_task_state.nova` (`:37–41`), `crates/nova-cli/tests/run_tests.rs` (`:1231–1232`, `:1265–1267`, `:8678–8679`)
- `CHANGELOG.md` (`[Unreleased]`: one Fixed and one Changed bullet)
- `docs/superpowers/specs/2026-10-05-gc-unix-stack-bounds-design.md` (one note, after §5.6)

**Interfaces:**
- Consumes: the names and behaviour from Tasks 1–3.
- Produces: no code.

- [ ] **Step 1: Apply every note and comment with one script**

Write this script with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t4_records.py`. Run it with `python -X utf8`, then rename it to `t4_records.py.applied`. Every anchor below was checked on 2026-10-05 to match exactly once (26 anchors). The script aborts before writing anything if one does not.

```python
import sys
ROOT = "D:/Projects/nona/nova/"

def run(edits):
    state = {}
    for f, anchor, text, mode in edits:
        if f not in state:
            raw = open(ROOT + f, encoding="utf-8", newline="").read()
            state[f] = [raw, "\r\n" if "\r\n" in raw else "\n"]
        raw, nl = state[f]
        a = anchor.replace("\n", nl)
        t = text.replace("\n", nl)
        n = raw.count(a)
        if n != 1:
            sys.exit("ABORT before any write: %s: anchor matched %d times: %r" % (f, n, anchor[:70]))
        if mode == "after":
            state[f][0] = raw.replace(a, a + t)
        elif mode == "before":
            state[f][0] = raw.replace(a, t + a)
        else:
            state[f][0] = raw.replace(a, t)
    for f, (raw, nl) in state.items():
        open(ROOT + f, "w", encoding="utf-8", newline="").write(raw)
        print("wrote", f)

ADR24 = "`docs/adr/0024-gc-stack-bounds-on-unix.md`"
EDITS = []

EDITS.append(("docs/adr/0002-phase1-leaking-allocator.md",
"back to the original leak-until-exit behavior described below until their\nstack-bounds query is added.\n",
"""
**Amended 2026-10-05 (branch `gc-unix-stack-bounds`):** glibc Linux and macOS
have their stack-bounds query now, so they collect too (""" + ADR24 + """).
Any other platform still falls back to leak-until-exit. On GCC and Clang the
register spill is no longer `setjmp` alone; ADR 0024 says why.
""", "after"))

EDITS.append(("docs/adr/0009-async-execution-model.md",
"  exists (ADR 0020), so the question it raised is moot. ADR 0012's decision\n  stands.\n",
"""  **Amended 2026-10-05 (branch `gc-unix-stack-bounds`):** the platform gap
  this paragraph cites is closed too: collection runs on glibc Linux and
  macOS as well as Windows (""" + ADR24 + """). ADR 0012's decision still
  stands, on its one remaining reason.
""", "after"))

EDITS.append(("docs/adr/0010-conservative-scan-root-test-gating.md",
"marking anything, which would make every `is_some()` assertion pass vacuously\nand every `is_none()` assertion fail outright.\n",
"""
**Amended 2026-10-05 (branch `gc-unix-stack-bounds`):** none of the eight is
`#[cfg(windows)]` any more, and neither module in the table above is.
`stack_base()` has glibc Linux and macOS versions now (""" + ADR24 + """).
All eight stay `#[ignore]`d for the reason below. CI's advisory `--ignored`
step runs them on every OS, after gaining `--no-fail-fast`: on ubuntu it had
stopped at the first failing test binary, `nova-cli`'s `run_tests`, before
reaching `nova-runtime`. The first run off Windows was the 2026-10-05 spike
(draft PR #98), on macOS. The four `is_some()` tests passed, and three of the
four `is_none()` tests failed, every one by over-retention, the direction this
ADR describes. The `setjmp` shim named under Context describes the MSVC path
only now; on GCC and Clang it also calls `__builtin_unwind_init` (ADR 0024).
""", "after"))

EDITS.append(("docs/adr/0012-file-descriptor-lifecycle.md",
"   project's own CI (ubuntu, macos, windows) exists to catch before it\n   ships.\n",
"""
**Amended 2026-10-05 (branch `gc-unix-stack-bounds`): reason 2 no longer
holds.** Collection runs on glibc Linux and macOS as well as Windows now
(""" + ADR24 + """); other platforms still skip it. Close-on-collect stays
foreclosed by reason 1 alone, which is unchanged: `fd: Int` still makes it
impossible. This note does not revisit the decision, and a `File` still needs
an explicit `close`.
""", "after"))

EDITS.append(("docs/adr/0012-file-descriptor-lifecycle.md",
"  system-wide, not only this one).\n",
"""  **Amended 2026-10-05 (branch `gc-unix-stack-bounds`):** the second has
  changed (see the note under the two reasons); the first has not, so this
  still needs the collector first.
""", "after"))

EDITS.append(("docs/adr/0012-file-descriptor-lifecycle.md",
"  unboundedly on the other two is a worse thing to ship, silently, than a\n  uniform documented leak.\n",
"""  **Amended 2026-10-05 (branch `gc-unix-stack-bounds`):** the platform
  reason given here, and the cookie alternative's "collection still would not
  run off Windows" above, are gone (ADR 0024). Revisiting either alternative
  is not part of that change.
""", "after"))

EDITS.append(("docs/adr/0012-file-descriptor-lifecycle.md",
"  declines to use), `stack_base` (the Windows-only precise-bounds gap)\n  **Amended 2026-10-01 (gc-page-heap):** that call is gone; see the\n  amendment under Context and ADR 0020.\n",
"""  **Amended 2026-10-05 (branch `gc-unix-stack-bounds`):** `stack_base` is
  no longer Windows-only (ADR 0024).
""", "after"))

EDITS.append(("docs/adr/0013-io-poller.md",
"  re-argued on thread-locality grounds it does not actually turn on.\n",
"""  **Amended 2026-10-05 (branch `gc-unix-stack-bounds`):** `stack_base()` is
  no longer Windows-only (""" + ADR24 + """); as this bullet says, that never
  decided this alternative.
""", "after"))

EDITS.append(("docs/adr/0013-io-poller.md",
"  itself decide the question, unlike the first\n",
"""  **Amended 2026-10-05 (branch `gc-unix-stack-bounds`):** collection runs on
  glibc Linux and macOS too now (ADR 0024); ADR 0012 records what that changes
  for its own decision.
""", "after"))

EDITS.append(("docs/adr/0018-std-json-scope-and-build-order.md",
"no collect-and-retry on that path and with the collector a no-op off Windows.\n",
"""
**Amended 2026-10-05 (branch `gc-unix-stack-bounds`):** the collector is no
longer a no-op on glibc Linux and macOS (""" + ADR24 + """), only on other
platforms. The residual stands: heap exhaustion still aborts, with no
collect-and-retry.
""", "after"))

EDITS.append(("docs/adr/0020-size-class-page-heap.md",
"The collector stays conservative, non-moving and thread-local, and still\nskips collection off Windows.\n",
"""
**Amended 2026-10-05 (branch `gc-unix-stack-bounds`):** collection runs on
glibc Linux and macOS as well now (""" + ADR24 + """); other platforms still
skip it.
""", "after"))

EDITS.append(("nova-spec/13-RUNTIME.md",
"back to the system allocator. ADR 0020 records the decision.\n",
"""
**Amended 2026-10-05 (branch `gc-unix-stack-bounds`): which platforms collect,
and how registers are flushed.** The stack scan needs the calling thread's
stack top. `gc.rs`'s `stack_base()` finds it on three platforms:
`GetCurrentThreadStackLimits` on Windows, `pthread_getattr_np` with
`pthread_attr_getstack` on glibc Linux, and `pthread_get_stackaddr_np` on
macOS. Until this date only Windows had it, and every other platform skipped
collection, so every allocation leaked until exit. Any other platform still
does, and under `NOVA_GC_DEBUG` it prints
`nova-gc: no stack bounds on this platform; collection is disabled and every allocation leaks`
once per thread. On GCC and Clang the register flush above is no longer
`setjmp` alone, because glibc's and Apple's `setjmp` scramble some of the
registers they save: `gc_stack.c` also calls `__builtin_unwind_init()` and
scans from a deeper frame. MSVC keeps the `setjmp`-only path. ADR 0024 records
the decision.
""", "after"))

EDITS.append(("nova-spec/20-STDLIB.md",
"   collect-and-retry on that path, and off Windows the collector is a no-op, so\n   nothing is reclaimed until the process exits.\n",
"""
   **Amended 2026-10-05 (branch `gc-unix-stack-bounds`):** the collector is no
   longer a no-op on glibc Linux and macOS (""" + ADR24 + """), only on other
   platforms. The rest stands: heap exhaustion still aborts, with no
   collect-and-retry.
""", "after"))

EDITS.append(("docs/phase-2-plan.md",
"**precise GC stack bounds** for non-Windows.\n",
"""
**Amended 2026-10-05 (branch `gc-unix-stack-bounds`):** that last item is done
for glibc Linux and macOS (""" + ADR24 + """); other platforms still skip
collection.
""", "after"))

EDITS.append(("docs/phase-2-plan.md",
"  for lexer/parser, non-Windows GC stack bounds.\n",
"""  **Amended 2026-10-05 (branch `gc-unix-stack-bounds`):** the GC stack bounds
  are done for glibc Linux and macOS (ADR 0024); the rest of this item is not.
""", "after"))

EDITS.append(("docs/phase-2-plan.md",
"  finish non-Windows stack bounds so CI on Linux exercises real collection.\n",
"""  **Amended 2026-10-05 (branch `gc-unix-stack-bounds`):** done: CI on Linux
  and macOS exercises real collection (ADR 0024).
""", "after"))

EDITS.append(("crates/nova-runtime/src/crypto.rs",
"/// anything is allocated. What it guards is platform-dependent: ADR 0002 is\n/// superseded by a mark-and-sweep collector that reclaims, but precise stack\n/// bounds are implemented on Windows only, and the other platforms still\n/// fall back to leak-until-exit until their stack-bounds query lands.\n",
"""/// anything is allocated. What it guards is platform-dependent: ADR 0002 is
/// superseded by a mark-and-sweep collector that reclaims, but only where
/// `gc.rs`'s `stack_base` has an implementation
/// (`docs/adr/0024-gc-stack-bounds-on-unix.md` names the platforms); on any
/// other platform allocations still leak until exit.
""", "replace"))

EDITS.append(("std/json/lib.nova",
"//     is installed; there is no collect-and-retry on that path, and off\n//     Windows the collector is a no-op, so nothing is reclaimed until the\n//     process exits;\n",
"""//     is installed; there is no collect-and-retry on that path, and where
//     `gc.rs`'s `stack_base` has no implementation the collector is a no-op,
//     so nothing is reclaimed until the process exits;
""", "replace"))

EDITS.append(("std/collections/lib.nova",
"    // thread base *plus* the callee-saved registers the `setjmp` shim spills\n",
"    // thread base *plus* the callee-saved registers `gc_stack.c` spills\n",
"replace"))

EDITS.append(("tests/runtime/recycled_task_state.nova",
"// This needs `NOVA_GC_STRESS=1` to discriminate, and discriminates only where\n// the collector runs: `stack_base()` has an implementation on Windows and\n// returns `None` elsewhere, so on other platforms `collect()` returns before\n// marking, nothing is ever freed, no address is ever recycled, and this\n// program aborts whether or not the entry is pruned.\n",
"""// This needs `NOVA_GC_STRESS=1` to discriminate, and discriminates only where
// the collector runs: wherever `gc.rs`'s `stack_base()` has an implementation
// (`docs/adr/0024-gc-stack-bounds-on-unix.md`). Where it returns `None`,
// `collect()` returns before marking, nothing is ever freed, no address is
// ever recycled, and this program aborts whether or not the entry is pruned.
""", "replace"))

EDITS.append(("crates/nova-cli/tests/run_tests.rs",
"/// as wrong output or a crash. It discriminates only where the collector frees\n/// memory, which is Windows.\n",
"""/// as wrong output or a crash. It discriminates only where the collector frees
/// memory: wherever `gc::stack_base` has an implementation (ADR 0024).
""", "replace"))

EDITS.append(("crates/nova-cli/tests/run_tests.rs",
"/// **What it does not prove: soundness.** It discriminates only where the\n/// collector frees memory, which is Windows (`gc::stack_base` is `None`\n/// elsewhere). Even there,",
"""/// **What it does not prove: soundness.** It discriminates only where the
/// collector frees memory, which is wherever `gc::stack_base` has an
/// implementation (ADR 0024). Even there,""", "replace"))

EDITS.append(("crates/nova-cli/tests/run_tests.rs",
"/// `strings_under_gc_stress`. It discriminates only where the collector frees\n/// memory, which is Windows.\n",
"""/// `strings_under_gc_stress`. It discriminates only where the collector frees
/// memory: wherever `gc::stack_base` has an implementation (ADR 0024).
""", "replace"))

EDITS.append(("CHANGELOG.md",
"### Fixed\n- **A spawned task whose handle is never joined no longer keeps its state\n",
"""### Fixed
- **The garbage collector now runs on Linux and macOS, not only on Windows.**
  Until now `gc.rs`'s `stack_base()` returned `None` everywhere else, so
  `collect()` returned before marking and every allocation lived until the
  process exited: a server on Linux or macOS grew with every request. The
  calling thread's stack top now comes from `pthread_getattr_np` and
  `pthread_attr_getstack` on glibc Linux, and from `pthread_get_stackaddr_np`
  on macOS. On GCC and Clang the register-spill shim also calls
  `__builtin_unwind_init`, because glibc's and Apple's `setjmp` scramble some
  of the registers they save. Any other platform still skips collection, and
  under `NOVA_GC_DEBUG` now says so once per thread. The GC tests that were
  Windows-only now build everywhere. Two new tests send 3,000 requests to
  `examples/03-http-server`, each on its own connection, one client at a time
  and then eight at once, and require every collection to stay under 1,000
  and 2,000 live objects respectively. Decision:
  `docs/adr/0024-gc-stack-bounds-on-unix.md`.
- **A spawned task whose handle is never joined no longer keeps its state
""", "replace"))

EDITS.append(("CHANGELOG.md",
"### Changed\n- **Each runtime string is one leaf GC object.**",
"""### Changed
- **CI: the advisory GC-scan step runs every test binary, and clippy lints
  macOS.** The advisory `cargo test -- --ignored` step gained
  `--no-fail-fast`. Without it, ubuntu's run stopped at the first test binary
  with a failure, `nova-cli`'s `run_tests` (through `extern_ffi_run`, issue
  #3), before `nova-runtime`'s GC scan tests ever ran. Clippy gained a
  `macos-latest` leg, because `gc.rs` now has `#[cfg(target_os = "macos")]`
  code that no other leg compiles.
- **Each runtime string is one leaf GC object.**""", "replace"))

EDITS.append(("docs/superpowers/specs/2026-10-05-gc-unix-stack-bounds-design.md",
"reports 8 checks instead of 7, with the new `Clippy (macos-latest)`.\n",
"""
**Amended 2026-10-05 (plan `docs/superpowers/plans/2026-10-05-gc-unix-stack-bounds.md`):**
the plan adds a second leak test, for §5.7's item 3:
`http_server_example_keeps_a_bounded_live_set_under_concurrent_clients`, with
8 client threads of 375 connections each, and every collection under 2,000
live objects. It was measured on Linux while the plan was written (spike
`9aad29f`, debug `nova`): 61–544 live objects across 22 collections, flat.
1, 4 and 16 clients gave 39–104, 88–294 and 77–1,052. Each predicted count
above gains one passed test: ubuntu 1216 / 0 / 9, macOS 1217 / 0 / 8, windows
1222 / 0 / 8.
""", "after"))

run(EDITS)
```

Expected: a `wrote …` line for each of the 17 files, and no `ABORT`.

- [ ] **Step 2: Check the edits are clean**

Run: `cd /d/Projects/nona/nova && git diff --check && git diff --stat | tail -1 && git ls-files --eol docs/adr/0012-file-descriptor-lifecycle.md nova-spec/13-RUNTIME.md CHANGELOG.md crates/nova-runtime/src/crypto.rs && cargo fmt --all -- --check && echo FMT-OK`
Expected:
- no whitespace errors;
- `17 files changed`;
- the files still `i/lf w/crlf`;
- `FMT-OK`.

- [ ] **Step 3: Run the sweep, and take the set difference**

Write this with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/proximity_sweep.py`. It is read-only, and is not an edit script.

```python
# READ-ONLY sweep: every tracked text line that names a platform within three
# lines of a GC word, plus every `setjmp` mention. Skips the dated records
# under docs/superpowers/ and the CHANGELOG's released sections (from the
# first `## [0.` heading on). Prints file:line and the line, grouped by file.
import re, subprocess, sys
REPO = "D:/Projects/nona/nova"
files = subprocess.run(["git", "-C", REPO, "ls-files"], capture_output=True, text=True).stdout.split("\n")
PLAT = re.compile(r"windows|linux|macos|darwin|non-windows|off windows|platform|ubuntu|glibc|unix", re.I)
GC = re.compile(r"\bgc\b|gc::|gc\.rs|collect|collector|stack[ _-]?base|stack[ -]bounds|precise bounds|leak|sweep|swept|\broots?\b|scan|pinned", re.I)
SETJMP = re.compile(r"setjmp", re.I)
TEXT = (".rs", ".md", ".nova", ".c", ".yml", ".toml", ".txt", ".stdout")
hits = {}
for f in files:
    if not f or not f.endswith(TEXT) or f.startswith("docs/superpowers/"):
        continue
    try:
        lines = open(f"{REPO}/{f}", encoding="utf-8").read().replace("\r\n", "\n").split("\n")
    except (UnicodeDecodeError, FileNotFoundError):
        continue
    stop = len(lines)
    if f == "CHANGELOG.md":
        stop = next((i for i, l in enumerate(lines) if l.startswith("## [0.")), stop)
    for i in range(stop):
        l = lines[i]
        near = "\n".join(lines[max(0, i - 3): i + 4])
        if (PLAT.search(l) and GC.search(near)) or SETJMP.search(l):
            hits.setdefault(f, []).append((i + 1, l.strip()[:150]))
total = 0
for f in sorted(hits):
    print(f"== {f} ({len(hits[f])})")
    for n, l in hits[f]:
        print(f"  {n}: {l}")
    total += len(hits[f])
print(f"TOTAL {total} lines in {len(hits)} files", file=sys.stderr)
```

Run:

```bash
cd /d/Projects/nona/nova && G=/c/Users/SAKEER~1/AppData/Local/Temp/gcm && git grep -n -i -E "off windows|non-windows|windows-only|windows only|only on windows|windows alone|which is windows|collection is skipped|skips collection|no-op off|collector is a no-op|stack bounds|stack-bounds|precise bounds|stack_base|returns before marking|windows gate|setjmp" -- . ':!docs/superpowers' > $G/sweep_phrases.txt; git grep -n -E 'target_os = "macos"|target_vendor' -- . ':!docs/superpowers' > $G/sweep_macos.txt; python -X utf8 $G/proximity_sweep.py > $G/sweep_proximity.txt; (cut -d: -f1 $G/sweep_phrases.txt $G/sweep_macos.txt; grep '^== ' $G/sweep_proximity.txt | sed -E 's/^== (.*) \([0-9]+\)$/\1/') | sort -u > $G/sweep_files.txt; (git diff --name-only main...HEAD; git diff --name-only) | sort -u > $G/touched.txt; comm -23 $G/sweep_files.txt $G/touched.txt
```

Expected: the files still matched that this branch did not touch. For each one, and for every remaining match inside a touched file:
- Read the matching lines in context, with wrapped prose flattened. A line-oriented `grep` misses a phrase split across lines, so a miss is not absence.
- Decide whether it is now **false**. Records of their own date's state are not false: the CHANGELOG's released sections and the dated design records.
- Fix every false one in its own style: a dated note in a record, or a rewritten comment in source. Re-run this step afterwards.

**What the planning sweep classified as true or unrelated**, on 2026-10-05, before this branch's edits. Re-check each one, and do not take the list on trust:
- **Unrelated Windows-only behaviour:**
  - `PermissionDenied` and the Windows-only file fixtures: `docs/adr/0011-io-error-kinds.md`, `std/io/lib.nova`, `tests/runtime/file_errors.nova`, `file_open_dir.nova`, `fs_permission_denied.nova`, and `fs.rs:57`;
  - socket measurements: `net.rs`;
  - the linker: `crates/nova-driver/src/link.rs`;
  - `poll.rs:206`;
  - `docs/adr/0023-program-arguments.md:48`.
- **Labels, not claims:** `examples/05-json-api/BENCHMARK.md`'s "One host, Windows" measurement labels.
- **Still true as written:**
  - `task.rs:2895–2897`, ruled in Task 3;
  - `fs.rs:967–973` and `:1169–1171`;
  - `task.rs:3704–3706` and `:5368–5373`;
  - `lib.rs:79` and `:1376`;
  - `nova-spec/20-STDLIB.md:421`.
- **Matches in the released sections, which stay as history:** `CHANGELOG.md`'s `:2950`, `:4260`, `:4721–4722`, `:4806–4807` and `:4910–4911`.

List in the task report every file you judged still true, and why.

- [ ] **Step 4: Commit**

Write this message with the Write tool to `C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t4_commit.txt`:

```text
docs: record collection on Linux and macOS

Dated notes in every record this change makes stale: ADRs 0002, 0009,
0010, 0012 (four), 0013 (two), 0018 and 0020; 13-RUNTIME section 3.1;
20-STDLIB; phase-2-plan (three); and this branch's own spec, for the
plan's concurrent leak test. Source comments that said collection is
Windows-only are rewritten in crypto.rs, std/json, std/collections,
the recycled-task-state fixture and three run_tests.rs stress tests.
The CHANGELOG gets one Fixed and one Changed bullet.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
```

Run: `cd /d/Projects/nona/nova && git add -A docs nova-spec CHANGELOG.md crates std tests && git status --short && git commit -q -F C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t4_commit.txt && git log --oneline -1`
Expected:
- the status lists only the 17 files from step 2, plus any that step 3 fixed;
- one new commit, `docs: record collection on Linux and macOS`.

---

### Task 5: Final verification

**Files:** none changed, unless a check fails. If one does, fix it in the task that owns the code, and re-run this task.

- [ ] **Step 1: The whole suite on Windows**

Check port 3000 (Conventions). Run in the background: `cd /d/Projects/nona/nova && cargo build -p nova-runtime 2>&1 | tail -1 && cargo build --locked --workspace 2>&1 | tail -1 && cargo test --locked --workspace --all-features --no-fail-fast > /c/Users/SAKEER~1/AppData/Local/Temp/gcm/t5_win_full.txt 2>&1`. Count it as in Conventions.
Expected: `1222 passed, 0 failed, 8 ignored`. These must be among the passes:
- `stack_base_lies_above_the_current_frame`;
- `stack_base_is_the_calling_threads_own`;
- `gc_reclaims_garbage`;
- `http_server_example_keeps_a_bounded_live_set`;
- `http_server_example_keeps_a_bounded_live_set_under_concurrent_clients`.

Check them with `grep -E "^test (gc::tests::)?(stack_base_lies_above_the_current_frame|stack_base_is_the_calling_threads_own|gc_reclaims_garbage|http_server_example_keeps_a_bounded_live_set(_under_concurrent_clients)?) \.\.\. ok" /c/Users/SAKEER~1/AppData/Local/Temp/gcm/t5_win_full.txt | wc -l`, which prints `5`.

- [ ] **Step 2: The whole suite on Linux, and its advisory run**

Run in the background: `bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked --workspace --all-features --no-fail-fast > /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/t5_full.txt 2>&1`. Count it as in Conventions.
Expected:
- passed = the Linux baseline + 5;
- failed = the baseline's failed;
- ignored = the baseline + 8;
- the same five tests `ok`, checked by the same `grep` on this file;
- the 17 stress tests `ok`, checked by Task 2 step 7's loop on this file.

Then run CI's advisory command: `bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh test --locked --workspace --all-features --no-fail-fast -- --ignored > /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/t5_ignored.txt 2>&1`. Count it as in Conventions.
Expected:
- about 50 result lines, so every binary ran;
- the eight GC scan tests among them, with results as in Task 3 step 3;
- `extern_ffi_run` `FAILED`, as known (issue #3).

Write the result to the ledger.

- [ ] **Step 3: Lint and format as CI does, on both OSes**

Run: `cd /d/Projects/nona/nova && cargo clippy --locked --all-targets --all-features -- -D warnings 2>&1 | tail -2 && bash /c/Users/SAKEER~1/AppData/Local/Temp/gcm/linux/run.sh clippy --locked --all-targets --all-features -- -D warnings 2>&1 | tail -2 && cargo fmt --all -- --check && echo FMT-OK && git diff --exit-code main -- Cargo.lock && echo LOCK-UNCHANGED && python -X utf8 C:/Users/SAKEER~1/AppData/Local/Temp/gcm/_once/t2_macos_check.py && cd /c/Users/SAKEER~1/AppData/Local/Temp/gcm/macos_check && cargo clippy --offline --target x86_64-apple-darwin -- -D warnings 2>&1 | tail -1`
Expected:
- Windows clippy `Finished`, no warning;
- Linux clippy `Finished`, no warning;
- `FMT-OK`;
- `LOCK-UNCHANGED`;
- a `wrote` line, then the macOS check `Finished`, no warning.

- [ ] **Step 4: Nothing stray**

Run: `cd /d/Projects/nona/nova && git status --short && git log --oneline main..HEAD && git grep -c -E "^\s*#\[cfg\(windows\)\]" -- '*.rs' | tr -d '\r'`
Expected:
- an empty status;
- seven commits: the spec, its correction, this plan, and Tasks 1–4;
- 32 `#[cfg(windows)]` attribute lines across 7 files. That is 36 across 8 before this branch: `gc.rs` loses two, `task.rs` its only one, and `run_tests.rs` one.

- [ ] **Step 5: Hand off to the whole-branch review**

The execution skill's final review takes over from here, followed by `superpowers:finishing-a-development-branch`. The user's standing practice is to push and open a PR, and to merge only on their word.

**The branch is not done until CI passes on all three operating systems.** CI then reports eight checks, `Clippy (macos-latest)` included. Its blocking `cargo test` step should match:

| OS | PR #97 | Predicted |
|---|---|---|
| ubuntu | 1211 / 0 / 1 | 1216 / 0 / 9 |
| macOS | 1212 / 0 / 0 | 1217 / 0 / 8 |
| windows | 1218 / 0 / 8 | 1222 / 0 / 8 |

The advisory step should report about 50 `test result:` lines on every OS. That CI run is the first measurement of the collector on macOS outside the spike. It also runs the two leak tests on macOS for the first time (Review Focus 5).
