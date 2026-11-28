//! The playground's compiler (spec
//! `docs/superpowers/specs/2026-11-28-phase-6-1-the-playground-design.md`
//! §3.1): S0's front end and WASM backend, compiling one source text, std
//! embedded, into diagnostics or `app.wasm` and `app.js`. Built for
//! `wasm32-unknown-unknown`, it is the module the playground's page runs in
//! a worker, through the C ABI of `abi.rs`; on every target, [`compile`] is
//! a safe function, which the host tests call.
//!
//! The front end is `nova-driver`'s, copied: the driver depends on the
//! native backends and runtime, and keeps its front end private and
//! printing to standard error. Each copy's doc comment names its original;
//! `tests/the_same_as_nova_driver.rs` holds the copies to the driver's
//! output.

#![deny(unsafe_code)]

#[cfg(target_arch = "wasm32")]
mod abi;

use std::collections::HashMap;

use nova_diagnostics::{render, Diagnostic, FileDb, FileId, Severity};
use nova_mir::ExternAbi;
use nova_resolver::{CompileTarget, Def, DefKind, Definitions, ModuleId, ModuleSource};

/// The name the source is compiled under, as diagnostics show it.
pub const FILE_NAME: &str = "main.nova";

/// What [`compile`] gives.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// `app.wasm`, `app.js`, whether it calls std's `extern "js"`
    /// functions (`std/dom`, `std/ui`), and its warnings, rendered.
    Compiled {
        wasm: Vec<u8>,
        js: String,
        page: bool,
        warnings: String,
    },
    /// The diagnostics as `nova` renders them (or a limit's), the errors.
    Failed { text: String, errors: usize },
    /// The compiler's own error, a bug, in `nova`'s words.
    Internal { text: String },
}

/// What the playground compiles for: `nova build --target wasm`'s target.
const TARGET: CompileTarget = CompileTarget::Wasm;

/// The context `nova-driver`'s `build_program_wasm` gives an error of the
/// backend's or the shim's that is not a limit.
const A_BUG: &str = "internal WASM codegen error (this is a compiler bug)";

/// Compile `source` as `nova build --target wasm` compiles a loose
/// `main.nova` that imports no file, with std embedded.
pub fn compile(source: &str) -> Outcome {
    let mut shown = Shown::default();
    let Some(mir) = lower(source, &mut shown) else {
        return Outcome::Failed {
            text: shown.text(),
            errors: shown.errors,
        };
    };
    let page = draws(&mir);
    // Copied from `nova-driver`'s `build_program_wasm`: the module and its
    // shim, which it then writes beside `index.html`.
    let output = match nova_codegen_wasm::compile_module(&mir) {
        Ok(output) => output,
        // A limit of WebAssembly's is the program's to change: `nova` ends
        // with it as its error.
        Err(e) if e.is::<nova_codegen_wasm::Limit>() => {
            return Outcome::Failed {
                text: shown.text() + &report(e.chain()),
                errors: 1,
            };
        }
        Err(e) => {
            return Outcome::Internal {
                text: report(e.context(A_BUG).chain()),
            }
        }
    };
    match nova_codegen_wasm::shim(&output.imports) {
        Ok(js) => Outcome::Compiled {
            wasm: output.wasm,
            js,
            page,
            warnings: shown.text(),
        },
        Err(e) => Outcome::Internal {
            text: report(e.context(A_BUG).chain()),
        },
    }
}

/// An error, by its chain, as `nova` prints the error it ends with:
/// `Error: `, the error, then its causes, as `anyhow` reports `main`'s
/// `Err`, less the backtrace a host's `RUST_BACKTRACE` would add.
fn report<'a>(mut chain: impl Iterator<Item = &'a (dyn std::error::Error + 'static)>) -> String {
    let mut text = match chain.next() {
        Some(error) => format!("Error: {error}\n"),
        None => "Error\n".to_string(),
    };
    let causes: Vec<String> = chain.map(ToString::to_string).collect();
    if let [cause] = causes.as_slice() {
        text.push_str(&format!("\nCaused by:\n    {cause}\n"));
    } else if !causes.is_empty() {
        text.push_str("\nCaused by:\n");
        for (n, cause) in causes.iter().enumerate() {
            text.push_str(&format!("{n:>5}: {cause}\n"));
        }
    }
    text
}

/// Whether `mir` calls one of `std/dom`'s functions, which are the
/// `extern "js"` functions named `dom_…`, so the program draws on a page.
/// `std/rt`'s own, `host_…`, which every program that collects reaches, do
/// not count.
fn draws(mir: &nova_mir::Module) -> bool {
    mir.externs
        .iter()
        .any(|e| e.abi == ExternAbi::Js && e.symbol.starts_with("dom_"))
}

/// The diagnostics shown so far, kept to be rendered once, with the files
/// they name, and their errors: what `nova-driver`'s `FrontendContext`
/// prints to standard error stage by stage.
#[derive(Default)]
struct Shown {
    db: FileDb,
    diagnostics: Vec<Diagnostic>,
    errors: usize,
}

impl Shown {
    /// Show `diagnostics`, and say whether an error has been shown. Copied
    /// from `FrontendContext::render`: a loose program has no registry
    /// package whose warnings it hides, and the playground maps no
    /// diagnostic, so each is shown and counted.
    fn add(&mut self, diagnostics: &[Diagnostic]) -> bool {
        self.errors += diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        self.diagnostics.extend_from_slice(diagnostics);
        self.errors > 0
    }

    /// Every diagnostic shown, as `nova` renders them, without colour.
    fn text(&self) -> String {
        render::render_to_string(&self.db, &self.diagnostics)
    }
}

/// The front end and MIR, as `nova build --target wasm` runs them on a
/// loose `main.nova` that imports no file: `None` once a stage has shown an
/// error, as each stops. Copied from `FrontendContext::check`, for one
/// module without `std/test`, and `lower_to_mir_for`, for WASM.
fn lower(source: &str, shown: &mut Shown) -> Option<nova_mir::Module> {
    let file = shown.db.add(FILE_NAME, source);
    let mut ast = parse(source, file, shown);
    if shown.errors > 0 {
        return None;
    }
    strip_test_functions(&mut ast);
    let std_files = register_std(&mut shown.db);
    // A loose module is named by its file's stem, and an import of a file
    // beside it would be "cannot find module".
    let sources = [ModuleSource {
        name: "main".to_string(),
        file: &ast,
        imports: HashMap::new(),
        text: shown.db.get_source(file),
        package: None,
        importable: Vec::new(),
    }];
    let resolved = nova_resolver::resolve_program_for(TARGET, &sources, &std_files, None);
    if shown.add(&resolved.diagnostics) {
        return None;
    }
    let checked = nova_typeck::check_with(
        &resolved.file,
        &resolved.definitions,
        &nova_typeck::CheckOptions {
            sources: Some(&shown.db),
            ..Default::default()
        },
    );
    if shown.add(&checked.diagnostics) {
        return None;
    }
    let mut module = checked.module;
    keep_the_entry_main(&mut module, &resolved.definitions);
    let runtime = runtime_impls(&module, &resolved.definitions);
    match nova_mir::lower_module_with(&module, &mir_options(Some(&runtime))) {
        Ok(mir) => Some(mir),
        Err(diagnostics) => {
            shown.add(&diagnostics);
            None
        }
    }
}

/// Lex and parse `source`, the module `file`, showing their diagnostics.
/// Copied from `nova-driver`'s `load_program` (`program.rs`), for the one
/// module.
fn parse(source: &str, file: FileId, shown: &mut Shown) -> nova_ast::File {
    let (tokens, lex_errors) = nova_lexer::lex(source, file);
    let mut diagnostics: Vec<Diagnostic> = lex_errors
        .iter()
        .map(|e| Diagnostic::error("L0001", e.to_string()).with_primary_label(e.span(), "here"))
        .collect();
    let parsed = nova_parser::parse_recovering(&tokens, file);
    diagnostics.extend(parsed.errors.iter().map(|e| {
        e.notes().into_iter().fold(
            Diagnostic::error(e.code(), e.to_string()).with_primary_label(e.span(), "here"),
            |d, n| d.with_note(n),
        )
    }));
    shown.add(&diagnostics);
    parsed.file
}

/// Remove every top-level `@test` function, before resolving. Copied from
/// `nova-driver`'s `strip_test_functions`, in its case of a loose root
/// without `nova test`: every one goes, so a test's body, written for
/// `std/test`, is never resolved.
fn strip_test_functions(file: &mut nova_ast::File) {
    file.items.retain(|item| match &item.value {
        nova_ast::Item::Function(f) => !f.attrs.iter().any(|a| a.name.value == "test"),
        _ => true,
    });
}

/// Register each of WASM's std modules in `db`, as `<std/core>`: their
/// ids, in order. Copied from `nova-driver`'s `register_std`, less the
/// labels its dumps read.
fn register_std(db: &mut FileDb) -> Vec<FileId> {
    TARGET
        .std_modules()
        .iter()
        .map(|&(name, src)| {
            let short = name.strip_prefix("$std.").unwrap_or(name);
            db.add(format!("<std/{short}>"), src)
        })
        .collect()
}

/// What every function called `main` outside the entry module is renamed
/// to: a name the lexer cannot produce. Copied verbatim from `nova-driver`'s
/// `NOT_THE_ENTRY_MAIN`.
const NOT_THE_ENTRY_MAIN: &str = "main.not_the_entry";

/// `std/rt`'s functions in `module`, by name. Copied verbatim from
/// `nova-driver`'s `runtime_impls`.
fn runtime_impls(module: &nova_hir::Module, definitions: &Definitions) -> nova_mir::RuntimeImpls {
    let mut impls = nova_mir::RuntimeImpls::default();
    for function in &module.functions {
        let def = definitions.defs().get(function.def_id.0 as usize);
        if let Some(Def {
            kind: DefKind::Fn { item_index },
            ..
        }) = def
        {
            if definitions.module_name(definitions.module_of(*item_index)) == "$std.rt" {
                impls
                    .functions
                    .insert(function.name.clone(), function.def_id);
            }
        }
    }
    impls
}

/// The options MIR lowering takes. Copied verbatim from `nova-driver`'s
/// `mir_options`.
fn mir_options(runtime: Option<&nova_mir::RuntimeImpls>) -> nova_mir::Options<'_> {
    nova_mir::Options {
        runtime,
        ..Default::default()
    }
}

/// Rename every `main` declared outside the entry module, `ModuleId(0)`,
/// so that MIR's search by name finds the entry's own `main`, or reports
/// E0601. A call reaches its function by `DefId`, so the rename breaks no
/// call. Copied verbatim from `nova-driver`'s `keep_the_entry_main`.
fn keep_the_entry_main(module: &mut nova_hir::Module, definitions: &Definitions) {
    for function in &mut module.functions {
        if function.name != "main" {
            continue;
        }
        let def = definitions.defs().get(function.def_id.0 as usize);
        if let Some(Def {
            kind: DefKind::Fn { item_index },
            ..
        }) = def
        {
            if definitions.module_of(*item_index) != ModuleId(0) {
                function.name = NOT_THE_ENTRY_MAIN.to_string();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// `compile`'s success: the module, the shim, `page` and the warnings.
    fn compiled(source: &str) -> (Vec<u8>, String, bool, String) {
        match compile(source) {
            Outcome::Compiled {
                wasm,
                js,
                page,
                warnings,
            } => (wasm, js, page, warnings),
            other => panic!("not compiled: {other:?}"),
        }
    }

    /// `compile`'s failure: the text and the error count.
    fn failed(source: &str) -> (String, usize) {
        match compile(source) {
            Outcome::Failed { text, errors } => (text, errors),
            other => panic!("not failed: {other:?}"),
        }
    }

    #[test]
    fn hello_world_compiles_to_a_module_and_its_shim() {
        let (wasm, js, page, warnings) = compiled("fn main() { println(\"Hello, World!\") }\n");
        assert_eq!(&wasm[..4], b"\0asm");
        assert!(js.contains("export default async function init("), "{js}");
        assert!(!page, "hello world does not draw");
        assert_eq!(warnings, "");
    }

    #[test]
    fn a_type_error_fails_with_one_error_in_main_nova() {
        let (text, errors) = failed("fn main() {\n    let s: String = 1\n}\n");
        assert_eq!(errors, 1, "{text}");
        assert!(text.starts_with("error[E0010]: "), "{text}");
        assert!(text.contains("┌─ main.nova:2:"), "{text}");
    }

    #[test]
    fn warnings_travel_with_a_success() {
        let (wasm, _, _, warnings) = compiled("fn main() {\n    1 + 2\n    println(\"hi\")\n}\n");
        assert_eq!(&wasm[..4], b"\0asm");
        assert!(
            warnings.starts_with("warning[E0103]: this value is not used"),
            "{warnings}"
        );
        assert!(warnings.contains("┌─ main.nova:2:5"), "{warnings}");
    }

    /// `assert` is `std/test`'s, in scope only under `nova test`: kept, the
    /// test function would fail to resolve.
    #[test]
    fn a_test_function_is_dropped() {
        compiled("fn main() { println(\"hi\") }\n\n@test\nfn t() { assert(1 == 1) }\n");
    }

    /// The counter of `examples/06-counter-spa`, a `std/ui` component, reaches
    /// `std/dom`'s functions.
    #[test]
    fn the_counter_draws() {
        let (_, _, page, _) = compiled(include_str!(
            "../../../examples/06-counter-spa/src/main.nova"
        ));
        assert!(page, "the counter draws");
    }

    #[test]
    fn an_empty_source_fails() {
        let (text, errors) = failed("");
        assert_eq!(errors, 1, "{text}");
        assert!(text.starts_with("error[E0601]: "), "{text}");
    }
}
