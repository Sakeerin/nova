// Click the button three times, a turn apart, then print the page.
import { install } from "./fake_dom.mjs";
install();
const { default: init } = await import("./dist/app.js");
await init();
const button = document.getElementById("add");
for (let i = 0; i < 3; i++) {
  button.click();
  await new Promise((r) => setTimeout(r, 0));
}
console.log(document.getElementById("count").textContent);
console.log(document.body.outerHTML);
