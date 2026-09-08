# `std/crypto`: hashes, HMAC and random — design

**Date:** 2026-09-03
**Branch:** `std-crypto-hashes-hmac-random`
**Base:** `e31c997` (main)
**Status:** design, approved in dialogue; no code written

Position 12 of Phase 2's build order, `std/crypto`, is the one module group
this tree has not started. This increment starts it, and covers deliberately
less than `nova-spec/20-STDLIB.md` section 8 declares.

**Every factual claim here was checked against its cited source before this
document was committed**, by agents reading those sources independently. What
that pass changed is recorded in the commit message rather than narrated
below, so that what follows states only what is true.

---

## 1. What ships, and what this declines

**Ships:** SHA-256, SHA-512, HMAC-SHA-256, a constant-time HMAC tag check,
random bytes, and a bounded random integer.

**Declines, each for a stated reason:**

- **AEAD** (`Aead`, `aes_gcm_256`, `chacha20_poly1305`, `encrypt`, `decrypt`).
  This is where the design cost concentrates: opaque key material in a GC'd
  record, nonce-reuse discipline the type system cannot enforce, an
  allocating-versus-in-place decision, and a `CryptoError` whose variants
  section 8 never declares. It earns its own spec.
- **BLAKE3.** Section 8 declares it and names a backing that cannot provide
  it. See section 3.
- **Any streaming or incremental hasher.** Section 8 declares one-shot
  functions only. A caller with more data than fits in one `Bytes` therefore
  has no route, and that is recorded as unstarted rather than left looking
  supported.

---

## 2. Section 8 is unwritable as declared, and this is measured

`nova-spec/20-STDLIB.md` section 8 declares its surface in types this
language does not have. Probed on this tree rather than inferred:

- **`u8` does not name a type.** `fn take(x: [u8]) -> Int` fails with
  `error[E0001]: cannot find type u8`. `[u8]` itself parses — the array
  wrapper is fine — so this is a type-resolution failure, not a parse one,
  and bare `u8` fails identically.
- **A fixed-length array type does not parse.** `type Digest = [Int; 4]`
  fails with `error[P0001]: expected ] (in array type), found ;`. The parser
  has no length branch at all: its array-type production reads the inner type
  and then expects `]`. So `[u8; 32]`, `[u8; 64]` and `[u8; 12]` are
  unwritable twice over, once for the element type and once for the length.
- **`CryptoError` is used and never declared.** Section 8's `Aead`
  constructors return `Result<Aead, CryptoError>` and its `decrypt` returns
  `Result<[u8], CryptoError>`; no declaration of that type or its variants
  appears anywhere in the file, on a whitespace-flattened search.

**The type checker's nullary primitive names are Bool, Bytes, Char, Float,
Int and String** — six, confirmed both from the resolution table and by
probing each name plus several plausible others, all of which fail with
`E0001`. The word "nullary" is doing work: `Future` is a seventh built-in
type name, resolved by its own branch because it takes a type argument, and
this repo's own comment on `RESERVED_TYPE_NAMES` states that `Future` "is
here and is not a primitive". `Byte` does not resolve at all despite a design
doc of that name existing.

This is the same finding shape as `examples/05-json-api`, one module over: a
listing written in a Nova that does not exist. The response is the same —
restate the surface in types that do exist, and amend the record rather than
silently diverge from it.

**What is lost, stated rather than glossed:** a digest becomes `Bytes` with
its length documented instead of typed. Nothing can force a caller to hold
exactly 32 bytes, so a caller who truncates or pads a digest gets no
diagnostic. That is a real reduction in safety against section 8's evident
intent, and it is not recoverable without a fixed-length array type.

---

## 3. The backing: `ring`, and the contradiction it exposes

Section 8 says "Backed by `ring` in nova-runtime" and the build order in
`nova-spec/00-MASTER-SPEC.md` section 3 says position 12 is "wrap `ring` at
runtime".

**`ring` 0.17.14 builds and works on this host.** A throwaway crate outside
the workspace built it and computed a SHA-256 that matched an independent
implementation exactly. **That crate was deleted, so a reader cannot
re-verify this from the tree** — the reproduction is a scratch crate
depending on `ring = "0.17"` calling `ring::digest::digest`, and the
implementer should re-run it rather than take this paragraph's word.

Worth recording because the naive check said otherwise: `cl.exe` and `nasm`
are both absent from `PATH`. That is not decisive, because build scripts on
this target discover MSVC tooling by registry lookup rather than by `PATH` —
`rustc` links successfully here for the same reason — and `find-msvc-tools`
is already in this workspace's lockfile. **An absence on `PATH` is not an
absence of the tool**, and this project has twice recorded acting on that
mistake.

**Its declared `rust-version` is 1.66.0**, under this project's 1.78 floor,
so the MSRV job is unaffected.

**What `ring` 0.17.14 provides**, read from its source: digests SHA-1 (marked
for legacy use only), SHA-256, SHA-384, SHA-512 and SHA-512/256; HMAC over
SHA-1, SHA-256, SHA-384 and SHA-512; `SystemRandom` with the `SecureRandom`
trait; and `hmac::verify` for constant-time tag checking.

**What it does not provide: BLAKE3.** `ring::digest::BLAKE3` does not exist.
So section 8's declaration of `blake3` and its statement that the module is
backed by `ring` cannot both hold. That is the spec's contradiction rather
than this increment's, and the resolution is to ship what the named backing
provides and amend section 8 — the same treatment the gate's two
non-equivalent criteria received.

**The dependency cost, and which quantity to count.** A total transitive
graph is the wrong measure for a dependency argument, because most of a
graph may already be in the tree. Counted as **new `Cargo.lock` entries**,
this workspace already holds `cc`, `cfg-if`, `libc`, `shlex`, `windows-sys`,
`windows-targets`, `find-msvc-tools` and the `windows_*` set.

- **`ring` adds four entries:** `ring`, `untrusted`, `wasi`, and
  `getrandom` 0.2.x — that last because `ring` requires `getrandom` 0.2.10
  while the lockfile carries only 0.3.4, which is semver-incompatible.
- **The declined alternative** — `sha2` plus `hmac` plus `blake3` plus
  `getrandom` — **adds roughly a dozen**, among them `sha2`, `hmac`,
  `blake3`, `digest`, `generic-array`, `typenum`, `block-buffer`,
  `crypto-common`, `cpufeatures`, `arrayvec`, `constant_time_eq` and
  `subtle`. **Re-derive that set rather than quoting this list**: a probe of
  it resolved `cpufeatures` at two versions, and a naive walk of blake3's
  graph includes `arrayref`, which the probe did not.

So `ring` is the smaller addition, by something like a factor of three, on
top of being the backing the spec names in two places and needing one
dependency argument rather than four. The RustCrypto path would additionally
ship a BLAKE3 that this host has no second implementation available to
verify against.

---

## 4. The surface

```nova
pub type CryptoErrorKind =
    | EntropyUnavailable
    | InvalidLength
    | RequestTooLarge
    | InvalidRange

pub record CryptoError {
    pub kind: CryptoErrorKind
    pub message: String
}

// Unkeyed digests over a caller-supplied buffer.
pub fn sha256(data: Bytes) -> Bytes                          // 32 bytes
pub fn sha512(data: Bytes) -> Bytes                          // 64 bytes

// Keyed digest, and a constant-time check of one.
pub fn hmac_sha256(key: Bytes, data: Bytes) -> Bytes         // 32 bytes
pub fn hmac_sha256_verify(key: Bytes, data: Bytes, tag: Bytes) -> Bool

// Entropy. Fallible, for reasons in section 6.
pub fn random_bytes(n: Int) -> Result<Bytes, CryptoError>
pub fn random_int(min: Int, max: Int) -> Result<Int, CryptoError>
```

`CryptoError` and `CryptoErrorKind` mirror `std/http`'s `HttpError` and
`HttpErrorKind` shape, so the module reads like its neighbours. Section 8
uses the name `CryptoError` without declaring it, so this is new rather than
restated.

**The four digest functions return no `Result`.** `ring`'s one-shot digest
and HMAC signing take a byte slice and return a fixed-size output with no
failure path for any input this surface can produce. The bound is the input
being a `Bytes` the caller already holds; a length that could not be
allocated would have failed before reaching here.

**`hmac_sha256_verify` is not in section 8, and is shipped anyway.** A
caller can already compare two `Bytes` exactly: `impl Eq for Bytes` ships in
`std/bytes/lib.nova`, so `.eq()` is available. The hazard is not that
checking a tag is impossible but that the available way leaks:

- The `==` **operator** is unavailable — `a == b` on two `Bytes` fails with
  `error[E0013]: equality operators are not defined for Bytes (operator
  traits arrive later in Phase 1)` — so a caller reaches for `.eq()`.
- **`.eq()` is not constant-time.** It bottoms out in Rust slice equality,
  which returns early when the lengths differ -- leaking the length -- and
  then, because `u8` is `BytewiseEq`, dispatches to the specialised
  `equal_same_length` that calls `memcmp`. (The per-byte short-circuiting
  loop in `core::slice::cmp` is the GENERIC impl, which `u8` does not take.)
  `memcmp` carries no constant-time guarantee.

So the danger is not that checking a tag is awkward. It is that the obvious,
convenient, correct-looking way to check one leaks timing, and nothing in the
surface warns a caller off it. Shipping the verifier gives them a right
answer to reach for instead. Section 8 gets an amendment recording that it
declared a tag producer with no consumer.

**Digest lengths are documented, not enforced.** See section 2.

---

## 5. The intrinsic boundary

**The cost, from ADR 0018 section 3.** Adding one intrinsic touches 12 sites
across five files under a counting rule that ADR states explicitly, of which
7 are compiler-forced. Reaching all 7 requires `--all-targets`, because one
forced site is a description table inside `nova-typeck`'s `#[cfg(test)]`
module and a plain `cargo check --workspace` finds 6 and reports success.

**Three intrinsics — but do not multiply 12 by 3 and plan against 36.** That
figure was measured for adding *one* builtin to a tree that had none of its
neighbours; how much of it recurs per additional intrinsic in the same
increment is not established, and some sites may take several variants in one
edit. The implementer should re-derive the real count with the grep ADR 0018
prescribes rather than trust an arithmetic product, and report what it
actually was.

The reason for three rather than six is that six user-facing functions over
one intrinsic each would be six times the intrinsic seam of the entire
`std/http` increment, which shipped one. The reason for three rather than two
is that fewer would mean overloading "give me n bytes" and "give me an
integer in a range" behind a mode flag, and that grab-bag shape needs a
payoff to justify it.

```rust
pub unsafe extern "C" fn nova_rt_crypto_hash(
    op: i64,
    key: *const NovaStr,
    data: *const NovaStr,
    tag: *const NovaStr,
) -> i64;
// 0 = ok, digest in Slot::Buffer | 1 = tag mismatch

pub unsafe extern "C" fn nova_rt_crypto_random_bytes(n: i64) -> i64;
// 0 = ok, bytes in Slot::Buffer | negative = negated error kind

pub unsafe extern "C" fn nova_rt_crypto_random_int(min: i64, max: i64) -> i64;
// 0 = ok, value read back with decode_count | negative = negated error kind
```

**`nova_rt_crypto_hash` has no error range**, and that is deliberate rather
than an omission: section 6 assigns it no kinds, because none of its four
operations can fail. Its status carries only "ok" and "tag mismatch", and a
negative value from it would be a runtime invariant violation rather than an
error kind to map.

**AMENDED 2026-09-08 (branch `std-crypto-hashes-hmac-random`): this amendment
corrects two claims, not one, and both are enumerated here. The first is
"cannot fail", stated unconditionally and needing a scope.** Three of the
four operations route through a `ring` entry point that ends in an
`.unwrap()` — `ring::digest::digest`, `ring::hmac::Key::new` and
`ring::hmac::sign` each unwrap an `InputTooLongError`. That error needs an
input near 2^61 bytes, so **no input a Nova program can construct reaches
it** and every conclusion this document draws from "cannot fail" still holds.
What does not hold is the unqualified form: the operations cannot fail *for
reachable inputs*, on a bound that is `ring`'s rather than this tree's.
`ring::hmac::verify`, which the tag check uses, is the clean one — it returns
a `Result` and unwraps nothing, which matters because section 5's
constant-time argument rests on exactly that function. **This first
correction governs every other place this document states "cannot fail"**:
section 5's "The four hash operations cannot fail, so their status is never
negative" bullet, its **Panic discipline** paragraph — whose disclosed blind
spot is this module's own indexing and arithmetic, and which should also have
said the scan reads only the module's own source and never the dependency it
calls into — and section 6's "The four hash functions raise no kinds". The
shipped comments in `crates/nova-runtime/src/crypto.rs` carry the scoped
wording; the wording above is left as written and superseded by this marker
rather than edited.

**The second correction is a deletion, and it lands in the first of those
three places.** That bullet also called the asymmetry "pinned by a runtime
test", and no test pins it: the failure is unreachable, so no fixture can
reach it. The phrase is struck rather than scoped, because there is nothing
left to scope it to — it survives quoted in the bullet's own parenthetical,
which records instead what the two goldens actually hold. So the amendment
changed two things and no more: it scoped "cannot fail", and it struck that
phrase. Two of the three places above carry a short parenthetical pointing
back to this marker — section 5's bullet and section 6's "raise no kinds"
sentence; the **Panic discipline** paragraph is governed by this marker alone
and its text was not touched.

**Why each is `unsafe extern "C"`, stated per function rather than
collectively**, since only one of the three takes a pointer:

- `nova_rt_crypto_hash` takes three `*const NovaStr` and dereferences them,
  following `nova_rt_http_parse_request`, which does the same.
- The two random intrinsics take no pointer, and are marked for the reason
  this crate already records on the same-shaped `nova_rt_file_close` and
  `nova_rt_file_read`: "No pointer argument, so no dereference precondition;
  marked `unsafe extern "C"` for uniformity with this crate's other
  JIT-registered symbols."

**No general rule about plain versus `unsafe` holds here**, and none is
asserted: `nova_rt_int_to_str(v: i64)`, `nova_rt_alloc(size: i64)` and
`nova_rt_check_bounds(index: i64, len: i64)` are plain `extern "C"` and take
arguments, so "nullary" is not the criterion; `nova_rt_file_close(fd: i64)`
and `nova_rt_file_read(fd: i64, max: i64)` are `unsafe` and take none, so
"takes a pointer" is not it either.

**Why status-plus-slot rather than a direct return.** Both conventions exist
in this runtime. `std/fs` and `std/net` return a status `Int` and stash the
payload in `Slot::Buffer`, which Nova collects with `fs_take_bytes()`, or
with `decode_count(fs_take_bytes())` for an integer; `decode_count` lives in
`std/io`. `nova_rt_http_parse_request` instead returns a GC-allocated `[Int]`
directly, carrying its status in element 0.

ADR 0019 argues for that structured return on grounds of FFI-crossing count
and leak-freedom — one crossing for the whole request instead of two per
header, and no Rust-side state to release, since Nova has no destructors and
a per-request handle-table entry "would leak at the request rate".
**Allocation count is not among its grounds**: the roughly twenty
allocations per request that ADR mentions are a cost the shipped code still
pays, in the section that discloses it as an open escape hatch.

Status-plus-slot is chosen here because it gives one channel carrying all
three shapes this module needs — digest bytes, a boolean, and an error — and
because the `fs_` prefix on the take functions is already documented in the
source as historical rather than filesystem-specific, so nothing new is
invented. It costs a second FFI crossing per call. No figure for that is
quoted here: the claim being relied on is only that a crossing is far cheaper
than hashing a buffer, which does not need one, and any number would be
carried over from a different call shape.

**`op` selects the operation**, with `key` and `tag` passed as empty `Bytes`
where they do not apply. An empty `Bytes` is representable — `gc_bytes(&[])`
already produces one.

**Two things about this boundary needing care rather than confidence:**

- **The `op` constants live on both sides** and can drift silently. A test
  must pin each constant to the algorithm it selects, not merely that the
  functions run. Swapping the SHA-256 and SHA-512 constants must fail a test.
- **The four hash operations cannot fail, so their status is never
  negative**, and the Nova wrapper has no error branch. That asymmetry with
  the two random intrinsics is documented rather than papered over by
  inventing fallible signatures. *(Scoped by this section's 2026-09-08
  amendment, on two clauses. First: "cannot fail" holds for any input a Nova
  program can construct, on a bound that is `ring`'s. Second: "pinned by a
  runtime test" is struck from this bullet, because no test pins it. What the
  goldens hold was read rather than inferred —
  `tests/runtime/crypto_hashes.stdout` is successful digests and `verify`
  booleans and names no error kind anywhere, while
  `tests/runtime/crypto_random.stdout` names `InvalidLength`,
  `RequestTooLarge` and `InvalidRange`. So the fixtures pin the random side's
  reachable kinds and the hash side's successes; they cannot pin "cannot
  fail", because that failure is unreachable and so no test can reach it.
  Neither golden names `EntropyUnavailable`, and `std/crypto/lib.nova`
  discloses why at the `crypto_error_kind_of` arm. The load-bearing half of
  the sentence survives untouched: the signatures were not made fallible to
  paper the asymmetry over.)*

**Panic discipline.** No panic may cross a generated poll boundary. The
concrete commitment is the one `std/http`'s intrinsic already makes and
enforces: a guard test that scans the module's own source for `unwrap()`,
`.expect(`, `panic!`, `format!` and `RefCell` borrows. That scan is a
source-text check, not a proof of panic-freedom — indexing and arithmetic can
still panic without matching any of those patterns — so the commitment is
"carries the same guard `std/http` carries", not "cannot panic".

**The cap is checked before allocating**, following `MAX_HEAD_BYTES` and
`MAX_HEADER_COUNT` in `crates/nova-runtime/src/http.rs`. 64 KiB is proposed:
keys, nonces and tokens run to tens of bytes, so the cap is generous by
orders of magnitude for every intended use.

**The reason for a cap is platform-dependent.** ADR 0002, titled for a
leaking allocator, is **Superseded** as of 2026-07-23 by a conservative
mark-and-sweep collector in `crates/nova-runtime/src/gc.rs`, under which
unreachable objects are reclaimed — so read that ADR's status, not its title.
Precise stack bounds are implemented on Windows today, and **other platforms
fall back to the original leak-until-exit behaviour** until their
stack-bounds query is added. So an unbounded request is collectible on
Windows and a permanent leak on Linux and macOS, two of the three platforms
CI runs.

---

## 6. Error handling

Each kind maps to exactly one condition, and a `crypto_error_kind_of(status)`
mapper mirrors `std/http`'s `http_error_kind_of`:

| kind | condition | raised by |
|---|---|---|
| `EntropyUnavailable` | the OS entropy source failed | both random functions |
| `InvalidLength` | `n` below zero | `random_bytes` |
| `RequestTooLarge` | `n` above the cap | `random_bytes` |
| `InvalidRange` | `min` above `max` | `random_int` |

`random_bytes(0)` is valid and returns an empty `Bytes`. `random_int(5, 5)`
is valid and returns 5 — a degenerate range needs no randomness and is not an
error. The four hash functions raise no kinds, which is why their intrinsic
has no error range. *(Scoped by section 5's 2026-09-08 amendment: they raise
no kinds for any input a Nova program can construct, on a bound that is
`ring`'s.)*

**The overflow that sent `random_int` to the Rust side.** Computing
`max - min + 1` overflows a 64-bit signed integer for a wide range, and `min`
at the minimum with `max` at the maximum is exactly such a range. Nova's
`Int` wraps silently and has no unsigned counterpart, so the correction would
have to be hand-written in a language that cannot express it naturally. In
Rust it is computed in wider arithmetic with the full-span case handled
explicitly.

---

## 7. Testing

**Vectors are cross-checked against an independent implementation.** SHA-256
of the empty string and of `"abc"`, SHA-512 of `"abc"`, and HMAC-SHA-256 with
a key of twenty `0x0b` bytes over `"Hi There"`. Each was confirmed against
Python's `hashlib`/`hmac` before being written down, giving
`ba7816bf...` for `sha256("abc")` and `b0344c61...` for the HMAC case.

**On provenance, precisely.** Those two are widely published — the first as
FIPS 180-4's worked example, the second as RFC 4231's first test case — and
this design has **not** verified them against those documents, only against a
second implementation. That distinction matters: an independent
implementation agreeing establishes that our output is not self-referential,
which is the property that guards against the
measure-then-record-as-golden failure. It does not establish that the
standards body published these exact bytes. **The fixture should record which
of the two claims it is making**, and the implementer who has the documents
to hand may upgrade the claim.

**One design consequence that changes the code shape.** The rejection branch
of `random_int`'s reduction cannot be reached *on demand* against a live
entropy source, because a live source cannot be asked to produce a
particular draw. So **the reduction is factored into a pure function taking
raw bytes as input**, unit-tested in Rust with crafted draws including one
that falls in the biased tail and must be rejected. Without that split,
"unbiased" would be a claimed but unpinned property.

**The branch is not, however, unreachable from a live source.** For a
64-bit draw and a span S the rejection probability is `(2^64 mod S) / 2^64`,
which the caller's range controls. Computed: a span of `2^62 + 1` rejects
exactly a quarter of draws, and `2^63 + 1` exactly half, so a live-source
test at such a span reaches the branch within a few draws. For the narrow
ranges callers actually use it is unreachable in practice — a span of ten
rejects with probability about 3 in 10^19. So a wide-span live test is worth
adding alongside the crafted-input one, since it exercises the composition
of source and reduction that the pure function alone cannot.

**What the random functions can be asserted to do:** return exactly the
requested length for a range of lengths; return empty for zero; produce
differing output across two successive calls, which is probabilistic but at
32 bytes is astronomically safe and is the test that catches an entropy
source stuck at zero; reject a negative length as `InvalidLength` and an
over-cap length as `RequestTooLarge`; return `min` when `min` equals `max`;
stay within bounds across many draws; reject an inverted range as
`InvalidRange`; and reach the rejection branch at a span near `2^63`.

**What is explicitly not tested.** The constant-time property of
`hmac_sha256_verify` is **inherited from `ring::hmac::verify`, not
demonstrated** — no unit test observes timing reliably. The comment at that
call states it as inherited rather than claiming a test pins it.

**Mutations to run and report, including one expected to survive:**

- Swap the SHA-256 and SHA-512 `op` constants. Vector tests must fail. This
  is what actually pins the constants; that the functions run does not.
- Make `sha256` return SHA-512's digest. Vector tests must fail.
- Make `random_bytes` return a fixed buffer. The differing-output test must
  fail.
- Make the reduction skip its rejection branch. The crafted-input test must
  fail, and so should the wide-span live test.
- Replace `ring::hmac::verify` with `.eq()`, or with a short-circuiting byte
  loop. **The correctness tests are expected to keep passing**, because they
  assert only which tags are accepted, and both replacements accept and
  reject exactly the same tags. Run it and report what actually happened; if
  something does fail, that is more interesting than the prediction. This is
  the honest measure of what this suite covers, and the reason the
  constant-time property is described as inherited.

Report what each mutation actually did, not what was predicted. A mutation
whose failure mode is a hang rather than a failed assertion must be observed
hanging and killed, not recorded as a clean failure.

**Fixtures** follow the existing pattern: `tests/runtime/crypto_*.nova` with
goldens, each fixture path unique per process.

---

## 8. Records to amend

Each is a dated marker beside existing text. Nothing is rewritten — the
convention this project states in its own words in
`docs/adr/0018-std-json-scope-and-build-order.md`, that wording is "left as
written and superseded by this marker rather than edited".

- **`nova-spec/20-STDLIB.md`'s entropy paragraph**, which is the record this
  increment bears on most directly and the easiest to amend wrongly. Read the
  whole passage before writing the marker, because its headline and its later
  lines disagree by design.
  Its **headline** claim is "no runtime function exposes entropy to Nova".
  **That was already falsified before this increment**, and the same passage
  says so further down: "There IS a new route from Nova to entropy, and it is
  stated here rather than denied ... `str_hash` is the route", since
  `("").hash()` recovers the per-process seed through an invertible
  finalizer. So `std/crypto` is **not** the first breach of it, and a marker
  crediting it with that would be a new false claim.
  What this increment does falsify is the passage's present-tense inventory:
  "`random_bytes` and `random_int` in §8 below are unstarted declarations,
  with no `ring` in `Cargo.lock` and no `std/crypto/` directory" — three
  clauses, all three of which change. What it adds beyond the existing route
  is a **deliberate, per-call entropy surface**, as distinct from a seed
  recoverable as a side effect of hashing, and that distinction is the
  amendment's content.
- **`nova-spec/20-STDLIB.md` section 8** — that its listing is written in
  types this language does not have, naming which and what each fails with;
  that `blake3` cannot come from the backing the same section names; that
  `CryptoError` is used without declaration; that it declares a tag producer
  with no consumer, and that a verifier ships anyway because the available
  comparison is not constant-time; and that AEAD remains unstarted.
- **`nova-spec/00-MASTER-SPEC.md` section 3** — that position 12 is now
  partially built, naming what ships and what does not. Cite it **by
  heading**: that section's line numbering shifted twice this week, and it
  now carries a note explaining why a line number is not a durable citation
  there.
- **`nova-spec/20-STDLIB.md`'s Phase 2 status notes** — several state that
  position 12 is unstarted with no `ring` in `Cargo.lock` and no
  `std/crypto/` directory. All three clauses change.
- **`docs/adr/0018-...` and `docs/adr/0019-...`** — both carry present-tense
  claims that `std/crypto` is the one unstarted Phase 2 module group.
- **`CHANGELOG.md`** under `[Unreleased]` — the surface, the dependency and
  its four new lockfile entries, the intrinsic count as actually measured,
  the vectors' provenance and its limit, what is not tested and why, and that
  AEAD and BLAKE3 are unstarted.

**Sweep before committing**, with whitespace-tolerant patterns that normalise
`//`, `///` and `>` gutters. A line-oriented grep misses claims that wrap
across lines; that has happened on this project twice. Fix every artifact the
sweep names, not only the one being edited. The entropy paragraph above was
found by exactly such a sweep after a first pass missed it.

---

## 9. Success criteria

1. `sha256`, `sha512`, `hmac_sha256` match the vectors, with each vector's
   provenance recorded beside it, distinguishing cross-checked from
   document-verified.
2. `hmac_sha256_verify` accepts a correct tag and rejects a wrong one, a
   truncated one and a wrong-length one, and its constant-time property is
   described as inherited from `ring` rather than as tested.
3. The `op` constants are pinned by a test that fails when two are swapped.
4. The reduction's rejection branch is exercised twice: by a crafted input to
   the pure function, and by a live-source draw at a span near `2^63`.
5. `random_bytes` and `random_int` reject every invalid argument with the
   kind section 6 assigns it.
6. The cap is enforced before allocation, with its reasoning recorded at the
   constant, including that the leak it guards against is platform-dependent.
7. The five mutations in section 7 are run and their actual outcomes
   reported, including the one expected to survive.
8. The real intrinsic site count is measured and reported, rather than
   assumed to be a multiple of 12.
9. The suite is green with every `test result:` line summed, clippy
   `--all-targets -- -D warnings` and `cargo fmt --all -- --check` clean, and
   the MSRV job unaffected.
10. Every record in section 8 carries its amendment, and the sweep's findings
    are reported with the patterns used.

---

## 10. Out of scope

AEAD; BLAKE3; streaming or incremental hashing; SHA-384 and SHA-512/256,
which the backing provides but section 8 does not declare; any
key-derivation function; and anything TLS-adjacent.

**The three Phase 2 gate examples are also out of scope, and are not
"unblocked".** Two of the three are recorded in this tree as unwritable as
specified: the 2026-09-02 gate-benchmark design spec measured that
`examples/05-json-api` needs a router whose `Handler` type is `P0001`,
`@derive`, `?`, turbofish, `Map::values()` and a String-to-number
conversion — none of which this language has — and that `03-http-server` is
"milder but still unwritable". `std/http` shipping removed one of their
blockers; it did not remove the rest.

**Nothing here changes the hash seeding `std/collections` already uses.**
That seeding is per-process and derives from Rust's own `RandomState`, which
draws from an OS-backed source; there are separate seeds for string and
integer hashing rather than one, and this module's entropy would add nothing
to either. A note in `nova-spec/20-STDLIB.md` saying `std/collections`
"has no seed to hand a `Hasher`" predates the seeded-mix64 work and is stale.
It sits **inside** the same entropy passage section 8 lists for amendment, so
an amender is going to meet it: the marker should correct the entropy
inventory and may note this clause as separately stale, without claiming this
increment is what falsified it.
