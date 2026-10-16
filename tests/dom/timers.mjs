// Wait until the page says the last timeout has run, then a little more
// for anything that should not come.
import { install } from "./fake_dom.mjs";
install();
const { default: init } = await import("./dist/app.js");
await init();
console.log("init resolved");
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
for (let i = 0; i < 1000 && document.body.getAttribute("data-done") === null; i++) await sleep(5);
await sleep(50);
console.log("waited");
