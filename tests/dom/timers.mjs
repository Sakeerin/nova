// Wait for the last timeout, and a little more for anything that should
// not come.
import { install } from "./fake_dom.mjs";
install();
const { default: init } = await import("./dist/app.js");
await init();
console.log("init resolved");
await new Promise((r) => setTimeout(r, 600));
console.log("waited");
