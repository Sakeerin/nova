// Click the outer button twice, a turn apart.
import { install } from "./fake_dom.mjs";
install();
const { default: init } = await import("./dist/app.js");
await init();
const outer = document.querySelector("button");
for (let i = 0; i < 2; i++) {
  outer.click();
  await new Promise((r) => setTimeout(r, 0));
}
