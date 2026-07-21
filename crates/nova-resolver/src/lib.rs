//! Name resolution for the Nova compiler.
//!
//! Phase 1 scope: a single-file module. The resolver walks the top-level
//! items of a parsed [`nova_ast::File`] and produces a [`Definitions`] table
//! mapping every top-level name (functions, sum types, sum-type variants,
//! constants) to a stable [`DefId`]. Duplicate definitions are reported as
//! `E0002` diagnostics.
//!
//! Local (block-scoped) variable resolution happens during type checking,
//! where scoping interacts with inference; this crate owns the *item-level*
//! namespace and the shared [`DefId`] / [`Res`] vocabulary used by the rest
//! of the pipeline.

use indexmap::IndexMap;
use nova_ast::item::TypeDef;
use nova_ast::{File, Item};
use nova_diagnostics::{Diagnostic, Span};
use rustc_hash::FxHashMap;

/// A stable identifier for a top-level definition within a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DefId(pub u32);

/// A compiler-provided builtin function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Builtin {
    /// `println(s: String)` — write a line to stdout.
    Println,
    /// `print(s: String)` — write to stdout without a trailing newline.
    Print,
}

impl Builtin {
    /// The source-level name of the builtin.
    pub fn name(self) -> &'static str {
        match self {
            Builtin::Println => "println",
            Builtin::Print => "print",
        }
    }

    /// All builtins injected into the prelude scope.
    pub const ALL: [Builtin; 2] = [Builtin::Println, Builtin::Print];
}

/// What kind of definition a [`DefId`] refers to.
#[derive(Debug, Clone)]
pub enum DefKind {
    /// A function; the payload is the index of the item in `File::items`.
    Fn { item_index: usize },
    /// A sum type declaration with its variants.
    Sum {
        item_index: usize,
        variants: Vec<VariantDef>,
    },
    /// A record (struct) declaration.
    Record { item_index: usize },
    /// A constant; payload is the item index.
    Const { item_index: usize },
}

/// One variant of a sum type.
#[derive(Debug, Clone)]
pub struct VariantDef {
    pub name: String,
    pub span: Span,
    /// Number of payload fields (types live in the AST, indexed by the parent).
    pub arity: usize,
}

/// A single top-level definition.
#[derive(Debug, Clone)]
pub struct Def {
    pub name: String,
    pub span: Span,
    pub kind: DefKind,
}

/// What a name in expression position resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Res {
    /// A top-level definition.
    Def(DefId),
    /// Variant `variant_index` of the sum type `DefId`.
    Variant(DefId, usize),
    /// A compiler builtin.
    Builtin(Builtin),
}

/// The item-level namespace of one module.
#[derive(Debug, Default)]
pub struct Definitions {
    defs: Vec<Def>,
    /// Value namespace: functions, consts, bare variant names, builtins.
    values: FxHashMap<String, Res>,
    /// Type namespace: sum types (and later records, aliases).
    types: FxHashMap<String, DefId>,
}

impl Definitions {
    /// All collected definitions, indexable by `DefId.0`.
    pub fn defs(&self) -> &[Def] {
        &self.defs
    }

    /// Look up a definition by id.
    pub fn def(&self, id: DefId) -> &Def {
        &self.defs[id.0 as usize]
    }

    /// Resolve a name in *value* position (function call, variant constructor…).
    pub fn resolve_value(&self, name: &str) -> Option<Res> {
        self.values.get(name).copied()
    }

    /// Resolve a name in *type* position.
    pub fn resolve_type(&self, name: &str) -> Option<DefId> {
        self.types.get(name).copied()
    }

    /// Iterate all function definitions as `(DefId, item_index)`.
    pub fn functions(&self) -> impl Iterator<Item = (DefId, usize)> + '_ {
        self.defs
            .iter()
            .enumerate()
            .filter_map(|(i, d)| match d.kind {
                DefKind::Fn { item_index } => Some((DefId(i as u32), item_index)),
                _ => None,
            })
    }

    fn push(&mut self, def: Def) -> DefId {
        let id = DefId(self.defs.len() as u32);
        self.defs.push(def);
        id
    }
}

/// Output of [`resolve`]: the namespace table plus any diagnostics.
#[derive(Debug)]
pub struct ResolveResult {
    pub definitions: Definitions,
    pub diagnostics: Vec<Diagnostic>,
}

/// Collect the item-level namespace of a single file.
///
/// Reports `E0002` for duplicate definitions in the same namespace. Imports
/// and `module` declarations are accepted but ignored in Phase 1 (single-file
/// compilation); traits/impls/records are collected in later Phase 1 steps.
pub fn resolve(file: &File) -> ResolveResult {
    let mut definitions = Definitions::default();
    let mut diagnostics = Vec::new();

    // Prelude: builtins occupy the value namespace first so user shadowing
    // of e.g. `println` is reported as a duplicate rather than silently
    // replacing the builtin.
    for b in Builtin::ALL {
        definitions
            .values
            .insert(b.name().to_string(), Res::Builtin(b));
    }

    // Track first-definition spans for duplicate reporting.
    let mut first_value_span: IndexMap<String, Span> = IndexMap::new();
    let mut first_type_span: IndexMap<String, Span> = IndexMap::new();

    for (item_index, item) in file.items.iter().enumerate() {
        match &item.value {
            Item::Function(f) => {
                let name = f.name.value.clone();
                let span = f.name.span;
                let id = definitions.push(Def {
                    name: name.clone(),
                    span,
                    kind: DefKind::Fn { item_index },
                });
                insert_value(
                    &mut definitions,
                    &mut first_value_span,
                    &mut diagnostics,
                    name,
                    span,
                    Res::Def(id),
                );
            }
            Item::Type(t) => match &t.def {
                TypeDef::Sum(variants) => {
                    let name = t.name.value.clone();
                    let span = t.name.span;
                    let variant_defs: Vec<VariantDef> = variants
                        .iter()
                        .map(|v| VariantDef {
                            name: v.name.value.clone(),
                            span: v.name.span,
                            arity: v.fields.len(),
                        })
                        .collect();
                    let id = definitions.push(Def {
                        name: name.clone(),
                        span,
                        kind: DefKind::Sum {
                            item_index,
                            variants: variant_defs,
                        },
                    });
                    insert_type(
                        &mut definitions,
                        &mut first_type_span,
                        &mut diagnostics,
                        name,
                        span,
                        id,
                    );
                    // Bare variant names live in the value namespace so
                    // `Some(x)` / `Circle(1.0)` resolve without a type prefix.
                    for (vi, v) in variants.iter().enumerate() {
                        insert_value(
                            &mut definitions,
                            &mut first_value_span,
                            &mut diagnostics,
                            v.name.value.clone(),
                            v.name.span,
                            Res::Variant(id, vi),
                        );
                    }
                }
                TypeDef::Alias(_) => {
                    diagnostics.push(unsupported(
                        t.name.span,
                        "type aliases are not supported yet in the Phase 1 compiler",
                    ));
                }
            },
            Item::Const(c) => {
                let name = c.name.value.clone();
                let span = c.name.span;
                let id = definitions.push(Def {
                    name: name.clone(),
                    span,
                    kind: DefKind::Const { item_index },
                });
                insert_value(
                    &mut definitions,
                    &mut first_value_span,
                    &mut diagnostics,
                    name,
                    span,
                    Res::Def(id),
                );
            }
            // Accepted and ignored in Phase 1 single-file compilation.
            Item::Import(_) | Item::Module(_) => {}
            Item::Record(r) => {
                let name = r.name.value.clone();
                let span = r.name.span;
                let id = definitions.push(Def {
                    name: name.clone(),
                    span,
                    kind: DefKind::Record { item_index },
                });
                insert_type(
                    &mut definitions,
                    &mut first_type_span,
                    &mut diagnostics,
                    name,
                    span,
                    id,
                );
            }
            Item::Trait(t) => {
                diagnostics.push(unsupported(
                    t.name.span,
                    "traits are not supported yet in the Phase 1 compiler",
                ));
            }
            Item::Impl(i) => {
                diagnostics.push(unsupported(
                    i.ty.span,
                    "impl blocks are not supported yet in the Phase 1 compiler",
                ));
            }
            Item::Extern(_) => {
                diagnostics.push(Diagnostic::error(
                    "E0900",
                    "extern blocks are not supported yet in the Phase 1 compiler",
                ));
            }
        }
    }

    ResolveResult {
        definitions,
        diagnostics,
    }
}

fn insert_value(
    definitions: &mut Definitions,
    first: &mut IndexMap<String, Span>,
    diagnostics: &mut Vec<Diagnostic>,
    name: String,
    span: Span,
    res: Res,
) {
    if definitions.values.contains_key(&name) {
        let mut diag = Diagnostic::error("E0002", format!("duplicate definition of `{name}`"))
            .with_primary_label(span, "redefined here");
        if let Some(prev) = first.get(&name) {
            diag = diag.with_secondary_label(*prev, "first defined here");
        } else {
            diag = diag.with_note(format!("`{name}` is a compiler builtin"));
        }
        diagnostics.push(diag);
        return;
    }
    first.insert(name.clone(), span);
    definitions.values.insert(name, res);
}

fn insert_type(
    definitions: &mut Definitions,
    first: &mut IndexMap<String, Span>,
    diagnostics: &mut Vec<Diagnostic>,
    name: String,
    span: Span,
    id: DefId,
) {
    if definitions.types.contains_key(&name) {
        let mut diag = Diagnostic::error("E0002", format!("duplicate definition of type `{name}`"))
            .with_primary_label(span, "redefined here");
        if let Some(prev) = first.get(&name) {
            diag = diag.with_secondary_label(*prev, "first defined here");
        }
        diagnostics.push(diag);
        return;
    }
    first.insert(name.clone(), span);
    definitions.types.insert(name, id);
}

fn unsupported(span: Span, msg: &str) -> Diagnostic {
    Diagnostic::error("E0900", msg).with_primary_label(span, "not supported yet")
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_diagnostics::FileId;
    use nova_lexer::lex;
    use nova_parser::parse;

    fn resolve_src(src: &str) -> ResolveResult {
        let file_id = FileId::DUMMY;
        let (tokens, lex_errors) = lex(src, file_id);
        assert!(lex_errors.is_empty(), "lex errors: {lex_errors:?}");
        let (ast, parse_errors) = parse(&tokens, file_id);
        assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
        resolve(&ast.expect("no AST produced"))
    }

    #[test]
    fn collects_functions_and_sum_types() {
        let r = resolve_src(
            "type Shape = | Circle(Int) | Empty\n\
             fn area(s: Shape) -> Int { 0 }\n\
             fn main() { }\n",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        assert!(matches!(
            r.definitions.resolve_value("area"),
            Some(Res::Def(_))
        ));
        assert!(matches!(
            r.definitions.resolve_value("Circle"),
            Some(Res::Variant(_, 0))
        ));
        assert!(matches!(
            r.definitions.resolve_value("Empty"),
            Some(Res::Variant(_, 1))
        ));
        assert!(r.definitions.resolve_type("Shape").is_some());
        assert!(matches!(
            r.definitions.resolve_value("println"),
            Some(Res::Builtin(Builtin::Println))
        ));
    }

    #[test]
    fn collects_records() {
        let r = resolve_src("record Point { x: Float, y: Float }\nfn main() { }\n");
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let id = r.definitions.resolve_type("Point").expect("Point resolves");
        assert!(matches!(r.definitions.def(id).kind, DefKind::Record { .. }));
    }

    #[test]
    fn duplicate_function_reports_e0002() {
        let r = resolve_src("fn f() { }\nfn f() { }\n");
        assert_eq!(r.diagnostics.len(), 1);
        assert_eq!(r.diagnostics[0].code, "E0002");
    }

    #[test]
    fn shadowing_builtin_reports_e0002() {
        let r = resolve_src("fn println() { }\n");
        assert_eq!(r.diagnostics.len(), 1);
        assert_eq!(r.diagnostics[0].code, "E0002");
    }
}
