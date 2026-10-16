// A small DOM for `cargo test`, in plain JS with no dependencies (spec
// docs/superpowers/specs/2026-10-16-phase-4-2-std-dom-and-the-shim-design.md
// §9). A test's harness calls `install()` before it imports `app.js`, then
// drives the page and prints it.
//
// It holds what a browser holds and no more: a listener it keeps is the
// function it was given, so if std/rt's closure table were not a root, a
// collection under stress would free the closure behind it. And as a
// browser does, it keeps an exception a listener throws, in `uncaught`,
// and goes on: the dispatch that called the listener returns normally.

// What listeners threw, as a browser reports them.
const uncaught = [];

class FakeEvent {
  constructor(type, init = {}) {
    this.type = type;
    this.bubbles = init.bubbles === true;
    this.cancelable = init.cancelable === true;
    this.key = init.key;
    this.target = null;
    this.currentTarget = null;
    this.defaultPrevented = false;
    this.stopped = false;
  }
  preventDefault() {
    if (this.cancelable) this.defaultPrevented = true;
  }
  stopPropagation() {
    this.stopped = true;
  }
}

// What can have listeners: a node, the document, the window.
class Target {
  constructor() {
    this.listeners = new Map();
  }
  addEventListener(type, fn) {
    let list = this.listeners.get(type);
    if (!list) this.listeners.set(type, (list = []));
    if (!list.includes(fn)) list.push(fn);
  }
  removeEventListener(type, fn) {
    const list = this.listeners.get(type);
    if (!list) return;
    const at = list.indexOf(fn);
    if (at >= 0) list.splice(at, 1);
  }
  // The target an event bubbles to next.
  eventParent() {
    return null;
  }
  fire(event) {
    const list = this.listeners.get(event.type);
    if (!list) return;
    event.currentTarget = this;
    for (const fn of [...list]) {
      // A listener removed earlier in this dispatch does not run, as in a
      // browser.
      if (!list.includes(fn)) continue;
      try {
        fn.call(this, event);
      } catch (e) {
        uncaught.push(e);
      }
    }
  }
  dispatchEvent(event) {
    event.target = this;
    for (let at = this; at !== null; at = at.eventParent()) {
      at.fire(event);
      if (event.stopped || !event.bubbles) break;
    }
    return !event.defaultPrevented;
  }
}

function domError(name, message) {
  const e = new Error(message);
  e.name = name;
  return e;
}

class FakeNode extends Target {
  constructor(nodeName, doc) {
    super();
    this.nodeName = nodeName;
    this.ownerDocument = doc;
    this.parentNode = null;
    this.childNodes = [];
  }
  get firstChild() {
    return this.childNodes[0] ?? null;
  }
  get nextSibling() {
    if (this.parentNode === null) return null;
    const siblings = this.parentNode.childNodes;
    return siblings[siblings.indexOf(this) + 1] ?? null;
  }
  contains(node) {
    for (let at = node; at !== null; at = at.parentNode) if (at === this) return true;
    return false;
  }
  appendChild(child) {
    return this.insertBefore(child, null);
  }
  insertBefore(child, before) {
    if (!(child instanceof FakeNode)) throw new TypeError("parameter 1 is not of type 'Node'");
    if (child.contains(this)) {
      throw domError("HierarchyRequestError", "The new child element contains the parent.");
    }
    if (before !== null && before.parentNode !== this) {
      throw domError("NotFoundError", "The node before which the new node is to be inserted is not a child of this node.");
    }
    if (child === before) return child;
    child.remove();
    const at = before === null ? this.childNodes.length : this.childNodes.indexOf(before);
    this.childNodes.splice(at, 0, child);
    child.parentNode = this;
    return child;
  }
  removeChild(child) {
    if (!(child instanceof FakeNode) || child.parentNode !== this) {
      throw domError("NotFoundError", "The node to be removed is not a child of this node.");
    }
    child.remove();
    return child;
  }
  remove() {
    if (this.parentNode === null) return;
    const siblings = this.parentNode.childNodes;
    siblings.splice(siblings.indexOf(this), 1);
    this.parentNode = null;
  }
  get textContent() {
    return this.childNodes.map((c) => c.textContent).join("");
  }
  set textContent(value) {
    for (const c of [...this.childNodes]) c.remove();
    const s = String(value);
    if (s !== "") this.appendChild(new FakeText(s, this.ownerDocument));
  }
  eventParent() {
    return this.parentNode;
  }
}

class FakeText extends FakeNode {
  constructor(data, doc) {
    super("#text", doc);
    this.data = data;
  }
  get textContent() {
    return this.data;
  }
  set textContent(value) {
    this.data = String(value);
  }
  get outerHTML() {
    return escapeText(this.data);
  }
}

const VOID = new Set(["br", "hr", "img", "input", "link", "meta"]);

function escapeText(s) {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

function escapeAttribute(s) {
  return s.replace(/&/g, "&amp;").replace(/"/g, "&quot;");
}

// Each node under `root`, before its children.
function walk(root, visit) {
  for (const c of root.childNodes) {
    visit(c);
    walk(c, visit);
  }
}

class FakeElement extends FakeNode {
  constructor(tag, doc) {
    super(tag.toUpperCase(), doc);
    this.localName = tag.toLowerCase();
    this.attrs = new Map();
  }
  get tagName() {
    return this.nodeName;
  }
  setAttribute(name, value) {
    this.attrs.set(String(name).toLowerCase(), String(value));
  }
  getAttribute(name) {
    return this.attrs.get(String(name).toLowerCase()) ?? null;
  }
  hasAttribute(name) {
    return this.attrs.has(String(name).toLowerCase());
  }
  removeAttribute(name) {
    this.attrs.delete(String(name).toLowerCase());
  }
  get id() {
    return this.getAttribute("id") ?? "";
  }
  set id(value) {
    this.setAttribute("id", value);
  }
  get className() {
    return this.getAttribute("class") ?? "";
  }
  set className(value) {
    this.setAttribute("class", value);
  }
  get value() {
    return this.ownValue ?? this.getAttribute("value") ?? "";
  }
  set value(v) {
    this.ownValue = String(v);
  }
  get checked() {
    return this.ownChecked ?? this.hasAttribute("checked");
  }
  set checked(v) {
    this.ownChecked = Boolean(v);
  }
  get disabled() {
    return this.hasAttribute("disabled");
  }
  set disabled(v) {
    if (v) this.setAttribute("disabled", "");
    else this.removeAttribute("disabled");
  }
  click() {
    this.dispatchEvent(new FakeEvent("click", { bubbles: true, cancelable: true }));
  }
  querySelectorAll(selector) {
    const matches = compile(selector);
    const out = [];
    walk(this, (n) => {
      if (n instanceof FakeElement && matches(n)) out.push(n);
    });
    return out;
  }
  querySelector(selector) {
    return this.querySelectorAll(selector)[0] ?? null;
  }
  get innerHTML() {
    return this.childNodes.map((c) => c.outerHTML).join("");
  }
  get outerHTML() {
    let attrs = "";
    for (const [name, value] of this.attrs) attrs += ` ${name}="${escapeAttribute(value)}"`;
    if (VOID.has(this.localName)) return `<${this.localName}${attrs}>`;
    return `<${this.localName}${attrs}>${this.innerHTML}</${this.localName}>`;
  }
}

// A selector of compound parts (`tag#id.class[attr][attr="v"]`), joined by
// the descendant combinator, and lists of them joined by commas.
function compile(selector) {
  const lists = String(selector)
    .split(",")
    .map((s) => s.trim().split(/\s+/).map(compound));
  return (el) => lists.some((parts) => matchFrom(el, parts, parts.length - 1));
}

function compound(text) {
  const tests = [];
  const token = /([a-zA-Z][a-zA-Z0-9-]*)|#([a-zA-Z0-9_-]+)|\.([a-zA-Z0-9_-]+)|\[([a-zA-Z0-9_-]+)(?:="([^"]*)")?\]/y;
  let at = 0;
  while (at < text.length) {
    token.lastIndex = at;
    const m = token.exec(text);
    if (m === null) throw domError("SyntaxError", `'${text}' is not a valid selector`);
    at = token.lastIndex;
    if (m[1] !== undefined) {
      const tag = m[1].toLowerCase();
      tests.push((el) => el.localName === tag);
    } else if (m[2] !== undefined) {
      tests.push((el) => el.id === m[2]);
    } else if (m[3] !== undefined) {
      tests.push((el) => el.className.split(/\s+/).includes(m[3]));
    } else if (m[5] !== undefined) {
      tests.push((el) => el.getAttribute(m[4]) === m[5]);
    } else {
      tests.push((el) => el.hasAttribute(m[4]));
    }
  }
  return (el) => tests.every((t) => t(el));
}

function matchFrom(el, parts, i) {
  if (!parts[i](el)) return false;
  if (i === 0) return true;
  for (let at = el.parentNode; at instanceof FakeElement; at = at.parentNode) {
    if (matchFrom(at, parts, i - 1)) return true;
  }
  return false;
}

class FakeDocument extends FakeNode {
  constructor(win) {
    super("#document", null);
    this.window = win;
    this.documentElement = new FakeElement("html", this);
    this.head = new FakeElement("head", this);
    this.body = new FakeElement("body", this);
    this.documentElement.appendChild(this.head);
    this.documentElement.appendChild(this.body);
    this.appendChild(this.documentElement);
  }
  createElement(tag) {
    return new FakeElement(String(tag), this);
  }
  createTextNode(data) {
    return new FakeText(String(data), this);
  }
  getElementById(id) {
    return this.documentElement.querySelectorAll(`#${id}`)[0] ?? null;
  }
  querySelectorAll(selector) {
    const matches = compile(selector);
    const out = [];
    walk(this, (n) => {
      if (n instanceof FakeElement && matches(n)) out.push(n);
    });
    return out;
  }
  querySelector(selector) {
    return this.querySelectorAll(selector)[0] ?? null;
  }
  eventParent() {
    return this.window;
  }
}

// `hashchange` and `popstate` come after the call that caused them, as in a
// browser.
function later(target, type) {
  setTimeout(() => target.dispatchEvent(new FakeEvent(type)), 0);
}

class FakeLocation {
  constructor(win, url) {
    this.win = win;
    this.url = new URL(url);
  }
  get href() {
    return this.url.href;
  }
  get pathname() {
    return this.url.pathname;
  }
  get search() {
    return this.url.search;
  }
  get hash() {
    return this.url.hash;
  }
  set hash(value) {
    const before = this.url.href;
    this.url.hash = String(value);
    if (this.url.href !== before) {
      this.win.history.entries.splice(this.win.history.index + 1);
      this.win.history.entries.push(this.url.href);
      this.win.history.index += 1;
      later(this.win, "hashchange");
    }
  }
}

class FakeHistory {
  constructor(win, url) {
    this.win = win;
    this.entries = [url];
    this.index = 0;
  }
  get length() {
    return this.entries.length;
  }
  pushState(state, title, url) {
    const next = new URL(String(url), this.win.location.href).href;
    this.entries.splice(this.index + 1);
    this.entries.push(next);
    this.index += 1;
    this.win.location.url = new URL(next);
  }
  replaceState(state, title, url) {
    const next = new URL(String(url), this.win.location.href).href;
    this.entries[this.index] = next;
    this.win.location.url = new URL(next);
  }
  back() {
    if (this.index === 0) return;
    this.index -= 1;
    this.win.location.url = new URL(this.entries[this.index]);
    later(this.win, "popstate");
  }
}

class FakeWindow extends Target {
  constructor(url) {
    super();
    this.location = new FakeLocation(this, url);
    this.history = new FakeHistory(this, this.location.href);
    this.document = new FakeDocument(this);
    // `fetch`'s answers, by "METHOD path" or by path, and what it was asked.
    this.routes = {};
    this.requests = [];
    this.uncaught = uncaught;
  }
  // A reply after a timer, never from the network: a route's
  // `{ status, body }`, or a `TypeError` for `{ error }` or no route.
  fetch(url, init = {}) {
    const method = (init.method ?? "GET").toUpperCase();
    const path = new URL(String(url), this.location.href).pathname;
    this.requests.push({ method, path, headers: init.headers ?? {}, body: init.body ?? "" });
    const route = this.routes[`${method} ${path}`] ?? this.routes[path];
    return new Promise((resolve, reject) => {
      setTimeout(() => {
        if (route === undefined) {
          reject(new TypeError("Failed to fetch"));
        } else if (route.error !== undefined) {
          reject(new TypeError(route.error));
        } else {
          const status = route.status ?? 200;
          const body = route.body ?? "";
          resolve({ status, ok: status >= 200 && status < 300, text: async () => body });
        }
      }, 1);
    });
  }
}

// Install the page as `window`, `document`, `location`, `history` and
// `fetch`, at `options.url` (`http://localhost/` by default).
export function install(options = {}) {
  const win = new FakeWindow(options.url ?? "http://localhost/");
  globalThis.window = win;
  globalThis.document = win.document;
  globalThis.location = win.location;
  globalThis.history = win.history;
  globalThis.fetch = (url, init) => win.fetch(url, init);
  return win;
}

export { FakeEvent };
