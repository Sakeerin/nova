# ADR 0051 — The self-hosted lexer and the differential

## Status

Accepted (2026-10-30). Branch `claude/loving-archimedes-frtf5q-51a`
(`docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
§3). The two parts of 5.1, the Rust fuzzer with fixed seeds, floats dumped
by their bits, and the differential on all three systems are the user's
decisions of 2026-10-30 (the spec's decisions 1, 5 and 7, and its
recommendation on floats); the rest is what the spec left to the plan.

## Context

Phase 5 writes N, the Nova compiler, in Nova, in `selfhost/`
(`docs/phase-5-plan.md`; ADR 0048). Its first module is the lexer. S0's
lexer, `nova-lexer`, is 1,002 code lines of Rust over `logos` 0.14, whose
longest-match rules decide some of its behaviour: an integer that
overflows is an unexpected character, and `r#` with no `"` is the only
invalid number literal. From 5.1a on the lexer exists twice, and nothing
but a test can keep the two equal. The 5.0 spike compared a port with S0
by a dump on each side and a random-input generator, and the generator
found the one bug the corpus could not.

## Decision

1. **`selfhost/` is a Nova package,** `selfhost` 0.1.0, edition 2026, no
   dependencies, with flat modules in `src/`:
   - `token.nova`: the 92 token kinds, named as `Token`'s variants, the
     `Span` and `Spanned` records (byte offsets, no `FileId`), and each
     kind's `description()`, word for word. No dump prints a description:
     an `@test` checks thirteen kinds' against `token.rs`'s, and 5.1b's
     parser messages, which hold them, exercise every one through the
     differential;
   - `lexer.nova`: `lex`, `lex_with_comments`, `Tokens` with its two
     marks, the lexer, the tuple-index split, comments, and `LexError`
     with its `Display`, function for function with `lib.rs`, `tokens.rs`
     and `error.rs`, each function naming the Rust it ports. `logos`
     becomes a hand-written scanner with its rules: the longest match, a
     keyword over an identifier of the same text, and, where no regex
     matches or a callback refuses, `UnexpectedCharacter` of the first
     character, then one character on. Its classes are ASCII but two: the
     char literal's `[^'\\]`, any character but those two, and `.`, any
     but `\n`; and the doc comment's `trim_end`, which drops Unicode's
     White_Space. Keywords are found by first byte and length before any
     comparison, since a string literal allocates each time it is
     evaluated;
   - `dump.nova`: the token dump (decision 3);
   - `main.nova`: the command line (decision 2);
   - `lexer_tests.nova`: N's own `@test`s of the lexer's edges, each
     against S0's dump of the same source, and of the descriptions. They
     are in `src/`, imported by
     `main.nova`, not in `tests/` as the spec planned: a `tests/` module
     sees only the files beside it and the package's `src/lib.nova`, and
     Nova has no re-exports, so a test there cannot reach `lexer.nova`.

   N is 989 code lines for what the Rust's 1,002 do (`token.nova` and
   `lexer.nova`), and 176 for the dump, whose 92-arm kind name the Rust
   takes from `Debug`.
2. **The command line is `selfhost tokens FILE…`.** For each file, in
   order, a header line `== FILE`, the path as given, then the file's
   dump. Lex errors are part of the dump and leave the exit code 0. A file
   that cannot be read gives `== FILE` then `IOERR KIND`, and the rest
   are dumped before the exit code 2. No file at all is refused with a
   usage line and the exit code 2, as S0's clap refuses it. `main` is an
   `async fn`, since `std/fs` reads only so.
3. **S0 prints the same dump, by a hidden flag: `nova parse
   --dump-tokens FILE…`** (`hide = true`; no stability promise).
   `nova_lexer::dump_tokens(source)` writes one file's dump, next to the
   lexer it describes; the command adds the headers and `IOERR`. Without
   the flag, `nova parse` takes one file as before: the further files are
   a hidden list, `more`, that only the flag reads, taken unchecked, and a
   second file without it, an empty one too, is refused with clap's own
   "unexpected argument" error, in the words clap gave before the list
   existed and built as clap builds it, so a terminal shows the same
   colours. Its usage line names the command as clap does: the file name
   of the first argument, then `parse`, so a renamed `nova` names itself;
   or `parse` alone when that argument has no file name in UTF-8 (`""`,
   `/`, `..`, bytes that are not UTF-8). `--help` is unchanged.
   The flag conflicts with `--ast` and `--check`. Per file, every line
   ending in `\n`:
   - **a line per token, `Eof` last:** `start end Kind[ payload][ NL][ PD]`.
     `start` and `end` are the span's byte offsets in decimal. `Kind` is
     the Rust `Token` variant's name. The payloads:
     - `Int`: the value in decimal;
     - `Float`: the 16 lowercase hex digits of `f64::to_bits`
       (`Float.to_bits` in N), so no decimal rendering has to agree;
     - `Char`: the code point in decimal;
     - `StrPart`, `RawStr`, `Ident`, `DocComment`: the text in double
       quotes, each byte below 0x20, 0x7F, `"` and `\` written `\xHH`
       with two lowercase hex digits, every other byte, so every
       non-ASCII character, as it is;
     - no other kind has one.

     `NL` follows when `Tokens::line_break_before` holds, then `PD` when
     `Tokens::plain_decimal` does;
   - **a line per lex error, in the order found:** `ERR start end
     message`, the message being the error's `Display`, with each byte
     below 0x20 and 0x7F written `\xHH`, so that an escape of a line
     break stays on its line; `"` and `\` stay as they are, since the
     message is not quoted;
   - **a line per comment, in source order:** `COMMENT start end Line` or
     `COMMENT start end Block`;
   - **`IOERR KIND`,** in place of all three for a file that cannot be
     read: the kind as `std/io`'s `IoErrorKind` names it, `NotFound`,
     `PermissionDenied`, `AlreadyExists`, `InvalidData`, `Interrupted`,
     `TimedOut`, `ConnectionRefused` or `Other`. S0 maps Rust's
     `io::ErrorKind` to those words, every other kind to `Other`, as N's
     runtime does.

   The errors and comments come from `lex_with_comments`, whose tokens and
   marks are `lex`'s by construction: both run `tokenize` and
   `Tokens::marked`.
4. **The differential is `crates/nova-cli/tests/selfhost.rs`,** with
   `tests/selfhost/compare.rs` and `tests/selfhost/fuzz.rs`, a `cargo
   test` on Linux, macOS and Windows:
   - N is built once per test binary, through a `OnceLock`, by `nova
     build -o` into a fixed directory under the system temp directory,
     from `selfhost/` in place: `-o` writes nothing into the package, and
     neither does `nova test`. No `clang` is needed. The `OnceLock` holds
     the build's result, so a failed build is made once and reported by
     every test that needs N.
   - Every process a test starts, `nova build`, `nova test` or one side
     on one batch, has two minutes. Its stdout and stderr are read on
     threads of their own, so a full pipe cannot stall it; past the limit
     it is killed, and the test fails naming the batch and its first file,
     so a lexer that loops forever cannot hold CI for hours. After a
     process exits, its pipes are read to their end within what is left of
     the two minutes: a process it started that still holds them is
     reported, not waited for. A process that cannot be waited for is
     killed before it is reported. `Child::kill` reaches the process
     alone, not one it started: an N `@test` that looped would leave its
     test process running, orphaned, when `nova test` is killed, though
     that process holds none of the harness's pipes, since `nova test`
     reads its output itself. Each takes a second or two on Linux; CI's
     Windows runners are several times slower.
   - **The corpus** is every `.nova` file under the repository, walked
     from the source tree, not listed by `git`, so an untracked file is
     lexed too, skipping `target/`, `node_modules/`, directories whose
     names begin with `.`, symbolic links, `selfhost/` and the fixed cases,
     sorted; then N's own sources. 300 files at 5.1a, the two that do not
     parse among them. The corpus runs a second time as a Windows checkout
     writes it, each `\n` as `\r\n`, on every system.
   - Files go 50 to a process through S0 and N, and the two outputs must be
     identical byte for byte, compared as bytes, so output that is not
     UTF-8 differs where its bytes do, with the same exit code. On a
     difference the
     test names the file, the line of its dump and of its source, both
     lines, and the two commands that reproduce it; where one side's
     output ends before a file's header, it names that file; where the
     two lines differ only in a final line break, the one without it is
     marked `(no line break)`. Every report
     gives both exit codes and the start of each side's stderr, so a side
     that crashed or printed nothing says so rather than showing a
     confusing difference.
   - **The fixed cases** in `tests/selfhost/cases/` run the same way:
     the spike's one difference, `0_81.5`, and eleven probes that agreed,
     kept so the edges stay pinned: numbers, tuple indexes, nested holes,
     raw strings, chars, punctuation, keywords, `$` and braces, slashes and
     an open block comment. They are `.nova` files, and `nova-fmt`'s corpus
     gate skips their directory, since many are not Nova on purpose.
   - **The edge inputs,** in `fuzz.rs`, are the cases no file there can
     hold as it is: a lone `\r` or `\r\n` in every kind of token and
     ending a doc or line comment at the end of the file, NUL inside and
     outside tokens, an empty file, a byte order mark, a file without a
     final line break, an opening left at the last byte, every escape, the
     Unicode escapes at and past their limits (seventeen hex digits among
     them, which wrap an `Int`), and a doc comment ending in each
     White_Space character, but `\n`, which ends it, and `\r`, above, and
     in five that are not: U+180E, U+200B, U+FEFF, 0x7F and 0x01. A
     Windows checkout
     would rewrite such a file's line endings, and the repository's text
     holds no line ending in `\r`, no NUL and no backslash-u escape.
   - Other tests: the flag is absent from `nova parse --help` and from the
     error for a second file, an empty one included, and conflicts with
     `--ast` and `--check`; S0's dump has a header per file and `IOERR` for
     a missing one, the rest dumped, exit code 2; unreadable files, a
     missing one, one that is not UTF-8 and an empty path after the first,
     are reported alike; the report names the file and line, run on a
     deliberately wrong dump, and a side that fails is reported with its
     exit code and stderr; both sides refuse no file; a process past its
     limit is killed and its batch named, and a pipe still open at the
     limit is reported; a failed build is made once, and N is one however
     often asked; a missing final line break is marked; a seed's mutations
     are the same from a Windows checkout; a renamed `nova` names itself in
     the second file's error, and on Unix a first argument with no file
     name gives `parse` alone; `nova-lexer`'s tests hold one dump with
     every field, a float's bits among them in lowercase hex; and
     `nova test` in `selfhost/` passes as many tests as N's sources hold,
     so a run of none fails.
5. **The fuzzer is Rust, in the test, with a splitmix64 PRNG** and no new
   crate:
   - **soup:** the spike's atoms, with `\r` and `\r\n` alone and inside
     strings, raw strings, chars, block comments and ending doc and line
     comments, NUL, overflowing numbers in every radix, non-ASCII text and
     Unicode spaces, tuple indexes, raw strings, nested holes and bad
     escapes, and once in five an unterminated tail (a hole, a string, a
     raw string, a block comment, `r#`, a doc or line comment ending in
     `\r`). The test asserts that all seven lex errors occur among the
     fixed seeds' inputs;
   - **mutated corpus files:** a corpus file is lexed by `nova-lexer`,
     already a dependency of `nova-cli`, into its tokens' spans; one to
     eight edits delete, duplicate, swap or insert whole tokens, then up
     to three ASCII bytes are replaced by bytes that open or close
     tokens. The files are picked from their sorted list and read with
     each `\r\n` as `\n`, so a seed gives the same inputs on every
     system, a Windows checkout included; the soup's need no files at
     all. An untracked local `.nova` file joins the corpus, and changes
     the mutations;
   - **deep nesting** is 5.1b's, and plugs in beside them: the harness
     compares any inputs through `agree_on_inputs`, and any pair of dumps
     through a `Dump` (`TOKENS` now; the AST dump in 5.1b).

   CI runs soup seeds 1, 2 and 3, 2,000 inputs each, and mutation seeds
   101, 102 and 103, 250 inputs each: the whole file takes about 10 s on a
   4-core Linux machine, building N included. `NOVA_SELFHOST_FUZZ=SEED:COUNT`
   runs COUNT more of each from SEED, locally.
6. **A change to the tokens changes both compilers in one pull request**
   (phase plan decision 6): `nova-lexer`, `dump.rs`, and `selfhost/`'s
   `token.nova`, `lexer.nova` and `dump.nova`, with the differential
   green. The dump changes with the tokens and promises nothing else.
7. **`nova fmt --check` covers `selfhost/`** in CI's step with `std` and
   `examples`.
8. **Windows and macOS are reasoned from the code, and pinned on Linux
   where a test can.**
   - **Line endings:** GitHub's Windows runners check out with
     `core.autocrlf`, so the corpus there holds `\r\n`. Both lexers treat
     `\r` alike everywhere: whitespace between tokens; a line break for
     `NL`; text in strings and raw strings; dropped by a doc comment's
     `trim_end`; and left out of a line comment's span when it is the last
     byte before the `\n` or the end of the file, one `\r` only. The
     corpus's second run with `\r\n`, the edge inputs and the soup check
     it on every system.
   - **Paths:** the harness passes each file relative to the directory it
     runs both sides in, its parts joined by `/`, and both print the path
     as given: S0 through `Path::display`, N its argument. Neither
     canonicalizes, so no `\\?\` prefix appears on one side only.
   - **The executable:** `nova build -o` writes the path it is given, and
     the harness names it `selfhost` plus `EXE_SUFFIX`, `selfhost.exe` on
     Windows.
   - **Files that cannot be read:** a file that is not UTF-8 is `IOERR
     InvalidData` on both sides (Rust's `read_to_string` and `std/fs`'s);
     a directory is `IOERR Other` on Linux, whose `IsADirectory` neither
     side names, and the same kind on both elsewhere, since both map one
     Rust error. A byte order mark is kept by both reads, and is an
     unexpected character to both lexers.
   - **The stack:** N's lexer is loops, not recursion, so Windows' 1 MB
     main-thread stack does not bound it. S0's is not, as the tests build
     it: `logos`'s generated code for a decimal number recurses once per
     character, so a long one overflows debug S0's stack, at about 4,000
     characters under 1 MB (Consequences). No generator makes such a
     number. The depth limit, and the big-stack thread for S0, are 5.1b's.

## Consequences

- No difference was found while building: the corpus agreed at once, and
  so did 40,000 inputs of a local search (`NOVA_SELFHOST_FUZZ=1000:20000`)
  and the fixed seeds. Nor did the probes before the split, of `\r` in
  every position, files with no text, a byte order mark or no final line
  break, files that cannot be read, and the edges of numbers, tuple
  indexes, escapes, holes, chars and comments; the interesting ones became
  the fixed cases and the edge inputs. A later check's own search, about
  50,000 inputs, found none either.
- An empty first path is refused by S0's clap and read as `IOERR NotFound`
  by N; the harness never passes one. An empty path after the first is
  `IOERR NotFound` on both, as `unreadable_files_are_reported_alike` checks.
- A file name that is not UTF-8 differs too: S0 reads the file and dumps
  it, under a header that shows the name lossily, while N, whose `args()`
  are strings, gets the lossy name and prints `IOERR NotFound`. The
  harness's names are its own, and UTF-8.
- **A known S0 bug, S0's stack:** `logos`'s generated code for a decimal
  number, digits, digits with `_`, or digits before `.5`, recurses once
  per character, so debug S0, as the tests build it, overflows its stack
  on a long one and aborts (`thread 'main' has overflowed its stack`,
  exit code 134 on Linux). Under `ulimit -s 1024`, Windows' main-thread
  size, a decimal literal of about 4,000 characters aborts it; under
  Linux's 8 MB it survives 16,000 and aborts at 32,000. N lexes them all.
  Hexadecimal, binary and octal numbers, identifiers, strings and
  comments of 40,000 characters are fine on S0 under 1 MB. No generator
  makes such a number and no fixed case holds one: 5.1b's big-stack
  thread for S0's pipeline (spec §4.4), which `nova parse` and so its
  `--dump-tokens` will run on, will cover it.
- **Speed, measured with the debug runtime,** as the tests build N (not
  5.1b's measurement with an optimized runtime): both lexers are
  quadratic on a run of decimal digits that overflows, since `logos`'s
  rule moves one character on and lexes the rest again; N takes 5.7 s on
  16,000 digits, 23 s on 32,000 and 89 s on 64,000, and debug S0 7.3 s on
  16,000. Otherwise both are linear, but N is about 10 to 15 times slower
  than debug S0: 5 MB of `lexer.nova` repeated, 11.8 s against 1.1 s;
  1 MB of `{`, 10.2 s against 0.83 s; 1 MB of bad escapes, 2.2 s against
  0.12 s. The differential's inputs are small, and take about 10 s in
  all.
- `selfhost tokens` with no file printed nothing and exited 0, where S0's
  clap refuses it with the exit code 2; N now refuses it too, with its
  usage line, and the harness checks that both do.
- N's tests run inside N's binary's module graph, so `nova build`
  compiles them into `selfhost` too, unused.
- `IOERR` cannot name a kind `std/io` lacks; such a failure reads `Other`
  on both sides.
- CI time grows by the build of N and about 10 s of comparison per system,
  more on Windows. The counts are the knob.
- 5.1b adds the AST dump, `selfhost ast` and `nova parse --dump-ast`, the
  deep-nesting generator and the error cases (spec §4).

## References

- `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
  §§1–3, 6–9.
- `docs/phase-5-plan.md` §4 (5.1, amended 2026-10-30) and decision 6.
- `nova-spec/11-PARSER.md` §4 and `00-MASTER-SPEC.md`'s Phase 5, and their
  notes of 2026-10-30.
- `crates/nova-lexer/src/{lib,token,tokens,error,dump}.rs`;
  `crates/nova-cli/src/cmd/parse.rs`; `selfhost/`;
  `crates/nova-cli/tests/selfhost.rs` and `tests/selfhost/`.
- ADR 0048, Phase 5's scope and backend; ADR 0050, the std a compiler
  needs.
