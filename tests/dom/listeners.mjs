// Click the button three times, the link once, send a key and a window
// event, a turn apart each, then print what the page and its handlers said.
import { install, FakeEvent } from "./fake_dom.mjs";
const win = install();
const { default: init } = await import("./dist/app.js");
await init();
const turn = () => new Promise((r) => setTimeout(r, 0));
const button = document.querySelector("button");
for (let i = 0; i < 3; i++) {
  console.log(`click ${i + 1}`);
  button.click();
  await turn();
}
const link = document.querySelector("a");
const event = new FakeEvent("click", { bubbles: true, cancelable: true });
console.log(`link dispatch returned ${link.dispatchEvent(event)}`);
await turn();
button.dispatchEvent(new FakeEvent("keydown", { bubbles: true, key: "Enter" }));
await turn();
win.dispatchEvent(new FakeEvent("ping"));
await turn();
console.log(document.body.outerHTML);
