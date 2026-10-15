
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

// The event loop, for a module with an `async fn main` (spec
// docs/superpowers/specs/2026-10-15-phase-4-1-the-language-on-wasm-design.md
// §5): `nova_turn` runs what is due and returns the milliseconds until the
// next deadline, or -1 once no task waits, and a timer calls it again. A
// timer that fires early makes a turn that only says what is left.
function turns() {
  return new Promise((resolve, reject) => {
    const turn = () => {
      let delay;
      try {
        delay = Number(wasm.nova_turn());
      } catch (e) {
        reject(e);
        return;
      }
      if (delay < 0) resolve();
      else setTimeout(turn, Math.min(delay, 2147483647));
    };
    turn();
  });
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
// resolves when the program ends. A panic or a trap ends the program as a
// native build ends: its message on stderr and, under Node, the exit status
// of an abort (134) or of a trap (132).
export default async function init(options = {}) {
  const { instance } = await load(options);
  wasm = instance.exports;
  if (wasm.nova_gc_switches) wasm.nova_gc_switches(BigInt(gcSwitches(options)));
  try {
    wasm.main();
    if (wasm.nova_turn) await turns();
  } catch (e) {
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
      throw e;
    }
    if (isNode) process.exitCode = status;
  } finally {
    flush();
  }
}

// Node resolves symbolic links in this module's URL, so the path it was run
// by is resolved the same way before the two are compared.
if (isNode && process.argv[1]) {
  const { pathToFileURL } = await import("node:url");
  const { realpathSync } = await import("node:fs");
  if (import.meta.url === pathToFileURL(realpathSync(process.argv[1])).href) await init();
}
