
// std/dom's side of the boundary (spec
// docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md
// §4, §8): the JS objects Nova holds, by handle. Slot 0 is never used, so a
// handle of 0 is "none".
const handles = [undefined];
const freeHandles = [];

// A handle for `obj`, or 0 for null and undefined.
function handle(obj) {
  if (obj === null || obj === undefined) return 0n;
  const id = freeHandles.length > 0 ? freeHandles.pop() : handles.length;
  handles[id] = obj;
  return BigInt(id);
}

// The object behind handle `id`; a slot that holds nothing is a panic.
function object(id) {
  const obj = handles[Number(id)];
  if (obj === undefined) novaPanic(`std/dom: handle ${id} is not live`);
  return obj;
}

// Empty handle `id`'s slot, which only the collector asks for (§4.2).
function release(id) {
  const i = Number(id);
  if (i > 0 && handles[i] !== undefined) {
    handles[i] = undefined;
    freeHandles.push(i);
  }
}

// A panic raised by the shim, as `panic` raises one for Nova.
function novaPanic(message) {
  write(2, `nova: panic: ${message}\n`);
  throw new NovaPanic();
}

// `f` as an `env` function named `name`: a JS exception becomes a Nova panic
// that names the function and quotes the exception (§3.1). A panic, a trap
// or a stack overflow from Nova code it called passes through as it is. If
// the program ended while `f` ran, in a listener a browser called and kept
// the panic of, the Nova code that called `f` unwinds too (§5.3).
function guard(name, f) {
  return (...args) => {
    try {
      const result = f(...args);
      if (ended) throw new NovaPanic();
      return result;
    } catch (e) {
      if (
        e instanceof NovaPanic ||
        e instanceof WebAssembly.RuntimeError ||
        (e instanceof RangeError && /call stack/i.test(e.message))
      ) {
        throw e;
      }
      const kind = e != null && e.name ? e.name : "Error";
      const message = e != null && e.message !== undefined ? e.message : String(e);
      novaPanic(`std/dom: ${name}: ${kind}: ${message}`);
    }
  };
}
