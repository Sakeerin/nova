# ADR 0050 — The std a compiler needs

## Status

Accepted (2026-10-29). Branch `claude/loving-archimedes-frtf5q-50c`
(`docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
§5 and §5.8). The builder over `Vec<String>`, `OrderedMap` beside `Map`,
`Float.to_bits` with a shim, and `run` and `status` as plain blocking
functions, native only, are the user's decisions 6 and 8 of 2026-10-26;
the rest is what spec §13 left to the plan.

## Context

Phase 5's compiler, N, is written in Nova, and before 5.0c std lacked what
a compiler reads and writes with. A lexer read a string a `Char` at a time
through `chars()`, which builds an array, and had no way from a `Char` to
its code or back, nor any parsing of numbers. Text was built with `+`, which
is quadratic in a loop. A `Map`'s order is seeded per process, so a symbol
table walked in order could not give the same output twice, and N's
bootstrap compares its stages' output byte for byte. N's driver runs
`clang` and reads `NOVA_CLANG` and the path of its own executable, and
`std/process` had only `args` and `exit`.

The spec files say otherwise in places: `14-CODEGEN.md` §6.1 builds strings
with `String::with_capacity`, `20-STDLIB.md`'s index promises `spawn` in
`std/process`, and `std/fs`, the model for I/O, is `async`.

## Decision

1. **`StringBuilder` is a std record over `Vec<String>`** (decision 8),
   not `14-CODEGEN.md` §6.1's `String::with_capacity` nor a runtime byte
   buffer. `push` is a `Vec` push; `to_string` copies the parts into an
   array of their count and joins them in one copy through `str_join`, so
   building is linear. There is no counted `str_join`: the extra pointer
   copy costs little, and the `Vec`'s storage is private to
   std/collections. It counts bytes, `byte_len`, and has no `len`.
2. **Byte access goes through a view that copies nothing.** The std-only
   builtin `str_as_bytes` reads a `String` as `Bytes`, a register move,
   since the two share one layout and neither changes; `byte_len`,
   `byte_at` and `byte_slice` are then `Bytes`' builtins, in constant time.
   `byte_at` and `byte_slice` panic rather than return an `Option`, so a
   lexer allocates nothing per byte, and `byte_slice` panics too on a bound
   inside a character. `int_to_char` is the same move the other way, after
   `char_from_int` checks the range in Nova.
3. **`Float.to_bits` is a runtime call,** `float_to_bits`, natively, and a
   shim entry on WASM, through a `DataView`; there is no MIR bit cast, so
   no backend has a new statement, and there is no `from_bits`.
   `parse_float` pairs `str_to_float` with a new `str_is_float`, so a
   refused text is told from a `0.0`, by Rust's grammar on both targets.
   The shim's `str_to_float` now gives `-nan` its sign bit, as Rust's
   parser does natively, since `to_bits` makes the sign visible. The NaN
   is read at each parse from a one-element `Float64Array` made from its
   bits, `0xFFF8000000000000`, not written `-NaN`: once the code is hot,
   V8's optimising tiers replace an expression whose every value is a NaN,
   `-NaN` or even `minus ? negNaN : NaN` with a constant `negNaN`, by the
   positive NaN, and a load from an array is not folded (the review of
   5.0c, `float_from_keeps_the_sign_of_minus_nan_once_hot`,
   `minus_nan_keeps_its_sign_on_wasm_once_hot`).
4. **Arrays have a type head,** `TyHead::Array`, so std writes
   `impl<T> [T]` (`iter`, `slice`). Before 5.0c every impl on an array was
   E0010, "impl blocks are only supported on named types"; now an inherent
   one, `impl<T> [T] { … }` or `impl [Int] { … }`, checks, as on `String`,
   and one that defines `iter` or `slice` again, or `len`, built into every
   array and taken by a call first, is E0074 (the review of 5.0c found a
   program's `len` ignored). A trait impl on an array stays E0010, "a
   trait cannot be implemented for an array" (the user's ruling on the
   first fact-check): an array takes inherent impls alone. The resolver's
   name for an array's method, `Arr_T.slice`, is shown in diagnostics as
   `[T]::slice` (E0016, E0060), from the owning impl's type as written, so
   `Arr_Vec_Int.bump` is `[Vec<Int>]::bump` (the review of 5.0c), and a
   generic type's, `Vec_T.at`, as `Vec::at`; an array in an interpolation
   is E0013 without the advice to implement `Display`.
   E0074 between two inherent impls now labels both. Where one is std's,
   the program's impl is the primary label, "here", and std's is labelled
   "`iter` is also defined by std's impl" ("`len` is also defined by std,
   for every array" for `len`); between two of a program's impls the later
   is the primary label and the earlier "`iter` is also defined by this
   impl". Before, the error was reported at the later impl alone, which
   for a program's impl against std's is std's, so the program's own impl
   appeared nowhere in it.
   `Vec::to_array` and `from_array` copy. `Vec` gains `at` and `last`,
   which panic where `get` and `pop` give `None`, since `get` allocates a
   `Some` per read.
5. **`OrderedMap` is a new type beside `Map`** (decision 8), which does not
   change: three `Vec`s, keys, values and a live flag, one slot per key in
   insertion order, and a `Map<K, Int>` from key to slot, read for lookups
   only, so the order never depends on a seed. `remove` leaves a gap.
   **Only `insert` compacts, before it appends a new key, and only when
   the gaps are at least half the slots.** Half keeps the slots within
   twice the entries plus one after any insertion, the slack `Vec`'s
   doubling allows, so a walk is linear in the entries; each compaction
   walks at most twice the gaps it removes, so `insert` and `remove` stay
   amortized constant time; and since a removal never moves an entry,
   removing while an iterator runs skips nothing else. Same `Hash` and
   `Eq` bounds as `Map`, and on both targets.
6. **`std/process` has `run` and `status` in place of `spawn`, with `env`
   and `current_exe`** (decision 6): plain `fn`s that block the thread,
   executor included, though `std/fs` is `async`, since a driver runs a
   tool and waits; native only, so on WASM, where the module is not
   compiled, naming one is E0092. The program is found as Rust's `Command`
   finds it, `PATH` and `.exe` included, its arguments passed with no
   shell, the environment, less `NOVA_TEST_INDEX` (Consequences), and the
   working directory inherited; `run`'s standard input is empty and both
   outputs are captured whole, read at once so neither pipe stalls the
   child; `status` inherits all three streams, after flushing standard
   output. A death by signal on Unix is `128 + signal`. The record is
   `ProcessOutput`, not `Output`.
7. **The process builtins are on `std/fs`'s wire contract.** Six std-only
   builtins: `process_run(program, args, capture) -> Int`, a status code,
   `0` or an `IoErrorKind`'s, then `process_take_status`,
   `process_take_stdout` and `process_take_stderr`; `process_env(name) ->
   Bool` and `process_current_exe() -> Int`, whose text waits for
   `fs_take_string`; every failure's message waits for
   `fs_last_error_message`, through `fs.rs`'s own `fail`, so a missing
   program is `NotFound` and one that cannot be run `PermissionDenied` by
   the same table. One builtin with a `capture` flag serves both `run` and
   `status`, and the takes reuse `std/fs`'s where a payload is text, so
   there are six names, not ten. The child's output waits in the thread's
   cells as Rust bytes, not GC objects, so nothing is rooted while it
   waits, and the `Bytes` is allocated when taken; a thread's cells, not a
   task's slots, because the wrappers are plain `fn`s with nothing between
   the run and its takes.
8. **`env(name)` refuses an empty name, or one that holds `=` or NUL,**
   without asking the system, which may panic on one. A test sets a
   variable on a built program through its own process: `Command::env` in
   `run_tests.rs`, and the `--release` walk's `RELEASE_GOLDEN_ENV` table,
   as its `RELEASE_GOLDEN_STDIN` gives standard input. The `=` refusal is
   pinned on Linux by a variable whose value holds `=`: with
   `NOVA_PROCESS_TEST_EQ=x=y` set, glibc's `getenv` answers `y` for the
   name `NOVA_PROCESS_TEST_EQ=x`, and macOS's the whole value, so the
   program's `None` is `env`'s own; the empty and NUL refusals change no
   result on Linux, where glibc and Rust refuse those names themselves,
   and stay as the spec asks. Under `nova run` the
   program sees `nova`'s environment, and `current_exe()` is `nova`.
   `current_exe()` of a path that is not Unicode is `InvalidData`, since a
   path with its bad bytes replaced names another file.
9. **std's inherent method wins over a program's trait method of the same
   name, as in Rust, and E0105 warns** (the user's ruling on the first
   fact-check). 5.0c adds inherent methods to `String` (`byte_len`,
   `byte_at`, `byte_slice`, `parse_int`, `parse_int_radix`,
   `parse_float`), `Char` (`to_int`), `Float` (`to_bits`), `Vec` (`at`,
   `last`, `to_array`, and the function `from_array`) and arrays (`iter`,
   `slice`): fourteen. A method call takes an inherent method before a
   trait's, so a program's `impl Size for String { fn byte_len … }`, whose
   `"é".byte_len()` ran the trait's method before 5.0c, now runs std's;
   and a path takes an inherent function before a trait's, so beside a
   program's `impl Mk for Vec<Int> { fn from_array(a: [Int]) -> Vec<Int> …
   }`, `Vec::from_array(a)`, E0011 at the base since `Vec` alone could not
   pick the impl, is std's now. E0105, a
   warning, is reported at each trait impl's method, or at the impl for a
   default it takes, that one of std's inherent methods on an overlapping
   type hides: "std's inherent method `byte_len` on `String` hides
   `Size::byte_len`", with the note that Nova has no `Size::byte_len(x)`
   call, so the trait's method is reached through a function generic over
   `T: Size`, whose `x.byte_len()` is the trait's, or by renaming it; for a
   static function, "std's inherent function `from_array` …", the generic
   function's `T::from_array(…)`. The note writes the trait's method with
   the trait's own arguments, `Top::at(x, …)` for one that takes more than
   `self`, and names the generic function's parameter `T`, or `U` where the
   impl has a `T` of its own (the review of 5.0c). Where only one of the
   two takes `self` they never meet, and E0105 is quiet (the second
   fact-check). It counts every std method a program can call, older ones
   such as `Vec::iter` or `Map::len` and private ones such as
   `OrderedMap`'s `compact`, since a method's privacy is not enforced at a
   call. A program's own inherent method of one of these names,
   `impl String { fn byte_len … }`, compiled before 5.0c and is E0074 now,
   labelled first at the program's impl.
10. **One new diagnostic code, E0105.** E0092 covers `std/process`'s new
   names on WASM; E0010, E0013, E0014, E0060 and E0074 cover the rest. A
   program that
   calls one of the std-only builtins std's surface now wraps by its own
   name, `char_to_int`, `int_to_char`, `str_from_chars`, `str_as_bytes`,
   `str_is_float`, `str_to_float` or `float_to_bits`, gets E0001 with a
   note naming the public form, `c.to_int()` and the like, in place of
   "did you mean", whose nearest name for `char_to_int` was
   `char_from_int`, the conversion the other way. The note is for a
   value or a call alone: the name where a type is wanted is an unknown
   type, with no note (the review of 5.0c).

## Consequences

- **`run` and `status` stop every task** while the child runs. An
  asynchronous process, a `Command` builder with a working directory, an
  environment or standard input, and processes on WASM are out of 5.0
  (spec §11).
- **`run`, `status`, `env` and `current_exe` join every module's scope,**
  and a program's own shadows them silently, as `args` is shadowed; seven
  programs of the repository define a `run` of their own and are
  unchanged.
- **A process test builds its program first,** since under `nova run`
  `current_exe()` is `nova`; the programs that run themselves are tested
  built and through the `--release` walk, not under `nova run`. On Windows
  a running executable is locked, so a test removes its build after its
  children end.
- **A program that runs itself must guard against a child that lost its
  arguments,** which would run the parent's part again without end: the
  prototype's arguments-dropped mutant was a fork bomb. `process_run`
  writes a marker in its temporary directory and stops if one is there;
  its tests give it a fresh directory, then check that a run that finds
  the marker starts nothing.
- **A test that runs its own binary would run itself again,** since under
  `nova test` `current_exe()` is the test binary, which picks its test
  from `NOVA_TEST_INDEX`, and a child inherits the environment. The plan
  saw this for doc tests alone, whose examples in `std/process` therefore
  start no program; the review of 5.0c found it for any `@test`. So `run`
  and `status` remove `NOVA_TEST_INDEX`, the one variable a test binary
  reads to pick what to run, from the child's environment, and a test
  binary without it prints its list of tests and ends
  (`a_test_that_runs_its_own_binary_runs_once`). A program that runs a
  test binary on purpose cannot pick its test through `run`.
- **An `OrderedMap` that only shrinks keeps its slots,** and with them its
  dead keys and values, until it next grows; a key inserted while an
  iterator runs can move the entries under it. `OrderedSet` is not built.
- **A program's inherent `impl<T> [T]` is new and checks, but one that
  defines `iter`, `slice` or `len` is E0074,** std's impl defining the
  first two, and `len` built into every array. As with `impl String`,
  std's own calls of `iter` or `slice` then resolve to the program's, so a
  program whose `slice` takes other arguments also sees errors inside std,
  after the E0074; the E0074 comes first and labels the program's impl.
- **Programs that compiled before 5.0c can change.** A program's own
  inherent method named as one of std's new ones (`impl String { fn
  byte_len … }`, `impl Char { fn to_int … }`, `impl Float { fn to_bits
  … }`, `impl<T> Vec<T> { fn last … }`) is E0074; a program's trait method
  of such a name is hidden by std's at every method call on that type, and
  E0105 says where; a static trait function `from_array` on `Vec<Int>`,
  whose path was E0011 at the base, meets std's at the path, and E0105
  says so too. No program in the repository meets the E0074;
  `tests/runtime/hidden_trait_methods` hides three trait methods on
  purpose, to show E0105's advice at work on every backend.
- **On Windows, a `.bat` or `.cmd` program runs through `cmd.exe`,** whose
  quoting is not the C runtime's; Rust's `Command` refuses arguments it
  cannot pass to one safely, and `run` then gives an `IoError` of kind
  `Other`. Every other program gets its arguments as they are.
- **`STD_ONLY` grows from 90 to 100** over the part.

## References

- `docs/superpowers/specs/2026-10-26-phase-5-0-the-language-and-std-a-compiler-needs-design.md`
  §5, §6, §7, §10 risks 7 and 9, §11, §12 items 18 to 24, and §13.
- `nova-spec/20-STDLIB.md` §1, §2, §12, §17 and §18, `14-CODEGEN.md` §6.1
  and `12-TYPESYSTEM.md` §5.2 and §8, and their notes of 2026-10-29.
- `std/strings/lib.nova`, `std/core/lib.nova`, `std/collections/lib.nova`,
  `std/process/lib.nova`; `crates/nova-runtime/src/process.rs`.
- ADR 0011, I/O error kinds; ADR 0023, program arguments; ADR 0048,
  Phase 5's scope.
