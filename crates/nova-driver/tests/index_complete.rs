//! Every name in std, `examples/` and `tests/runtime/` is in the index
//! (spec `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`
//! §8.2):
//! 1. every use has exactly one declaration, unless it is a builtin, a
//!    builtin method, a primitive or a module;
//! 2. every identifier token lies inside an occurrence, two for a
//!    shorthand, apart from the names `skipped` lists.

use std::collections::HashMap;
use std::path::PathBuf;

use nova_diagnostics::{FileId, Severity, Span, Spanned};
use nova_driver::{analyze, Analysis, DiskSources, Options};
use nova_lexer::Token;
use nova_resolver::{Role, Target};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// Every program the checks analyse, sorted.
fn programs() -> Vec<PathBuf> {
    let root = root();
    let mut out = Vec::new();
    for entry in std::fs::read_dir(root.join("examples")).unwrap() {
        let main = entry.unwrap().path().join("src").join("main.nova");
        if main.is_file() {
            out.push(main);
        }
    }
    for entry in std::fs::read_dir(root.join("tests").join("runtime")).unwrap() {
        let path = entry.unwrap().path();
        // Meant to fail `nova check` with E0089 (run_tests.rs).
        if path.file_name().is_some_and(|n| n == "bytes_reserved.nova") {
            continue;
        }
        if path.extension().is_some_and(|e| e == "nova") {
            out.push(path);
        }
    }
    // One program of three files, which holds the corpora's only imports.
    out.push(
        root.join("tests")
            .join("runtime")
            .join("modules")
            .join("main.nova"),
    );
    out.sort();
    out
}

fn options() -> Options {
    Options {
        keep_going: true,
        tests: true,
        module_only: true,
        index: true,
        ..Options::default()
    }
}

/// std's files in `a`, which can be checked only as std (spec §8.2).
fn std_files(a: &Analysis) -> Vec<FileId> {
    nova_resolver::STD_MODULES
        .iter()
        .chain(std::iter::once(&nova_resolver::STD_TEST_MODULE))
        .map(|(name, _)| {
            let short = name.strip_prefix("$std.").unwrap_or(name);
            a.db.id_of(&format!("<std/{short}>"))
                .unwrap_or_else(|| panic!("<std/{short}> is not in the analysis"))
        })
        .collect()
}

fn place(a: &Analysis, span: Span) -> String {
    let name = a.db.get_name(span.file).unwrap_or("?");
    let (line, column) = a.db.location(span.file, span.start).unwrap_or((0, 0));
    let text = a.db.get_source(span.file).unwrap_or("");
    let word = text
        .get(span.start as usize..span.end as usize)
        .unwrap_or("?");
    format!("{name}:{line}:{column} `{word}`")
}

/// Check 1, over the whole analysis.
fn every_use_has_its_declaration(a: &Analysis, failures: &mut Vec<String>) {
    let index = a.index.as_ref().unwrap();
    let mut declared: HashMap<Target, usize> = HashMap::new();
    for o in &index.occurrences {
        if o.role == Role::Declaration {
            *declared.entry(o.target).or_default() += 1;
        }
    }
    for o in &index.occurrences {
        if o.role != Role::Use {
            continue;
        }
        if matches!(
            o.target,
            Target::Builtin(_)
                | Target::BuiltinMethod(_)
                | Target::Primitive(_)
                | Target::Module(_)
        ) {
            continue;
        }
        let n = declared.get(&o.target).copied().unwrap_or(0);
        if n != 1 {
            failures.push(format!(
                "{}: {:?} has {n} declarations",
                place(a, o.span),
                o.target
            ));
        }
    }
}

/// The identifiers check 2 skips, each with its reason (spec §8.2).
fn skipped(tokens: &[Spanned<Token>], i: usize) -> bool {
    let Token::Ident(name) = &tokens[i].value else {
        return true;
    };
    // `_` declares nothing.
    if name == "_" {
        return true;
    }
    let prev = i.checked_sub(1).map(|j| &tokens[j].value);
    // An attribute's name, `@test`.
    if matches!(prev, Some(Token::At)) {
        return true;
    }
    // A `module m` declaration's name, which the resolver ignores.
    if matches!(prev, Some(Token::Module)) {
        return true;
    }
    // `component` and `view` where they are keywords (the 4.4 spec §6):
    // before a name and `(`, and before `{`.
    let next = |k: usize| tokens.get(i + k).map(|t| &t.value);
    if name == "component"
        && matches!(next(1), Some(Token::Ident(_)))
        && matches!(next(2), Some(Token::LParen))
    {
        return true;
    }
    if name == "view" && matches!(next(1), Some(Token::LBrace)) {
        return true;
    }
    // A named argument's name in an element, `div(class: "a")`: the
    // attribute it names is no symbol.
    if in_element_arguments(tokens, i) {
        return true;
    }
    // An attribute's arguments, `@test(should_panic)`.
    in_attribute_arguments(tokens, i)
}

/// Whether the identifier at `i` names an element's argument: it is
/// followed by `:`, starts an argument, and the innermost open bracket is
/// the `(` of a call, not of a `fn` or a `component`'s parameters.
fn in_element_arguments(tokens: &[Spanned<Token>], i: usize) -> bool {
    if !matches!(tokens.get(i + 1).map(|t| &t.value), Some(Token::Colon)) {
        return false;
    }
    if !matches!(
        i.checked_sub(1).map(|j| &tokens[j].value),
        Some(Token::LParen | Token::Comma)
    ) {
        return false;
    }
    let mut depth = 0usize;
    let mut j = i;
    while j > 0 {
        j -= 1;
        match &tokens[j].value {
            Token::RParen | Token::RBracket | Token::RBrace => depth += 1,
            Token::LParen if depth == 0 => {
                let callee = j.checked_sub(1).map(|k| &tokens[k].value);
                let before = j.checked_sub(2).map(|k| &tokens[k].value);
                return matches!(callee, Some(Token::Ident(_)))
                    && !matches!(before, Some(Token::Fn))
                    && !matches!(before, Some(Token::Ident(w)) if w == "component");
            }
            Token::LParen | Token::LBracket | Token::LBrace => {
                if depth == 0 {
                    return false;
                }
                depth -= 1;
            }
            _ => {}
        }
    }
    false
}

fn in_attribute_arguments(tokens: &[Spanned<Token>], i: usize) -> bool {
    let mut j = i;
    while j > 0 {
        j -= 1;
        match tokens[j].value {
            Token::LParen => {
                return j >= 2
                    && matches!(tokens[j - 1].value, Token::Ident(_))
                    && matches!(tokens[j - 2].value, Token::At);
            }
            Token::Ident(_) | Token::Comma => continue,
            _ => return false,
        }
    }
    false
}

/// Check 2, over one file.
fn every_name_is_covered(a: &Analysis, file: FileId, failures: &mut Vec<String>) {
    let index = a.index.as_ref().unwrap();
    let mut at: HashMap<Span, (usize, bool)> = HashMap::new();
    for o in index.occurrences.iter().filter(|o| o.span.file == file) {
        let slot = at.entry(o.span).or_default();
        slot.0 += 1;
        slot.1 |= o.shorthand;
    }
    let source = a.db.get_source(file).unwrap();
    let (tokens, _) = nova_lexer::lex(source, file);
    for i in 0..tokens.len() {
        if skipped(&tokens, i) {
            continue;
        }
        let span = tokens[i].span;
        match at.get(&span) {
            None => failures.push(format!("{}: no occurrence", place(a, span))),
            Some(&(n, true)) if n != 2 => {
                failures.push(format!(
                    "{}: a shorthand with {n} occurrences",
                    place(a, span)
                ));
            }
            Some(_) => {}
        }
    }
}

#[test]
fn every_name_in_the_corpora_is_indexed() {
    let programs = programs();
    // 8 examples, 142 top-level runtime programs and the modules program,
    // on 2026-10-17, after 4.4 added `06-counter-spa` and `ui_view`.
    assert_eq!(programs.len(), 151, "{programs:#?}");
    let mut files = 0;
    let mut failures: Vec<String> = Vec::new();
    for (k, path) in programs.iter().enumerate() {
        let a = analyze(path, &DiskSources, &options()).unwrap();
        let errors: Vec<String> = a
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .map(|d| format!("{} {}", d.code, d.message))
            .collect();
        assert!(
            errors.is_empty(),
            "{} does not check cleanly: {errors:?}",
            path.display()
        );
        every_use_has_its_declaration(&a, &mut failures);
        let mut own: Vec<FileId> = a.modules.iter().map(|(f, _)| *f).collect();
        // std is the same in every analysis: check its files once.
        if k == 0 {
            own.extend(std_files(&a));
        }
        files += own.len();
        for file in own {
            every_name_is_covered(&a, file, &mut failures);
        }
    }
    // 8 + 142 + 3 files of user code, 05-geometry's library's 2, and
    // native std's 20.
    assert_eq!(files, 175);
    failures.sort();
    failures.dedup();
    assert!(
        failures.is_empty(),
        "{} problem(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}
