
// A closure JS calls, entry `id` of std/rt's table (§5.2): only the
// outermost call into Nova runs a turn.
function call(id, arg) {
  run(() => wasm.nova_call(id, arg, depth === 1 ? 1n : 0n));
}

// A JS listener that calls entry `id` with the event's handle; after the
// end of the program, it makes no handle and calls nothing.
function listener(id) {
  return (event) => {
    if (!ended) call(id, handle(event));
  };
}
