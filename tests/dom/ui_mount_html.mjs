// Mount, then print the page; `main` panics at its second mount, before any
// of its nodes is in the page.
import { install } from "./fake_dom.mjs";
install();
const app = document.createElement("div");
app.setAttribute("id", "app");
document.body.appendChild(app);
const { default: init } = await import("./dist/app.js");
try {
  await init();
} finally {
  console.log(document.body.outerHTML);
}
