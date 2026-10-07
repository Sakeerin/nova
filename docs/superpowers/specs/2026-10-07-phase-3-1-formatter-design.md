# Phase 3.1, "Formatter": design

- **Date:** 2026-10-07.
- **Status:**
  - The user approved this design on 2026-10-07: the brief, three
    questions, an approach, and five sections, each time with the
    recommended answer.
  - A fresh fact-check of the written spec found 24 problems, all fixed
    here. §13 lists the decisions they led to.
  - This written spec awaits the user's review.
- **Branch:** `phase-3-1-formatter`, cut from `main` at `2bf8006`.
- **Inputs:**
  - `docs/phase-3-plan.md`: decision 6 in §3, the 3.1 entry in §4, and
    risk 3 in §6;
  - `nova-spec/40-TOOLING.md` §2;
  - `nova-spec/10-LEXER.md` §2.4 and §6;
  - the master spec's Phase 3 item 1 (`nova-spec/00-MASTER-SPEC.md:625`).

## 1. What 3.1 delivers

1. The lexer can keep the comments it skips (§3).
2. `///` doc comments parse, and attach to the items, members, fields and
   variants they precede (§4).
3. The crate `nova-fmt` formats Nova source (§5, §6).
4. `nova fmt` formats a project, or named files and directories, with
   `--check` and `--stdin` (§7).
5. `std/` and `examples/` are formatted, and CI keeps them so (§8).
6. ADR 0028 records the formatter's rules and its departures (§10).

**The gate** (the plan's §4, made precise):
- **The corpus test** (§9.4): every `.nova` file in the repository that
  parses must pass three checks. Today that is 167 of the 168 tracked
  files. The checks:
  - formatting it twice changes nothing;
  - the output parses to the same AST (§5.5);
  - every comment survives, in order (§5.5).
- `nova check` accepts a `///` before an item.
- `nova fmt --check` exits 1 on an unformatted file, and 0 once `nova fmt`
  has formatted it.
- CI runs `nova fmt --check std examples` on all three systems, and the
  full suite passes there after §8's reformat.

## 2. The starting point (read on 2026-10-07)

**Comments**
- **Comments are trivia.**
  - `skip_trivia` (`crates/nova-lexer/src/lib.rs:114`) drops whitespace,
    `//` line comments and non-nesting `/* */` block comments, so nothing
    after the lexer sees a comment.
  - `logos` has skip rules of its own (`lib.rs:544-546`), but `skip_trivia`
    runs before every token outside a string (`lib.rs:162`). So every
    comment passes through that one function.
  - No block comment appears in the corpus: every `/*` in a `.nova` file
    sits inside a `//` comment.
- **An unterminated `/*` silently comments out the rest of the file:**
  `skip_trivia` runs to the end (`lib.rs:140-141`).
  `LexError::UnterminatedBlockComment` exists
  (`crates/nova-lexer/src/error.rs:19`), and `10-LEXER.md` §6 lists it,
  but nothing emits it.
- **`///` lexes but does not parse.**
  - The pattern `///[^\n]*` (`lib.rs:731`) makes a `DocComment` token. Its
    text is trimmed at both ends (`lib.rs:534`).
  - Four or more slashes match the same pattern, so `////` is a doc
    comment today too.
  - The parser has no rule for this token. A `///` before an item fails
    with `error[P0001]: expected item (fn, record, type, trait, impl, const,
    import, module, extern), found doc comment`.
  - No tracked `.nova` file contains `///`.

**Parsing**
- **Newlines are invisible to the parser.**
  - Tokens carry no newlines, and nothing in `crates/nova-parser/src/grammar.rs`
    looks at line positions.
  - The grammar alone separates statements.
  - **`;`** is optional after statements and after several declarations
    (`grammar.rs:601`, `:635`, `:658`, `:741`, `:804`, `:843`, `:860`,
    `:888`, `:1210`, `:1237`). No AST field records it:
    `try_parse_stmt` reads it into `has_semi` and discards it. A
    statement's span does end at its `;`.
  - **Match arms** take an optional `,` (`grammar.rs:2013`).
  - **`return` and `break`** take a value unless the next token is `;`,
    `}` or the end of the file (`grammar.rs:1839-1861`).
  - So `a`, with `(b)` on the next line, parses as the call `a(b)`. A
    program holding `let g = f` with `(1)` on the next line, where `f`
    adds 41, printed `42`.
- **Two tokens are easy to glue.**
  - The lexer makes `&&` and `||` single tokens, and the parser uses them
    only as binary operators (`grammar.rs:1327`, `:1345`).
  - A closure starts only with a single `|` (`:1869`). So a closure with
    no parameters must be written `| |`.
  - A reference to a reference must be written `& &x`, or `& &T` as a
    type (`:928-940`).
- **Trailing commas.** Every comma-list site in `grammar.rs` accepts a
  trailing comma except one. By the fact-check's count that is 20 of 21
  sites, and probing 25 list shapes with `nova parse` on `2bf8006` agreed.
  - The exception is the `where` clause: `where T: A, T: B, {` fails with
    ``expected type (in where bound type), found `{` ``.
  - Separately, a record pattern takes nothing after its `..`:
    `P { x, .., }` fails with ``expected `}` (in record pattern), found
    `,` `` (`:2367-2388`).

**The AST** (`crates/nova-ast`)
- **It forgets spellings and layout.**
  - **Literals:** `Literal` keeps values, not spellings: `Int(i64)`,
    `Float(f64)`, and decoded `Str` and `Char`. So `0xFF`, `1_000` and
    `1e3` would print back differently.
  - **Strings:** every string expression is an `Expr::StringInterp`, with
    or without `${…}` holes (`grammar.rs:1705`, `:1901-1946`). Raw strings,
    which appear only in expressions (`:1706-1710`), and string patterns
    become `Literal::Str`.
  - **Parentheses:** there is no parenthesis node. A parenthesised
    expression or type is its inner node, with its span widened over the
    parentheses (`grammar.rs:1713-1733`, `:976-985`). A parenthesised
    pattern, and the unit pattern `()`, keep only the `(` token's span
    (`:2239`, `:2243`).
  - **Impl members:** `ImplBlock` keeps its methods, consts and
    associated-type bindings in three separate lists (`item.rs:163-170`),
    so their order relative to each other is lost.
  - **Blank lines and comments** are not recorded.
- **Not every node has a span.**
  - Most nodes are wrapped in `Spanned`. These are not, and have no span
    of their own:
    - `MatchArm`, `FieldInit`, `StringPart::Lit`;
    - `Param`, `WhereBound`, `TypeParam`;
    - `RecordField`, `Variant`, `FieldPat`;
    - `TraitItem`, `FunctionSig`, `ExternItem`;
    - the impl members and `AssocTypeBinding`.
  - Some spans are irregular. `Attribute.span` covers only the `@`
    (`grammar.rs:269-271`). A `fn(T)` type with no `->` gets a unit return
    type spanning the next token, and the type's own span extends over
    that token (`:1014-1027`).
  - `Span` derives `PartialEq`, but `Spanned` derives only `Debug` and
    `Clone` (`crates/nova-diagnostics/src/lib.rs:15`, `:55`). So do the
    AST's nodes. Only `BinOp`, `UnOp`, `AssignOp` and `Visibility` can be
    compared.

**Imports, crates and the CLI**
- **Import order carries no meaning.** `import m` imports all of `m`'s
  public names, and a name bound twice is `E0002` (ADR 0003). Reordering
  imports therefore changes no program. std's modules are glob-imported
  into every module (`crates/nova-resolver/src/lib.rs:1583`), and no
  `.nova` file imports `std::…`.
- **`nova-fmt`** is a one-line stub. Its manifest already lists
  `nova-diagnostics`, `nova-ast`, `nova-lexer`, `nova-parser` and `anyhow`.
- **`insta`** is a workspace dependency (`Cargo.toml:38`), used by the tests
  of three crates: `nova-cli`, `nova-lexer` and `nova-parser`.
- **The CLI** has `parse`, `run`, `build`, `check`, `test`, `new`, `init`
  and `version` (`crates/nova-cli/src/main.rs:28`), plus 3.0's project
  discovery.

**The corpus**
- **168 tracked `.nova` files:**
  - 137 runtime fixtures under `tests/runtime/`;
  - 17 in `std/` (6,100 lines);
  - 6 in `examples/`;
  - 6 parser fixtures under `crates/nova-parser/tests/fixtures/`;
  - 2 under `docs/benchmarks/`.

  A `nova parse` sweep of all 168 on `2bf8006` found one failure:
  `crates/nova-parser/tests/fixtures/async.nova`, which uses a `::<User>`
  turbofish (3:24). No test reads that fixture.
- **The style of `std/` and `examples/` today:**
  - 447 lines hold a `{ … }` group on one line (blocks, record literals,
    record declarations);
  - 65 functions are one-liners, and 55 multi-line functions have a
    one-line body;
  - 65 lines contain `} else {` or `} else if`, and `else` starts a line
    once;
  - 215 lines contain `=>`, and 4 of them end with `,`;
  - no line ends with `(`;
  - 14 lines in `std/` are longer than 100 characters, and none in
    `examples/`.

  Across the whole corpus, no code line ends with `;` (29 comment lines
  do). One code line uses `;` mid-line to separate two statements:
  `std/collections/lib.nova:388`.

**Line endings and text**
- **Line endings are mixed** in this Windows working tree, which uses
  `core.autocrlf=true`.
  - 144 of the 168 `.nova` files are `w/crlf` and 24 are `w/lf`; all are
    `i/lf`.
  - The repo has no `.editorconfig`.
- **A UTF-8 byte-order mark is a lex error** (`L0001: unexpected
  character`).
- **Multi-line string literals exist.** A raw newline inside `"…"` is part
  of the string, and the lexer copies it byte for byte. So in a CRLF file
  the string holds `\r\n`: a two-line `"a…b"` literal's `len()` was 4.

**Line-number citations**
- **No test asserts a line number in `std/` or `examples/`.** Prose does,
  in 14 lines outside dated records:
  - `nova-spec/20-STDLIB.md:200`, `:1798`, `:2158` and `:2382`;
  - `docs/benchmarks/README.md:458`;
  - comments on tests at `crates/nova-cli/tests/run_tests.rs:2475`,
    `:2605-2606` and `:2889`;
  - std's own comments at `std/sync/lib.nova:85`, `:86`, `:145`, `:183`
    and `:184`.
- **Eight of the 14 already point at the wrong line today.** For example,
  `20-STDLIB.md:200` cites `std/core/lib.nova:98` for `Display`, which is
  at `:105`.
- **Dated records** also cite std lines: the CHANGELOG, and ADRs 0014 to
  0017 and 0019.
- **One fixture's history:** comments in
  `tests/runtime/channel_clamps_buffer_below_one.nova:3` and `:5` describe
  std lines as history.

## 3. Comments in the lexer

### 3.1 `lex_with_comments`

`nova_lexer::lex_with_comments(source, file)` returns what `lex` returns,
plus every comment `skip_trivia` passes over, in source order:

```rust
pub struct Comment { pub kind: CommentKind, pub span: Span }
pub enum CommentKind { Line, Block }
```

- A line comment's span runs from `//` to the end of its line, without the
  line break. A block comment's span runs from `/*` through `*/`.
- The text is read from the source through the span.
- Comments inside an interpolation hole (`"${x /* c */}"`) are captured
  too.
- `lex` keeps its signature. Its output changes only as §3.2 and §3.3
  say.
- Both functions return the same tokens. A test checks that on every corpus
  file.

### 3.2 Doc comments

- A doc comment is a line comment that starts with exactly three slashes.
  - Four or more slashes make a plain comment, as in Rust.
  - Today `////` lexes as a doc comment and fails to parse. So a
    `//// ----` separator line becomes legal.
- `Token::DocComment` carries the text after `///`:
  - leading whitespace is kept, so indentation inside a doc's Markdown
    survives for `nova doc` (3.5);
  - trailing whitespace is removed, `\r` included, so formatting, which
    removes trailing whitespace (§5.3), cannot change a doc's AST.

### 3.3 Unterminated block comments

An unterminated `/*` now reports `UnterminatedBlockComment`, the error
`10-LEXER.md` §6 lists, spanning from the `/*` to the end of the file.
Today it silently comments out the rest of the file. A file that relied on
that now fails to lex; no tracked file does.

## 4. The parser and the AST

- **Where doc comments go.** One or more consecutive `///` lines are
  accepted in the prefix of:
  - a top-level or nested item: `fn`, `record`, `type`, `trait`, `impl`,
    `const`, `import`, `module` or `extern`;
  - a trait member (a required or provided method, or an associated type),
    and an impl member (a method, a `const`, or an associated-type
    binding);
  - a record field, a sum-type variant, and a function in an `extern`
    block.

  Only top-level items can also carry `@attributes`, since only
  `try_parse_item` parses them (`grammar.rs:278`). There, docs may come
  before, between or after the attributes. The formatter prints them
  first (§6).
- **What they become.** Each of these AST nodes gains
  `pub docs: Vec<Spanned<String>>`, one entry per `///` line, in order:
  - `Function` and `FunctionSig`;
  - `Record` and `RecordField`;
  - `TypeDecl` and `Variant`;
  - `TraitDecl` and `TraitItem::AssocType`;
  - `ImplBlock` and `AssocTypeBinding`;
  - `ConstDecl`, `Import`, `Module` and `ExternBlock`.

  Required trait methods and extern functions carry their docs on their
  `FunctionSig`.
- **Anywhere else**, a `///` is a parse error: "a doc comment must come
  right before an item, a field or a variant; use `//` for a plain
  comment". That covers a `///` before a `let`, an expression statement, a
  match arm, a parameter or a closing brace, and one at the end of the
  file.
- **Consumers.** The parser builds these structs. Code elsewhere that
  destructures one without `..` gains it.
  `crates/nova-typeck/src/check.rs:779` is the one such place found: it
  matches `TraitItem::AssocType { name, bounds }`. In 3.1 only the
  formatter reads `docs`; hover (3.4) and `nova doc` (3.5) will.
- **Pattern spans.** A parenthesised pattern, and the unit pattern `()`,
  get a span covering their parentheses, as expressions and types already
  do (§2). That lets §5.3 keep the author's pattern parentheses, and lets
  §5.4 place a comment inside them. Today such a pattern's span is its `(`
  alone.
- **`where` clauses** accept a trailing comma, like the other comma lists
  (§2), so that a broken `where` can follow §6's comma rule.

## 5. The formatter

### 5.1 Pipeline and API

```rust
pub fn format(source: &str) -> Result<String, FormatError>;
pub enum FormatError {
    Syntax(Vec<Diagnostic>),               // the input does not lex or parse
    Internal { first_difference: String }, // §5.5's check failed
}
```

1. Convert every `\r\n` in the input to `\n`, and work on the result.
2. `lex_with_comments`, then `parse`. Any lex or parse error returns
   `Syntax`, holding the diagnostics `nova check` would show, and nothing
   is formatted.
3. Walk the AST into a document (§5.2). Take what the AST forgets from the
   source (§5.3), and place the comments (§5.4).
4. Render the document at a width of 100.
5. Check the output against the converted input (§5.5).

**Output:**
- `format` produces `\n` line endings and exactly one final newline. An
  empty or whitespace-only input formats to an empty string.
- `format_file(path)` and `--stdin` convert that output to §7.3's line
  ending. `format_file` is the entry point for `nova fmt`, and for 3.2's
  LSP, which calls the library rather than the command.
- A multi-line string literal therefore keeps its file's line ending
  inside it:
  - a CRLF file's string holds `\r\n` (§2);
  - `format` sees `\n` there;
  - writing the file back as CRLF restores `\r\n`.

  If §7.3 changes a file's line ending, such a string's value changes with
  it. Git's line-ending conversion on checkout already has the same effect.

### 5.2 The document and the renderer

`nova-fmt` implements Wadler's pretty printer itself, in about 250 lines,
with no new dependency.

- **The document:**
  - `Text(s)`;
  - `Line`: a space if its group is flat, otherwise a newline;
  - `SoftLine`: nothing if flat, otherwise a newline;
  - `HardLine`: always a newline, which breaks every group around it;
  - `Nest(4, d)`;
  - `Group(d)`: flat if it fits in the rest of the line, otherwise broken;
  - `IfBreak(broken, flat)`, which prints the trailing comma of a broken
    list.
- **Width:** 100 columns, counted in Unicode scalar values. A line holding
  something that cannot break, such as a long string, may run past 100.
- **Indentation:** 4 spaces. No line ends in whitespace the renderer
  added.
- **Verbatim text that spans lines,** such as a multi-line string literal
  or block comment:
  - only its first line counts when measuring whether a group fits;
  - after it, the renderer continues at the column where its last line
    ends;
  - its inner lines are never re-indented.

### 5.3 What comes from the source

The printer reads these from the source, through spans and the token
stream, because the AST does not keep them (§2):

- **Literals** are printed verbatim. That covers:
  - integers, floats and characters, in expressions and in patterns;
  - raw strings, which appear in expressions only;
  - string patterns;
  - every string expression, holes or not.

  The formatter never reflows inside a string, and never removes
  whitespace from a string's lines.
- **Parentheses.** This rule covers an expression, pattern or type whose
  span begins with `(`, where that `(`'s matching `)`, found in the token
  stream, ends the span. That node is printed inside one pair of
  parentheses, the author's.
  - `(a * b) + c` and `(p).hash()` stay as written, and `((x))` becomes
    `(x)`.
  - With §4's pattern spans, this covers patterns too.
  - A tuple's own parentheses are its syntax, so a parenthesised tuple
    loses its extra pair.
  - No other parentheses are added or removed, so no precedence model is
    needed.
- **Tokens that would merge.** Two tokens are never printed together if
  they would lex as one (§2):
  - a `&` before a `&` keeps its space: `& &x`, `& &T`;
  - an empty closure parameter list prints as `| |`.
- **Impl member order.** Impl members print in source order, recovered
  from their name spans, because the AST keeps them in three lists (§2).
  §5.5 cannot see that order, so tests pin it (§9.3).
- **Positions.** Some nodes have no span, or an irregular one (§2). Their
  position comes from their children's spans and the tokens around them.
  The blank-line rule below and §5.4 use these positions.
- **Blank lines.** One blank line is printed before a list element if the
  source has any there, however many. A list element is a statement,
  field, member, match arm or variant. A blank line before one of its
  leading comments counts the same way.
  - Top-level items are always separated by exactly one blank line.
  - A blank line just after `{` or `(`, or just before `}` or `)`, is
    dropped, as are blank lines at the start of the file.
- **Doc comments** are printed from their spans, minus trailing
  whitespace.

### 5.4 Comments

Each comment is attached to an AST node, the way prettier attaches them.

The enclosing node is the smallest node whose extent, from §5.3's
positions, contains the comment. Its children, in source order, give the
preceding and following nodes. Each comment is then classified by
position:

| The comment is | It becomes, by preference |
|---|---|
| **own-line**: only whitespace before it on its line | a leading comment of the following node; else a trailing comment of the preceding one; else dangling in the enclosing node |
| **end-of-line**: only whitespace after it, up to the line break | a trailing comment of the preceding node; else leading of the following one; else dangling |
| **remaining**: code on both sides, as in `f(a, /* b */ c)` | leading of the following node; else trailing of the preceding one; else dangling |

Printing:
- **Leading line comment:** followed by a line break.
- **Leading block comment:** followed by a space, or by a line break if it
  stood on its own line.
- **A trailing comment keeps the position it had in the source:**
  - one that ended a line goes at the end of the node's last line, after
    any `,` the list prints there, and the line breaks after it;
  - one that stood on its own line goes on its own line after the node;
  - a block comment in mid-line follows the node after a space.

  A group holding a trailing line comment can never be flat.
- **Dangling comment:** printed inside its node's delimiters, each on its
  own line: `{\n    // todo\n}`. At file level, each goes on its own line.
- **A comment inside a verbatim string** belongs to the string.

Comment text is never changed, except that trailing whitespace is removed
from each of its lines. A block comment's inner lines keep their own
indentation.

A comment the attacher cannot place is an internal error, never a silent
drop. §5.5 would catch a dropped comment anyway.

### 5.5 The self-check

On every call, before returning, `format` lexes and parses its own output and
compares it with the converted input (§5.1):

1. The output must lex and parse without errors.
2. Its AST must equal the input's, ignoring spans. Both ASTs are first
   normalised the same way: each run of consecutive `import` items is
   sorted by path, and each `{…}` import list by name (§6).
3. Its comments must equal the input's: the same number, kinds and text,
   ignoring trailing whitespace on each line. They must also come in the
   same order, except inside a run of imports: sorting moves comments with
   their imports, so there the comments are compared regardless of order.

Any difference returns `Internal`, naming the first difference, and
`nova fmt` writes nothing. Idempotence is checked by the tests (§9), not on
every call.

### 5.6 `;` and `,`

Statements are printed without `;`, and match arms without `,`, with two
exceptions. Both exist because the parser ignores newlines (§2).

1. **A statement or arm that begins with** `(`, `[`, `{`, `-`, `*`, `&` or
   `|` would continue the one before it. So a `;` or `,` is printed
   between them. For example, `f()` followed by `(a, b).show()` keeps the
   `;` that made them two statements.
2. **A `return` or `break` with no value** would take the next statement as
   its value. So it keeps its `;` whenever another statement follows it in
   the same block. A match arm ending in one is always the last arm,
   because `,` does not end a `return` or `break` value.

If these rules ever prove incomplete, §5.5 refuses the file.

## 6. The style

These are 40-TOOLING §2.1's rules, filled in where it is silent with what
`std/` and `examples/` already do (§2).

**Layout**
- **Width and indentation:** indent by 4 spaces, and fit lines in 100
  columns. A construct is printed on one line if it fits, and otherwise
  broken in its own way, as described below.
- **Blank lines:** exactly one between top-level items. Elsewhere, the
  author's blank lines, as §5.3 says.
- **Spacing:**
  - one space around binary operators, `=`, `=>` and `->`, and after `:`
    and `,`;
  - no space inside `()` or `[]`;
  - one space inside `{ }` when the braces hold something on one line;
  - `{}` when they are empty.
- **Separators:** no `;` after statements and no `,` between arms, except
  as §5.6 says.

**Blocks**

One rule covers function bodies, `if`, `else`, `while`, `for`, match-arm
bodies, closure bodies and bare blocks. A block holding one statement or
expression, and no comment, stays on one line if the whole construct fits:

```nova
fn len(self) -> Int { self.len }
if b { "true" } else { "false" }
```

Otherwise each statement goes on its own line, indented, with `}` on a
line of its own. `} else {` and `} else if` stay on the closing brace's
line.

**Comma lists**

This rule covers parameters, arguments, generic parameters and arguments,
tuples, arrays, record fields and record literals, variant fields, import
lists, attribute arguments, patterns and `where` bounds.
- On one line: `a, b, c`, with no trailing comma.
- Broken: one element per line, indented, each followed by a comma,
  including the last.
- A record pattern's `..` and a record literal's `..base` come last, with
  no comma after them. The parser rejects a comma after `..` in a pattern
  (§2).

```nova
pub fn connect(
    host: String,
    port: Int,
    timeout: Duration,
) -> Result<Connection, NetError> {
```

**Items**
- **Order:** docs, then attributes one per line, then the item.
- **Records:** `record P { x: Int, y: Int }` if it fits; otherwise one
  field per line.
- **Sum types:** `type Option<T> = | Some(T) | None` if it fits. Otherwise
  each variant goes on its own line after `=`, indented and starting with
  `|`:

  ```nova
  pub type Method =
      | Get
      | Post
  ```
- **Signatures:** when a signature does not fit, its parameter list breaks
  first. If it still does not fit, `where` moves to a line of its own,
  with one bound per line, and the opening `{` goes on its own line after
  it.
- **`impl`, `trait` and `extern` bodies:** one member per line, in source
  order (§5.3), with blank lines as §5.3 says.
- **Imports:**
  - each run of consecutive `import` items is sorted by path, and each
    `{…}` list by name, both in byte order;
  - comments travel with their import;
  - runs are never merged across other items;
  - 40-TOOLING's "std first" applies once packages exist (3.3).

**Expressions and types**
- **Calls:** arguments form a comma list: `f(a, b)` when it fits.
- **Method chains:** `a.b().c()` when it fits. Otherwise each `.name(…)`
  after the first goes on its own line, indented once:

  ```nova
  let app = Router::new()
      .get("/", |_| Response::text(200, "Hello from Nova!"))
      .get("/health", |_| Response::json(status_ok()))
  ```
- **Binary expressions:** one that does not fit breaks before an operator,
  at the lowest precedence first, with the continuation indented once. A
  chain of `&&`, or of `||`, puts one operand per line.
- **Match:** `match x {`, then one arm per line, as
  `pattern if guard => body`. A body that does not fit breaks within
  itself.
- **Closures:** `|a, b| a + b`, and `| | f()` for none. A block body
  follows the block rule.
- **Record literals:** `P { x: 1, y: 2 }` when they fit, with any `..base`
  last; otherwise one field per line.
- **Function types** print no `-> ()`. The parser gives `fn(T)` a unit
  return type either way (§2).
- **Everything else** keeps its usual spacing: unary operators, ranges,
  `?`, `.await`, `as` and indexing. So: `-x`, `!x`, `&mut x`, `a..b`,
  `a..=b`, `x?`, `x.await`, `x as Int`, `a[i]`.
- **One-element tuples** keep their comma: `(a,)`.

## 7. `nova fmt`

### 7.1 Arguments and modes

`nova fmt [PATH]... [--check] [--stdin]`

- **No path, inside a project:** every `.nova` file under the project's
  `src/`. The project is the nearest `nova.toml`, found from the current
  directory by 3.0's discovery. 3.3 will add `tests/`.
- **No path, outside a project:** the current directory's `src/`, if
  `src/main.nova` exists, as 3.0's commands fall back to it. Otherwise an
  error asking for files or directories.
- **Paths** name files or directories.
  - A directory is searched recursively for `*.nova`. The search skips
    `target/`, any directory whose name begins with `.`, and symbolic
    links to directories.
  - Files are formatted in sorted path order.
  - A path that does not exist is an error.

  This extends 40-TOOLING §2.3, which formats a single file: CI needs
  `nova fmt --check std examples`, and neither directory has a
  `nova.toml`.
- **`--check`** writes nothing. It prints `would reformat: <path>` for each
  file that would change.
- **`--stdin`** reads source from standard input and writes it, formatted,
  to standard output. With `--check`, it prints nothing and only sets the
  exit code. It takes no paths.
- **Writing:** a file is written only if its content changes. It is
  written to a temporary file in the same directory, which is then renamed
  over it. An interrupted run therefore never leaves half a file, and an
  unchanged file keeps its modification time.
- On success, `nova fmt` prints nothing.

### 7.2 Exit codes and errors

- **0:** every file was formatted. With `--check`: every file was already
  formatted.
- **1:** `--check` found a file that would change.
- **2:** any error. With `--check`, an error outranks a file that would
  change.

What counts as an error:
- **A file that does not lex or parse,** including one with a byte-order
  mark (§2): its diagnostics are printed as `nova check` prints them, and
  it is left untouched. The other files are still formatted.
- **An `Internal` error:** the message names the file and says it was left
  unchanged.
- **A file that is not UTF-8:** left untouched.

### 7.3 Line endings and the final newline

- **Line endings:**
  - `.editorconfig`'s `end_of_line`, if it sets `lf` or `crlf` for the
    file;
  - otherwise the file's own, taken from its first line break;
  - LF if it has no line break.

  With `--stdin`, the input's own.
- **Final newline:** exactly one, unless `.editorconfig` sets
  `insert_final_newline = false` for the file. Then the file keeps
  whatever it had.
- **`.editorconfig`** is read by `nova-fmt`'s own small reader (about 150
  lines, no new dependency).
  - Files are found by walking up from the file's directory, stopping
    after one with `root = true`.
  - Sections use EditorConfig's globs: `*`, `**`, `?`, `[…]`, `[!…]` and
    `{a,b}`. Numeric ranges (`{1..3}`) are not supported, and a section
    using one never matches.
  - Nearer files override farther ones, and later sections override
    earlier ones. Keys and values are case-insensitive.
  - Only `end_of_line` and `insert_final_newline` are read (40-TOOLING
    §2.4). Every other key, including `indent_style` and `indent_size`, is
    ignored.

## 8. `std/`, `examples/` and CI

- **The reformat:** once `nova fmt` exists, one mechanical commit formats
  `std/` (17 files) and `examples/` (6 files) with it, and changes nothing
  else.
  - The full suite then runs on Windows, in the Linux container, and in
    CI.
  - The parser fixtures, the runtime fixtures and `docs/benchmarks/` are
    never rewritten (the plan's risk 3). The corpus test checks them in
    memory.
- **Line-number citations:** the 14 living ones (§2) are then re-derived
  by content. Each is pointed at the line that now holds what it
  describes. Eight of the 14 are stale even before the reformat, so
  mapping old line numbers to new ones would carry their errors forward.
  Dated records, and the fixture's history, keep theirs.
- **CI:** the Test job, on all three systems, gains a step after the tests:
  `cargo run --locked -q -p nova-cli -- fmt --check std examples`. The
  Windows leg checks whatever line endings its checkout produces: CRLF, if
  the runner's Git keeps Git for Windows' default `core.autocrlf=true`.

## 9. Testing

### 9.1 The lexer

- `lex_with_comments` returns line and block comments with exact spans,
  in order. That includes a comment at the end of a file with no final
  newline, one just before a string, and one inside `${…}`.
- `///` is a `DocComment`, and `////` a plain comment.
- Doc text keeps its leading indentation and loses trailing whitespace and
  `\r`. The lexer's doc-comment snapshot (`lexer_tests__doc_comment.snap`)
  is updated to match.
- An unterminated `/*` reports `UnterminatedBlockComment`.
- `lex` and `lex_with_comments` return identical tokens on every corpus
  file.

### 9.2 The parser

- A `///` in each position §4 lists lands in the right node's `docs`, in
  order. That includes docs between a top-level item's attributes.
- Each disallowed position gives the new error.
- A parenthesised pattern's span, and the unit pattern's, covers its
  parentheses.
- `where T: A, U: B,` parses, with the same AST as without the comma.
- The existing parser tests pass unchanged. The six AST snapshots change
  only by gaining `docs: []` lines, and are accepted again.

### 9.3 The formatter

`crates/nova-fmt/tests/` has a test for each rule in §5 and §6: an input
and its expected output. The test helper also formats the expected output
again, asserts it is unchanged, and asserts the self-check passed. Beyond
the constructs themselves, tests cover:
- width boundaries: a construct exactly 100 columns wide stays flat, and
  at 101 it breaks;
- comments in every kind of list, in each of §5.4's three positions, and
  inside expressions;
- literal spellings, raw strings, and interpolation;
- a multi-line string, in LF and in CRLF input;
- the parentheses rule, including `while (if …) {`, a parenthesised
  pattern, and a parenthesised tuple;
- `& &x`, `& &T` and `| |`;
- impl members of mixed kinds keeping their source order;
- §5.6's separators: one test per token, and a value-less `return` and
  `break` followed by a statement;
- a broken record pattern ending in `..`;
- a function type written with and without `-> ()`;
- import sorting, with and without comments, and blank lines;
- CRLF input, an empty file, and a file holding only comments;
- a syntax error, which returns `Syntax`.

### 9.4 The corpus test

`crates/nova-fmt/tests/corpus.rs` finds every `.nova` file in the
repository, skipping `target/` and dot-directories. For each one that
parses, it checks:

1. formatting the output again changes nothing;
2. the output's AST equals the input's (§5.5);
3. the output's comments equal the input's, in order (§5.5).

`crates/nova-parser/tests/fixtures/async.nova` is named as the only file
expected not to parse. The test fails if any other file fails to parse,
or if fewer than 167 files are checked, so no file can silently drop out
of the gate.

### 9.5 `nova fmt` end to end

These go in a new `crates/nova-cli/tests/fmt.rs`, not in `run_tests.rs`:
- `--check` exits 1 on an unformatted file, and 0 once `nova fmt` has
  formatted it (the gate);
- `--stdin`, alone and with `--check`;
- directories, skipping `target/` and dot-directories;
- no path, inside a project and run from a subdirectory;
- no path, outside a project, with and without `src/main.nova`;
- a file with a syntax error is left byte for byte untouched while the
  others are formatted, and the exit code is 2;
- a file that is already formatted is not rewritten;
- CRLF in gives CRLF out;
- `.editorconfig` forcing `lf`, and `insert_final_newline = false`;
- `nova check` accepts a `///` before an item (the gate).

### 9.6 Mutants

Each of these mutants, applied alone, must make a named test fail:
- the self-check skipped;
- trailing comments dropped;
- the author's parentheses dropped;
- §5.6's first rule never printing a separator;
- §5.6's second rule never keeping a `return` or `break`'s `;`;
- impl members printed grouped by kind;
- import runs merged across other items.

## 10. Records

- **ADR 0028, "The formatter,"** records:
  - §7.1's modes and paths. The master spec's Phase 3 item 1 says "only
    `--check`", which the ADR reads as "no style options".
  - canonical output: layout depends only on the code, its comments and
    its blank lines;
  - keeping the author's parentheses;
  - the doc-comment grammar, `////`, and the unterminated-comment error;
  - §5.6's separators;
  - line endings, including the multi-line-string caveat of §5.1;
  - the self-check on every call.
- **Spec notes:**
  - dated notes in `nova-spec/10-LEXER.md` (§2.4 and §6), covering doc
    comments, `////` and `UnterminatedBlockComment`;
  - dated notes in `nova-spec/11-PARSER.md`, covering doc-comment
    positions, `where`'s trailing comma and pattern spans;
  - dated notes in `nova-spec/40-TOOLING.md` §1.1 and §2.
- **Project docs:** `CHANGELOG.md` `[Unreleased]`, `ARCHITECTURE.md`'s
  `nova-fmt` row, and `docs/phase-3-plan.md`'s 3.1 entry.
- **The line-number citations** (§8).
- **A set-difference sweep,** like 3.0's: tokens for what changed, grepped
  across the repository, minus the files the branch touched.

## 11. Risks

1. **Losing a comment, or changing meaning.**
   - The self-check runs on every call, and the corpus test runs on 167
     files. Both compare the AST and the comments.
   - A file the formatter cannot handle is refused, never damaged.
2. **What the self-check cannot see.**
   - Impl member order and comment placement are not in the AST, so
     §5.5 cannot catch a mistake there.
   - Tests pin member order, and the common comment placements.
   - A comment in the middle of an expression can still land next to a
     different token than the one it was written beside.
3. **Reformatting `std/` touches every program,** because std is compiled
   into each one.
   - §5.5 guarantees its AST is unchanged.
   - The full suite on three systems is the check.
4. **A separator hazard §5.6 missed.**
   - §5.5 refuses the file.
   - The fix is to add the case.
5. **Line endings on Windows.**
   - Keeping each file's own line endings stops a CRLF checkout from
     failing `--check`.
   - CI's Windows leg is the check.

## 12. Not in 3.1

- Style options of any kind, and any `.editorconfig` key beyond the two.
- Formatting inside string literals, or the Markdown inside doc comments.
- Removing redundant parentheses.
- `//!` module docs, which stay plain comments in 3.1; doc tests; and
  anything `nova doc` reads (3.5).
- The LSP's document formatting (3.2), which will call `format_file`.
- Accepting a byte-order mark.
- `tests/` in project mode (3.3).
- Spans for the nodes that lack them (§2). §5.3 derives their positions
  instead.

## 13. Decisions made while writing this spec

The approved sections did not settle these, and each is open to the user's
review. Items 1, 2 and 4 correct what Sections 1, 2 and 5 said. Items 5 to
11 came from the fact-check. The rest add detail.

1. Doc text keeps leading whitespace and loses all trailing whitespace,
   not only `\r` (§3.2), so formatting cannot change a doc's AST.
2. There are 14 living line-number citations, not the four Section 5
   counted (§2). They are re-derived by content after the reformat,
   because eight are already stale (§8).
3. `where` clauses accept a trailing comma (§4). They were the only comma
   list that rejected one, and a broken `where` would otherwise violate §6's
   comma rule.
4. §5.6's token set is Section 2's six plus `{`, because a block statement
   after a path would read as a record literal. It does not include `&&`
   or `||`: no statement or arm can begin with either (§2).
5. A `return` or `break` without a value keeps its `;` when a statement
   follows (§5.6). Without it, `return` followed by `x = 1` would re-parse
   as `return x = 1`.
6. `& &x`, `& &T` and `| |` keep their space (§5.3), because `&&` and `||`
   lex as single tokens.
7. A parenthesised pattern and the unit pattern get spans over their
   parentheses (§4). This is a parser change: without it, the author's
   pattern parentheses would be lost, and the AST check could not notice.
8. A record pattern's `..` and a record literal's `..base` take no comma
   after them (§6). The parser rejects one after `..`.
9. Comments inside a run of imports are compared regardless of order
   (§5.5), because sorting moves them with their imports.
10. Impl members print in source order, recovered from their name spans
    (§5.3), and tests pin it, since the AST cannot.
11. `format` converts `\r\n` to `\n` before it works, and the writer
    converts back (§5.1). A multi-line string therefore keeps its file's
    line ending, unless §7.3 changes the file's.
12. Positions for nodes without spans come from their children and the
    tokens around them (§5.3).
13. Function types print no `-> ()` (§6).
14. Verbatim multi-line text (§5.2), and trailing line comments going after
    a `,` (§5.4).
15. Prettier's own-line, end-of-line and remaining classification for
    comments, and dangling comments (§5.4).
16. A blank line counts the same before a leading comment as before the
    element, and blank lines just inside delimiters are dropped (§5.3).
17. Columns are counted in Unicode scalar values (§5.2).
18. The directory search does not follow symbolic links to directories,
    and processes files in sorted order. A missing path or a non-UTF-8 file
    is an error (§7).
19. `nova fmt` prints nothing on success (§7.1).
20. `.editorconfig`'s numeric ranges are not supported (§7.3).
21. CI's step uses `cargo run` inside the Test job (§8).
22. An empty or whitespace-only input formats to an empty file (§5.1).
