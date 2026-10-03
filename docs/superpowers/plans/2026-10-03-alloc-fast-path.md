# Allocation Fast Path Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

> **Outcome (2026-10-03): only Task 1 landed.** Tasks 2 and 3 were implemented and measured, and
> showed no measurable cut in allocation's cost, so they were set aside. See
> `examples/05-json-api/BENCHMARK.md`, "(alloc-fast-path)".

**Goal:** Cut `gc::alloc`'s per-allocation overhead without changing what it promises.

**Architecture:** `alloc` takes one `HEAP` borrow per allocation. It tests the collection threshold and the stress flag inside that borrow, and calls `collect()` outside it only when over. Small requests skip the `Layout` check. `Pages::take` zeroes the eight smallest classes through constant-size `write_bytes` calls.

**Tech Stack:** Rust 1.95 (MSRV 1.78), `crates/nova-runtime`, the Nova toolchain in this repo, Bun 1.3.0 for the equivalence check.

**Spec:** `docs/superpowers/specs/2026-10-03-alloc-fast-path-design.md`

## Global Constraints

- No change to when a collection happens, what it marks or frees, the page layout, the size classes, the large-object path, or `next_gc`'s growth rule.
- Every slot is still fully zeroed when handed out.
- For a large request, the `Layout` check runs before the threshold test adds `taken` to `alloc_since_gc`.
- Collection still runs before the crossing allocation takes its slot.
- MSRV 1.78: no API newer than Rust 1.78 in `crates/nova-runtime`.
- Gates: `cargo test --locked --workspace`, `cargo clippy --locked --workspace --all-targets --all-features -- -D warnings`, `cargo fmt --all --check`.
- A runtime mutant is judged by JIT tests, never by a `*_build_standalone` test alone: `cargo test -p nova-cli` does not refresh the runtime library a standalone build links.

## Review Focus

1. A thread whose first allocation is the crossing one (`next_gc` already reached) must still collect before allocating. Task 2's test starts from a reset heap with `next_gc` lowered, which covers it.
2. `NOVA_GC_STRESS=1` must still collect before every allocation. The existing `*_under_gc_stress` end-to-end tests cover it; Task 4 runs them.
3. A request of exactly `SMALL_MAX` (2048) bytes takes the small path, and 2049 takes the large path. Task 2 pins both.
4. An undescribable size still aborts with today's diagnostic. Existing `heap_layout_rejects_undescribable_sizes` covers the predicate; the abort path itself is unchanged code.
5. Every one of the 24 classes is zeroed over stale bytes, not only the 64-byte class. Task 1 pins it.

---

### Task 1: Characterization tests on the base

These must PASS on the base commit before any change. They pin behaviour the later tasks must keep.

**Files:**
- Modify: `crates/nova-runtime/src/gc/pages.rs` (tests module, after `a_slot_is_zeroed_when_handed_out_even_over_stale_bytes`)
- Modify: `crates/nova-runtime/src/gc.rs` (tests module, after `heap_layout_describes_ordinary_sizes`)

**Interfaces:**
- Consumes: `Pages::new`, `Pages::alloc_slot(class, scan) -> usize`, `Pages::free_all`, `CLASS_SIZES`, `NUM_CLASSES`; `gc::alloc`, `HEAP`, `reset()`, `heap_layout`, `pages::SMALL_MAX`.
- Produces: tests `every_class_hands_out_a_zeroed_slot_over_stale_bytes`, `the_crossing_allocation_collects_before_taking_its_slot`, `small_sizes_are_always_describable`, `small_max_takes_a_slot_and_one_byte_more_takes_the_large_path`.

- [ ] **Step 1: Write the pages test**

```rust
    /// Every class, not only the 64-byte one: a slot is zeroed when handed
    /// out even when the memory still holds another pattern. `zero_slot`
    /// (Task 3) special-cases the smallest classes, so each needs its own
    /// check.
    #[test]
    fn every_class_hands_out_a_zeroed_slot_over_stale_bytes() {
        for class in 0..NUM_CLASSES {
            let slot = CLASS_SIZES[class];
            let mut p = Pages::new();
            let a = p.alloc_slot(class, true);
            // The next slot is free page memory: every class has at least
            // 32 slots per 64 KiB page.
            unsafe { std::ptr::write_bytes((a + slot) as *mut u8, 0xAB, slot) };
            let b = p.alloc_slot(class, true);
            assert_eq!(b, a + slot, "class {class}: slots come lowest first");
            let bytes = unsafe { std::slice::from_raw_parts(b as *const u8, slot) };
            assert!(
                bytes.iter().all(|&x| x == 0),
                "class {class} ({slot} bytes): a slot was handed out still holding stale bytes"
            );
            p.free_all();
        }
    }
```

- [ ] **Step 2: Write the gc tests**

```rust
    /// The allocation whose slot would reach `next_gc` collects first, then
    /// takes its slot: the count it leaves is its own slot alone. Holds off
    /// Windows too, where `collect()` resets the count and returns.
    #[test]
    fn the_crossing_allocation_collects_before_taking_its_slot() {
        reset();
        HEAP.with(|h| h.borrow_mut().next_gc = 64);
        let count = || HEAP.with(|h| h.borrow().alloc_since_gc);
        alloc(16, false);
        alloc(16, false);
        alloc(16, false);
        assert_eq!(count(), 48, "three 16-byte slots, all below the threshold");
        alloc(16, false);
        assert_eq!(
            count(),
            16,
            "the fourth reaches 64: it must collect, resetting the count, then charge its own slot"
        );
        reset();
    }

    /// `alloc` skips `heap_layout` for a small request, so every small size
    /// must be one `heap_layout` accepts.
    #[test]
    fn small_sizes_are_always_describable() {
        assert!(heap_layout(pages::SMALL_MAX).is_some());
    }

    /// The boundary between the two paths: `SMALL_MAX` bytes takes a page
    /// slot, one byte more takes the large path.
    #[test]
    fn small_max_takes_a_slot_and_one_byte_more_takes_the_large_path() {
        reset();
        let small = alloc(pages::SMALL_MAX, true) as usize;
        let large = alloc(pages::SMALL_MAX + 1, true) as usize;
        HEAP.with(|h| {
            let h = h.borrow();
            assert!(h.pages.slot_scan(small).is_some(), "2048 bytes must take a page slot");
            assert!(h.pages.slot_scan(large).is_none(), "2049 bytes must not take a page slot");
            assert!(h.large.iter().any(|o| o.addr == large), "2049 bytes must take the large path");
        });
        reset();
    }
```

- [ ] **Step 3: Run them on the base and see them pass**

Run: `cargo test --locked -p nova-runtime --lib -- every_class_hands_out the_crossing_allocation small_sizes_are_always small_max_takes_a_slot`
Expected: `test result: ok. 4 passed`

- [ ] **Step 4: Check the tests bite, on the base**

Run each mutant in turn, then restore `git checkout -- crates/nova-runtime/src`:
- In `maybe_collect`, `>=` → `>`. Expected: `the_crossing_allocation_collects_before_taking_its_slot` FAILS.
- In `maybe_collect`, `if over || stress()` → `if false`. Expected: the same test FAILS.
- In `Pages::take`, `write_bytes(addr as *mut u8, 0, page.slot)` → `write_bytes(addr as *mut u8, 0, 8)`. Expected: `every_class_hands_out_a_zeroed_slot_over_stale_bytes` FAILS.

- [ ] **Step 5: Commit**

```bash
git add crates/nova-runtime/src/gc.rs crates/nova-runtime/src/gc/pages.rs
git commit -m "test(gc): pin zeroing in every class, the crossing allocation and the small/large boundary"
```

---

### Task 2: One borrow per allocation

**Files:**
- Modify: `crates/nova-runtime/src/gc.rs` (`Heap`, `Heap::new`, `alloc`, `maybe_collect`, `stress`, the `collect_for_test` doc comment)

**Interfaces:**
- Consumes: Task 1's tests.
- Produces: `Heap::allocate(&mut self, class: Option<usize>, size: usize, taken: usize, scan: bool) -> *mut u8`, `Heap::stress(&mut self) -> bool`, field `Heap::stress: Option<bool>`. `maybe_collect` and the free fn `stress` are removed.

- [ ] **Step 1: Add the field and the two methods**

In `struct Heap`, after `freed_bytes: u64,`:

```rust
    /// Whether `NOVA_GC_STRESS` asks for a collection before every
    /// allocation: `None` until the first allocation reads the environment.
    stress: Option<bool>,
```

In `Heap::new`, after `freed_bytes: 0,`: `stress: None,`

Add to `impl Heap`:

```rust
    /// Whether `NOVA_GC_STRESS` asks for a collection before every
    /// allocation. Read from the environment once per thread, then kept.
    fn stress(&mut self) -> bool {
        *self
            .stress
            .get_or_insert_with(|| std::env::var_os("NOVA_GC_STRESS").is_some())
    }

    /// Allocate a `size`-byte object that costs `taken` bytes, without
    /// testing the collection threshold: [`alloc`] has already decided
    /// whether to collect first. `class` is `Some` for a page slot.
    fn allocate(&mut self, class: Option<usize>, size: usize, taken: usize, scan: bool) -> *mut u8 {
        let p = match class {
            // Zeroed by `alloc_slot`.
            Some(c) => self.pages.alloc_slot(c, scan) as *mut u8,
            None => {
                let layout =
                    heap_layout(size).expect("alloc rejects an undescribable size before allocating");
                // Zeroed so unwritten slots (e.g. skipped unit fields) read as
                // null and are never mistaken for pointers.
                let p = unsafe { alloc_zeroed(layout) };
                if p.is_null() {
                    handle_alloc_error(layout);
                }
                self.large.push(Obj {
                    addr: p as usize,
                    size,
                    scan,
                    marked: false,
                });
                p
            }
        };
        self.alloc_since_gc += taken;
        self.live_bytes += taken;
        #[cfg(test)]
        self.requested.insert(p as usize, size);
        p
    }
```

- [ ] **Step 2: Rewrite `alloc`**

Replace the body after `let size = size.max(8);` with:

```rust
    // A small request is always describable (`small_sizes_are_always_describable`),
    // so only a large one needs `heap_layout`'s check. A small object takes a
    // whole slot of its class, so a slot's full size is what it costs the
    // collection trigger and the live count.
    let (class, taken) = if size <= pages::SMALL_MAX {
        let class = pages::class_of(size);
        (Some(class), pages::CLASS_SIZES[class])
    } else {
        // Reject an undescribable size before doing any work, and before the
        // threshold test below adds `size` to `alloc_since_gc`. ... (keep the
        // existing comment block about why this is an abort, not
        // `handle_alloc_error`, verbatim)
        if heap_layout(size).is_none() {
            eprintln!(
                "nova: panic: allocation of {size} bytes exceeds the maximum object size of {MAX_HEAP_OBJECT} bytes"
            );
            std::process::abort();
        }
        (None, size)
    };
    // One borrow in the common case: test the threshold and allocate inside
    // it. Over the threshold, collect outside the borrow -- `collect` borrows
    // `HEAP` itself -- and then allocate, so the crossing allocation still
    // collects before taking its slot.
    let fast = HEAP.with(|h| {
        let mut h = h.borrow_mut();
        if h.alloc_since_gc + taken >= h.next_gc || h.stress() {
            None
        } else {
            Some(h.allocate(class, size, taken, scan))
        }
    });
    if let Some(p) = fast {
        return p;
    }
    collect();
    HEAP.with(|h| h.borrow_mut().allocate(class, size, taken, scan))
```

The existing comment block about the abort (the paragraph beginning "This is *not* an out-of-memory condition" through "any computed size can land on it.") moves inside the `else` branch unchanged.

- [ ] **Step 3: Delete `maybe_collect` and `fn stress()`**

Delete `fn maybe_collect(incoming: usize) { ... }` and `fn stress() -> bool { ... }` with its `static S: OnceLock<bool>`. Keep `fn debug()`.

- [ ] **Step 4: Update the `collect_for_test` doc comment**

`bypassing [`maybe_collect`]'s allocation-threshold trigger` → `bypassing [`alloc`]'s allocation-threshold trigger`.

- [ ] **Step 5: Run the gc and pages tests**

Run: `cargo test --locked -p nova-runtime --lib gc`
Expected: every test passes, Task 1's four included.

- [ ] **Step 6: Mutants**

Run each, then restore with `git checkout -- crates/nova-runtime/src/gc.rs`:
- `>=` → `>` in `alloc`'s threshold test. Expected: `the_crossing_allocation_collects_before_taking_its_slot` FAILS.
- `|| h.stress()` and the threshold test replaced by `false` (never collect). Expected: the same test FAILS.
- Slow path reordered: `let p = HEAP.with(|h| h.borrow_mut().allocate(class, size, taken, scan)); collect(); p`. Expected: the same test FAILS (the count is reset after the slot is charged).

- [ ] **Step 7: Commit**

```bash
git add crates/nova-runtime/src/gc.rs
git commit -m "perf(gc): one HEAP borrow per allocation, and no Layout check for small sizes"
```

---

### Task 3: Constant-size zeroing for the smallest classes

**Files:**
- Modify: `crates/nova-runtime/src/gc/pages.rs` (`Pages::take`, new `zero_slot`)

**Interfaces:**
- Consumes: Task 1's `every_class_hands_out_a_zeroed_slot_over_stale_bytes`.
- Produces: `unsafe fn zero_slot(addr: usize, slot: usize)`, private to `pages.rs`.

- [ ] **Step 1: Add `zero_slot`**

Above `impl Pages`:

```rust
/// Zero the `slot` bytes at `addr`, with a constant size for the eight
/// smallest classes so each can compile to inline stores rather than a call
/// to `memset`.
///
/// # Safety
/// `addr .. addr + slot` must be writable.
#[inline(always)]
unsafe fn zero_slot(addr: usize, slot: usize) {
    let p = addr as *mut u8;
    match slot {
        16 => std::ptr::write_bytes(p, 0, 16),
        32 => std::ptr::write_bytes(p, 0, 32),
        48 => std::ptr::write_bytes(p, 0, 48),
        64 => std::ptr::write_bytes(p, 0, 64),
        80 => std::ptr::write_bytes(p, 0, 80),
        96 => std::ptr::write_bytes(p, 0, 96),
        112 => std::ptr::write_bytes(p, 0, 112),
        128 => std::ptr::write_bytes(p, 0, 128),
        _ => std::ptr::write_bytes(p, 0, slot),
    }
}
```

- [ ] **Step 2: Call it from `take`**

`unsafe { std::ptr::write_bytes(addr as *mut u8, 0, page.slot) };` → `unsafe { zero_slot(addr, page.slot) };`. Keep the SAFETY comment above it.

- [ ] **Step 3: Run the pages tests**

Run: `cargo test --locked -p nova-runtime --lib pages`
Expected: every test passes.

- [ ] **Step 4: Mutants**

Run each, then restore with `git checkout -- crates/nova-runtime/src/gc/pages.rs`:
- The `16` arm writes 8 bytes. Expected: `every_class_hands_out_a_zeroed_slot_over_stale_bytes` FAILS.
- The `48` arm replaced by `48 => {}`. Expected: the same test FAILS.

- [ ] **Step 5: Commit**

```bash
git add crates/nova-runtime/src/gc/pages.rs
git commit -m "perf(gc): zero the eight smallest slot classes with constant-size writes"
```

---

### Task 4: Gates

- [ ] **Step 1: Run the gates, logged**

```bash
L=/tmp/gcm/af/gates.log; mkdir -p /tmp/gcm/af
{ echo "HEAD $(git rev-parse HEAD)"; cargo fmt --all --check && echo fmt-exit=0;
  cargo clippy --locked --workspace --all-targets --all-features -- -D warnings 2>&1 | tail -2; echo "clippy-exit=${PIPESTATUS[0]}";
  cargo test --locked --workspace > /tmp/gcm/af/suite.log 2>&1; echo "test-exit=$?";
  grep 'test result' /tmp/gcm/af/suite.log | awk '{p+=$4; f+=$6; i+=$8} END {print "totals passed="p, "failed="f, "ignored="i}'; } > $L 2>&1; cat $L
```

Expected: `fmt-exit=0`, `clippy-exit=0`, `test-exit=0`, totals `passed=1177 failed=0 ignored=8` (1173 plus Task 1's four).

- [ ] **Step 2: Confirm the stress tests ran**

Run: `grep -c 'under_gc_stress ... ok' /tmp/gcm/af/suite.log`
Expected: `12`

---

### Task 5: Measurement

Predictions first, in `/tmp/gcm/af/predict.txt`, before any binary is built.

- [ ] **Step 1: Write the per-allocation harness** at `/tmp/gcm/af/bench_alloc.nova` (scratch, never committed):

```nova
// Per-allocation cost before and after the allocation fast path (scratch, never committed).
record P { a: Int, b: Int }

fn bench_records(n: Int) {
    let t = Instant::now()
    let mut i = 0
    let mut sum = 0
    while i < n {
        let p = P { a: i, b: i + 1 }
        sum = sum + p.b - p.a
        i = i + 1
    }
    let d = t.elapsed()
    println("records ns_per_alloc=${d.nanos / n} check=${sum}")
}

fn bench_strings(n: Int) {
    let t = Instant::now()
    let mut i = 0
    let mut total = 0
    while i < n {
        let s = "${i}"
        total = total + s.len()
        i = i + 1
    }
    let d = t.elapsed()
    println("int_strings ns_per_call=${d.nanos / n} chars=${total}")
}

fn main() {
    let n = 2000000
    bench_records(n)
    bench_strings(n)
}
```

- [ ] **Step 2: Build before and after.** Release toolchain from `main` (`cb4a80a`) → `bc_before.exe`, `srv_before.exe`; from the branch head → `bc_after.exe`, `srv_after.exe`. Delete every sibling `NAME` and `NAME.exe` first. Record size and SHA-256 of each in `binaries.txt`.

- [ ] **Step 3: Per allocation.** Alternate `bc_before` and `bc_after`, three runs each, one fresh process per run, into `percall.log`. Expected: `check=2000000` and the same `chars` in every run.

- [ ] **Step 4: Equivalence, then server.** `bun docs/benchmarks/bun-equivalence.js <srv_after.exe>` must print `EQUIVALENCE OK`. Then `run_srv.sh` (ten users, `--warmup 5 --duration 15`) alternated before and after, three pairs, three more if the ranges overlap, into `server.log`.

- [ ] **Step 5: Profile the branch.** Apply `scratchpad/sampler-prof.patch`, build the json-api with the branch's release toolchain, run "(reprofile-2)"'s three sampled runs and `shares.py` and `gcsplit.py`. Revert the patch, rebuild, and confirm a plain json-api with no map and no `NOVA_PROF_SAMPLE` string.

---

### Task 6: Record and PR

- [ ] **Step 1:** A dated amendment `## AMENDMENT 2026-10-03 (alloc-fast-path)` in `examples/05-json-api/BENCHMARK.md`, before `## What was measured, and with what`. It covers predictions with verdicts, the per-call and server tables in run order, the profile's four parts against "(reprofile-2)", correctness (Task 1's tests, the mutants, the gates) and what is not settled. If `memset`'s share did not fall, say so and revert Task 3 as the spec requires.

- [ ] **Step 2:** CHANGELOG `[Unreleased]`: a Measured bullet and a Changed bullet.

- [ ] **Step 3:** An independent read-only verifier checks the code commits and the record against the raw logs. Apply its fixes.

- [ ] **Step 4:** Commit the record, push the branch, open the PR, and stop for the merge instruction.
