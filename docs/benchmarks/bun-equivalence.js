// Gates the measurement, not the build. `docs/benchmarks/README.md` says why
// there is no automated test: CI runners have no Bun, and an `#[ignore]`d
// test would fail on every push inside CI's advisory step, whose failures
// are tolerated and unread.
//
// Usage: bun docs/benchmarks/bun-equivalence.js <path-to-nova-example-binary>
//
// Drives the nine exchanges `crates/nova-cli/tests/run_tests.rs` already
// pins, in order, against each server in its own fresh process, and compares
// STATUS CODE and RESPONSE BODY BYTES. Order matters: the store accumulates,
// and the last exchange is the only one that drives the list loop body.

const novaBinary = process.argv[2];
if (!novaBinary) {
  console.error("usage: bun bun-equivalence.js <path-to-nova-example-binary>");
  process.exit(2);
}

const EXCHANGES = [
  { name: "1 GET /users empty", method: "GET", path: "/users", body: null },
  {
    name: "2 POST /users",
    method: "POST",
    path: "/users",
    body: '{"name":"ada","email":"a@example.com"}',
  },
  { name: "3 GET /users/1", method: "GET", path: "/users/1", body: null },
  { name: "4 GET /users/zz", method: "GET", path: "/users/zz", body: null },
  { name: "5 GET /users/99", method: "GET", path: "/users/99", body: null },
  { name: "6 GET /nope", method: "GET", path: "/nope", body: null },
  { name: "7 GET /nope/1", method: "GET", path: "/nope/1", body: null },
  {
    name: "8 POST /users escaped",
    method: "POST",
    path: "/users",
    body: '{"name":"a\\"b\\\\c","email":"q@example.com"}',
  },
  { name: "9 GET /users listed", method: "GET", path: "/users", body: null },
];

async function portOf(proc) {
  const reader = proc.stdout.getReader();
  const dec = new TextDecoder();
  let buf = "";
  const deadline = Date.now() + 20000;
  while (Date.now() < deadline) {
    const { value, done } = await reader.read();
    if (done) break;
    buf += dec.decode(value, { stream: true });
    const m = buf.match(/127\.0\.0\.1:(\d+)/);
    if (m) {
      reader.releaseLock();
      return Number(m[1]);
    }
  }
  throw new Error(`no port line; saw: ${JSON.stringify(buf)}`);
}

async function runAll(spawnArgs, label) {
  const proc = Bun.spawn(spawnArgs, { stdout: "pipe", stderr: "pipe" });
  let port;
  try {
    port = await portOf(proc);
  } catch (e) {
    proc.kill();
    throw new Error(`${label}: ${e.message}`);
  }
  const results = [];
  for (const x of EXCHANGES) {
    const init = { method: x.method };
    if (x.body !== null) {
      init.body = x.body;
      init.headers = { "content-type": "application/json" };
    }
    const res = await fetch(`http://127.0.0.1:${port}${x.path}`, init);
    const bytes = new Uint8Array(await res.arrayBuffer());
    results.push({ status: res.status, bytes });
  }
  proc.kill();
  await proc.exited;
  return results;
}

function sameBytes(a, b) {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) return false;
  return true;
}

const dec = new TextDecoder();
const nova = await runAll([novaBinary], "nova");
const bun = await runAll(["bun", "docs/benchmarks/bun-server.js"], "bun");

let differences = 0;
for (let i = 0; i < EXCHANGES.length; i++) {
  const n = nova[i];
  const b = bun[i];
  const ok = n.status === b.status && sameBytes(n.bytes, b.bytes);
  if (!ok) {
    differences++;
    console.log(`DIFFER  ${EXCHANGES[i].name}`);
    console.log(`  nova  ${n.status}  ${n.bytes.length}B  ${dec.decode(n.bytes)}`);
    console.log(`  bun   ${b.status}  ${b.bytes.length}B  ${dec.decode(b.bytes)}`);
  } else {
    console.log(`match   ${EXCHANGES[i].name}  ${n.status}  ${n.bytes.length}B`);
  }
}

if (differences > 0) {
  console.log(`EQUIVALENCE FAILED: ${differences} of ${EXCHANGES.length} exchanges differ`);
  process.exit(1);
}
console.log(`EQUIVALENCE OK: all ${EXCHANGES.length} exchanges match on status and body bytes`);
