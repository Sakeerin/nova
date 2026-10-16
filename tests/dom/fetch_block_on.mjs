import { install } from "./fake_dom.mjs";
const win = install();
win.routes["/ok"] = { status: 200, body: "hello" };
const { default: init } = await import("./dist/app.js");
await init();
console.log("init resolved");
