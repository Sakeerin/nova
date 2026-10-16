// Answer the program's requests, click once after `init` resolves, and
// print what the POST sent.
import { install } from "./fake_dom.mjs";
const win = install();
win.routes["/ok"] = { status: 200, body: "hello" };
win.routes["/missing"] = { status: 404, body: "not found" };
win.routes["/down"] = { error: "connection refused" };
win.routes["POST /echo"] = { status: 201, body: "pong" };
const { default: init } = await import("./dist/app.js");
await init();
console.log("init resolved");
document.querySelector("button").click();
await new Promise((r) => setTimeout(r, 50));
const post = win.requests.find((r) => r.method === "POST");
console.log(`${post.method} ${post.path} ${JSON.stringify(post.headers)} ${post.body}`);
console.log(`${win.requests.length} requests`);
