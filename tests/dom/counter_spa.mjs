// The gate of 4.4 (spec
// docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md
// §9.4): `examples/06-counter-spa`, mounted into `#app`. Print the count,
// then click `+` three times and `-` once, a turn apart, finding the
// buttons by their text, and print the count after each; then the page.
import { install } from "./fake_dom.mjs";
install();
const app = document.createElement("div");
app.setAttribute("id", "app");
document.body.appendChild(app);
const { default: init } = await import("./dist/app.js");
await init();
const button = (label) => document.querySelectorAll("button").find((b) => b.textContent === label);
const count = () => document.querySelector(".counter").firstChild.textContent;
console.log(count());
for (const label of ["+", "+", "+", "-"]) {
  button(label).click();
  await new Promise((r) => setTimeout(r, 0));
  console.log(count());
}
console.log(document.body.outerHTML);
