
// A `fetch`, as a token: a handle to the wait it settles (§7.1). Its
// headers come as `name: value` lines. When it settles, with the status and
// the whole body or with the error's message, `nova_wake` queues the tasks
// parked on the token and runs a turn.
function fetchToken(url, method, headers, body) {
  const wait = { done: false };
  const init = { method };
  if (headers !== "") {
    init.headers = {};
    for (const line of headers.split("\n")) {
      const at = line.indexOf(": ");
      if (at > 0) init.headers[line.slice(0, at)] = line.slice(at + 2);
    }
  }
  if (body !== "") init.body = body;
  const token = handle(wait);
  fetch(url, init)
    .then(async (response) => {
      wait.status = response.status;
      wait.ok = response.ok;
      wait.body = await response.text();
    })
    .catch((e) => {
      wait.error = e != null && e.message !== undefined ? String(e.message) : String(e);
    })
    .finally(() => {
      wait.done = true;
      run(() => wasm.nova_wake(token));
    });
  return token;
}
