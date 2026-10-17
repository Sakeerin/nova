//! Doc tests: the Nova code blocks in the docs a library's site shows,
//! each as a test module (spec
//! `docs/superpowers/specs/2026-10-11-phase-3-5b-doc-tests-design.md` §4
//! to §7).

use std::path::{Component, Path};

use nova_diagnostics::{FileDb, FileId, Spanned};
use nova_lexer::Token;

use crate::markdown::{self, Block, CodeBlock, DocText, Docs};
use crate::model::{self, Failure, Impl, Package};

// The test function each doc test module declares (spec §5.2), which
// the driver collects.
use nova_driver::DOC_TEST_FUNCTION as FUNCTION;

/// Where a line of a doc test's text comes from (spec §7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// A line the doc test adds: the package's import, or the test
    /// function's.
    Added,
    /// A line of the block.
    Code {
        /// The offset in the file that its column 0 maps to.
        at: u32,
        /// For a hidden line, the column its `# ` was cut at, and how many
        /// bytes were cut.
        cut: Option<(u32, u32)>,
    },
}

/// A doc test: one Nova code block of the docs a library's site shows, as
/// a test module (spec §5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocTest {
    /// `shapes::circle (src/lib.nova:41)` (spec §5.3).
    pub name: String,
    /// `src/lib.nova:41`: where the block starts, which an unknown word's
    /// warning names (spec §4.2).
    pub place: String,
    /// The module's text (spec §5), ending in a line break.
    pub text: String,
    /// The block's file, by its name in the program's `FileDb`: the path
    /// the loader read it from, as `Path::display` writes it, or
    /// `<std/time>` for std's.
    pub file: String,
    /// Where the block starts in `file`: its opening fence, or an indented
    /// block's first line, which added lines map to (spec §7).
    pub start: u32,
    /// Where each line of `text` comes from.
    pub lines: Vec<Origin>,
    /// The words after `nova` that mean nothing (spec §4.2).
    pub unknown: Vec<String>,
}

impl DocTest {
    /// Where byte `offset` of `text` is in `file` (spec §7). A hidden
    /// line's column after its cut moves past its `# `, and an added line
    /// is at `start`.
    pub fn map(&self, offset: u32) -> u32 {
        let bytes = self.text.as_bytes();
        let offset = (offset as usize).min(bytes.len());
        let line = bytes[..offset].iter().filter(|&&b| b == b'\n').count();
        let line_start = bytes[..offset]
            .iter()
            .rposition(|&b| b == b'\n')
            .map_or(0, |i| i + 1);
        let col = (offset - line_start) as u32;
        match self.lines.get(line) {
            Some(Origin::Code {
                at,
                cut: Some((c, len)),
            }) if col >= *c => at + col + len,
            Some(Origin::Code { at, .. }) => at + col,
            _ => self.start,
        }
    }
}

/// The doc tests of the package whose `nova.toml` is in `root` (spec
/// §4.1), in the site's order; `None` when it has no library.
pub fn package_doc_tests(root: &Path) -> Result<Option<Vec<DocTest>>, Failure> {
    let Some((package, a)) = model::library_package(root)? else {
        return Ok(None);
    };
    let Some(graph) = a.graph.as_ref() else {
        return Ok(Some(Vec::new()));
    };
    let (dir, import) = (&graph.root().dir, graph.root().import_name.as_str());
    let shown = |file: FileId| match a.modules.iter().find(|(f, _)| *f == file) {
        Some((_, path)) => slashed(path.strip_prefix(dir).unwrap_or(path)),
        None => a.db.get_name(file).unwrap_or("?").to_string(),
    };
    Ok(Some(collect(&package, &a.db, Some(import), &shown)))
}

/// std's doc tests (spec §6.1, `--std`), in the site's order.
pub fn std_doc_tests() -> Result<Vec<DocTest>, Failure> {
    let (package, a) = model::std_package()?;
    let shown = |file: FileId| std_place(a.db.get_name(file).unwrap_or("?"));
    Ok(collect(&package, &a.db, None, &shown))
}

/// A std module's file as a doc test's name shows it: `<std/time>` is
/// `std/time/lib.nova`, and `<std/ui.html>` `std/ui/html/lib.nova` (spec
/// §5.3, §7; spec 4.3 §3).
fn std_place(name: &str) -> String {
    match name.strip_prefix("<std/").and_then(|n| n.strip_suffix('>')) {
        Some(m) => format!("std/{}/lib.nova", m.replace('.', "/")),
        None => name.to_string(),
    }
}

/// `path`'s parts, joined with `/` (spec §5.3).
fn slashed(path: &Path) -> String {
    path.components()
        .filter_map(|c| match c {
            Component::Normal(s) => s.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

/// The doc tests of `package`'s docs. `import` is the package's import
/// name, `None` for std; `shown` names a file as a doc test's name shows
/// it.
fn collect(
    package: &Package,
    db: &FileDb,
    import: Option<&str>,
    shown: &dyn Fn(FileId) -> String,
) -> Vec<DocTest> {
    let mut tests = Vec::new();
    for (path, docs) in documented(package) {
        tests.extend(doc_tests_of(&path, &docs.source, db, import, shown));
    }
    tests
}

/// Every documented thing's docs, by the path a doc test of it is named
/// by (spec §5.3): the search index's paths, in the site's order. Docs no
/// file holds are among them, and give no doc test.
fn documented(package: &Package) -> Vec<(String, &Docs)> {
    fn impl_docs<'p>(out: &mut Vec<(String, &'p Docs)>, path: &str, i: &'p Impl) {
        out.push((path.to_string(), &i.docs));
        for m in &i.methods {
            out.push((format!("{path}::{}", m.name), &m.docs));
        }
    }
    let mut out = Vec::new();
    for m in &package.modules {
        out.push((m.qual.clone(), &m.docs));
        for it in &m.items {
            let path = format!("{}::{}", m.qual, it.name);
            out.push((path.clone(), &it.docs));
            for member in &it.members {
                out.push((format!("{path}::{}", member.name), &member.docs));
            }
            for i in &it.impls {
                impl_docs(&mut out, &path, i);
            }
        }
        for i in &m.other_impls {
            impl_docs(&mut out, &format!("{}::{}", m.qual, i.on), i);
        }
    }
    for pr in &package.primitives {
        for i in &pr.impls {
            impl_docs(&mut out, &format!("std::{}", pr.name), i);
        }
    }
    out
}

/// The doc tests of one doc text, `path`'s: its blocks that run (spec
/// §4.2).
fn doc_tests_of(
    path: &str,
    doc: &DocText,
    db: &FileDb,
    import: Option<&str>,
    shown: &dyn Fn(FileId) -> String,
) -> Vec<DocTest> {
    let Some(file) = doc.file else {
        return Vec::new();
    };
    let Some(file_name) = db.get_name(file) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for b in markdown::code_blocks(&doc.text) {
        let Block::Test { unknown } = &b.block else {
            continue;
        };
        let Some(start) = doc.source(b.start) else {
            continue;
        };
        let line = db.location(file, start).map_or(1, |(l, _)| l);
        let place = format!("{}:{line}", shown(file));
        let (text, lines) = module(import, run_lines(&b, doc));
        out.push(DocTest {
            name: format!("{path} ({place})"),
            place,
            text,
            file: file_name.to_string(),
            start,
            lines,
            unknown: unknown.clone(),
        });
    }
    out
}

/// A block's line as it runs (spec §4.3), and where it comes from.
struct RunLine {
    text: String,
    origin: Origin,
}

/// `b`'s lines as they run, each mapped through `doc` into its file: a
/// hidden line less its `# `.
fn run_lines(b: &CodeBlock, doc: &DocText) -> Vec<RunLine> {
    b.lines
        .iter()
        .map(|l| {
            // `text[pad..]` starts at `doc.source(l.at)`; a made-up column
            // maps into the tab before it.
            let at = doc.source(l.at).unwrap_or(0).saturating_sub(l.pad as u32);
            match markdown::hidden(&l.text) {
                Some((cut, len)) => RunLine {
                    text: format!("{}{}", &l.text[..cut], &l.text[cut + len..]),
                    origin: Origin::Code {
                        at,
                        cut: Some((cut as u32, len as u32)),
                    },
                },
                None => RunLine {
                    text: l.text.clone(),
                    origin: Origin::Code { at, cut: None },
                },
            }
        })
        .collect()
}

/// A block's test module (spec §5), and where each of its lines comes
/// from. `import` is the package's import name, `None` for std's. Lines
/// keep their columns: none is indented.
fn module(import: Option<&str>, lines: Vec<RunLine>) -> (String, Vec<Origin>) {
    let code = lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    // Comments and strings are not tokens, so `import` in either is not
    // an import.
    let (tokens, _) = nova_lexer::lex(&code, FileId::DUMMY);
    let hoisted = leading_imports(&code, &tokens);
    let main = declares_main(&tokens);
    let mut out: Vec<(String, Origin)> = Vec::new();
    if let Some(p) = import {
        if !imports(&tokens, p) {
            out.push((format!("import {p}"), Origin::Added));
        }
    }
    let mut rest = lines.into_iter();
    out.extend(rest.by_ref().take(hoisted).map(|l| (l.text, l.origin)));
    match main {
        Some(is_async) => {
            out.extend(rest.map(|l| (l.text, l.origin)));
            // What `main` returns is dropped, as a program drops it.
            let call = if is_async {
                "let _ = block_on(main())"
            } else {
                "let _ = main()"
            };
            out.push((format!("@test fn {FUNCTION}() {{ {call} }}"), Origin::Added));
        }
        None => {
            out.push((format!("@test fn {FUNCTION}() {{"), Origin::Added));
            out.extend(rest.map(|l| (l.text, l.origin)));
            out.push(("}".to_string(), Origin::Added));
        }
    }
    let mut text = out
        .iter()
        .map(|(t, _)| t.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    text.push('\n');
    (text, out.into_iter().map(|(_, o)| o).collect())
}

/// The index of the token after the import at `tokens[i]`: `import`, a
/// path, then `::{…}`, `{…}` or `as name`, and a `;`.
fn import_end(tokens: &[Spanned<Token>], i: usize) -> usize {
    let at = |j: usize| tokens.get(j).map(|t| &t.value);
    let mut j = i + 1;
    if matches!(at(j), Some(Token::Ident(_))) {
        j += 1;
    }
    while at(j) == Some(&Token::ColonColon) && matches!(at(j + 1), Some(Token::Ident(_))) {
        j += 2;
    }
    if at(j) == Some(&Token::ColonColon) {
        j += 1;
    }
    match at(j) {
        Some(Token::LBrace) => {
            while !matches!(at(j), None | Some(Token::RBrace) | Some(Token::Eof)) {
                j += 1;
            }
            if at(j) == Some(&Token::RBrace) {
                j += 1;
            }
        }
        Some(Token::As) => {
            j += 1;
            if matches!(at(j), Some(Token::Ident(_))) {
                j += 1;
            }
        }
        _ => {}
    }
    if at(j) == Some(&Token::Semicolon) {
        j += 1;
    }
    j
}

/// How many lines at the top of `code` are its leading imports, with the
/// blank and comment lines among them (spec §5.1). An import whose last
/// line goes on with other code stops them.
fn leading_imports(code: &str, tokens: &[Spanned<Token>]) -> usize {
    let line = |offset: u32| {
        code.as_bytes()[..(offset as usize).min(code.len())]
            .iter()
            .filter(|&&b| b == b'\n')
            .count()
    };
    let mut lines = 0;
    let mut i = 0;
    while tokens.get(i).is_some_and(|t| t.value == Token::Import) {
        let end = import_end(tokens, i);
        let last = line(tokens[end - 1].span.end.saturating_sub(1));
        let next = &tokens[end.min(tokens.len() - 1)];
        if next.value != Token::Eof && line(next.span.start) <= last {
            break;
        }
        lines = last + 1;
        i = end;
    }
    lines
}

/// Whether `tokens` declare `fn main` at the top level, and whether it is
/// `async` (spec §5.2).
fn declares_main(tokens: &[Spanned<Token>]) -> Option<bool> {
    let mut depth = 0usize;
    for (i, t) in tokens.iter().enumerate() {
        match &t.value {
            Token::LBrace => depth += 1,
            Token::RBrace => depth = depth.saturating_sub(1),
            Token::Fn if depth == 0 => {
                let named_main = matches!(
                    tokens.get(i + 1).map(|t| &t.value),
                    Some(Token::Ident(n)) if n == "main"
                );
                if named_main {
                    return Some(i > 0 && tokens[i - 1].value == Token::Async);
                }
            }
            _ => {}
        }
    }
    None
}

/// Whether `tokens` import the package `name` itself (spec §5.1):
/// `import name`, `import name::{…}` or `import name as …`.
fn imports(tokens: &[Spanned<Token>], name: &str) -> bool {
    tokens
        .windows(2)
        .any(|w| w[0].value == Token::Import && matches!(&w[1].value, Token::Ident(n) if n == name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signature::FileText;
    use std::path::PathBuf;

    /// A fresh, empty directory with a fixed name, so each run replaces the
    /// last.
    fn fresh(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nova-doc-doctest-{name}"));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(dir: &Path, files: &[(&str, &str)]) {
        for (path, text) in files {
            let path = dir.join(path);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, text).unwrap();
        }
    }

    fn manifest(name: &str) -> String {
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n")
    }

    /// [`package_doc_tests`], its diagnostics rendered when it fails.
    fn doc_tests(root: &Path) -> Option<Vec<DocTest>> {
        match package_doc_tests(root) {
            Ok(tests) => tests,
            Err(Failure::Diagnostics { db, diagnostics }) => panic!(
                "{}",
                nova_diagnostics::render::render_to_string(&db, &diagnostics)
            ),
            Err(Failure::Other(why)) => panic!("{why}"),
        }
    }

    /// Lines that run, line `i` from source offset `100 * i`.
    fn lines(texts: &[&str]) -> Vec<RunLine> {
        texts
            .iter()
            .enumerate()
            .map(|(i, t)| RunLine {
                text: t.to_string(),
                origin: Origin::Code {
                    at: 100 * i as u32,
                    cut: None,
                },
            })
            .collect()
    }

    /// The `///` lines of `src`, read as `file`.
    fn doc_lines(src: &str, file: FileId) -> Vec<Spanned<String>> {
        nova_lexer::lex(src, file)
            .0
            .into_iter()
            .filter_map(|t| match t.value {
                Token::DocComment(s) => Some(Spanned::new(s, t.span)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn a_block_becomes_a_module() {
        // Spec §5.1 and §5.2. Mutant 3: the package import is not doubled.
        let code = |at: u32| Origin::Code { at, cut: None };
        let (text, origins) = module(
            Some("shapes"),
            lines(&[
                "// Measure.",
                "import shapes::{circle}",
                "",
                "let c = circle(1.0)",
            ]),
        );
        assert_eq!(
            text,
            "// Measure.\nimport shapes::{circle}\n@test fn nova_doctest() {\n\n\
             let c = circle(1.0)\n}\n"
        );
        assert_eq!(
            origins,
            [
                code(0),
                code(100),
                Origin::Added,
                code(200),
                code(300),
                Origin::Added
            ]
        );
        for (import, block, expected) in [
            (
                Some("shapes"),
                &["let c = circle(1.0)"][..],
                "import shapes\n@test fn nova_doctest() {\nlet c = circle(1.0)\n}\n",
            ),
            (
                Some("shapes"),
                &["import shapes", "let c = circle(1.0)"][..],
                "import shapes\n@test fn nova_doctest() {\nlet c = circle(1.0)\n}\n",
            ),
            (
                Some("shapes"),
                &["let s = \"import shapes\" // import shapes"][..],
                "import shapes\n@test fn nova_doctest() {\n\
                 let s = \"import shapes\" // import shapes\n}\n",
            ),
            (
                Some("shapes"),
                &["fn main() {", "    let a = 1", "}"][..],
                "import shapes\nfn main() {\n    let a = 1\n}\n\
                 @test fn nova_doctest() { let _ = main() }\n",
            ),
            (
                Some("shapes"),
                &["async fn main() {", "}"][..],
                "import shapes\nasync fn main() {\n}\n\
                 @test fn nova_doctest() { let _ = block_on(main()) }\n",
            ),
            (
                Some("shapes"),
                &["fn helper() { fn main() {} }", "helper()"][..],
                "import shapes\n@test fn nova_doctest() {\nfn helper() { fn main() {} }\n\
                 helper()\n}\n",
            ),
            (
                None,
                &["import a::{", "  b,", "}", "f()"][..],
                "import a::{\n  b,\n}\n@test fn nova_doctest() {\nf()\n}\n",
            ),
            (
                None,
                &["import x; let a = 1", "let b = 2"][..],
                "@test fn nova_doctest() {\nimport x; let a = 1\nlet b = 2\n}\n",
            ),
        ] {
            assert_eq!(module(import, lines(block)).0, expected, "{block:?}");
        }
        let (_, origins) = module(Some("shapes"), lines(&["fn main() {", "}"]));
        assert_eq!(origins, [Origin::Added, code(0), code(100), Origin::Added]);
    }

    /// Serves `text` at every path.
    struct Memory(String);

    impl nova_driver::Sources for Memory {
        fn read(&self, _: &Path) -> std::io::Result<String> {
            Ok(self.0.clone())
        }
    }

    #[test]
    fn a_main_that_returns_a_value_runs_as_a_program_would() {
        // Spec §5.2: the test calls `main` and drops what it returns, so a
        // `main` that returns a value checks, as it does in a program.
        for (block, call) in [
            ("fn main() -> Int { 1 }", "let _ = main()"),
            ("async fn main() -> Int { 1 }", "let _ = block_on(main())"),
            ("fn main() {}", "let _ = main()"),
        ] {
            let (text, _) = module(None, lines(&[block]));
            let program = nova_driver::Program::doc_tests(
                None,
                vec![PathBuf::from("<doc tests>").join("doc0.nova")],
            );
            let options = nova_driver::Options {
                tests: true,
                ..nova_driver::Options::default()
            };
            let a = nova_driver::analyze_program(program, &Memory(text.clone()), &options)
                .expect("the module reads");
            let errors: Vec<&str> = a
                .diagnostics
                .iter()
                .filter(|d| d.severity == nova_diagnostics::Severity::Error)
                .map(|d| d.message.as_str())
                .collect();
            assert_eq!(errors, Vec::<&str>::new(), "{block}");
            assert_eq!(
                text,
                format!("{block}\n@test fn nova_doctest() {{ {call} }}\n")
            );
        }
    }

    #[test]
    fn the_line_map_points_into_the_doc_comment() {
        // Spec §7: a body line; a hidden line, less its `# `; and an added
        // line, at the opening fence or an indented block's first line.
        // Plain, CRLF, in a list, in a quote, and indented by a tab.
        for (src, line) in [
            (
                "/// A circle.\n///\n/// ```\n/// # let r = 1.0\n/// let c = circle(r)\n/// ```\n\
                 pub fn circle(r: Float) -> Float { r }\n",
                3,
            ),
            (
                "/// A circle.\r\n///\r\n/// ```\r\n/// # let r = 1.0\r\n/// let c = circle(r)\r\n\
                 /// ```\r\npub fn circle(r: Float) -> Float { r }\r\n",
                3,
            ),
            (
                "    /// - In a list:\n    ///\n    ///   ```\n    ///   # let r = 1.0\n    \
                 ///   let c = circle(r)\n    ///   ```\n    pub fn circle(r: Float) -> Float { r }\n",
                3,
            ),
            (
                "///> ```\n///> # let r = 1.0\n///> let c = circle(r)\n///> ```\nfn f() {}\n",
                1,
            ),
            ("///\tlet r = 1.0\n///\tlet c = circle(r)\nfn f() {}\n", 1),
        ] {
            let mut db = FileDb::new();
            let file = db.add("src/lib.nova", src);
            let doc = markdown::join(&doc_lines(src, file));
            let tests = doc_tests_of("shapes::circle", &doc, &db, Some("shapes"), &|_| {
                "src/lib.nova".to_string()
            });
            assert_eq!(tests.len(), 1, "{src:?}");
            let t = &tests[0];
            assert_eq!(t.name, format!("shapes::circle (src/lib.nova:{line})"));
            assert_eq!(t.place, format!("src/lib.nova:{line}"));
            assert_eq!(t.file, "src/lib.nova");
            // Mutant 2: a hidden line runs less its `# `.
            assert_eq!(
                t.text,
                "import shapes\n@test fn nova_doctest() {\nlet r = 1.0\nlet c = circle(r)\n}\n",
                "{src:?}"
            );
            let at = |s: &str| src.find(s).unwrap() as u32;
            let made = |s: &str| t.text.find(s).unwrap() as u32;
            assert_eq!(t.map(made("c = circle")), at("c = circle"), "{src:?}");
            assert_eq!(t.map(made("let r")), at("let r"), "{src:?}");
            assert_eq!(t.map(made("1.0")), at("1.0"), "{src:?}");
            let first = src.find("```").or_else(|| src.find("let r")).unwrap() as u32;
            assert_eq!(t.start, first, "{src:?}");
            // The added import, and the added closing brace.
            assert_eq!(t.map(0), first);
            assert_eq!(t.map(t.text.len() as u32 - 2), first);
        }
    }

    #[test]
    fn a_std_doc_test_imports_nothing_and_names_its_modules_file() {
        let src = "//! Time.\n//!\n//! ```\n//! let d = 1\n//! ```\npub fn now() -> Int { 1 }\n";
        let mut db = FileDb::new();
        let file = db.add("<std/time>", src);
        let doc = FileText::new(src, file).module_docs();
        let tests = doc_tests_of("std::time", &doc, &db, None, &|f| {
            std_place(db.get_name(f).unwrap_or("?"))
        });
        assert_eq!(tests.len(), 1);
        let t = &tests[0];
        assert_eq!(t.name, "std::time (std/time/lib.nova:3)");
        assert_eq!(t.file, "<std/time>");
        assert_eq!(t.text, "@test fn nova_doctest() {\nlet d = 1\n}\n");
        assert_eq!(
            t.map(t.text.find("d = 1").unwrap() as u32),
            src.find("d = 1").unwrap() as u32
        );
        let Ok(tests) = std_doc_tests() else {
            panic!("std's doc tests are not read")
        };
        for t in &tests {
            assert!(t.name.starts_with("std::"), "{}", t.name);
            assert!(t.place.starts_with("std/"), "{}", t.place);
            assert!(t.file.starts_with("<std/"), "{}", t.file);
        }
    }

    const SHAPES_LIB: &str = "//! Shapes.\n//!\n//! ```\n//! let a = circle(1.0)\n//! ```\n\n\
        import units\n\n\
        /// A circle's area.\n///\n/// ```nova,should_panic\n/// let a = circle(2.0)\n/// ```\n\
        ///\n/// ```text\n/// not a test\n/// ```\n\
        pub fn circle(r: Float) -> Float { 3.0 * r * r }\n\n\
        /// A rectangle.\npub record Rect {\n    /// Its width.\n    ///\n    /// ```\n    \
        /// let r = Rect { width: 1.0 }\n    /// ```\n    pub width: Float,\n}\n\n\
        /// Rect's own methods.\n///\n///     let r = Rect { width: 2.0 }\n\
        impl Rect {\n    /// Its width, twice.\n    ///\n    /// ```ignore\n    \
        /// let w = Rect { width: 1.0 }.twice()\n    /// ```\n    \
        pub fn twice(self) -> Float { self.width * 2.0 }\n}\n\n\
        /// Not exported.\n///\n/// ```\n/// let s = secret()\n/// ```\n\
        fn secret() -> Int { 1 }\n";

    const SHAPES_UNITS: &str = "//! Lengths.\n//!\n//! ```\n//! let l = Length { metres: 1.0 }\n\
        //! ```\n\n/// A length.\npub record Length {\n    pub metres: Float,\n}\n\n\
        impl Length {\n    /// The sum of two lengths.\n    ///\n    /// ```\n    \
        /// let l = Length { metres: 1.0 }.plus(Length { metres: 2.0 })\n    /// ```\n    \
        pub fn plus(self, other: Length) -> Length {\n        \
        Length { metres: self.metres + other.metres }\n    }\n}\n";

    #[test]
    fn doc_tests_are_named_by_the_sites_paths() {
        // Spec §5.3: the package, an item, a member, an impl's own docs, a
        // module, and a method by the site's path.
        let dir = fresh("names");
        write(
            &dir,
            &[
                ("nova.toml", &manifest("shapes")),
                ("src/lib.nova", SHAPES_LIB),
                ("src/units.nova", SHAPES_UNITS),
            ],
        );
        let tests = doc_tests(&dir).expect("a library");
        let names: Vec<&str> = tests.iter().map(|t| t.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "shapes (src/lib.nova:3)",
                "shapes::circle (src/lib.nova:11)",
                "shapes::Rect::width (src/lib.nova:24)",
                "shapes::Rect (src/lib.nova:32)",
                "shapes::units (src/units.nova:3)",
                "shapes::units::Length::plus (src/units.nova:15)",
            ]
        );
        assert_eq!(tests[1].unknown, ["should_panic"]);
        assert!(tests[0].unknown.is_empty());
        assert_eq!(
            tests[1].text,
            "import shapes\n@test fn nova_doctest() {\nlet a = circle(2.0)\n}\n"
        );
        // The names the loader gives the files, which the doc test
        // program's database shares.
        let lib = dir.join("src").join("lib.nova").display().to_string();
        let units = dir.join("src").join("units.nova").display().to_string();
        assert_eq!(tests[0].file, lib);
        assert_eq!(tests[5].file, units);
        assert_eq!(tests[0].start, SHAPES_LIB.find("```").unwrap() as u32);
    }

    #[test]
    fn only_a_librarys_documented_items_give_doc_tests() {
        // Spec §4.1. Mutant 5: a program's blocks are not tests.
        let dir = fresh("program");
        write(
            &dir,
            &[
                ("nova.toml", &manifest("app")),
                (
                    "src/main.nova",
                    "/// Runs.\n///\n/// ```\n/// main()\n/// ```\nfn main() {}\n",
                ),
            ],
        );
        assert_eq!(doc_tests(&dir), None);
        // A private item's block, a `text` block and an `ignore`d one.
        let dir = fresh("quiet");
        write(
            &dir,
            &[
                ("nova.toml", &manifest("quiet")),
                (
                    "src/lib.nova",
                    "/// ```\n/// let a = secret()\n/// ```\nfn secret() -> Int { 1 }\n\n\
                     /// ```text\n/// shown()\n/// ```\npub fn shown() -> Int { secret() }\n\n\
                     /// ```ignore\n/// skipped()\n/// ```\npub fn skipped() -> Int { 2 }\n",
                ),
            ],
        );
        assert_eq!(doc_tests(&dir), Some(Vec::new()));
    }
}
