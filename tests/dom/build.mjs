import { install } from "./fake_dom.mjs";
install();
const { default: init } = await import("./dist/app.js");
await init();
console.log(document.body.outerHTML);
