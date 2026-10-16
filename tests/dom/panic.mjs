// Click three times, a turn apart: the second click panics, and the third
// calls nothing.
import { install } from "./fake_dom.mjs";
install();
const { default: init } = await import("./dist/app.js");
await init();
const button = document.querySelector("button");
for (let i = 0; i < 3; i++) {
  button.click();
  console.log(`clicked ${i + 1}`);
  await new Promise((r) => setTimeout(r, 0));
}
console.log(`${window.uncaught.length} kept by the page`);
