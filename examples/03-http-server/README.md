# 03-http-server

The smallest `std/http` server: two routes, and a clean exit on SIGTERM.

## What this demonstrates

- **`std/http`'s `Server`:** `Server::new()`, `get(path, handler)` and
  `listen(addr)`, with handlers written as closures.
- **`Response::text` and `Response::json`.**
- **`std/log`:** `Log::init()` and `Log::info(...)`, which write to stderr.
- **Graceful shutdown.** On SIGTERM or SIGINT (Ctrl+Break or Ctrl+C on Windows)
  the server stops accepting, finishes the requests already in flight, and
  exits 0. A second signal ends it at once.

## Run it

```bash
cd examples/03-http-server
nova run
```

Then, from another terminal:

```bash
curl http://localhost:3000/
curl http://localhost:3000/health
```

## Expected output

The server writes one log line to stderr and nothing to stdout. The timestamp
differs from run to run:

```
2026-10-04T10:27:52.126Z INFO listening on :3000
```

The two requests return:

```
Hello from Nova!
{"status":"ok"}
```

## Notes

- **This is not `nova-spec/60-EXAMPLES.md` §3's listing, on purpose.** That
  listing is written in a Nova that does not exist, starting with its first
  line, `import std/http`, which does not parse. It is kept as the aspiration,
  as §5's is. Every substitution this example makes, and the evidence for it,
  is in `docs/superpowers/specs/2026-10-04-examples-03-http-server-design.md`
  §3.
- **The log line is printed before the server is ready.** It comes before
  `listen` binds the port and installs the signal handler. Readiness, for a
  request or for a graceful signal, means the port accepts a connection.
- **`curl http://localhost:3000/` pays about 0.2 s on Windows.** curl tries
  `::1` first, and this server binds IPv4 only (`0.0.0.0`).
  `curl http://127.0.0.1:3000/` does not pay it.
- **Port 3000 must be free.** If it is not, `listen` returns an error and
  `.unwrap()` ends the process with ``nova: panic: called `unwrap` on an `Err`
  value``. The error itself is not printed.
- **Shutdown takes up to 100 ms with nothing in flight.** A request still
  arriving can hold it for up to 10 s, its deadline. Each idle connection
  costs a wake every 100 ms.
- **Windows has no SIGTERM.** Use Ctrl+Break or Ctrl+C in the server's
  console. A plain `taskkill` cannot stop a console program; `taskkill /F`
  ends it at once, without a graceful shutdown.
- **The tests** are in `crates/nova-cli/tests/run_tests.rs`, the
  `http_server_example_*` functions, not in a `tests/` folder here: Nova code
  cannot send a signal to another process.
