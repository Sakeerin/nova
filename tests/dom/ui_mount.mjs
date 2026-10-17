// Mount into an element that holds a paragraph already, then print the
// page; `main` panics at its second mount.
import { install } from "./fake_dom.mjs";
install();
const app = document.createElement("div");
app.setAttribute("id", "app");
const old = document.createElement("p");
old.setAttribute("id", "old");
old.textContent = "here first";
app.appendChild(old);
document.body.appendChild(app);
const { default: init } = await import("./dist/app.js");
try {
  await init();
} finally {
  console.log(document.body.outerHTML);
}
