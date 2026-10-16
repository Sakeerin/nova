// Click once: the task the click spawns calls `block_on`.
import { install } from "./fake_dom.mjs";
install();
const { default: init } = await import("./dist/app.js");
await init();
document.querySelector("button").click();
console.log("clicked");
