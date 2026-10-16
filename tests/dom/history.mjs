// After the hashchange's turn, go back once.
import { install } from "./fake_dom.mjs";
install();
const { default: init } = await import("./dist/app.js");
await init();
const turn = () => new Promise((r) => setTimeout(r, 0));
await turn();
console.log("back");
history.back();
await turn();
console.log(location.href);
