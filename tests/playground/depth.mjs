// The depth cases the playground's tests compile (spec
// docs/superpowers/specs/2026-11-28-phase-6-1-the-playground-design.md
// §5, tests 6 and 7), each a function from a depth `d` (at least 2) to a
// source whose deepest level is `d`, as 5.1b's parser counts it
// (`crates/nova-parser/tests/depth.rs`): the outermost expression, type or
// pattern is level 0, so `d` levels are `d` constructs of the kind, each
// inside the one before. At 128 each parses; at 129 each is P0002.
//
// `parseKinds` are depth.rs's 26 kinds as it writes them, which parse but
// do not check (their names are unknown). `typedKinds` are 16 kinds that
// check, build and run, each with what it prints at `d`; nested `fn` items
// are E0900, as natively.

const rep = (s, n) => s.repeat(n);
const inBody = (body) => `fn f() {\n    ${body}\n}\n`;
const main = (items, body) => `${items}fn main() {\n    ${body}\n}\n`;

// Wrappers taken in turn, the `i`th one level `i`.
function nest(wrappers, d) {
  let open = "";
  let close = "";
  for (let i = 0; i < d; i++) {
    const [o, c] = wrappers[i % wrappers.length];
    open += o;
    close = c + close;
  }
  return [open, close];
}

export const parseKinds = [
  ["parentheses", (d) => inBody(`${rep("(", d)}leaf${rep(")", d)}`)],
  ["blocks", (d) => inBody(`${rep("{ ", d)}leaf${rep(" }", d)}`)],
  ["calls", (d) => inBody(`${rep("g(", d)}leaf${rep(")", d)}`)],
  ["indexes", (d) => inBody(`${rep("a[", d)}leaf${rep("]", d)}`)],
  ["arrays", (d) => inBody(`${rep("[", d)}leaf${rep("]", d)}`)],
  ["record literals", (d) => inBody(`${rep("R { f: ", d)}leaf${rep(" }", d)}`)],
  ["interpolation holes", (d) => inBody(`${rep('"${', d)}leaf${rep('}"', d)}`)],
  ["closures", (d) => inBody(`${rep("|| ", d)}leaf`)],
  ["returns", (d) => inBody(`${rep("return ", d)}leaf`)],
  [
    "prefix operators",
    (d) => {
      const ops = ["! ", "- ", "~ ", "* "];
      let prefix = "";
      for (let i = 0; i < d - 1; i++) prefix += ops[i % ops.length];
      return inBody(`${prefix}&mut leaf`);
    },
  ],
  ["a binary chain", (d) => inBody(`${rep("1 + ", d)}leaf`)],
  ["a postfix chain", (d) => inBody(`a${rep(".x", d)}.leaf`)],
  ["method calls", (d) => inBody(`a${rep(".f()", d)}.leaf()`)],
  [
    "calls after `.await` and tuple indexes",
    (d) => {
      const ops = [".await()", ".0()"];
      let chain = "";
      for (let i = 0; i < d; i++) chain += ops[i % ops.length];
      return inBody(`a${chain}.leaf()`);
    },
  ],
  ["assignments", (d) => inBody(`${rep("a = ", d)}leaf`)],
  ["casts", (d) => inBody(`0${rep(" as Int", d - 1)} as Leaf`)],
  ["ifs", (d) => inBody(`${rep("if c { ", d - 1)}if leaf { 0 } else { 0 }${rep(" } else { 0 }", d - 1)}`)],
  ["an else-if chain", (d) => inBody(`if c { 0 }${rep(" else if c { 0 }", d - 2)} else if leaf { 0 } else { 0 }`)],
  ["matches", (d) => inBody(`${rep("match c { _ => ", d - 1)}match leaf { _ => 0 }${rep(" }", d - 1)}`)],
  ["whiles", (d) => inBody(`${rep("while c { ", d - 1)}while leaf { 0 }${rep(" }", d - 1)}`)],
  ["fors", (d) => inBody(`${rep("for x in c { ", d - 1)}for leaf in c { 0 }${rep(" }", d - 1)}`)],
  ["items in blocks", (d) => inBody(`${rep("fn g() { ", d)}leaf${rep(" }", d)}`)],
  [
    "types",
    (d) => {
      const [open, close] = nest(
        [["Vec<", ">"], ["[", "]"], ["(", ", Int)"], ["&", ""], ["*mut ", ""], ["fn(", ") -> Int"]],
        d,
      );
      return `fn f(x: ${open}Leaf${close}) {}\n`;
    },
  ],
  ["optional types", (d) => inBody(`let x: Int${rep("?", d + 1)}\n    leaf`)],
  [
    "patterns",
    (d) => {
      const [open, close] = nest(
        [["S(", ")"], ["(", ", 0)"], ["[", "]"], ["R { f: ", " }"], ["m::S(", ")"], ["a @ ", ""]],
        d,
      );
      return inBody(`let ${open}leaf${close} = 0`);
    },
  ],
  [
    "mixed expressions",
    (d) => {
      // depth.rs's generator, on 64-bit wrapping arithmetic as `usize` is
      // there.
      const wrappers = [
        ["(", ")"],
        ["{ ", " }"],
        ["g(", ")"],
        ["a[", "]"],
        ["[", "]"],
        ["!", ""],
        ["1 + ", ""],
        ["|| ", ""],
        ["if c { ", " } else { 0 }"],
        ["match c { _ => ", " }"],
        ["R { f: ", " }"],
      ];
      const mask = (1n << 64n) - 1n;
      let state = 7n;
      let last = 0;
      let open = "";
      let close = "";
      for (let i = 0; i < d - 1; i++) {
        state = (state * 6364136223846793005n + 1442695040888963407n) & mask;
        let w = Number((state >> 33n) % BigInt(wrappers.length));
        if (last === 5 && w === 6) w = 0;
        last = w;
        open += wrappers[w][0];
        close = wrappers[w][1] + close;
      }
      return inBody(`${open}(leaf)${close}`);
    },
  ],
];

// `{ source, prints }`, or `{ source, code }` for a kind the compiler
// refuses at every depth.
const runs = (source, prints) => ({ source, prints });

export const typedKinds = [
  ["parentheses", (d) => runs(main("", `let leaf = 1\n    let x = ${rep("(", d)}leaf${rep(")", d)}\n    println("\${x}")`), "1\n")],
  ["blocks", (d) => runs(main("", `let leaf = 2\n    let x = ${rep("{ ", d)}leaf${rep(" }", d)}\n    println("\${x}")`), "2\n")],
  [
    "calls",
    (d) =>
      runs(main("fn id(n: Int) -> Int { n }\n\n", `let leaf = 3\n    let x = ${rep("id(", d)}leaf${rep(")", d)}\n    println("\${x}")`), "3\n"),
  ],
  [
    "arrays",
    (d) => runs(main("", `let leaf = 4\n    let x = ${rep("[", d)}leaf${rep("]", d)}\n    println("\${x.len()}")`), "1\n"),
  ],
  [
    "interpolation holes",
    (d) => runs(main("", `let leaf = "deep"\n    let x = ${rep('"${', d)}leaf${rep('}"', d)}\n    println(x)`), "deep\n"),
  ],
  ["closures", (d) => runs(main("", `let leaf = 5\n    let x = ${rep("|| ", d)}leaf\n    println("made")`), "made\n")],
  [
    "prefix minus",
    (d) => runs(main("", `let leaf = 6\n    let x = ${rep("- ", d)}leaf\n    println("\${x}")`), `${d % 2 === 0 ? 6 : -6}\n`),
  ],
  ["a binary chain", (d) => runs(main("", `let leaf = 7\n    let x = ${rep("1 + ", d)}leaf\n    println("\${x}")`), `${d + 7}\n`)],
  [
    "method calls",
    (d) =>
      runs(
        main(
          "record Box { n: Int }\n\nimpl Box {\n    fn same(self) -> Box { self }\n    fn leaf(self) -> Int { self.n }\n}\n\n",
          `let b = Box { n: 8 }\n    let x = b${rep(".same()", d)}.leaf()\n    println("\${x}")`,
        ),
        "8\n",
      ),
  ],
  [
    "ifs",
    (d) =>
      runs(
        main("", `let c = true\n    let leaf = true\n    let x = ${rep("if c { ", d - 1)}if leaf { 9 } else { 0 }${rep(" } else { 0 }", d - 1)}\n    println("\${x}")`),
        "9\n",
      ),
  ],
  [
    "an else-if chain",
    (d) =>
      runs(
        main("", `let c = false\n    let leaf = true\n    let x = if c { 0 }${rep(" else if c { 0 }", d - 2)} else if leaf { 10 } else { 0 }\n    println("\${x}")`),
        "10\n",
      ),
  ],
  [
    "matches",
    (d) =>
      runs(
        main("", `let c = 0\n    let leaf = 1\n    let x = ${rep("match c { _ => ", d - 1)}match leaf { _ => 11 }${rep(" }", d - 1)}\n    println("\${x}")`),
        "11\n",
      ),
  ],
  [
    "whiles",
    (d) =>
      runs(
        main("", `let c = false\n    let leaf = false\n    ${rep("while c { ", d - 1)}while leaf { }${rep(" }", d - 1)}\n    println("looped")`),
        "looped\n",
      ),
  ],
  [
    "items in blocks",
    (d) => ({ source: main("", `${rep("fn g() { ", d)}leaf()${rep(" }", d)}\n    println("nested")`), code: "E0900" }),
  ],
  [
    "types",
    (d) =>
      runs(main(`fn first(v: ${rep("Vec<", d)}Int${rep(">", d)}) -> Int { v.len() }\n\n`, `println("typed")`), "typed\n"),
  ],
  [
    "tuple patterns",
    (d) =>
      runs(
        main("", `let ${rep("(", d)}leaf${rep(", _)", d)} = ${rep("(", d)}12${rep(", 0)", d)}\n    println("\${leaf}")`),
        "12\n",
      ),
  ],
];
