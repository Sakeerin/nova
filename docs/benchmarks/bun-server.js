// An equivalent of `examples/05-json-api`, for the ratio
// `nova-spec/60-EXAMPLES.md` section 5 asks for.
//
// `Bun.serve` with a hand-written path match and NO router library, because
// Nova's side is a raw `std/http` accept loop -- `std/http` has no router,
// `pub type Handler = async fn(Request) -> Response` being P0001. Putting a
// routing framework here would measure Nova against that framework.
//
// Response BODIES are byte-identical to the example's; `docs/benchmarks/
// bun-equivalence.js` is what checks that, over the nine exchanges the
// example's golden test drives. Wire framing is NOT identical: `Bun.serve`
// adds a `Date` header the example does not, measured at 37 bytes, and that
// difference is recorded beside the ratio rather than removed.

const store = { users: new Map(), nextId: 1 };

// `{"id":N,"name":...,"email":...}` with no spaces and in that key order,
// matching the example's interpolated `user_json`. `JSON.stringify` escapes
// a quote as backslash-quote and a backslash as two backslashes, which is
// what the example's `stringify(String(...))` does -- exchange 8 of the
// equivalence check is what establishes that rather than this comment.
function userJson(u) {
  return JSON.stringify({ id: u.id, name: u.name, email: u.email });
}

// Walks ids ascending from 1, which is what the example does and why its
// output is deterministic where a hash-order walk would not be.
function usersJson() {
  const out = [];
  for (let id = 1; id < store.nextId; id++) {
    const u = store.users.get(id);
    if (u !== undefined) out.push(userJson(u));
  }
  return "[" + out.join(",") + "]";
}

function errorJson(msg) {
  return JSON.stringify({ error: msg });
}

function json(status, body) {
  return new Response(body, {
    status,
    headers: { "content-type": "application/json" },
  });
}

// The example's `path_id` is `parse(seg)` then `Int::from_json`, so a
// segment that is not JSON at all and a JSON value that is not an integer
// both land on the same 400.
function pathId(seg) {
  let v;
  try {
    v = JSON.parse(seg);
  } catch {
    return null;
  }
  return Number.isInteger(v) ? v : null;
}

const server = Bun.serve({
  port: 0,
  hostname: "127.0.0.1",
  async fetch(req) {
    const path = new URL(req.url).pathname;

    if (req.method === "GET") {
      if (path === "/users") return json(200, usersJson());
      // Both halves are load-bearing, exactly as in the example: the length
      // bounds the index, and `parts[1] === "users"` is what makes this
      // `/users/:id` rather than `/<anything>/:id`. Exchange 7 of the
      // equivalence check is what catches losing the second half.
      const parts = path.split("/");
      if (parts.length === 3 && parts[1] === "users") {
        const id = pathId(parts[2]);
        if (id === null) return json(400, errorJson("id must be an integer"));
        const u = store.users.get(id);
        if (u === undefined) return json(404, errorJson("no such user"));
        return json(200, userJson(u));
      }
      return json(404, errorJson("not found"));
    }

    if (req.method === "POST") {
      if (path !== "/users") return json(404, errorJson("not found"));
      let text;
      try {
        text = await req.text();
      } catch {
        return json(400, errorJson("body is not UTF-8"));
      }
      let v;
      try {
        v = JSON.parse(text);
      } catch (e) {
        return json(400, errorJson(String(e && e.message)));
      }
      if (v === null || typeof v !== "object" || Array.isArray(v)) {
        return json(400, errorJson("missing field name"));
      }
      if (typeof v.name !== "string") return json(400, errorJson("missing field name"));
      if (typeof v.email !== "string") return json(400, errorJson("missing field email"));
      const id = store.nextId;
      store.nextId = id + 1;
      const u = { id, name: v.name, email: v.email };
      store.users.set(id, u);
      return json(201, userJson(u));
    }

    return json(404, errorJson("not found"));
  },
});

// The same line shape the example prints, so one port parser serves both.
console.log(`listening on 127.0.0.1:${server.port}`);
