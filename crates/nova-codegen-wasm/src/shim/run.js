
async function load(options) {
  const imports = { env };
  if (options.wasm) return WebAssembly.instantiate(options.wasm, imports);
  const url = new URL("app.wasm", import.meta.url);
  if (isNode) {
    const { readFile } = await import("node:fs/promises");
    return WebAssembly.instantiate(await readFile(url), imports);
  }
  return WebAssembly.instantiateStreaming(fetch(url), imports);
}

// The program's turns (spec
// docs/superpowers/specs/2026-10-15-phase-4-1-the-language-on-wasm-design.md
// §5, and docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md
// §5, §7.3). Every call into Nova from JS, `main` included, goes through
// `run`, which counts the calls in progress, and each outermost call's
// result goes to `schedule`: 0 when tasks are still queued (the budget of
// ADR 0040), a positive number of milliseconds to the next deadline, -1 when
// no task waits, or -2 when only waits that JS ends remain. One timer at most
// is pending; a timer that fires early makes a turn that only says what is
// left. `init` resolves at the first -1, or when the program ends.
let depth = 0;
let timer = null;
let due = Infinity;
let ended = false;
let finish = null;

function schedule(result) {
  const delay = Number(result);
  if (delay === -1 && finish !== null) end();
  if (delay < 0) return;
  const at = Date.now() + delay;
  if (timer !== null) {
    if (at >= due) return;
    clearTimeout(timer);
  }
  due = at;
  timer = setTimeout(() => {
    timer = null;
    due = Infinity;
    run(() => wasm.nova_turn());
  }, Math.min(delay, 2147483647));
}

// Call into Nova. A panic or a trap ends the program where it happens; an
// inner call passes it on, so the WASM stack unwinds to the outermost call.
// After the end, nothing calls in.
function run(call) {
  if (ended) return undefined;
  depth += 1;
  let result;
  try {
    result = call();
  } catch (e) {
    depth -= 1;
    if (!ended) fail(e);
    if (depth > 0) throw e;
    return undefined;
  }
  depth -= 1;
  if (depth === 0 && !ended) schedule(result);
  return result;
}

// The end of the program by a panic or a trap, as a native build ends: its
// message on stderr and, under Node, the exit status of an abort (134) or of
// a trap (132). Anything else is the shim's own bug, thrown on.
function fail(e) {
  let status;
  if (e instanceof NovaPanic) {
    status = 134;
  } else if (e instanceof RangeError && /call stack/i.test(e.message)) {
    write(2, "nova: stack overflow\n");
    status = 134;
  } else if (e instanceof WebAssembly.RuntimeError) {
    write(2, `nova: trap: ${e.message}\n`);
    status = 132;
  } else {
    ended = true;
    end();
    throw e;
  }
  ended = true;
  if (timer !== null) clearTimeout(timer);
  timer = null;
  if (isNode) process.exitCode = status;
  end();
}

// Resolve `init`, writing the part of a line still unprinted.
function end() {
  flush();
  if (finish !== null) {
    const f = finish;
    finish = null;
    f();
  }
}

// The collector's switches (spec 4.1 §6.6): under Node, NOVA_GC_STRESS and
// NOVA_GC_DEBUG, set to anything, as natively; else `init`'s `gcStress` and
// `gcDebug`, which also override them.
function gcSwitches(options) {
  const env = isNode ? process.env : {};
  const stress = options.gcStress ?? env.NOVA_GC_STRESS !== undefined;
  const debug = options.gcDebug ?? env.NOVA_GC_DEBUG !== undefined;
  return (stress ? 1 : 0) | (debug ? 2 : 0);
}

// Load app.wasm and run `main`, then its turns, if it has any: the promise
// resolves when nothing is left to run, though listeners, timers and replies
// may call in later, or when the program ends by a panic or a trap.
export default async function init(options = {}) {
  const { instance } = await load(options);
  wasm = instance.exports;
  if (wasm.nova_gc_switches) wasm.nova_gc_switches(BigInt(gcSwitches(options)));
  await new Promise((resolve) => {
    finish = resolve;
    run(() => {
      wasm.main();
      return wasm.nova_turn ? wasm.nova_turn() : -1n;
    });
  });
}

// Node resolves symbolic links in this module's URL, so the path it was run
// by is resolved the same way before the two are compared.
if (isNode && process.argv[1]) {
  const { pathToFileURL } = await import("node:url");
  const { realpathSync } = await import("node:fs");
  if (import.meta.url === pathToFileURL(realpathSync(process.argv[1])).href) await init();
}
