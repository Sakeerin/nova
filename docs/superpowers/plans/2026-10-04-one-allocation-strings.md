# One Allocation per Runtime String Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every runtime-made Nova `String` and `Bytes` value one leaf GC
object — the `NovaStr` header at offset 0 and its bytes inline at offset 16 —
instead of a scanned header plus a separate leaf buffer.

**Architecture:** One new crate-private allocator,
`alloc_str_object(len) -> (*mut NovaStr, *mut u8)`. It allocates a single
leaf object of `16 + max(len, 1)` bytes and writes the header, with
`ptr = base + 16`. It returns the header and the address of the bytes.
`gc_str_filled` (and through it `gc_str` and `fs::gc_message`) and
`bytes::gc_bytes` both build on it, so the two layouts are identical by
construction. `nova_rt_str_new`'s literal header becomes a 16-byte leaf.
Nothing else changes: not `NovaStr`'s fields, not `as_str`/`as_bytes`, not
`gc.rs`'s code (Task 4 rewrites one sentence of its module doc), not any
code generator.

**Tech Stack:** Rust (workspace at `D:\Projects\nona\nova`, MSRV 1.78 in
CI), the runtime crate `crates/nova-runtime`, Git Bash on Windows 11 for
commands, Python for scratch analysis.

**Spec:** `docs/superpowers/specs/2026-10-04-one-allocation-strings-design.md`
(commits `2796c81`, `368e6d1` on branch `one-alloc-strings`).

## Global Constraints

- One leaf object per runtime string or `Bytes` value, of
  `16 + max(len, 1)` bytes, with `ptr = base + 16` (spec §4.1).
- The size is computed as `16usize.saturating_add(len.max(1))`, never
  `16 + len` (spec §4.2).
- `gc_str_filled` and `gc_bytes` change in the same commit (spec §4.3).
- `nova_rt_str_new`'s literal header is a 16-byte leaf (spec §4.4).
- Two invariants are documented on `NovaStr`: `ptr` never points into
  another GC object, and `gc_str_filled`'s `fill` must not allocate GC
  memory (spec §4.5).
- No change to `NovaStr`'s fields, `as_str`, `as_bytes`, or any code
  generator (spec §2), and no change to the code of `gc.rs` or `gc/pages.rs`
  (spec §9: the collector is out of scope). `gc.rs`'s module doc comment IS
  rewritten, by Task 4, as spec §6 requires.
- **This checkout's working tree is CRLF** (`core.autocrlf=true`; `git
  ls-files --eol` shows `i/lf w/crlf`). Git Bash's `grep` and `sed` do not
  show the `\r`, so check line endings with Python on raw bytes, and make any
  script that matches multi-line text handle `\r\n`.
- **Run Python with `-X utf8`.** This host's Python 3.13 writes redirected
  stdout in cp874 and raises on characters such as `−`, `≥` and `§`.
- **Every `nova-cli` test that runs `nova build` (the `build_and_run`
  helper's callers, the `*_build_standalone` family and others) links
  `target/debug/nova_runtime.lib`, which `cargo test` does not rebuild.**
  Run `cargo build --locked -p nova-runtime` after a runtime change and
  before any `nova-cli` test run whose built-executable results are to
  count.
- Gates: `cargo fmt --all --check`, `cargo clippy --locked --workspace
  --all-targets --all-features -- -D warnings`, and `cargo test --locked
  --workspace`, all passing.
- Historical records (`BENCHMARK.md`, `CHANGELOG.md` history,
  `docs/benchmarks/README.md`) get dated notes, never rewrites (spec §6).
- **Scripts that edit repository files live only in `/tmp/gcm/_once/`** and
  are renamed to `*.applied` right after they run. Evidence directories hold
  read-only scripts only (memory: a verifier once ran a leftover edit script).
- **Write any Python containing a backslash with the Write tool,** not a
  Bash heredoc; heredocs here collapse `\\`.
- **Commit the measurement record before launching any verifier over it.**

## Review Focus

1. **A string or `Bytes` of exactly 2032, 2033 or 2048 bytes,** at the
   small/large boundary. The object must be the right size and the content
   unchanged. Pinned by Task 2's layout tests, whose length set includes all
   three.
2. **A `Bytes` slice taken from the middle of a combined object.** The new
   value must be its own leaf object with the right bytes. Pinned by Task
   2's `a_bytes_slice_of_a_combined_object_is_its_own_leaf_object`.
3. **An empty `String` or `Bytes` from every builder.** Its `ptr` must lie
   inside its own object. Pinned by Task 2's length-0 cases and its
   empty-pointer test.
4. **A string reachable only through a pointer into the middle of its
   bytes, not at offset 16.** It must survive a collection. Pinned by Task
   2's liveness test with `k = len - 1`.
5. **The string, JSON and bytes fixtures under `NOVA_GC_STRESS`.** Their
   output must be unchanged. Pinned by the existing stress tests, run in
   Task 5's full suite.

---

### Task 1: Predictions and before-binaries (no code)

**Files:**
- Create: `/tmp/gcm/oas/predict.txt` (scratch, never committed)
- Create: `/tmp/gcm/oas/bc_before.exe`, `/tmp/gcm/oas/srv_before.exe` (scratch)

**Interfaces:**
- Consumes: the harness `/tmp/gcm/gd/bench.nova` and server runner
  `/tmp/gcm/gd/run_srv.sh` from "(gc-direct-strings)".
- Produces: `predict.txt`, written before any code; two before-binaries
  with recorded size and SHA-256.

- [ ] **Step 1: Write the predictions before any code exists**

```bash
mkdir -p /tmp/gcm/oas && cd /tmp/gcm/oas && cat > predict.txt <<'EOF'
Predictions, written before any code for one-alloc-strings exists (branch one-alloc-strings
from main 45d05b0; only the spec and this plan are committed). Change: every runtime-made String and Bytes
is one leaf GC object (header + bytes inline) instead of a scanned header plus a leaf buffer;
literal headers become leaves.

Object count per ten-user GET /users request ("(alloc-mix)" counters, 3 runs, 15 s, no
warmup): 225-245 objects (before: 300.9). Scanned objects fall from 233.9 by
67 + L and leaf objects rise from 67.0 by L, where the 67 are the runtime-string
headers (one per leaf buffer) and L is the literal headers evaluated per request,
which become leaves. L >= 40, since user_json alone has four literal parts per
user. So scanned <= 127, leaf >= 107, and scanned objects of the 16-byte class
fall from 197.9 to <= 91.
Per call (ps harness, 5 alternated runs each, fresh process per run):
- stringify_name: after 10-30% lower than before, disjoint
- stringify_email: after 10-30% lower, disjoint
- stringify_escaped: after 8-25% lower, disjoint
- users_json (ten users): after 12-30% lower, disjoint
Server, ten users, --warmup 5 --duration 15, six alternated pairs (three each order):
  after 1-6% above before per pair; ranges may overlap.
Mutants (Task 3), each fails its listed test(s) with exit codes checked.
EOF
date | tee -a predict.txt
```

Expected: a `date` line at the end of `predict.txt`.

- [ ] **Step 2: Build the before-binaries from `main`'s code**

```bash
cd /d/Projects/nona/nova && git status --short | head -3 && cargo build --release --locked 2>&1 | tail -1
G=/tmp/gcm/oas && cp /tmp/gcm/gd/bench.nova $G/bench.nova && sed 's#/tmp/gcm/gd#/tmp/gcm/oas#g' /tmp/gcm/gd/run_srv.sh > $G/run_srv.sh && chmod +x $G/run_srv.sh
rm -f $G/bc_before $G/bc_before.exe $G/srv_before $G/srv_before.exe
./target/release/nova.exe build $G/bench.nova -o $G/bc_before.exe 2>&1 | tail -1
./target/release/nova.exe build examples/05-json-api/src/main.nova -o $G/srv_before.exe 2>&1 | tail -1
for b in bc_before srv_before; do echo "$b size=$(stat -c %s $G/$b.exe) sha=$(sha256sum $G/$b.exe | cut -c1-16)"; done | tee $G/binaries.txt
echo "release nova sha=$(sha256sum target/release/nova.exe | cut -c1-16) from $(git rev-parse --short HEAD)" | tee -a $G/binaries.txt
```

Expected: `git status` clean; the server binary at 701,440 bytes, the plain
release size; two size/SHA lines and a release-`nova` line in
`binaries.txt`.

No commit: everything here is scratch.

---

### Task 2: One leaf object per string — tests first, then the change

**Files:**
- Modify: `crates/nova-runtime/src/lib.rs` — `NovaStr` doc (around line 95),
  `gc_str` doc (around 102-111), `gc_str_filled` (around 113-155), a new
  `alloc_str_object` beside it, `nova_rt_str_new` (around 202-215), and the
  `mod tests` block (starts around line 1171).
- Modify: `crates/nova-runtime/src/bytes.rs` — module doc (lines 3-8),
  `gc_bytes` (around 22-42), `nova_rt_bytes_to_string_unchecked`'s comment
  (around 99-104), and `mod tests` (starts around 247).

**Interfaces:**
- Consumes: `gc::alloc(size: usize, scan: bool) -> *mut u8`;
  test-only `gc::object_info(addr: usize) -> Option<(usize, bool)>` and
  `gc::sweep_with_roots_for_test(roots: &[usize])`; the `lib.rs` test
  helpers `make_str(&'static str) -> *mut NovaStr` and `to_string`.
- Produces: `pub(crate) const STR_HEADER: usize` (16) and
  `pub(crate) fn alloc_str_object(len: usize) -> (*mut NovaStr, *mut u8)`
  in `lib.rs`. `gc_str_filled`'s and `gc_bytes`'s signatures are unchanged.

- [ ] **Step 1: Write the failing `lib.rs` tests**

Append inside `mod tests` in `crates/nova-runtime/src/lib.rs`, after the
existing helpers (`to_string`, `make_str`, `str_array`):

```rust
    /// The lengths every layout test covers: empty, short, one whose
    /// `16 + len` crosses a class boundary, the largest that stays a small
    /// object (`16 + 2032 = 2048`), the first that goes to the large path, and
    /// a large one at a class size.
    const STR_LENGTHS: [usize; 6] = [0, 2, 113, 2032, 2033, 2048];

    /// `len` ASCII bytes with a static lifetime, so `make_str` can take them.
    fn text_of(len: usize) -> &'static str {
        let s: String = "abcdefghij".repeat(len / 10 + 1)[..len].to_string();
        Box::leak(s.into_boxed_str())
    }

    /// One leaf object per runtime string: the header at its start, the bytes
    /// inline at offset 16
    /// (`docs/superpowers/specs/2026-10-04-one-allocation-strings-design.md`).
    unsafe fn assert_one_leaf_string(s: *mut NovaStr, expected: &str) {
        let len = expected.len();
        assert_eq!(
            gc::object_info(s as usize),
            Some((16 + len.max(1), false)),
            "len {len}: one leaf object of 16 + max(len, 1) bytes"
        );
        assert_eq!((*s).ptr as usize, s as usize + 16, "len {len}: bytes inline at offset 16");
        assert_eq!((*s).len, len as u64, "len {len}");
        assert_eq!(as_str(s), expected, "len {len}");
    }

    #[test]
    fn a_runtime_string_is_one_leaf_object_with_its_bytes_inline() {
        for len in STR_LENGTHS {
            let expected = text_of(len);
            unsafe {
                assert_one_leaf_string(gc_str(expected), expected);
                // `gc_str_filled` through a builtin, the text split across two
                // literals.
                let (a, b) = expected.split_at(len / 2);
                assert_one_leaf_string(nova_rt_str_concat(make_str(a), make_str(b)), expected);
            }
        }
    }

    #[test]
    fn a_literal_header_is_a_sixteen_byte_leaf() {
        unsafe {
            let s = make_str("literal");
            assert_eq!(gc::object_info(s as usize), Some((16, false)));
            assert_eq!(as_str(s), "literal");
        }
    }

    #[test]
    fn an_empty_strings_pointer_lies_inside_its_own_object() {
        unsafe {
            let s = gc_str("");
            let base = s as usize;
            let ptr = (*s).ptr as usize;
            let size = gc::object_info(base).expect("a live object").0;
            assert!(
                base + 16 <= ptr && ptr < base + size,
                "ptr {ptr:#x} must lie in [{:#x}, {:#x})",
                base + 16,
                base + size
            );
        }
    }

    /// A string reached only through a pointer into its bytes -- at offset 16
    /// and at its last byte -- survives a collection, with an unrooted control
    /// swept. Explicit roots, so it is deterministic on every platform.
    #[test]
    fn a_runtime_string_reached_only_through_its_bytes_survives_a_collection() {
        for len in [2usize, 113, 2033] {
            let expected = text_of(len);
            unsafe {
                let s = gc_str(expected);
                let control = gc_str(expected);
                assert!(gc::object_info(s as usize).is_some(), "len {len}");
                assert!(gc::object_info(control as usize).is_some(), "len {len}");
                for k in [0, len - 1] {
                    gc::sweep_with_roots_for_test(&[(*s).ptr as usize + k]);
                    assert_eq!(
                        gc::object_info(s as usize),
                        Some((16 + len, false)),
                        "len {len} k {k}: the whole object survives"
                    );
                }
                assert_eq!(
                    gc::object_info(control as usize),
                    None,
                    "len {len}: the unrooted control is swept"
                );
                assert_eq!(as_str(s), expected, "len {len}");
            }
        }
    }
```

- [ ] **Step 2: Rewrite and add the `bytes.rs` tests**

In `crates/nova-runtime/src/bytes.rs`, inside `mod tests`:
- delete `a_bytes_value_has_a_scanned_header_over_a_leaf_buffer` (lines
  251-275, `#[test]` through its closing brace);
- delete `a_bytes_buffer_above_the_gc_floor_is_tracked_at_its_exact_size`
  together with its "**Review finding I1.**" doc comment (lines 296-322);
- **keep** `is_utf8_distinguishes_valid_bytes_from_invalid` and its
  "**Decided before execution.**" doc comment (lines 277-294), unchanged;
- insert the block below where the first deleted test was.

Afterwards `grep -c is_utf8_distinguishes_valid_bytes_from_invalid
crates/nova-runtime/src/bytes.rs` prints 1.

The block:

```rust
    /// One leaf object per `Bytes` value: the header at its start, the payload
    /// inline at offset 16
    /// (`docs/superpowers/specs/2026-10-04-one-allocation-strings-design.md`).
    unsafe fn assert_one_leaf_bytes(b: *mut NovaStr, expected: &[u8]) {
        let len = expected.len();
        assert_eq!(
            crate::gc::object_info(b as usize),
            Some((16 + len.max(1), false)),
            "len {len}: one leaf object of 16 + max(len, 1) bytes"
        );
        assert_eq!((*b).ptr as usize, b as usize + 16, "len {len}: bytes inline at offset 16");
        assert_eq!((*b).len, len as u64, "len {len}");
        assert_eq!(as_bytes(b), expected, "len {len}");
    }

    const BYTES_LENGTHS: [usize; 6] = [0, 2, 113, 2032, 2033, 2048];

    fn payload_of(len: usize) -> Vec<u8> {
        (0..len).map(|i| (i % 251) as u8).collect()
    }

    #[test]
    fn a_bytes_value_is_one_leaf_object_with_its_bytes_inline() {
        for len in BYTES_LENGTHS {
            let payload = payload_of(len);
            // SAFETY: `gc_bytes` returns a live header.
            unsafe { assert_one_leaf_bytes(gc_bytes(&payload), &payload) };
        }
    }

    /// **Review finding I1, restated for one object.** A payload above 16
    /// bytes makes the tracked size, `16 + len`, discriminate an allocation
    /// that ignores `len`. Hardcoding the size to `17` must fail here; for any
    /// payload over one byte that mutation is a heap-buffer overflow, since
    /// the copy still writes `len` bytes.
    #[test]
    fn a_bytes_value_is_tracked_at_its_full_size() {
        let b = gc_bytes(&[7u8; 32]);
        assert_eq!(
            crate::gc::object_info(b as usize),
            Some((16 + 32, false)),
            "a 32-byte payload is one object of 16 + 32 bytes"
        );
    }

    #[test]
    fn a_string_from_bytes_is_one_leaf_object_with_its_bytes_inline() {
        for len in BYTES_LENGTHS {
            let text: String = "abcdefghij".repeat(len / 10 + 1)[..len].to_string();
            // SAFETY: the bytes are ASCII, so valid UTF-8.
            unsafe {
                let s = nova_rt_bytes_to_string_unchecked(gc_bytes(text.as_bytes()));
                assert_one_leaf_bytes(s, text.as_bytes());
            }
        }
    }

    #[test]
    fn a_bytes_slice_of_a_combined_object_is_its_own_leaf_object() {
        let payload = payload_of(113);
        // SAFETY: `gc_bytes` returns a live header.
        unsafe {
            let b = gc_bytes(&payload);
            let s = nova_rt_bytes_slice(b, 10, 50);
            assert_ne!(s, b, "a slice is a fresh object, never a view");
            assert_one_leaf_bytes(s, &payload[10..50]);
        }
    }
```

In the same module, change the two doc comments that named the old tests.

In `bytes_len_reports_the_real_length_for_more_than_one_size`'s doc
comment, the last sentence today reads (wrapped over five lines):

> Two different real lengths rule out a constant; the second clears
> `gc::alloc`'s 8-byte floor so this doubles as a second, independent probe
> of the same floor `a_bytes_buffer_above_the_gc_floor_is_tracked_at_its_exact_size`
> exercises, this time through the header's `len` field rather than
> `object_info`.

Replace it with:

```rust
    /// the only one left open). Two different real lengths rule out a
    /// constant; the second is the 32-byte payload
    /// `a_bytes_value_is_tracked_at_its_full_size` probes through
    /// `object_info`, read here through the header's `len` field instead.
```

(the first line shown is the end of the existing sentence before it, kept).

In `to_ints_writes_the_array_layout_codegen_expects`'s doc, replace
"(see this module's own `a_bytes_buffer_above_the_gc_floor_is_tracked_at_its_exact_size`
for the same reasoning applied to `gc_bytes`'s buffer)" with "(see this
module's own `a_bytes_value_is_tracked_at_its_full_size` for the same
reasoning applied to a `Bytes` object)". Rewrap to 80 columns as the file
does.

Afterwards, `git grep -n "a_bytes_buffer_above\|has_a_scanned_header" --
crates/` must print nothing and exit 1 (no match). Any line it prints is a
reference that was missed; exit 2 means the check itself failed.

- [ ] **Step 3: Run the new tests and watch them fail**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-runtime --lib -- one_leaf literal_header empty_strings reached_only tracked_at_its_full_size bytes_slice_of_a_combined 2>&1 | grep -E "^test |test result"
```

Expected: every new test FAILS. The layout tests see `Some((16, true))` from
`object_info`. The literal test sees `Some((16, true))`. The empty-pointer
test fails its bounds assertion. The liveness test sees `None` at `k = 0`.
The bytes-slice test fails on `object_info`. Nothing fails to compile.

- [ ] **Step 4: Implement `alloc_str_object` and `gc_str_filled`**

In `crates/nova-runtime/src/lib.rs`, replace lines 95-155 with the block
below: everything from `/// A Nova string value` through the closing brace
of `gc_str_filled`, **the `#[repr(C)] pub struct NovaStr` definition
included**, because the block carries its own copy of the struct. Keep
`Fill` and everything after it. Afterwards `grep -c '^pub struct NovaStr'
crates/nova-runtime/src/lib.rs` prints 1.

The block:

```rust
/// A Nova string value: immutable UTF-8, `{ len, ptr }`.
///
/// A runtime-made `String` or `Bytes` value is **one leaf GC object**: this
/// header at offset 0, and its bytes inline at offset 16, where `ptr` points
/// ([`alloc_str_object`]). A string literal's header from
/// [`nova_rt_str_new`] is a 16-byte leaf whose `ptr` targets static data.
///
/// **Invariant: `ptr` never points into another GC object.** Every header is
/// a leaf, so the collector never traces `ptr`. A header that pointed into a
/// different GC object would not keep it alive. A future zero-copy slice or
/// view must not reuse this type with a leaf header.
///
/// **Invariant: [`gc_str_filled`]'s `fill` must not allocate GC memory.**
/// Every current writer only calls [`Fill::put`].
#[repr(C)]
pub struct NovaStr {
    pub len: u64,
    pub ptr: *const u8,
}

/// The size of a [`NovaStr`] header, and the offset of a runtime-made
/// string's bytes inside its one object.
pub(crate) const STR_HEADER: usize = std::mem::size_of::<NovaStr>();

/// Allocate one leaf GC object for a `String` or `Bytes` value of `len` bytes:
/// the [`NovaStr`] header at offset 0, written here, then room for the bytes
/// at offset [`STR_HEADER`], which the caller fills. Returns the header and
/// the address of the bytes.
///
/// `max(len, 1)` keeps `ptr` inside the object even when `len` is 0; without
/// it an empty value's `ptr` would sit one past a 16-byte slot, in the next
/// object. The size saturates, so an overflowing `len` reaches `gc::alloc`'s
/// size-limit abort instead of wrapping to a too-small object.
///
/// One allocation where there were two: in
/// `examples/05-json-api/BENCHMARK.md`'s "(alloc-mix)", the second (the
/// bytes) was 67.0 of 300.9 allocations per ten-user request.
pub(crate) fn alloc_str_object(len: usize) -> (*mut NovaStr, *mut u8) {
    let size = STR_HEADER.saturating_add(len.max(1));
    let base = gc::alloc(size, false);
    let node = base as *mut NovaStr;
    // SAFETY: `base` has `size` writable bytes: the header and at least one
    // byte after it.
    let bytes = unsafe { base.add(STR_HEADER) };
    // SAFETY: as above; nothing else refers to this object yet.
    unsafe {
        (*node).len = len as u64;
        (*node).ptr = bytes;
    }
    (node, bytes)
}

/// Store a Rust string as a GC-managed `NovaStr` value: one leaf object
/// holding the header and, inline after it, a copy of the bytes.
///
/// `pub(crate)`, not private: `fs`'s `gc_message` reuses this rather than
/// reproducing `NovaStr { len, ptr }`'s layout a second time, which is
/// precisely the drift class this shared helper exists to avoid.
pub(crate) fn gc_str(s: &str) -> *mut NovaStr {
    // SAFETY: the writer puts exactly `s.len()` bytes of valid UTF-8.
    unsafe { gc_str_filled(s.len(), |out| out.put(s.as_bytes())) }
}

/// Allocate a GC string of exactly `len` bytes as one leaf object -- the
/// header, then the bytes inline ([`alloc_str_object`]) -- and let `fill`
/// write the bytes straight into it.
///
/// The builtins that build a result from their arguments use this rather
/// than collecting into a Rust `String` and calling [`gc_str`]. That buffer
/// cost a system-heap allocation, a free and a second copy per call: in
/// `examples/05-json-api/BENCHMARK.md`'s "(reprofile-3)", the three largest
/// such builtins spent 3.5–3.6% of the server thread in the system heap.
///
/// GC safety: there is one allocation, and `fill` runs after it. That
/// allocation can run a collection, so whatever `fill` reads from the GC heap
/// must still be reachable then. A caller's arguments are: a pointer held in
/// the caller's frame or in a callee-saved register is a root, and marking is
/// range-based, so a pointer into a string's bytes keeps that string's whole
/// object alive (`gc.rs`'s module doc comment). [`nova_rt_str_chars`] relies
/// on the same. **`fill` must not allocate GC memory**; every writer only
/// calls [`Fill::put`].
///
/// # Safety
/// `fill` must write exactly `len` bytes of valid UTF-8 through
/// [`Fill::put`]. Writing more panics. Writing fewer leaves zero bytes in the
/// string, which a debug build catches.
pub(crate) unsafe fn gc_str_filled(len: usize, fill: impl FnOnce(&mut Fill<'_>)) -> *mut NovaStr {
    let (node, bytes) = alloc_str_object(len);
    // SAFETY: `alloc_str_object` reserved `len` writable bytes at `bytes`
    // that nothing else refers to yet.
    let mut out = Fill {
        out: unsafe { std::slice::from_raw_parts_mut(bytes, len) },
        at: 0,
    };
    fill(&mut out);
    debug_assert_eq!(
        out.at, len,
        "a GC string's writer filled a different length than it asked for"
    );
    node
}
```

- [ ] **Step 5: Make the literal header a leaf**

Replace `nova_rt_str_new` with:

```rust
/// Create a string value from raw bytes (used for string literals).
///
/// The header is a 16-byte **leaf**: its `ptr` targets static literal data,
/// which the collector never frees, so tracing it could never reach a GC
/// object. That depends on this function's safety contract.
///
/// # Safety
/// `ptr` must point to `len` bytes of valid UTF-8 that outlive the program
/// (string literal data emitted by codegen), never into the GC heap.
#[no_mangle]
pub unsafe extern "C" fn nova_rt_str_new(ptr: *const u8, len: u64) -> *mut NovaStr {
    let node = gc::alloc(STR_HEADER, false) as *mut NovaStr;
    (*node).len = len;
    (*node).ptr = ptr;
    node
}
```

- [ ] **Step 6: Build `gc_bytes` on the same allocator**

In `crates/nova-runtime/src/bytes.rs`, replace `gc_bytes` and its doc
comment with:

```rust
/// Store `bytes` as a GC-managed `Bytes` value: one **leaf** object holding
/// the header and, inline after it, a copy of the payload.
///
/// The sibling of `crate::gc_str`, differing only in taking `&[u8]` rather than
/// `&str`. Both build through [`crate::alloc_str_object`], so the layout is
/// identical by construction.
///
/// GC safety: three callers (`nova_rt_bytes_from_string`,
/// `nova_rt_bytes_to_string_unchecked`, `nova_rt_bytes_slice`) pass a slice of
/// a live GC string, read here after this function's allocation. It stays
/// alive across that allocation because the caller's slice pointer is a root
/// and marking is range-based (`gc.rs`'s module doc comment).
pub(crate) fn gc_bytes(bytes: &[u8]) -> *mut NovaStr {
    let (node, dst) = crate::alloc_str_object(bytes.len());
    // SAFETY: `alloc_str_object` reserved `bytes.len()` writable bytes at
    // `dst`, inside a fresh object that cannot overlap `bytes`.
    unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), dst, bytes.len()) };
    node
}
```

In `nova_rt_bytes_to_string_unchecked`, replace the comment's "`gc_bytes`
allocates a fresh header and buffer and copies `b`'s bytes into it" with
"`gc_bytes` allocates a fresh object and copies `b`'s bytes into it". Rewrap
to the file's width.

In the module doc (lines 5-6), replace "a scanned `{len, ptr}` header over a
GC **leaf** buffer" with "one GC **leaf** object, a `{len, ptr}` header
followed inline by the bytes `ptr` points at".

- [ ] **Step 7: Run the new tests and watch them pass**

```bash
cd /d/Projects/nona/nova && cargo fmt --all && cargo test --locked -p nova-runtime --lib -- one_leaf literal_header empty_strings reached_only tracked_at_its_full_size bytes_slice_of_a_combined 2>&1 | grep -E "^test |test result"
```

Expected: all 8 tests pass, `test result: ok. 8 passed`.

- [ ] **Step 8: Run the runtime crate and the string, JSON and bytes fixtures**

```bash
cd /d/Projects/nona/nova && cargo test --locked -p nova-runtime 2>&1 | grep -E "test result|FAILED|panicked" | head; cargo build --locked -p nova-runtime 2>&1 | tail -1; cargo test --locked -p nova-cli --test run_tests -- json strings interpolation bytes fs 2>&1 | grep -E "test result|FAILED" | head
```

Expected: `0 failed` on every `test result` line. The `cargo build -p
nova-runtime` refreshes `target/debug/nova_runtime.lib`, which every test
that runs `nova build` links. Under this step's filters those are
`strings_build_standalone`, `fs_not_found_build_standalone`,
`bytes_api_build_standalone`, and the build half of
`record_literal_inside_an_interpolation_runs`. Without the rebuild, those
builds link a pre-change runtime and pass regardless.

- [ ] **Step 9: Commit the code and its tests together**

```bash
cd /d/Projects/nona/nova && git add crates/nova-runtime/src/lib.rs crates/nova-runtime/src/bytes.rs && git commit -F - <<'EOF'
perf(runtime): make each runtime string one leaf GC object

A runtime-made String or Bytes value was a scanned 16-byte header over a
separate leaf buffer. It is now one leaf object: the header at offset 0
and its bytes inline at offset 16, built by a new alloc_str_object that
gc_str_filled and gc_bytes share, so the layouts are identical by
construction. Literal headers from nova_rt_str_new become leaves too.
A NovaStr's ptr never points into another GC object, documented on the
type. Tests pin the layout of every builder at six lengths around the
small/large boundary, a leaf literal header, an empty string's pointer,
and liveness through a pointer into the bytes alone.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

---

### Task 3: Named mutants

**Files:**
- Create: `/tmp/gcm/_once/mutate_oas.py` (edits repository files while it
  runs; renamed to `.applied` afterwards)
- Create: `/tmp/gcm/oas/mutations.log` (scratch)

**Interfaces:**
- Consumes: Task 2's code and tests.
- Produces: `mutations.log`, one line per mutant: name, failing tests,
  exit codes, result lines.

- [ ] **Step 1: Write the mutation script with the Write tool**

Write `/tmp/gcm/_once/mutate_oas.py` with this content:

```python
import io
import os
import subprocess
import sys

sys.stdout.reconfigure(encoding='utf-8')
os.chdir('D:/Projects/nona/nova')
LIB = 'crates/nova-runtime/src/lib.rs'
BYT = 'crates/nova-runtime/src/bytes.rs'
GC_BYTES_START = 'pub(crate) fn gc_bytes(bytes: &[u8]) -> *mut NovaStr {\n'


def fn_span(src, start_marker, nl='\n'):
    """The span of the function that starts at `start_marker`, through its closing brace.

    `nl` is the file's own line ending: this checkout's working tree is CRLF.
    """
    start = src.index(start_marker.replace('\n', nl))
    return start, src.index(nl + '}' + nl, start) + len(nl) + 1


# Mutant 4's body is `main`'s own two-object `gc_bytes`, read from git, not retyped.
main_bytes = subprocess.run(['git', 'show', 'main:' + BYT], capture_output=True, text=True,
                            encoding='utf-8').stdout
s0, e0 = fn_span(main_bytes, GC_BYTES_START)
OLD_GC_BYTES = main_bytes[s0:e0]
assert 'size_of::<NovaStr>(), true' in OLD_GC_BYTES, 'main gc_bytes not found'
M = [
    ('1 string object left scanned', LIB,
     'let base = gc::alloc(size, false);', 'let base = gc::alloc(size, true);'),
    ('2 header ptr at base + 8, bytes still at base + 16', LIB,
     '(*node).ptr = bytes;', '(*node).ptr = base.add(8);'),
    ('3 max(len, 1) dropped', LIB,
     'let size = STR_HEADER.saturating_add(len.max(1));', 'let size = STR_HEADER.saturating_add(len);'),
    ('4 gc_bytes left two-object', BYT, None, None),
    ('5 literal header left scanned', LIB,
     'let node = gc::alloc(STR_HEADER, false) as *mut NovaStr;',
     'let node = gc::alloc(STR_HEADER, true) as *mut NovaStr;'),
]
CMDS = [
    ['cargo', 'test', '--locked', '-p', 'nova-runtime', '--lib'],
    ['cargo', 'test', '--locked', '-p', 'nova-cli', '--test', 'run_tests', '--', 'json', 'strings', 'bytes'],
]
orig = {p: io.open(p, encoding='utf-8', newline='').read() for p in (LIB, BYT)}
# Fail before mutating anything if mutant 4's marker is missing.
fn_span(orig[BYT], GC_BYTES_START, '\r\n' if '\r\n' in orig[BYT] else '\n')
try:
    for name, path, old, new in M:
        src = orig[path]
        nl = '\r\n' if '\r\n' in src else '\n'
        if old is None:
            start, end = fn_span(src, GC_BYTES_START, nl)
            mutated = src[:start] + OLD_GC_BYTES.replace('\n', nl) + src[end:]
        else:
            o = old.replace('\n', nl)
            if src.count(o) != 1:
                print(name, 'ANCHOR', src.count(o), flush=True)
                continue
            mutated = src.replace(o, new.replace('\n', nl))
        io.open(path, 'w', encoding='utf-8', newline='').write(mutated)
        failed, results, codes = [], [], []
        for cmd in CMDS:
            r = subprocess.run(cmd, capture_output=True, text=True, encoding='utf-8', errors='replace')
            out = r.stdout + r.stderr
            codes.append(r.returncode)
            failed += [l.split()[1] for l in out.splitlines() if l.startswith('test ') and l.rstrip().endswith('FAILED')]
            res = [l.split(';')[0] for l in out.splitlines() if l.startswith('test result')]
            results += res or ['NO RESULT LINE: ' + ' | '.join(out.strip().splitlines()[-2:])]
        print(name, '->', failed or 'NONE FAILED', '| exits', codes, '| results', results, flush=True)
        io.open(path, 'w', encoding='utf-8', newline='').write(orig[path])
finally:
    for p, s in orig.items():
        io.open(p, 'w', encoding='utf-8', newline='').write(s)
print('restored:', repr(subprocess.run(['git', 'status', '--short'], capture_output=True, text=True).stdout.strip()))
```

- [ ] **Step 2: Run it, then defuse it**

```bash
python -X utf8 /tmp/gcm/_once/mutate_oas.py 2>&1 | tee /tmp/gcm/oas/mutations.log; mv /tmp/gcm/_once/mutate_oas.py /tmp/gcm/_once/mutate_oas.py.applied
```

Expected, each mutant failing at least what the spec names:

| mutant | must fail |
|---|---|
| 1 left scanned | `a_runtime_string_is_one_leaf_object_with_its_bytes_inline`, `a_bytes_value_is_one_leaf_object_with_its_bytes_inline` |
| 2 `ptr` at `base + 8` | the same two; `a_runtime_string_reached_only_through_its_bytes_survives_a_collection` through its content assertion; and existing content tests and JSON/strings fixtures |
| 3 `max(len, 1)` dropped | the layout tests at `len` 0, and `an_empty_strings_pointer_lies_inside_its_own_object` |
| 4 `gc_bytes` two-object | `a_bytes_value_is_one_leaf_object_with_its_bytes_inline` |
| 5 literal scanned | `a_literal_header_is_a_sixteen_byte_leaf` |

Mutant 4 swaps in `main`'s own `gc_bytes`, read with `git show`, so the
mutant is exactly the code before this change, not a retyped copy.

The final line is `restored: ''`. Any `ANCHOR` line means an anchor failed
to match after `cargo fmt`. A Python traceback, or a missing `restored:`
line, means the run stopped part-way: the mutants after the one that raised
did not run, and `mv` renames the script anyway. In either case fix the
cause in a copy under `/tmp/gcm/_once/`, rerun only the mutants that did not
report, rename that copy to `*.applied`, and confirm `git status --short` is
empty. Do not count a partial log as complete.

The second command's `*_build_standalone` tests link
`target/debug/nova_runtime.lib`, which no mutant rebuilds, so only the JIT
and unit-test results say anything about a mutant.

No commit.

---

### Task 4: Documentation

**Files** (line numbers are on `main` `45d05b0`; Task 2 shifts `lib.rs`'s, so
find those by their text):
- Modify, present tense rewritten:
  - `crates/nova-runtime/src/gc.rs:23-25`;
  - `crates/nova-runtime/src/lib.rs:9-10` and `292-294`;
  - `crates/nova-hir/src/lib.rs:33-34`;
  - `std/bytes/lib.nova:6-7`;
  - `nova-spec/13-RUNTIME.md:131`, `136-138` and `171-173`;
  - `nova-spec/20-STDLIB.md:234-235`;
  - `crates/nova-cli/tests/run_tests.rs:2808-2810`, `6710-6712` and
    `6813-6814`.
- Modify, dated notes only:
  - `examples/05-json-api/BENCHMARK.md` (after lines 1498, 1512, 2229,
    5010 and 5020);
  - `CHANGELOG.md` (after 502, 574 and 3951);
  - `docs/benchmarks/README.md` (after 903).

**Interfaces:**
- Consumes: Task 2's layout.
- Produces: no code.

**Plan ruling against the spec:** `lib.rs:292` ("one result, two GC
objects") is missing from spec §6's list. It states the representation in
the present tense and this change makes it false, so it is rewritten here.
The sweep in Step 3 adds the term "two GC objects", because the spec's
term "two objects" does not match it.

**Second plan ruling:** spec §6 lists the restatement at
`docs/benchmarks/README.md:101-104` ("20 GC allocations for ten headers'
strings") as "the example's `README.md:101-104`".
`examples/05-json-api/README.md` never mentions allocation, so this plan
names the right file.

**Third plan ruling:** a string literal is not one object holding its
bytes. Its header is a 16-byte leaf pointing at static data (spec §4.4).
So the rewrites below say "runtime-made" wherever they describe `String`
values.

- [ ] **Step 1: Rewrite the present-tense descriptions**

Make each replacement exactly. Where the new text is longer, rewrap the
rest of that paragraph to the file's existing width.

`crates/nova-runtime/src/gc.rs`, module doc:

```text
BEFORE  //! held transiently) keep their containing object alive. Objects flagged
        //! `scan = false` (string byte buffers) are leaves and are not traced.
AFTER   //! held transiently) keep their containing object alive. Objects flagged
        //! `scan = false` are leaves and are not traced: strings and `Bytes` (a
        //! runtime-made value is one object holding its header and bytes; a string
        //! literal's is a 16-byte header pointing at static data).
```

`crates/nova-runtime/src/lib.rs`, crate doc:

```text
BEFORE  //! **Memory:** heap values (records, sums, arrays, closures, strings, and
        //! byte buffers) are managed by a conservative mark-and-sweep garbage
AFTER   //! **Memory:** heap values (records, sums, arrays, closures, strings, and
        //! `Bytes`) are managed by a conservative mark-and-sweep garbage
```

`crates/nova-runtime/src/lib.rs`, `nova_rt_str_concat_n`'s doc:

```text
BEFORE  /// One copy of each part and one result, two GC objects, where pairwise
        /// concatenation made a new string per part, each copying everything built
        /// so far. Each part is copied straight into the result's buffer.
AFTER   /// One copy of each part into one result, where pairwise concatenation made
        /// a new string per part, each copying everything built so far. Each part is
        /// copied straight into the result's bytes.
```

`crates/nova-hir/src/lib.rs`, `Ty::Bytes`'s doc:

```text
BEFORE      /// Structurally identical to [`Ty::String`] -- both are a scanned
            /// `{len, ptr}` header over a GC leaf buffer -- and semantically distinct:
AFTER       /// Structurally identical to [`Ty::String`] -- both are a `{len, ptr}` GC
            /// leaf header; a runtime-made value's bytes follow it inline in the same
            /// object, a `String` literal's are static data -- and semantically distinct:
```

`std/bytes/lib.nova`:

```text
BEFORE  // `Bytes` is an immutable byte buffer. It shares `String`'s representation -- a
        // scanned header over a leaf buffer -- and differs in carrying no encoding
AFTER   // `Bytes` is an immutable byte buffer. It shares `String`'s representation --
        // a `{len, ptr}` leaf header, and every `Bytes` value is one object with its
        // bytes inline after it -- and differs in carrying no encoding
```

`nova-spec/13-RUNTIME.md:131`:

```text
BEFORE  - `String` = heap object: `{ len: usize, data: ptr<u8> }` UTF-8
AFTER   - `String` = one leaf heap object: a `{ len: u64, ptr }` header with the UTF-8 bytes inline after it, at offset 16, where `ptr` points; a string literal is a 16-byte leaf header whose `ptr` targets static data
```

`nova-spec/13-RUNTIME.md`, §2.3 "String Encoding": add one bullet directly
after "- Slicing returns `Str` (view) — no copy":

```text
- [Amended 2026-10-04: no `Str` view type is implemented, and `String` and `Bytes` slices copy. A future view must not be a `NovaStr` with a leaf header, because the collector never traces a leaf header's `ptr` (`docs/superpowers/specs/2026-10-04-one-allocation-strings-design.md` §4.5).]
```

`nova-spec/13-RUNTIME.md:171-173`:

```text
BEFORE  transiently, say) keeps its containing object alive. Objects allocated with `scan =
        false` — string byte buffers — are leaves and are never traced.
AFTER   transiently, say) keeps its containing object alive. Objects allocated with `scan =
        false` — strings and `Bytes`: a runtime-made value is one object holding its header
        and bytes, a string literal's is a 16-byte header pointing at static data — are
        leaves and are never traced.
```

`nova-spec/20-STDLIB.md:234-235`:

```text
BEFORE  // in the remaining increments needs them). `Bytes` (a scanned `{len, ptr}`
        // header over a GC leaf buffer, `std/bytes`) is the concrete buffer type:
AFTER   // in the remaining increments needs them). `Bytes` (one GC leaf object, a
        // `{len, ptr}` header followed inline by its bytes, `std/bytes`) is the
        // concrete buffer type:
```

`crates/nova-cli/tests/run_tests.rs`, `strings_under_gc_stress`'s doc:

```text
BEFORE  /// two new allocation shapes reachable from a builtin: a scanned array of
        /// scalars, and a leaf byte buffer plus a scanned header. Every method built
AFTER   /// two new allocation shapes reachable from a builtin: a scanned array of
        /// scalars, and a string, which was a leaf byte buffer plus a scanned header
        /// and since 2026-10-04 is one leaf object holding both. Every method built
```

`crates/nova-cli/tests/run_tests.rs`, the bytes stress test's doc:

```text
BEFORE  /// fresh scanned array block, `slice`/`concat`/`bytes_from_ints` each allocate
        /// a fresh header and leaf buffer, and `index_of`/`contains` hold both
AFTER   /// fresh scanned array block, `slice`/`concat`/`bytes_from_ints` each allocate
        /// a fresh `Bytes` object, and `index_of`/`contains` hold both
```

`crates/nova-cli/tests/run_tests.rs`, the `fs` read stress test's doc:

```text
BEFORE  /// allocation). `nova_rt_fs_read` stashes a freshly allocated `Bytes` header
        /// and leaf buffer into the current task's `Slot::Buffer` entry, GC-rooting it
AFTER   /// allocation). `nova_rt_fs_read` stashes a freshly allocated `Bytes` object
        /// into the current task's `Slot::Buffer` entry, GC-rooting it
```

- [ ] **Step 2: Add the dated notes to historical records**

Append each note after the exact sentence named, inside the same bullet
or paragraph, separated by one space. Never change the sentence itself.

| file | after the sentence ending | note |
|---|---|---|
| `examples/05-json-api/BENCHMARK.md` (1498) | "each a new two-object string copying everything built so far." | `[Amended 2026-10-04: since "(one-alloc-strings)" a runtime string is one object holding its header and bytes; these bullets describe the build measured here.]` |
| same (1512) | "array and one two-object result." | `[Amended 2026-10-04: since "(one-alloc-strings)" a runtime string is one object; this arithmetic describes the build measured here.]` |
| same (2229) | "two allocations also counted under allocation." | `[Amended 2026-10-04: since "(one-alloc-strings)" that buffer and node are one allocation.]` |
| same (5010) | "allocates the GC byte buffer at its exact final length and lets the caller write into it." | `[Amended 2026-10-04: since "(one-alloc-strings)" the buffer and its header are one object.]` |
| same (5020) | "the argument `nova_rt_str_chars` already relies on." | `[Amended 2026-10-04: since "(one-alloc-strings)" there is one allocation, not a first of two.]` |
| `CHANGELOG.md` (502) | "Output is unchanged." (the end of the "Five string builtins write their result straight into GC memory" bullet) | `[Amended 2026-10-04: since one allocation per string, the GC buffer and its header are one object.]` |
| same (574) | "two-object string that copied everything built so far." | `[Amended 2026-10-04: since one allocation per string, a runtime string is one object.]` |
| same (3951) | "rather than a second Rust struct with the identical layout." | `[Amended 2026-10-04: since one allocation per string, a runtime-made String or Bytes value is one leaf object, its header followed inline by its bytes; a String literal's header is a 16-byte leaf pointing at static data.]` |

`docs/benchmarks/README.md`: insert a new paragraph after the one ending
"so nothing here establishes allocation as the cause." (line 903), before
"### Amplification, on matched populations":

```text
**Amended 2026-10-04:** the source reading above counts each `String` as
one allocation. Until 2026-10-04 a runtime `String` was two, a 16-byte
header and a separate byte buffer, so that reading undercounted. Since one
allocation per string
(`docs/superpowers/specs/2026-10-04-one-allocation-strings-design.md`) it
is one. The figure's restatements (`CHANGELOG.md`, ADR 0019,
`examples/05-json-api/BENCHMARK.md`, this file's own "20 GC allocations for
ten headers' strings", `docs/benchmarks/server.nova`) are left as they are; ADR 0019 already
points readers here.
```

- [ ] **Step 3: Run the wrap-tolerant sweep for anything missed**

Write `/tmp/gcm/oas/docsweep.py` with the Write tool. It is read-only.

```python
"""Read-only: every tracked mention of the two-object string layout, wrap-tolerant."""
import re
import subprocess
import sys

sys.stdout.reconfigure(encoding='utf-8')

ROOT = 'D:/Projects/nona/nova/'
files = subprocess.run(['git', 'ls-files'], capture_output=True, text=True, cwd=ROOT).stdout.split()
TERMS = re.compile(r'two-object|two (?:GC )?objects|two allocations|`NovaStr` node|header over|'
                   r'leaf buffer|string byte buffer|GC buffer|\{len, ptr\}|\{ len, data', re.I)
NEAR = re.compile(r'header[^.]{0,200}buffer', re.I)
total = 0
for f in files:
    if f.startswith('docs/superpowers/') or not f.endswith(('.rs', '.md', '.nova')):
        continue
    try:
        t = open(ROOT + f, encoding='utf-8').read()
    except (UnicodeDecodeError, OSError):
        continue
    flat = re.sub(r'[ \t]*\r?\n[ \t]*(?:///?!?|#|\*|>)?[ \t]*', ' ', t)
    hits = [(m, 'term') for m in TERMS.finditer(flat)]
    hits += [(m, 'near') for m in NEAR.finditer(flat)]
    for m, kind in sorted(hits, key=lambda h: h[0].start()):
        ctx = flat[max(0, m.start() - 100):m.end() + 100]
        if kind == 'near' and re.search(r'HTTP|http|request|response', ctx):
            continue
        noted = 'Amended 2026-10-04' in flat[max(0, m.start() - 200):m.end() + 600]
        total += 1
        print(('[noted] ' if noted else '[CHECK] ') + f + ' :: ' + ctx)
print('hits:', total)
```

Run it:

```bash
cd /d/Projects/nona/nova && python -X utf8 /tmp/gcm/oas/docsweep.py > /tmp/gcm/oas/docsweep.log; tail -1 /tmp/gcm/oas/docsweep.log | grep -q '^hits:' || echo SWEEP-INCOMPLETE; grep -c '^\[CHECK\]' /tmp/gcm/oas/docsweep.log; grep '^\[CHECK\]' /tmp/gcm/oas/docsweep.log
```

Expected: no `SWEEP-INCOMPLETE` and no traceback (either means the sweep
stopped part-way and must be re-run, not counted), and every `[CHECK]` line
is one of five kinds:
- this change's own new wording: any line containing "header followed
  inline" or "one leaf object", plus the `Ty::Bytes` doc in
  `crates/nova-hir/src/lib.rs` ("`{len, ptr}` GC leaf header", 1 hit) and
  the `std/bytes/lib.nova` module comment ("`{len, ptr}` leaf header", 1
  hit);
- a restatement the spec deliberately leaves (spec §6: `CHANGELOG.md`
  34-35 and 1947-1949, ADR 0019:338-340, `BENCHMARK.md:1067-1072`, the
  `docs/benchmarks/README.md:101-104`, `docs/benchmarks/server.nova:18-19`);
- a historical record whose note sits beyond the 600-character window;
- an HTTP header, which spec §6 sets aside: `CHANGELOG.md:95`,
  `examples/05-json-api/BENCHMARK.md:1059`, the `ERR_*` constants in
  `crates/nova-runtime/src/http.rs`, `HttpErrorKind` in `std/http/lib.nova`,
  and `tests/runtime/http_*.nova`;
- a statement still true under one object, left as it is:
  `crates/nova-runtime/src/fs.rs:134`, `crates/nova-runtime/src/bytes.rs:49`,
  `tests/runtime/fs_bytes_roundtrip.nova:12`, `CHANGELOG.md:3978`,
  `crates/nova-cli/tests/run_tests.rs:8658`, the spec's optional "result's
  buffer" sentences (`crates/nova-runtime/src/lib.rs` near 387), and
  `examples/05-json-api/BENCHMARK.md:5679`, a historical proposal.

Fix any other `[CHECK]` line that states the two-object layout in the
present tense, or a historical statement of it with no dated note, then
re-run. Note the final `[CHECK]` count
for the record.

- [ ] **Step 4: Commit the documentation**

```bash
cd /d/Projects/nona/nova && git status --short && git add -u && git commit -F - <<'EOF'
docs: describe a runtime string as one leaf object

Present-tense descriptions of String and Bytes as a scanned header over a
leaf buffer are rewritten (runtime and HIR comments, std/bytes,
nova-spec 13-RUNTIME and 20-STDLIB, stress-test comments, and
nova_rt_str_concat_n's "two GC objects"). 13-RUNTIME's String Encoding
notes that a future Str view must not be a leaf-headered NovaStr.
Historical records in BENCHMARK.md, CHANGELOG.md and
docs/benchmarks/README.md get dated notes, not rewrites.

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

Expected: `git status --short` lists only the ten files named above.

---

### Task 5: Gates

**Files:**
- Create: `/tmp/gcm/oas/gates.log`, `/tmp/gcm/oas/suite.log` (scratch)

**Interfaces:**
- Consumes: Tasks 2 and 4.
- Produces: the gate results the record quotes.

- [ ] **Step 1: Run fmt, clippy and the full suite**

```bash
cd /d/Projects/nona/nova && { echo "base $(git rev-parse --short HEAD) branch $(git branch --show-current)"; git status --short; cargo fmt --all --check; echo "fmt exit $?"; cargo clippy --locked --workspace --all-targets --all-features -- -D warnings 2>&1 | tail -2; echo "clippy exit ${PIPESTATUS[0]}"; cargo build --locked -p nova-runtime 2>&1 | tail -1; cargo test --locked --workspace > /tmp/gcm/oas/suite.log 2>&1; echo "test exit $?"; grep '^test result' /tmp/gcm/oas/suite.log | awk '{p+=$4; f+=$6; i+=$8} END {print "passed",p,"failed",f,"ignored",i}'; grep -E "FAILED|panicked" /tmp/gcm/oas/suite.log | head -5; } 2>&1 | tee /tmp/gcm/oas/gates.log
```

Expected: `fmt exit 0`, `clippy exit 0`, `test exit 0`, and
`passed 1189 failed 0 ignored 8`. The baseline is 1183 / 0 / 8 (recorded in
`examples/05-json-api/BENCHMARK.md`'s "(gc-direct-strings)"; every commit
since has been docs only). Task 2 adds eight tests and replaces two, which
gives 1189.

---

### Task 6: Measure, record, verify, and open the PR

**Files:**
- Create: `/tmp/gcm/oas/` measurement files (scratch)
- Modify: `examples/05-json-api/BENCHMARK.md` (two amendments, "(alloc-mix)"
  and "(one-alloc-strings)", inserted before "## What was measured, and with
  what"), `CHANGELOG.md` (`[Unreleased]` Measured and Changed bullets)

**Interfaces:**
- Consumes: Task 1's predictions and before-binaries, Task 3's mutations,
  Task 5's gates, `/tmp/gcm/mix/` "(alloc-mix)" logs and patch.
- Produces: the committed record and PR.

- [ ] **Step 1: Build the after-binaries**

```bash
cd /d/Projects/nona/nova && cargo build --release --locked 2>&1 | tail -1 && G=/tmp/gcm/oas && rm -f $G/bc_after $G/bc_after.exe $G/srv_after $G/srv_after.exe && ./target/release/nova.exe build $G/bench.nova -o $G/bc_after.exe 2>&1 | tail -1 && ./target/release/nova.exe build examples/05-json-api/src/main.nova -o $G/srv_after.exe 2>&1 | tail -1 && for b in bc_after srv_after; do echo "$b size=$(stat -c %s $G/$b.exe) sha=$(sha256sum $G/$b.exe | cut -c1-16)"; done | tee -a $G/binaries.txt && echo "release nova sha=$(sha256sum target/release/nova.exe | cut -c1-16) from $(git rev-parse --short HEAD)" | tee -a $G/binaries.txt
```

Expected: two more size/SHA lines; the release `nova` line names Task 4's
commit.

- [ ] **Step 2: Equivalence, per-call and server pairs in one background run**

Write `/tmp/gcm/oas/measure.sh`:

```bash
#!/bin/bash
G=/tmp/gcm/oas; D=/d/Projects/nona/nova
cd $D
echo "start $(date +%T)"
bun docs/benchmarks/bun-equivalence.js $G/srv_after.exe > $G/equivalence.log 2>&1; echo "equivalence exit=$?" >> $G/equivalence.log
tail -2 $G/equivalence.log
rm -f $G/percall.log $G/server.log
for i in 1 2 3 4 5; do
  if [ $((i % 2)) = 1 ]; then order="before after"; else order="after before"; fi
  for side in $order; do $G/bc_$side.exe | sed "s/^/run=$i side=$side /" >> $G/percall.log; done
done
echo "percall done $(date +%T)"
for p in 1 2 3 4 5 6; do
  if [ $p -le 3 ]; then order="before after"; else order="after before"; fi
  for side in $order; do $G/run_srv.sh ${side}_$p $G/srv_$side.exe > /dev/null; done
done
echo "end $(date +%T)"
```

Run it in the background with a 30-minute timeout:

```bash
chmod +x /tmp/gcm/oas/measure.sh && /tmp/gcm/oas/measure.sh > /tmp/gcm/oas/measure.log 2>&1
```

Expected: `EQUIVALENCE OK: all 9 exchanges match on status and body bytes`;
`percall.log` holds 50 lines: 40 measurements (`stringify_name`,
`stringify_email`, `stringify_escaped` and `users_json`, ten each) and 10
`escaped_sample=` lines; and `server.log` holds 12 lines with `errors=0`.

- [ ] **Step 3: Analyse per call and server**

Copy `/tmp/gcm/gd/analyze.py` to `/tmp/gcm/oas/analyze.py`. Change its `G`
from `'C:/Users/SAKEER~1/AppData/Local/Temp/gcm/gd/'` to
`'C:/Users/SAKEER~1/AppData/Local/Temp/gcm/oas/'`; Python on Windows does
not resolve `/tmp`. Then run it with `python /tmp/gcm/oas/analyze.py`.

Expected: before and after ranges per call, per-pair cuts, the server ranges
with per-pair ratios, and a split by order.

- [ ] **Step 4: Confirm the object count with the scratch counters**

The alloc-mix patch touches only `gc.rs`, whose code this change leaves
alone. Task 4 grows its module doc by two lines (a 2-line sentence
becomes 4), which moves the patch's hunks (at line 107 and later) down by
two lines; `git apply` accepts that offset. So it applies:

```bash
cd /d/Projects/nona/nova && git apply /tmp/gcm/mix/alloc-mix.patch && cargo build --release --locked 2>&1 | tail -1 && G=/tmp/gcm/oas && rm -f $G/m $G/m.exe && ./target/release/nova.exe build examples/05-json-api/src/main.nova -o $G/m.exe 2>&1 | tail -1 && sed 's#/tmp/gcm/mix#/tmp/gcm/oas#g' /tmp/gcm/mix/run.sh > $G/run_mix.sh && chmod +x $G/run_mix.sh && rm -f $G/runs.log && for r in 1 2 3; do $G/run_mix.sh $r; done; git checkout -- crates/nova-runtime/src/gc.rs && cargo build --release --locked 2>&1 | tail -1 && git status --short
```

Then copy `/tmp/gcm/mix/analyze.py` to `/tmp/gcm/oas/analyze_mix.py`.
Change its `G` from `'C:/Users/SAKEER~1/AppData/Local/Temp/gcm/mix/'` to
`'C:/Users/SAKEER~1/AppData/Local/Temp/gcm/oas/'`, then run it.

Expected: about 234 objects per request. Scanned objects fall from 233.9
by 67 + L and leaf objects rise from 67.0 by L, where L is the literal
headers evaluated per request (L >= 40: `user_json` alone has four literal
parts per user). No 16-byte scanned object comes from a string (spec §7.2
item 1). Derive L from the scanned drop and record it. Record zeroed bytes
per request, the per-class table and requests per collection (`requests=`
over `collections=` on each `runs.log` line) beside the "(alloc-mix)"
figures (11,742–11,758 zeroed bytes; 176.9 requests per collection),
whichever way they move. `git status` clean; the release build restored.

- [ ] **Step 5: Write the record and commit it before verification**

Insert two amendments before "## What was measured, and with what" in
`examples/05-json-api/BENCHMARK.md`, in this order:

1. **"## AMENDMENT 2026-10-04 (alloc-mix): what a ten-user request
   allocates"**, recording spec §1's figures from
   `/tmp/gcm/mix/runs.log`, `r_*.err` and `analyze.py`:
   - 300.91–300.94 objects per request;
   - 66.98 leaf, 22.3%;
   - the 16-byte class 77.7%;
   - 11,742–11,758 zeroed bytes, 39.5–39.6% of them leaf;
   - no large objects;
   - the per-class table;
   - the method;
   - its predictions table, from `/tmp/gcm/mix/predict.txt`.
2. **"## AMENDMENT 2026-10-04 (one-alloc-strings): one leaf object per
   runtime string"**, in "(gc-direct-strings)"'s structure:
   - the change;
   - the per-call results;
   - the server results with order split;
   - the object count after, with L derived, zeroed bytes per request, the
     per-class table and requests per collection, each beside the
     "(alloc-mix)" before figures (spec §8: size-class shifts and collection
     cadence are measured, not assumed);
   - how it was measured, with binary sizes and SHAs, ordering timestamps,
     the predictions table from Task 1, and the per-call and server tables;
   - correctness: the new tests, the mutants from Task 3, and the gates
     from Task 5;
   - what it does not settle: the gate, and whether size-class shifts cost
     memory elsewhere.

Add to `CHANGELOG.md`:
- under `[Unreleased]` `### Measured`, a bullet for each amendment;
- under `### Changed`, at the top: "**Each runtime string is one leaf GC
  object.** `String` and `Bytes` values made at run time were a scanned
  header over a separate leaf buffer; now the header and bytes share one
  leaf object, built by `alloc_str_object`. Literal headers are leaves too.
  Output is unchanged."

Insert with a one-off script in `/tmp/gcm/_once/`, renamed `.applied` after
it runs. Then commit:

```bash
cd /d/Projects/nona/nova && git add examples/05-json-api/BENCHMARK.md CHANGELOG.md && git commit -F - <<'EOF'
docs(bench): record the allocation mix and one allocation per string

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>
EOF
```

- [ ] **Step 6: Verify the record with a workflow, apply the fixes, and open the PR**

Launch a verification workflow:
- **Checkers:** one per slice — numbers, predictions and ordering, method
  and identity, and interpretation — and a refuter per finding.
- **Allowed scripts:** only the named read-only ones:
  - `/tmp/gcm/oas/analyze.py`, `analyze_mix.py` and `docsweep.py`;
  - `/tmp/gcm/mix/analyze.py`.
- **Forbidden:** globbing `*.py`.

Check `git status` after it returns. Apply the upheld findings as a new
commit. Write `/tmp/gcm/oas/pr.md` in the structure of the reprofile-4 PR's body
(`/tmp/gcm/rp4/pr.md`): a one-paragraph summary; "## Results" bullets
with ranges; "## Method and verification" naming the predictions, the
binary identities, the mutants, the gates, the commit made before
verification, and each workflow pass as checkers returned / findings /
upheld. End it with the Claude Code attribution line. Then push and open
the PR:

```bash
cd /d/Projects/nona/nova && git push -u origin one-alloc-strings && gh pr create --base main --head one-alloc-strings --title "perf(runtime): make each runtime string one leaf GC object" --body-file /tmp/gcm/oas/pr.md
```

Expected: a PR URL. Its body ends with the Claude Code attribution line.
