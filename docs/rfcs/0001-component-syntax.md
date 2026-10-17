# RFC 0001: Component syntax

**Status:** Accepted, 2026-10-17 (Phase 4.4; ADR 0043)
**Spec:** `docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md`

## Summary

A component is a function that returns a `View`, written with
`component`; its view is a tree of elements, text and views, written with
`view { … }`. Inside a view an element takes its attributes by name and its
children in a trailing block. Everywhere, `||` is a closure with no
parameters.

```nova
import ui
import ui::html::{button, div}

component Counter() {
    let count = signal(0)

    view {
        div(class: "counter") {
            text("Count: ${count.value()}")
            button(on_click: || count.update(|n| n + 1)) { text("+") }
            button(on_click: || count.update(|n| n - 1)) { text("-") }
        }
    }
}

fn main() { mount("#app", Counter) }
```

## Grammar

```
item       += [ "pub" ] "component" IDENT "(" params ")" block
atom       += "view" "{" child* "}"
            | "||" [ "->" type ] expr
child       = ( element | postfix_expr ) [ ";" ]
element     = IDENT "{" child* "}"
            | IDENT "(" [ view_args ] ")" [ "{" child* "}" ]
view_args   = view_arg { "," view_arg } [ "," ]
view_arg    = NAME ":" expr | expr
NAME        = IDENT | keyword
```

- `component` is a keyword only where an item starts, a name and `(`
  follow it, and a block follows its parameters (or a `->`, refused below).
  A component has no type parameters, no `async` and no written
  return type: `component A() -> Int` and `component A<T>()` do not parse.
- `view` is a keyword only where an expression starts and `{` follows it,
  and not in the head of an `if`, `while`, `for` or `match`, where `{`
  opens the body, even inside parentheses there, as a record literal is
  not: `if f(|| view { … }) {` does not parse, and the view is bound to a
  name before the `if`.
- A child is an element or a postfix expression: a name, a call, a method
  call, a field, a literal, a parenthesised expression, a block, or another
  view. An expression with an operator is written in parentheses.
- An element is a name followed by a trailing block, or by an argument
  list holding at least one named argument, or both. A call with only
  positional arguments and no block, such as `Counter()` or `show(open,
  || view { … })`, is an ordinary expression. So a void element with no
  attributes is written `br([])`: `br()` calls `br` with no arguments, and
  `br {}` is E0095.
- Line breaks are skipped, so a child that begins with `(`, `[` or `{`
  would continue the one before; a `;` ends a child, and `nova fmt` writes
  one there, as it does between statements.
- `||` where an expression starts is the closure `| |`; elsewhere it is
  the logical or.

## Meaning

The parser keeps the sugar as its own nodes, and the type checker turns it
into `std/ui`'s API, as it does string interpolation.

| Written | Means |
|---|---|
| `component Name(params) { body }` | `fn Name(params) -> View { body }` |
| `view { c }` | the `View` `c` means; no child, or several, is E0094 |
| `name(args) { children }`, `name` taking `([Attr], [View])` | `name([attrs], [children])` |
| `name(args)`, `name` taking `([Attr])` | `name([attrs])`; a block is E0095 |
| an element whose `name` is anything else | E0096 |
| `text("literal")` | `Text("literal")` |
| `text(e)` | `DynText(\|\| e)` |
| a positional argument `a` | `a`, an `Attr` |
| `on_kind: h`, `h: fn()` | `on("kind", h)`, which is `On("kind", \|v: String\| h())` |
| `on_kind: h`, `h: fn(String)` | `On("kind", h)` |
| `value: "literal"` | `Property("value", "literal")` |
| `value: v` | `DynProperty("value", \|\| v)` |
| `name: "literal"` | `Static("name", "literal")` |
| `name: v` | `Dynamic("name", \|\| v)` |

- An attribute's name is `name` with each `_` made `-`: `aria_label` is
  `aria-label`. The event after `on_` keeps the DOM's spelling:
  `on_keydown` is `keydown`.
- A literal is a string literal without interpolation; anything else,
  `"a ${b}"` and `name` included, is read again when a signal it reads
  changes. A value that is not a `String`, or a handler of another type, is
  E0097, naming the attribute and the type found.
- A child that is an expression must be a `View`.
- `text(e)` needs no import, and the element's builder is the function of
  that name in scope: `import ui::html::{div}` brings `div`.

## Alternatives the user weighed

- **`count.value` for a signal's value**, as `30-FRONTEND.md` §3 writes
  it: `Signal` is a record whose value is a field, so a read could not be
  tracked. `count.value()` it is.
- **Helper functions for attributes**, `div(class("a"))`, as 4.3's builders
  take them: they still work as positional arguments, but a name is
  shorter and needs no import.
- **Explicit reactivity**, `text(dyn("…"))`: every text that reads a
  signal would need it, and forgetting it shows a stale page with no
  error.
- **`| |` kept**, with `||` only inside views: one spelling everywhere is
  simpler, and the formatter's tree check proves the reformatting changes
  nothing.
- **`if` and `for` inside `view`**: `show` with `|| view { … }` and
  `for_each` with `|item| view { … }` do the same with what `std/ui`
  already has.

## Not in this RFC

- `if` and `for` in a view; a view with several roots, or fragments.
- Named arguments and trailing blocks outside `view`.
- Boolean properties such as `checked`: `Attr` holds strings only.
- Type parameters on a component.
