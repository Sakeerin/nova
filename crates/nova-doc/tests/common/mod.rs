//! What a site must hold, read from the sources by the parser alone, not
//! through nova-doc's model (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5a-the-documentation-site-design.md`
//! §12.2, §16.4; plan decision 17).

// Each test file uses a part of this module.
#![allow(dead_code)]

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use nova_ast::item::{ExternItem, TraitItem, TypeDef};
use nova_ast::{File, Item, Type, Visibility};
use nova_diagnostics::{FileId, Spanned};
use nova_lexer::Token;

/// The primitive types, each with a page.
pub const PRIMITIVES: [&str; 7] = ["Int", "Float", "Bool", "Char", "String", "Bytes", "Future"];

/// `text`, parsed; it must lex and parse cleanly.
pub fn parse(text: &str) -> File {
    let (tokens, errors) = nova_lexer::lex(text, FileId::DUMMY);
    assert!(errors.is_empty(), "{errors:?}");
    let (file, errors) = nova_parser::parse(&tokens, FileId::DUMMY);
    assert!(errors.is_empty(), "{errors:?}");
    file.expect("the file parses")
}

/// The `//!` lines before `text`'s first token, without their `//!`.
pub fn module_docs(text: &str) -> String {
    let (tokens, comments, _) = nova_lexer::lex_with_comments(text, FileId::DUMMY);
    let first = tokens
        .iter()
        .find(|t| t.value != Token::Eof)
        .map_or(text.len(), |t| t.span.start as usize);
    comments
        .iter()
        .filter(|c| c.span.end as usize <= first)
        .filter_map(|c| text[c.span.start as usize..c.span.end as usize].strip_prefix("//!"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// The last segment of a path type.
fn type_name(ty: &Type) -> Option<&str> {
    match ty {
        Type::Path { path, .. } => path.segments.last().map(|s| s.value.as_str()),
        _ => None,
    }
}

/// The primitive types, and every `pub` record and sum type in `sources`.
pub fn public_types(sources: &[Source]) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = PRIMITIVES.iter().map(|p| p.to_string()).collect();
    for s in sources {
        for item in &s.file.items {
            match &item.value {
                Item::Record(r) if r.vis == Visibility::Pub => {
                    out.insert(r.name.value.clone());
                }
                Item::Type(t) if t.vis == Visibility::Pub => {
                    out.insert(t.name.value.clone());
                }
                _ => {}
            }
        }
    }
    out
}

/// Each thing in `file` that must have `///` docs (spec §12.2): what it
/// is, and its doc lines. `public` holds the types whose inherent methods
/// count.
pub fn needing_docs<'f>(
    file: &'f File,
    public: &BTreeSet<String>,
) -> Vec<(String, &'f [Spanned<String>])> {
    let mut out: Vec<(String, &'f [Spanned<String>])> = Vec::new();
    for item in &file.items {
        match &item.value {
            Item::Function(f) if f.vis == Visibility::Pub => {
                out.push((format!("fn {}", f.name.value), &f.docs))
            }
            Item::Const(c) if c.vis == Visibility::Pub => {
                out.push((format!("const {}", c.name.value), &c.docs))
            }
            Item::Record(r) if r.vis == Visibility::Pub => {
                out.push((format!("record {}", r.name.value), &r.docs));
                for f in r.fields.iter().filter(|f| f.vis == Visibility::Pub) {
                    out.push((format!("field {}.{}", r.name.value, f.name.value), &f.docs));
                }
            }
            Item::Type(t) if t.vis == Visibility::Pub => {
                out.push((format!("type {}", t.name.value), &t.docs));
                if let TypeDef::Sum(variants) = &t.def {
                    for v in variants {
                        out.push((
                            format!("variant {}::{}", t.name.value, v.name.value),
                            &v.docs,
                        ));
                    }
                }
            }
            Item::Trait(t) if t.vis == Visibility::Pub => {
                out.push((format!("trait {}", t.name.value), &t.docs));
                for ti in &t.items {
                    match ti {
                        TraitItem::Required(s) => out.push((
                            format!("method {}::{}", t.name.value, s.name.value),
                            &s.docs,
                        )),
                        TraitItem::Provided(f) => out.push((
                            format!("method {}::{}", t.name.value, f.name.value),
                            &f.docs,
                        )),
                        TraitItem::AssocType { docs, name, .. } => {
                            out.push((format!("type {}::{}", t.name.value, name.value), docs))
                        }
                    }
                }
            }
            Item::Impl(imp) if imp.trait_.is_none() => {
                let Some(on) = type_name(&imp.ty.value) else {
                    continue;
                };
                if !public.contains(on) {
                    continue;
                }
                for f in imp.functions.iter().filter(|f| f.vis == Visibility::Pub) {
                    out.push((format!("method {on}.{}", f.name.value), &f.docs));
                }
            }
            _ => {}
        }
    }
    out
}

/// One module of a site: its page, its package, its parsed file, and
/// whether it documents every item, as a program's does.
pub struct Source {
    pub page: String,
    pub package: String,
    pub file: File,
    pub all: bool,
}

/// std's 17 modules.
pub fn std_sources() -> Vec<Source> {
    nova_std::ALL
        .iter()
        .map(|(name, text)| Source {
            page: format!("std/{name}/index.html"),
            package: "std".to_string(),
            file: parse(text),
            all: false,
        })
        .collect()
}

/// Every (page, id) a site must hold for `sources`: each documented item
/// and member (spec §5.2, §6.2).
pub fn anchors(sources: &[Source]) -> BTreeSet<(String, String)> {
    let mut out = BTreeSet::new();
    // Where each documented record and sum type is: (package, name, page,
    // prefix).
    let mut types: Vec<(&str, &str, &str, &str)> = Vec::new();
    for s in sources {
        let shown = |v: Visibility| s.all || v == Visibility::Pub;
        for item in &s.file.items {
            match &item.value {
                Item::Function(f) if shown(f.vis) => {
                    out.insert((s.page.clone(), format!("fn.{}", f.name.value)));
                }
                Item::Const(c) if shown(c.vis) => {
                    out.insert((s.page.clone(), format!("const.{}", c.name.value)));
                }
                Item::Extern(block) if s.all => {
                    for e in &block.items {
                        let ExternItem::Fn(sig) = e;
                        out.insert((s.page.clone(), format!("fn.{}", sig.name.value)));
                    }
                }
                Item::Record(r) if shown(r.vis) => {
                    let a = format!("record.{}", r.name.value);
                    for f in r.fields.iter().filter(|f| shown(f.vis)) {
                        out.insert((s.page.clone(), format!("{a}.field.{}", f.name.value)));
                    }
                    out.insert((s.page.clone(), a));
                    types.push((
                        s.package.as_str(),
                        r.name.value.as_str(),
                        s.page.as_str(),
                        "record",
                    ));
                }
                Item::Type(t) if shown(t.vis) => {
                    if let TypeDef::Sum(variants) = &t.def {
                        let a = format!("type.{}", t.name.value);
                        for v in variants {
                            out.insert((s.page.clone(), format!("{a}.variant.{}", v.name.value)));
                        }
                        out.insert((s.page.clone(), a));
                        types.push((
                            s.package.as_str(),
                            t.name.value.as_str(),
                            s.page.as_str(),
                            "type",
                        ));
                    }
                }
                Item::Trait(t) if shown(t.vis) => {
                    let a = format!("trait.{}", t.name.value);
                    for ti in &t.items {
                        let id = match ti {
                            TraitItem::Required(sig) => format!("{a}.method.{}", sig.name.value),
                            TraitItem::Provided(f) => format!("{a}.method.{}", f.name.value),
                            TraitItem::AssocType { name, .. } => format!("{a}.type.{}", name.value),
                        };
                        out.insert((s.page.clone(), id));
                    }
                    out.insert((s.page.clone(), a));
                }
                _ => {}
            }
        }
    }
    for s in sources {
        for item in &s.file.items {
            let Item::Impl(imp) = &item.value else {
                continue;
            };
            if imp.trait_.is_some() {
                continue;
            }
            let Some(on) = type_name(&imp.ty.value) else {
                continue;
            };
            for f in imp
                .functions
                .iter()
                .filter(|f| s.all || f.vis == Visibility::Pub)
            {
                if s.package == "std" && PRIMITIVES.contains(&on) {
                    out.insert((
                        format!("std/primitive.{on}.html"),
                        format!("method.{}", f.name.value),
                    ));
                } else if let Some((_, _, page, prefix)) = types
                    .iter()
                    .find(|(p, n, _, _)| *p == s.package && *n == on)
                {
                    out.insert((
                        page.to_string(),
                        format!("{prefix}.{on}.method.{}", f.name.value),
                    ));
                }
            }
        }
    }
    out
}

/// The entries of `expected` the site at `dir` lacks, as `page#id`.
pub fn missing(dir: &Path, expected: &BTreeSet<(String, String)>) -> Vec<String> {
    let mut pages: HashMap<&str, String> = HashMap::new();
    let mut out = Vec::new();
    for (page, id) in expected {
        let text = pages
            .entry(page.as_str())
            .or_insert_with(|| std::fs::read_to_string(dir.join(page)).unwrap_or_default());
        if !text.contains(&format!(" id=\"{id}\"")) {
            out.push(format!("{page}#{id}"));
        }
    }
    out
}
