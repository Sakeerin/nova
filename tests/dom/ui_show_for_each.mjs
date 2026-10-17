// Toggle the panel on and off, add rows, set the same rows again, click a
// row, and remove every row, a turn apart, printing the page after each.
import { install } from "./fake_dom.mjs";
install();
const app = document.createElement("div");
app.setAttribute("id", "app");
document.body.appendChild(app);
const { default: init } = await import("./dist/app.js");
await init();
const turn = () => new Promise((r) => setTimeout(r, 0));
const show = (what) => console.log(`${what}: ${app.innerHTML}`);
show("mounted");
for (const step of ["toggle", "more", "more", "again", "toggle"]) {
  document.getElementById(step).click();
  await turn();
  show(step);
}
document.querySelectorAll("#list li")[2].click();
await turn();
document.getElementById("none").click();
await turn();
show("none");
