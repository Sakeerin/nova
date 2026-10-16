// Click once, as soon as `main` has made the button, while it sleeps.
import { install } from "./fake_dom.mjs";
install();
const { default: init } = await import("./dist/app.js");
const done = init();
while (document.querySelector("button") === null) {
  await new Promise((r) => setTimeout(r, 1));
}
document.querySelector("button").click();
await done;
console.log("init resolved");
