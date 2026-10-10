//! Semantic tokens: every name the index records, by what it means (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
//! §7).

use lsp_types as lsp;
use nova_diagnostics::LineIndex;
use nova_driver::Analysis;
use nova_resolver::{DefId, DefKind, Definitions, Index, Occurrence, Role, Target};

use crate::analysis::Answer;

/// The token types, in the legend's order (spec §7.1).
const TYPES: [lsp::SemanticTokenType; 12] = [
    lsp::SemanticTokenType::NAMESPACE,
    lsp::SemanticTokenType::TYPE,
    lsp::SemanticTokenType::STRUCT,
    lsp::SemanticTokenType::ENUM,
    lsp::SemanticTokenType::INTERFACE,
    lsp::SemanticTokenType::TYPE_PARAMETER,
    lsp::SemanticTokenType::PARAMETER,
    lsp::SemanticTokenType::VARIABLE,
    lsp::SemanticTokenType::PROPERTY,
    lsp::SemanticTokenType::ENUM_MEMBER,
    lsp::SemanticTokenType::FUNCTION,
    lsp::SemanticTokenType::METHOD,
];

const NAMESPACE: u32 = 0;
const TYPE: u32 = 1;
const STRUCT: u32 = 2;
const ENUM: u32 = 3;
const INTERFACE: u32 = 4;
const TYPE_PARAMETER: u32 = 5;
const PARAMETER: u32 = 6;
const VARIABLE: u32 = 7;
const PROPERTY: u32 = 8;
const ENUM_MEMBER: u32 = 9;
const FUNCTION: u32 = 10;
const METHOD: u32 = 11;

/// The modifiers' bits, in the legend's order (spec §7.1).
const DECLARATION: u32 = 1;
const READONLY: u32 = 2;
const DEFAULT_LIBRARY: u32 = 4;
const MUTABLE: u32 = 8;

/// The legend the server advertises (spec §7.1).
pub fn legend() -> lsp::SemanticTokensLegend {
    lsp::SemanticTokensLegend {
        token_types: TYPES.to_vec(),
        token_modifiers: vec![
            lsp::SemanticTokenModifier::DECLARATION,
            lsp::SemanticTokenModifier::READONLY,
            lsp::SemanticTokenModifier::DEFAULT_LIBRARY,
            lsp::SemanticTokenModifier::new("mutable"),
        ],
    }
}

/// The tokens of `answer`'s file (spec §7): one per span the index
/// records, the meaning 3.4a's ranking puts first; none for `self` or an
/// unresolved name.
pub fn tokens(answer: &Answer) -> Vec<lsp::SemanticToken> {
    let a = &answer.analysis;
    let (Some(index), Some(defs), Some(text)) = (
        a.index.as_ref(),
        a.definitions.as_ref(),
        a.db.get_source(answer.file),
    ) else {
        return Vec::new();
    };
    let mut found: Vec<&Occurrence> = index
        .occurrences
        .iter()
        .filter(|o| o.span.file == answer.file && o.span.start < o.span.end)
        .collect();
    found.sort_by_key(|o| {
        (
            o.span.start,
            o.span.end,
            nova_resolver::index::rank(defs, &o.target),
        )
    });
    found.dedup_by_key(|o| (o.span.start, o.span.end));
    let mut spans: Vec<(u32, u32, u32, u32)> = Vec::new();
    for o in found {
        let Some(name) = text.get(o.span.start as usize..o.span.end as usize) else {
            continue;
        };
        if name == "self" || name.contains('\n') {
            continue;
        }
        let (ty, mods) = classify(a, defs, index, o);
        spans.push((o.span.start, o.span.end, ty, mods));
    }
    encode(text, &spans)
}

/// A name's token type and modifiers (spec §7.2).
fn classify(a: &Analysis, defs: &Definitions, index: &Index, o: &Occurrence) -> (u32, u32) {
    let std = |id: DefId| {
        let file = defs.def(id).span.file;
        if a.db.get_name(file).is_some_and(|n| n.starts_with("<std/")) {
            DEFAULT_LIBRARY
        } else {
            0
        }
    };
    let (ty, mods) = match o.target {
        Target::Def(id) => match defs.def(id).kind {
            DefKind::Fn { .. } | DefKind::ExternFn { .. } => (FUNCTION, std(id)),
            DefKind::Method { .. } => (METHOD, std(id)),
            DefKind::Record { .. } => (STRUCT, std(id)),
            DefKind::Sum { .. } => (ENUM, std(id)),
            DefKind::Trait { .. } => (INTERFACE, std(id)),
            DefKind::Const { .. } => (VARIABLE, std(id) | READONLY),
            DefKind::AssocType { .. } => (TYPE, std(id)),
        },
        Target::Variant(sum, _) => (ENUM_MEMBER, std(sum)),
        Target::Field(record, _) => (PROPERTY, std(record)),
        Target::TraitMethod(tr, _) => (METHOD, std(tr)),
        Target::Local(decl) => {
            let flags = index.locals.get(&decl).copied();
            let ty = if flags.is_some_and(|f| f.parameter) {
                PARAMETER
            } else {
                VARIABLE
            };
            (
                ty,
                if flags.is_some_and(|f| f.mutable) {
                    MUTABLE
                } else {
                    0
                },
            )
        }
        Target::TypeParam(_) => (TYPE_PARAMETER, 0),
        Target::Module(_) => (NAMESPACE, 0),
        Target::Builtin(_) => (FUNCTION, DEFAULT_LIBRARY),
        Target::BuiltinMethod(_) => (METHOD, DEFAULT_LIBRARY),
        Target::Primitive(_) => (TYPE, DEFAULT_LIBRARY),
    };
    let declaration = if o.role == Role::Declaration {
        DECLARATION
    } else {
        0
    };
    (ty, mods | declaration)
}

/// `spans` (start, end, type, modifiers), sorted by start, as the
/// protocol's relative tokens, columns and lengths in UTF-16 (spec §7.3).
fn encode(text: &str, spans: &[(u32, u32, u32, u32)]) -> Vec<lsp::SemanticToken> {
    let lines = LineIndex::new(text);
    let (mut prev_line, mut prev_col) = (0u32, 0u32);
    let mut out = Vec::new();
    for &(start, end, ty, mods) in spans {
        let (line, col) = lines.position(start);
        let length = text[start as usize..end as usize].encode_utf16().count() as u32;
        let delta_line = line - prev_line;
        let delta_start = if delta_line == 0 { col - prev_col } else { col };
        out.push(lsp::SemanticToken {
            delta_line,
            delta_start,
            length,
            token_type: ty,
            token_modifiers_bitset: mods,
        });
        prev_line = line;
        prev_col = col;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace::Overlay;

    fn answer(name: &str, files: &[(&str, &str)]) -> Answer {
        let dir = std::env::temp_dir().join(format!("nova-lsp-tokens-{name}"));
        let mut overlay = Overlay::default();
        for (file, text) in files {
            overlay = overlay.with(&dir.join(file), text.to_string());
        }
        let options = crate::analysis::options(None, true);
        crate::analysis::answering(&dir.join(files[0].0), &overlay, &options).expect("an answer")
    }

    /// Each token: its line, its UTF-16 column, its byte offset, its type's
    /// name and its modifiers' names.
    type Decoded = Vec<(u32, u32, usize, String, Vec<String>)>;

    fn decoded(text: &str, tokens: &[lsp::SemanticToken]) -> Decoded {
        let lines = LineIndex::new(text);
        let legend = legend();
        let (mut line, mut col) = (0, 0);
        tokens
            .iter()
            .map(|t| {
                line += t.delta_line;
                col = if t.delta_line == 0 {
                    col + t.delta_start
                } else {
                    t.delta_start
                };
                let ty = legend.token_types[t.token_type as usize]
                    .as_str()
                    .to_string();
                let mods = legend
                    .token_modifiers
                    .iter()
                    .enumerate()
                    .filter(|(i, _)| t.token_modifiers_bitset & (1 << i) != 0)
                    .map(|(_, m)| m.as_str().to_string())
                    .collect();
                (line, col, lines.offset(line, col) as usize, ty, mods)
            })
            .collect()
    }

    /// The token at the `n`th whole `word` of `text`: its type and modifiers.
    #[track_caller]
    fn token(found: &Decoded, text: &str, word: &str, n: usize) -> (String, Vec<String>) {
        let ident = |c: Option<char>| c.is_some_and(|c| c.is_alphanumeric() || c == '_');
        let start = text
            .match_indices(word)
            .map(|(i, _)| i)
            .filter(|&i| {
                !ident(text[..i].chars().next_back())
                    && !ident(text[i + word.len()..].chars().next())
            })
            .nth(n)
            .unwrap_or_else(|| panic!("no `{word}` number {n}"));
        found
            .iter()
            .find(|t| t.2 == start)
            .map(|t| (t.3.clone(), t.4.clone()))
            .unwrap_or_else(|| panic!("no token at `{word}` number {n}: {found:?}"))
    }

    fn of(ty: &str, mods: &[&str]) -> (String, Vec<String>) {
        (ty.to_string(), mods.iter().map(|m| m.to_string()).collect())
    }

    const EVERY_ROW: &str = "import geometry::{origin}\n\nconst LIMIT: Int = 3\n\nrecord Point { x: Int }\n\ntype Shape =\n  | Empty\n\ntrait Show {\n    fn show(self) -> String\n}\n\nimpl Show for Point {\n    fn show(self) -> String {\n        \"p\"\n    }\n}\n\nfn first<T>(mut x: T) -> T {\n    x\n}\n\nfn main() {\n    let mut n = LIMIT\n    n = n + origin()\n    let p = Point { x: n }\n    let s = Shape::Empty\n    let o = Some(1)\n    println(p.show())\n    let k = [1].len()\n}\n";

    #[test]
    fn every_row_of_the_table() {
        let a = answer(
            "every-row",
            &[
                ("main.nova", EVERY_ROW),
                ("geometry.nova", "pub fn origin() -> Int {\n    1\n}\n"),
            ],
        );
        let text = EVERY_ROW;
        let found = decoded(text, &tokens(&a));
        let decl = "declaration";
        assert_eq!(token(&found, text, "geometry", 0), of("namespace", &[]));
        assert_eq!(token(&found, text, "origin", 0), of("function", &[]));
        assert_eq!(
            token(&found, text, "LIMIT", 0),
            of("variable", &[decl, "readonly"])
        );
        assert_eq!(
            token(&found, text, "Int", 0),
            of("type", &["defaultLibrary"])
        );
        assert_eq!(token(&found, text, "Point", 0), of("struct", &[decl]));
        assert_eq!(token(&found, text, "x", 0), of("property", &[decl]));
        assert_eq!(token(&found, text, "Shape", 0), of("enum", &[decl]));
        assert_eq!(token(&found, text, "Empty", 0), of("enumMember", &[decl]));
        assert_eq!(token(&found, text, "Show", 0), of("interface", &[decl]));
        assert_eq!(token(&found, text, "show", 0), of("method", &[decl]));
        assert_eq!(token(&found, text, "show", 1), of("method", &[decl]));
        assert_eq!(token(&found, text, "first", 0), of("function", &[decl]));
        assert_eq!(token(&found, text, "T", 0), of("typeParameter", &[decl]));
        assert_eq!(
            token(&found, text, "x", 1),
            of("parameter", &[decl, "mutable"])
        );
        assert_eq!(
            token(&found, text, "n", 0),
            of("variable", &[decl, "mutable"])
        );
        assert_eq!(token(&found, text, "n", 1), of("variable", &["mutable"]));
        assert_eq!(
            token(&found, text, "LIMIT", 1),
            of("variable", &["readonly"])
        );
        assert_eq!(
            token(&found, text, "Some", 0),
            of("enumMember", &["defaultLibrary"])
        );
        assert_eq!(
            token(&found, text, "println", 0),
            of("function", &["defaultLibrary"])
        );
        assert_eq!(token(&found, text, "show", 2), of("method", &[]));
        assert_eq!(
            token(&found, text, "len", 0),
            of("method", &["defaultLibrary"])
        );
        // `self` is the grammar's.
        let at_self = text.find("self").unwrap();
        assert!(found.iter().all(|t| t.2 != at_self), "{found:?}");
    }

    #[test]
    fn one_token_per_span_the_value_wins() {
        let text = "record P { x: Int }\n\nfn main() {\n    let x = 1\n    let p = P { x }\n}\n";
        let found = decoded(text, &tokens(&answer("shorthand", &[("main.nova", text)])));
        assert_eq!(token(&found, text, "x", 2), of("variable", &[]));
        let at = text.rfind("x }").unwrap();
        assert_eq!(found.iter().filter(|t| t.2 == at).count(), 1, "{found:?}");
    }

    #[test]
    fn tokens_count_utf16_after_thai_and_an_emoji() {
        // Review Focus 2: `n` follows three Thai characters, one UTF-16 unit
        // each, and an emoji, two.
        let text = "fn f(a: String, b: Int) -> Int {\n    b\n}\n\nfn main() {\n    let n = 1\n    let s = f(\"ไทย😀\", n)\n}\n";
        let found = decoded(text, &tokens(&answer("utf16", &[("main.nova", text)])));
        let at = text.rfind("n)").unwrap();
        let t = found
            .iter()
            .find(|t| t.2 == at)
            .unwrap_or_else(|| panic!("{found:?}"));
        assert_eq!((t.0, t.1, t.3.as_str()), (6, 23, "variable"));
    }

    #[test]
    fn encode_writes_relative_deltas() {
        let found = encode("ab cd\nef", &[(0, 2, 0, 0), (3, 5, 1, 1), (6, 8, 2, 0)]);
        let raw: Vec<(u32, u32, u32, u32, u32)> = found
            .iter()
            .map(|t| {
                (
                    t.delta_line,
                    t.delta_start,
                    t.length,
                    t.token_type,
                    t.token_modifiers_bitset,
                )
            })
            .collect();
        assert_eq!(raw, [(0, 0, 2, 0, 0), (0, 3, 2, 1, 1), (1, 0, 2, 2, 0)]);
    }
}
