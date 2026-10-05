# 04-todo-cli

A todo list kept in `todos.json`, driven by the command line.

## What this demonstrates

- **`std/process`:** `args()` reads the command line, and `exit(1)` reports a
  usage error or a bad file.
- **`std/fs`:** `exists`, `read_to_string` and `write_string`, all `async`.
- **`std/json`:** `parse`, a hand-written `FromJson` for the `Todo` record, and
  JSON text built with `stringify`.
- **Records, `Vec`, and a `match` on a string.**

## Run it

```bash
cd examples/04-todo-cli
nova run -- add buy milk
nova run -- add walk dog
nova run -- list
nova run -- done 1
nova run -- list
```

Or build it once, and pass the same arguments to the executable. `nova build`
names it `main` (`main.exe` on Windows) in the current directory:

```bash
nova build
./main list
```

## Expected output

```
added: 1
added: 2
[ ] 1: buy milk
[ ] 2: walk dog
[x] 1: buy milk
[ ] 2: walk dog
```

`done 1` prints nothing. `todos.json` then holds:

```
[{"id":1,"title":"buy milk","done":true},{"id":2,"title":"walk dog","done":false}]
```

## Notes

- **This is not `nova-spec/60-EXAMPLES.md` §4's listing, on purpose.** That
  listing is written in a Nova that does not exist, starting with its first
  line, `import std/fs`, which does not parse. It is kept as the aspiration, as
  §5's is. Every substitution this example makes, and the evidence for it, is
  in `docs/superpowers/specs/2026-10-05-examples-04-todo-cli-design.md` §3.
- **`todos.json` lives in the working directory,** as compact JSON. The
  repository's `.gitignore` ignores it in this folder.
- **A `todos.json` that does not parse is refused, not overwritten:** every
  command prints `todo: cannot use todos.json: <reason>` on stderr and exits 1.
  Move the file aside to start over.
- **A title is every word after `add`,** so `add buy milk` and
  `add "buy milk"` store the same title.
- **`done` with an unknown id changes nothing,** silently, as the listing does.
  Ids are read through `std/json`'s `parse`, so `7.0` also means 7.
- **The tests** are the `todo_cli_*` functions in
  `crates/nova-cli/tests/run_tests.rs`, not a `tests/` folder here. They run the
  cycle under `nova run` and as a built executable.
