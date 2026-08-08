//! Compilation pipeline orchestration for Nova.
//!
//! Drives source → lex → parse → resolve → typecheck → MIR →
//! Cranelift (JIT for `nova run`, object emission + native linking for
//! `nova build`), rendering diagnostics from every stage through
//! `nova-diagnostics`. This is the crate behind `nova run`,
//! `nova build`, and `nova check`.

mod link;

use std::collections::{HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use nova_codegen_cranelift::CompiledProgram;
use nova_diagnostics::{render, Diagnostic, FileDb, FileId, Severity, Span};
use nova_hir as hir;
use nova_hir::Ty;
use nova_resolver::{Builtin, DefId, ModuleSource, TestFn};

/// Outcome of a pipeline invocation that may fail with user errors.
pub enum Outcome<T> {
    /// The pipeline succeeded.
    Ok(T),
    /// User-facing errors were reported (already rendered to stderr).
    Failed {
        /// Number of error-severity diagnostics.
        errors: usize,
    },
}

/// Type-check a file, printing diagnostics. `Outcome::Ok(())` means the
/// program is well-formed and would compile.
///
/// This also runs monomorphization (MIR lowering), since some checks —
/// notably trait-bound satisfaction — are performed there per
/// `12-TYPESYSTEM.md` §5.4. Without it `nova check` would call a program
/// "well-formed" that `nova run` / `nova build` then reject.
pub fn check_file(path: &Path) -> Result<Outcome<()>> {
    let mut ctx = FrontendContext::load(path)?;
    let Some((module, _tests, _fresh_def_id)) = ctx.check(false)? else {
        return Ok(Outcome::Failed { errors: ctx.errors });
    };
    if let Err(diags) = nova_mir::lower_module(&module) {
        ctx.render(&diags);
    }
    if ctx.errors > 0 {
        Ok(Outcome::Failed { errors: ctx.errors })
    } else {
        Ok(Outcome::Ok(()))
    }
}

/// Compile a file to native code via the Cranelift JIT.
pub fn compile_file(path: &Path) -> Result<Outcome<CompiledProgram>> {
    let mir = match lower_to_mir(path)? {
        Outcome::Ok(mir) => mir,
        Outcome::Failed { errors } => return Ok(Outcome::Failed { errors }),
    };
    match nova_codegen_cranelift::compile_jit(&mir) {
        Ok(program) => Ok(Outcome::Ok(program)),
        Err(e) => {
            // An unresolvable `extern` symbol is a user error (the symbol isn't
            // in the C runtime / any loaded library), not a compiler bug — the
            // JIT can only discover it at finalize time. Report it as a clean
            // diagnostic, mirroring the `nova build` linker-error path.
            if let Some(sym) = e.downcast_ref::<nova_codegen_cranelift::UnresolvedExternSymbol>() {
                let diag = Diagnostic::error(
                    "E0902",
                    format!(
                        "{sym} at run time; an `extern` function must be provided by the \
                         C runtime or an already-loaded library (a `nova build` executable \
                         resolves it at link time instead)"
                    ),
                );
                render::emit_all(&FileDb::new(), &[diag]);
                return Ok(Outcome::Failed { errors: 1 });
            }
            Err(e).context("internal codegen error (this is a compiler bug)")
        }
    }
}

/// Compile a file to a standalone native executable (`nova build`).
///
/// Emits a Cranelift object file next to `output`, links it with the
/// `nova-runtime` static library through the platform linker, and removes
/// the intermediate object on success.
pub fn build_file(path: &Path, output: &Path) -> Result<Outcome<PathBuf>> {
    let mir = match lower_to_mir(path)? {
        Outcome::Ok(mir) => mir,
        Outcome::Failed { errors } => return Ok(Outcome::Failed { errors }),
    };
    let bytes = nova_codegen_cranelift::compile_object(&mir)
        .context("internal codegen error (this is a compiler bug)")?;

    let obj_ext = if cfg!(windows) { "obj" } else { "o" };
    let obj_path = intermediate(output, obj_ext);
    std::fs::write(&obj_path, bytes)
        .with_context(|| format!("failed to write {}", obj_path.display()))?;

    let linked = link::link_executable(&obj_path, output);
    let _ = std::fs::remove_file(&obj_path);
    linked?;
    Ok(Outcome::Ok(output.to_path_buf()))
}

/// An intermediate-file path derived from `output` that can never alias it:
/// the full output file name plus `.nova.<ext>` (so even `-o out.ll` /
/// `-o out.obj` yield a distinct `out.ll.nova.ll` / `out.obj.nova.obj` and the
/// final binary is never overwritten or deleted as an intermediate).
fn intermediate(output: &Path, ext: &str) -> PathBuf {
    let mut name = output
        .file_name()
        .map(|n| n.to_os_string())
        .unwrap_or_else(|| std::ffi::OsString::from("out"));
    name.push(".nova.");
    name.push(ext);
    output.with_file_name(name)
}

/// Compile a file to an optimized standalone executable via the LLVM backend
/// (`nova build --release`).
///
/// Emits textual LLVM IR next to `output`, compiles it to an object file with
/// a discovered LLVM toolchain (`clang`/`llc`, `-O2`), then links it with the
/// `nova-runtime` static library through the same platform linker as the debug
/// build. If no LLVM toolchain is found the generated `.ll` is left in place
/// and a helpful error is returned.
pub fn build_file_release(path: &Path, output: &Path) -> Result<Outcome<PathBuf>> {
    let mir = match lower_to_mir(path)? {
        Outcome::Ok(mir) => mir,
        Outcome::Failed { errors } => return Ok(Outcome::Failed { errors }),
    };
    let ir = nova_codegen_llvm::compile_ir(&mir)
        .context("internal LLVM IR generation error (this is a compiler bug)")?;

    let ll_path = intermediate(output, "ll");
    std::fs::write(&ll_path, ir.as_bytes())
        .with_context(|| format!("failed to write {}", ll_path.display()))?;

    let obj_ext = if cfg!(windows) { "obj" } else { "o" };
    let obj_path = intermediate(output, obj_ext);

    // Compile IR → object, then link. Keep the `.ll` on failure (for the LLVM
    // toolchain to be pointed at or for inspection); remove it on success.
    let result = link::compile_ir_to_object(&ll_path, &obj_path)
        .and_then(|()| link::link_executable(&obj_path, output));
    let _ = std::fs::remove_file(&obj_path);
    match result {
        Ok(()) => {
            let _ = std::fs::remove_file(&ll_path);
            Ok(Outcome::Ok(output.to_path_buf()))
        }
        Err(e) => Err(e.context(format!(
            "release build failed; generated LLVM IR left at {}",
            ll_path.display()
        ))),
    }
}

/// A name no Nova source can ever produce for a top-level function: the
/// lexer's identifier grammar is `[a-zA-Z_][a-zA-Z0-9_]*` (`nova-lexer/src/
/// lib.rs`'s `RawToken::Ident` regex), which cannot lex a `.` inside one
/// `Ident` token — the same "impossible in surface syntax" property
/// `nova_mir::mangle` already relies on (its symbols are `name.<def_id>`).
/// `build_test_binary` uses this to rename a user's own `main`, if the
/// source declares one, so it cannot be mistaken for the synthesized one.
const SHADOWED_USER_MAIN_NAME: &str = "main.shadowed_by_nova_test";

/// Compile `path` as a test binary (`nova test`, Task 5): `main` is
/// synthesized to dispatch to exactly one collected `@test` by index
/// (`Builtin::TestSelector`) rather than to the file's own `main` — a test
/// file need not define one, since a missing `main` is enforced only at MIR
/// lowering (`E0601` in `nova_mir::lower_module`), which the synthesized
/// function supplies before that check ever runs.
///
/// If the source *does* declare its own `main`, that function is renamed
/// (never removed — something else in the module may still call it by
/// `DefId`) before the synthesized dispatcher is pushed, so the dispatcher
/// is the only function left named `"main"`. This matters because
/// `nova-mir/src/mono.rs`'s entry-point search is
/// `module.functions.iter().find(|f| f.name == "main")` — the *first*
/// match — while pushing onto `module.functions` puts the dispatcher
/// *last*: without the rename, a source file with both `@test` functions
/// and its own `main` would silently run the user's `main` instead of any
/// test, compiling and linking cleanly with no diagnostic anywhere.
///
/// Returns the built executable's path and the collected tests in source
/// order, so a caller can run the binary once per test with `NOVA_TEST_INDEX`
/// set without recompiling or re-deriving either the path or the inventory
/// from the source again.
pub fn build_test_binary(path: &Path) -> Result<(PathBuf, Vec<TestFn>)> {
    let mut ctx = FrontendContext::load(path)?;
    let Some((mut module, tests, fresh_def_id)) = ctx.check(true)? else {
        anyhow::bail!(
            "could not compile due to {} previous error{}",
            ctx.errors,
            if ctx.errors == 1 { "" } else { "s" }
        );
    };
    for f in &mut module.functions {
        if f.name == "main" {
            f.name = SHADOWED_USER_MAIN_NAME.to_string();
        }
    }
    module
        .functions
        .push(synthesize_test_main(fresh_def_id, &tests));

    let mir = match nova_mir::lower_module(&module) {
        Ok(mir) => mir,
        Err(diags) => {
            ctx.render(&diags);
            anyhow::bail!(
                "could not compile due to {} previous error{}",
                ctx.errors,
                if ctx.errors == 1 { "" } else { "s" }
            );
        }
    };
    let bytes = nova_codegen_cranelift::compile_object(&mir)
        .context("internal codegen error (this is a compiler bug)")?;

    // A caller-distinguishing directory, keyed on the *canonicalized* input
    // path rather than its file stem: fixtures overwhelmingly share the
    // name `main.nova` (50 occurrences in this workspace's own
    // `nova-cli/tests/run_tests.rs` alone), so two different source files in
    // two different directories must not resolve to the same output path —
    // a real race under `cargo test`'s default parallelism (a locked-file
    // link error on Windows; a binary overwritten mid-execution on Unix).
    let dir = std::env::temp_dir()
        .join("nova-test-bin")
        .join(path_fingerprint(path));
    std::fs::create_dir_all(&dir).with_context(|| format!("failed to create {}", dir.display()))?;
    let output = dir.join(format!(
        "{}{}",
        FrontendContext::module_name(path),
        std::env::consts::EXE_SUFFIX
    ));

    let obj_ext = if cfg!(windows) { "obj" } else { "o" };
    let obj_path = intermediate(&output, obj_ext);
    std::fs::write(&obj_path, bytes)
        .with_context(|| format!("failed to write {}", obj_path.display()))?;

    let linked = link::link_executable(&obj_path, &output);
    let _ = std::fs::remove_file(&obj_path);
    linked?;
    Ok((output, tests))
}

/// A short, deterministic fingerprint of `path`'s canonicalized form, for
/// `build_test_binary`'s output directory. Canonicalizing first (falling
/// back to the path as given if that fails — it cannot fail here in
/// practice, since `build_test_binary` has already opened and read this
/// file by the time this runs) means two different relative spellings of
/// the same file collide on purpose, while two files that merely share a
/// name in different directories do not.
fn path_fingerprint(path: &Path) -> String {
    let canon = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    canon.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

/// A zero-width span at the dummy file: there is no source location for a
/// hand-built `hir::Function`, so every node `synthesize_test_main` builds
/// uses the same sentinel `FileId::DUMMY` documents itself for.
fn dummy_span() -> Span {
    Span::point(0, FileId::DUMMY)
}

fn expr(kind: hir::ExprKind, ty: Ty) -> hir::Expr {
    hir::Expr {
        kind,
        ty,
        span: dummy_span(),
    }
}

/// Build the dispatching `main` that `build_test_binary` adds to the module.
///
/// Constructed directly as an `hir::Function` — mirroring the synthetic
/// `main` `nova-mir/src/mono.rs`'s own unit tests build, field for field —
/// rather than as generated Nova source. Generated source is how `std`
/// reaches a program, but it would route through the resolver, which
/// enforces `pub`, and `@test` functions are deliberately not `pub`;
/// constructing the HIR directly sidesteps visibility rather than fighting
/// it. Because `nova-mir/src/mono.rs`'s monomorphizer locates the entry point
/// by the name `"main"`, not by `def_id`, nothing downstream needs to know
/// this particular `main` was synthesized rather than parsed.
///
/// Body: bind `test_selector()` to local 0 (`sel`), then one
/// `if sel == i { test_i() }` per collected test in source order, then
/// (reached only when `sel` matched none of them — including whenever
/// `sel < 0`, the documented sentinel) print the inventory: a count line,
/// then one name per line.
fn synthesize_test_main(main_id: DefId, tests: &[TestFn]) -> hir::Function {
    let sel = hir::LocalId(0);

    let mut stmts = vec![expr(
        hir::ExprKind::Let {
            local: sel,
            init: Box::new(expr(
                hir::ExprKind::Call {
                    func: hir::Callee::Builtin(Builtin::TestSelector),
                    type_args: Vec::new(),
                    args: Vec::new(),
                },
                Ty::Int,
            )),
        },
        Ty::Unit,
    )];

    for (i, t) in tests.iter().enumerate() {
        let cond = expr(
            hir::ExprKind::Binary {
                op: hir::BinOp::Eq,
                lhs: Box::new(expr(hir::ExprKind::Local(sel), Ty::Int)),
                rhs: Box::new(expr(hir::ExprKind::IntLit(i as i64), Ty::Int)),
            },
            Ty::Bool,
        );
        let call = expr(
            hir::ExprKind::Call {
                func: hir::Callee::Def(t.def_id),
                type_args: Vec::new(),
                args: Vec::new(),
            },
            // Task 1/2 reject a non-`Unit`-returning `@test` function
            // (E0084), so every callee here is `Unit`-typed.
            Ty::Unit,
        );
        stmts.push(expr(
            hir::ExprKind::If {
                cond: Box::new(cond),
                then: Box::new(call),
                else_: None,
            },
            Ty::Unit,
        ));
    }

    let println = |s: String| {
        expr(
            hir::ExprKind::Call {
                func: hir::Callee::Builtin(Builtin::Println),
                type_args: Vec::new(),
                args: vec![expr(hir::ExprKind::StrLit(s), Ty::String)],
            },
            Ty::Unit,
        )
    };
    let mut inventory = vec![println(tests.len().to_string())];
    inventory.extend(tests.iter().map(|t| println(t.name.clone())));
    let sel_is_negative = expr(
        hir::ExprKind::Binary {
            op: hir::BinOp::Lt,
            lhs: Box::new(expr(hir::ExprKind::Local(sel), Ty::Int)),
            rhs: Box::new(expr(hir::ExprKind::IntLit(0), Ty::Int)),
        },
        Ty::Bool,
    );
    stmts.push(expr(
        hir::ExprKind::If {
            cond: Box::new(sel_is_negative),
            then: Box::new(expr(
                hir::ExprKind::Block {
                    stmts: inventory,
                    trailing: None,
                },
                Ty::Unit,
            )),
            else_: None,
        },
        Ty::Unit,
    ));

    hir::Function {
        def_id: main_id,
        name: "main".to_string(),
        generics: 0,
        bounds: Vec::new(),
        takes_env: false,
        capture_count: 0,
        params: 0,
        locals: vec![hir::Local {
            name: "sel".to_string(),
            ty: Ty::Int,
            is_mut: false,
            span: dummy_span(),
        }],
        ret_ty: Ty::Unit,
        // The synthesized `@test` harness entry point is plain generated
        // dispatch code (a chain of `if sel == i { test_i() }`), never a
        // user-written `async fn` — so it is never async.
        is_async: false,
        body: expr(
            hir::ExprKind::Block {
                stmts,
                trailing: None,
            },
            Ty::Unit,
        ),
        span: dummy_span(),
    }
}

/// Run the front end and MIR lowering, rendering any diagnostics.
fn lower_to_mir(path: &Path) -> Result<Outcome<nova_mir::Module>> {
    let mut ctx = FrontendContext::load(path)?;
    let Some((module, _tests, _fresh_def_id)) = ctx.check(false)? else {
        return Ok(Outcome::Failed { errors: ctx.errors });
    };
    match nova_mir::lower_module(&module) {
        Ok(mir) => Ok(Outcome::Ok(mir)),
        Err(diags) => {
            ctx.render(&diags);
            Ok(Outcome::Failed { errors: ctx.errors })
        }
    }
}

/// Compile and immediately execute a file (`nova run`).
pub fn run_file(path: &Path) -> Result<Outcome<()>> {
    match compile_file(path)? {
        Outcome::Ok(program) => {
            program.run();
            Ok(Outcome::Ok(()))
        }
        Outcome::Failed { errors } => Ok(Outcome::Failed { errors }),
    }
}

/// Shared front-end state: file database plus error accounting.
struct FrontendContext {
    db: FileDb,
    entry: PathBuf,
    errors: usize,
}

impl FrontendContext {
    fn load(path: &Path) -> Result<Self> {
        // The entry file must exist; imported modules are resolved lazily.
        std::fs::File::open(path).with_context(|| format!("failed to open {}", path.display()))?;
        Ok(Self {
            db: FileDb::new(),
            entry: path.to_path_buf(),
            errors: 0,
        })
    }

    fn render(&mut self, diagnostics: &[Diagnostic]) {
        self.errors += diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .count();
        if !diagnostics.is_empty() {
            render::emit_all(&self.db, diagnostics);
        }
    }

    /// The module name for a source path — its file stem.
    fn module_name(path: &Path) -> String {
        path.file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "main".to_string())
    }

    /// Load, lex, and parse the entry module plus every module it transitively
    /// `import`s (resolved to `<name>.nova` beside the entry). A module whose
    /// file is missing is skipped here — the resolver reports the dangling
    /// import against its `import` site. Lex/parse diagnostics are rendered.
    fn load_modules(&mut self) -> Result<Vec<(String, nova_ast::File)>> {
        let dir = self
            .entry
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let mut out: Vec<(String, nova_ast::File)> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        let mut queue: VecDeque<(String, PathBuf)> = VecDeque::new();
        queue.push_back((Self::module_name(&self.entry), self.entry.clone()));

        while let Some((name, path)) = queue.pop_front() {
            if !seen.insert(name.clone()) {
                continue;
            }
            let is_entry = out.is_empty() && seen.len() == 1;
            let source = match std::fs::read_to_string(&path) {
                Ok(s) => s,
                Err(e) if is_entry => {
                    return Err(e).with_context(|| format!("failed to read {}", path.display()))
                }
                // A missing imported module: skip; the resolver flags the import.
                Err(_) => continue,
            };
            let file_id = self.db.add(path.display().to_string(), source.as_str());

            let (tokens, lex_errors) = nova_lexer::lex(&source, file_id);
            let lex_diags: Vec<Diagnostic> = lex_errors
                .iter()
                .map(|e| {
                    Diagnostic::error("L0001", e.to_string()).with_primary_label(e.span(), "here")
                })
                .collect();
            self.render(&lex_diags);

            let (ast, parse_errors) = nova_parser::parse(&tokens, file_id);
            let parse_diags: Vec<Diagnostic> = parse_errors
                .iter()
                .map(|e| {
                    Diagnostic::error("P0001", e.to_string()).with_primary_label(e.span(), "here")
                })
                .collect();
            self.render(&parse_diags);

            let Some(ast) = ast else {
                continue;
            };

            // Queue imported modules (resolved beside the entry file). Only
            // single-segment imports name a module file; qualified/nested paths
            // (`a::b`) are unsupported and rejected by the resolver, so don't
            // chase a file for them (which would surface as a confusing error
            // against an unrelated module).
            for item in &ast.items {
                if let nova_ast::Item::Import(imp) = &item.value {
                    let segments = &imp.path.value.segments;
                    if let [seg] = segments.as_slice() {
                        let mod_name = seg.value.clone();
                        if !seen.contains(&mod_name) {
                            let mod_path = dir.join(format!("{mod_name}.nova"));
                            queue.push_back((mod_name, mod_path));
                        }
                    }
                }
            }
            out.push((name, ast));
        }
        Ok(out)
    }

    /// Run load → lex → parse → resolve → typecheck across all modules.
    /// Returns the merged typed module, the `@test` functions collected along
    /// the way (source order, per `nova_resolver::TestFn`'s doc comment), and
    /// a `DefId` guaranteed unused by anything resolution allocated — or
    /// `None` if any stage reported errors.
    ///
    /// `with_test_module` seeds `std/test` (`assert`/`assert_eq`/`assert_ne`)
    /// alongside the fixed three, so it is glob-imported the same way — but
    /// only when `true`. `build_test_binary` is the only caller that passes
    /// `true`; `check_file` and `lower_to_mir` (behind `check_file`,
    /// `compile_file`, `build_file`, `build_file_release`) pass `false`, so
    /// those names never resolve in an ordinary compile.
    ///
    /// The fresh `DefId` is `resolved.definitions.defs().len()`: `Definitions`
    /// documents `defs` as one global counter shared by every definition kind
    /// (functions, sums, records, traits, methods, externs, associated
    /// types), allocated by `push_def` in strict `Vec::len()` order — so every
    /// id resolution ever handed out is `< that length`, and this value
    /// cannot collide with any of them. `build_test_binary` uses it to name a
    /// synthesized `main` that cannot alias an id one of the `@test`
    /// functions it calls by `DefId` already owns.
    fn check(
        &mut self,
        with_test_module: bool,
    ) -> Result<Option<(nova_hir::Module, Vec<TestFn>, DefId)>> {
        let mut modules = self.load_modules()?;
        if self.errors > 0 {
            return Ok(None);
        }
        // Whole-branch review, finding 1 (Critical): a `@test` function's
        // body is written assuming `std/test`'s `assert`/`assert_eq`/
        // `assert_ne` are in scope, which — per the doc comment above — is
        // true only when `with_test_module` is `true`. Left in the
        // compilation unit under `nova run`/`build`/`check`, such a body's
        // calls to them fail to resolve (`E0001`), which means a program
        // could never both `nova build` and `nova test` cleanly, and `nova
        // check` (the editor path) reported spurious errors on every test in
        // the project. Stripping the items here — before any of them reaches
        // `nova_resolver::resolve_program` — removes the problem instead of
        // working around it: a stripped function gets no `DefId`, no scope
        // entry, and its body is never visited by anything downstream, the
        // same "does not exist in this build" semantics as Rust's
        // `#[cfg(test)]`. See `strip_test_functions`'s own doc comment for
        // why this must happen *before* resolution rather than after.
        if !with_test_module {
            strip_test_functions(&mut modules);
        }
        // Register each embedded std module's source in the `FileDb` so a
        // syntax error inside one (a compiler bug, since they ship with the
        // compiler) is reported against a real, named file instead of
        // `FileId::DUMMY`. One id per `nova_resolver::STD_MODULES` entry, in
        // order — e.g. `$std.core` names `<std/core>`.
        let std_files: Vec<FileId> = nova_resolver::STD_MODULES
            .iter()
            .map(|&(name, src)| {
                let short = name.strip_prefix("$std.").unwrap_or(name);
                self.db.add(format!("<std/{short}>"), src)
            })
            .collect();
        // `std/test`, registered the same way but only under `nova test`
        // (`with_test_module`): a fourth `FileId` allocated *conditionally*,
        // which is fine precisely because it rides alongside `STD_MODULES`
        // rather than being folded into it — nothing computes an index from
        // `STD_MODULES.len()` expecting it to already include this entry.
        let extra_std = with_test_module.then(|| {
            let (name, src) = nova_resolver::STD_TEST_MODULE;
            let short = name.strip_prefix("$std.").unwrap_or(name);
            let file_id = self.db.add(format!("<std/{short}>"), src);
            (nova_resolver::STD_TEST_MODULE, file_id)
        });
        let sources: Vec<ModuleSource> = modules
            .iter()
            .map(|(name, file)| ModuleSource {
                name: name.clone(),
                file,
            })
            .collect();
        let resolved = nova_resolver::resolve_program(&sources, &std_files, extra_std);
        self.render(&resolved.diagnostics);
        if self.errors > 0 {
            return Ok(None);
        }
        let fresh_def_id = DefId(resolved.definitions.defs().len() as u32);
        let tests = resolved.tests;

        let checked = nova_typeck::check(&resolved.file, &resolved.definitions);
        self.render(&checked.diagnostics);
        if self.errors > 0 {
            return Ok(None);
        }
        Ok(Some((checked.module, tests, fresh_def_id)))
    }
}

/// Remove every top-level `@test` function from `modules`, in place.
///
/// Only ever called with `with_test_module = false` (`nova run`/`build`/
/// `check`; never `build_test_binary`'s `nova test` path). A `@test`
/// function is ordinary Nova source with no special compilation unit of its
/// own — before this fix, `nova_resolver::resolve_program` collected it,
/// gave it a `DefId`, and `nova_typeck::check` type-checked its body exactly
/// like any other function, regardless of `with_test_module`. That body is
/// written assuming `std/test`'s `assert`/`assert_eq`/`assert_ne` are in
/// scope; they are seeded only under `nova test` (this file's own doc
/// comment on `check`), so under every other compilation mode those calls
/// failed to resolve (`E0001`) — a program containing so much as one test
/// could not be built or checked at all, and `nova check` (the editor path)
/// reported spurious errors on every test in the project on every keystroke.
///
/// **Removed here, before `resolve_program` ever runs**, rather than kept
/// through resolution and typeck and discarded afterward. Two designs were
/// available and only one is actually correct:
///
/// - *Strip before resolution* (this one): the function never gets a
///   `DefId`, never enters its module's scope, and its body is never visited
///   by anything — the same "does not exist in this build" semantics as
///   Rust's `#[cfg(test)]`. Anything that still tries to call it by name
///   (unusual, but not forbidden by anything in the language) gets the
///   ordinary, well-tested "cannot find function" `E0001`, exactly as if the
///   function had never been written.
/// - *Strip after resolution* (e.g. filter `nova_typeck::check`'s per-`DefId`
///   iteration instead): resolution would still assign the function a real
///   `DefId` and scope entry, and `collect_signatures` runs before any
///   per-function body pass, so *other*, kept, functions could still
///   type-check a call to it successfully against a signature that then has
///   no corresponding `hir::Function` body in the compiled module — a
///   dangling `DefId` reference that MIR lowering has no defined behavior
///   for. This is strictly worse than the bug being fixed: today's failure
///   is a clean compile error; that failure mode would be a compiler panic
///   or a silently broken binary, and only in the one case where non-test
///   code happens to reference a test function by name.
///
/// Deliberately done in `nova-driver`, not `nova_resolver`, and keyed off
/// this driver's own `with_test_module` rather than any new parameter on
/// `nova_resolver::resolve_program`: `nova_resolver::resolve`/`resolve_program`
/// are also called directly, with no notion of "compiling for `nova test`",
/// by a large fraction of `nova-resolver`'s and `nova-typeck`'s own unit
/// tests — including the very tests that pin `E0082`-`E0085` and `TestFn`
/// collection on `@test` functions (`nova-typeck/src/check.rs`'s
/// `an_unknown_attribute_is_e0082_and_names_it` and neighbors). Teaching
/// `resolve_program` to drop `@test` items itself, gated on whether it was
/// given `extra_std`, would silently empty out every one of those functions'
/// subject items before validation ever ran, breaking them. Confining the
/// strip to this driver function leaves `nova_resolver`'s validation and
/// collection behavior — and every test that exercises it directly —
/// completely unchanged; only the four real compilation entry points that
/// pass `with_test_module = false` are affected, and `nova test` itself
/// never calls this at all.
///
/// As a consequence, `nova_resolver::validate_test_function`'s attribute-
/// shape diagnostics (`E0082`-`E0085`) do not fire for a stripped function
/// under `nova build`/`check`/`run` — matching `#[cfg(test)]`, where an
/// invalid test signature is likewise uncaught outside `cargo test`. `nova
/// test` validates every `@test` function's shape regardless, since this
/// function is never called on that path.
fn strip_test_functions(modules: &mut [(String, nova_ast::File)]) {
    for (_, file) in modules.iter_mut() {
        file.items.retain(|item| {
            !matches!(
                &item.value,
                nova_ast::Item::Function(f) if f.attrs.iter().any(|a| a.name.value == "test")
            )
        });
    }
}

/// End-to-end proof that a generated `async fn` actually *runs*: the future
/// this compiler builds, JIT-compiled, driven by the real `nova-runtime`
/// executor, with the value read back out of the state object's output slot.
///
/// This lives here rather than in `nova-codegen-cranelift` (which owns the
/// MIR → native seam) or `nova-cli` (which owns `nova run`) because it needs
/// both halves at once: the whole front end, to get real MIR out of real
/// source, and an in-process runtime, to poll the resulting future and inspect
/// the answer. `nova-driver` is the only crate that already has the front end
/// and the JIT; `nova-runtime` is a dev-dependency for the executor entry
/// points.
///
/// **Why `main` is synthesized in MIR instead of written in Nova.** Nothing in
/// the language can reach `block_on` yet: `Future` is not a nameable type
/// (`nova-typeck`'s `resolve_type` returns `None` for it) and an `extern`
/// signature accepts only `Int`, `Float` and `Bool`, so no `.nova` source can
/// name a future, pass one, or await one. Task 7's `std/task` is what closes
/// that. Until then the only way to *execute* a generated poll function is to
/// build the calling code at the level where futures do exist — MIR — which is
/// exactly the level `nova-cli`'s `nova run` tests cannot reach. So those tests
/// assert that such a program compiles and runs; these assert that it computes
/// the right value.
#[cfg(test)]
mod async_end_to_end {
    use nova_diagnostics::FileId;
    use nova_mir::{Function, MirTy, Module, RtFunc, Stmt, Temp, Terminator};

    /// One probe per test, because the executor's task table is a
    /// `thread_local!` and libtest gives each `#[test]` its own thread: the
    /// probe's own task is therefore always id 0. A second probe on the same
    /// thread would shift the ids and make `take_output(0)` quietly return the
    /// first probe's answer, so that is refused rather than reasoned about.
    const PROBE_TASK_ID: i64 = 0;

    thread_local! {
        static PROBED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }

    /// An argument to pass to the async function under test.
    enum Arg {
        Int(i64),
        Float(f64),
    }

    /// Compile `src`, call its `async fn <name>` with `args`, drive the
    /// resulting future to completion through the real executor, and return
    /// the raw 64 bits of its output slot.
    ///
    /// Raw bits, not a typed value: `STATE_SLOT_OUTPUT` is one 8-byte slot that
    /// the poll function stores through with the output's own machine class, and
    /// the executor copies out as an `i64` because it cannot tell a `Float` from
    /// an `Int` from a pointer. Comparing against `f64::to_bits` is therefore
    /// the strongest available check on a `Float` output — it fails if the store
    /// or the load used the wrong class, the wrong offset, or the wrong width.
    fn run_async_fn(src: &str, name: &str, args: &[Arg]) -> i64 {
        assert!(
            !PROBED.with(|p| p.replace(true)),
            "one probe per test: see PROBE_TASK_ID"
        );

        let (tokens, lex_errors) = nova_lexer::lex(src, FileId::DUMMY);
        assert!(lex_errors.is_empty(), "lex: {lex_errors:?}");
        let (ast, parse_errors) = nova_parser::parse(&tokens, FileId::DUMMY);
        assert!(parse_errors.is_empty(), "parse: {parse_errors:?}");
        let resolved = nova_resolver::resolve(&ast.expect("no AST"));
        assert!(
            resolved.diagnostics.is_empty(),
            "resolve: {:?}",
            resolved.diagnostics
        );
        let checked = nova_typeck::check(&resolved.file, &resolved.definitions);
        assert!(
            checked.diagnostics.is_empty(),
            "typeck: {:?}",
            checked.diagnostics
        );
        let mut mir = nova_mir::lower_module(&checked.module).expect("MIR lowering");

        // The wrapper: the half of the transform that kept the original
        // mangled symbol. Its `$poll` sibling is deliberately NOT called
        // directly -- going through the wrapper is what exercises the state
        // allocation, the parameter seeding and the fat-pointer construction.
        let prefix = format!("{name}.");
        let wrapper = mir
            .functions
            .iter()
            .map(|f| f.name.clone())
            .find(|n| n.starts_with(&prefix) && !n.ends_with("$poll"))
            .unwrap_or_else(|| {
                panic!(
                    "no wrapper for `{name}`; have {:?}",
                    mir.functions.iter().map(|f| &f.name).collect::<Vec<_>>()
                )
            });
        assert!(
            mir.functions
                .iter()
                .any(|f| f.name == format!("{wrapper}$poll")),
            "the transform must also have emitted `{wrapper}$poll`"
        );

        replace_main(&mut mir, &wrapper, args);
        let program = nova_codegen_cranelift::compile_jit(&mir).expect("JIT compile");
        program.run();

        // SAFETY: `PROBE_TASK_ID` was registered by the `TaskSpawn` above, on
        // this same thread -- `program.run()` ran the synthesized `main`
        // in-process.
        assert_eq!(
            unsafe { nova_runtime::task::nova_rt_task_is_done(PROBE_TASK_ID) },
            1,
            "the future must have completed: the executor only marks a task \
             done on POLL_READY, so a poll fn returning anything else panics \
             instead of reaching here"
        );
        // SAFETY: same task id, now known complete, and taken exactly once.
        unsafe { nova_runtime::task::nova_rt_task_take_output(PROBE_TASK_ID) }
    }

    /// Overwrite `main` with MIR that builds the future twice: once spawned as
    /// the probe (whose output is left in place for the caller to take), and
    /// once as `block_on`'s root, whose only job is to drain the queue so the
    /// probe gets polled. Two separate futures, because one future must not be
    /// registered as two tasks sharing a state object.
    fn replace_main(mir: &mut Module, wrapper: &str, args: &[Arg]) {
        let mut temps: Vec<MirTy> = Vec::new();
        let mut stmts: Vec<Stmt> = Vec::new();
        fn push(temps: &mut Vec<MirTy>, ty: MirTy) -> Temp {
            let t = Temp(temps.len() as u32);
            temps.push(ty);
            t
        }

        let mut arg_temps = Vec::new();
        for a in args {
            match a {
                Arg::Int(v) => {
                    let t = push(&mut temps, MirTy::I64);
                    stmts.push(Stmt::ConstInt(t, *v));
                    arg_temps.push(t);
                }
                Arg::Float(v) => {
                    let t = push(&mut temps, MirTy::F64);
                    stmts.push(Stmt::ConstFloat(t, *v));
                    arg_temps.push(t);
                }
            }
        }

        for spawn in [true, false] {
            let fut = push(&mut temps, MirTy::Ptr);
            stmts.push(Stmt::Call {
                dst: Some(fut),
                callee: wrapper.to_string(),
                args: arg_temps.clone(),
            });
            let status = push(&mut temps, MirTy::I64);
            stmts.push(Stmt::CallRuntime {
                dst: Some(status),
                func: if spawn {
                    RtFunc::TaskSpawn
                } else {
                    RtFunc::TaskBlockOn
                },
                args: vec![fut],
            });
        }

        let main = mir
            .functions
            .iter_mut()
            .find(|f| f.name == "main")
            .expect("`main` was lowered");
        *main = Function {
            name: "main".to_string(),
            params: 0,
            takes_env: false,
            capture_count: 0,
            temps,
            ret: MirTy::Unit,
            is_async: false,
            blocks: vec![nova_mir::Block {
                stmts,
                term: Terminator::Return(None),
            }],
        };
    }

    #[test]
    fn an_await_free_async_fn_returning_int_runs_and_produces_its_value() {
        let out = run_async_fn(
            "async fn f() -> Int { 40 + 2 }\nfn main() { let x = f() }",
            "f",
            &[],
        );
        assert_eq!(out, 42);
    }

    /// **The `Float` case, and the one that matters most.** `mir_ty` collapses
    /// `Int`, `Char` and every pointer-like type onto one 64-bit integer class,
    /// so an `Int` probe passes even if the output is stored, loaded or
    /// returned through the wrong one of them. `F64` is the only class that
    /// crosses register banks, and the exact type at which this project already
    /// shipped a reachable ICE: an `async fn f() -> Float { 1.5 }` merely
    /// called from `main` hit a Cranelift verifier error that the `Int` version
    /// silently survived.
    #[test]
    fn an_await_free_async_fn_returning_float_runs_and_produces_its_value() {
        let out = run_async_fn(
            "async fn f() -> Float { 1.5 }\nfn main() { let x = f() }",
            "f",
            &[],
        );
        assert_eq!(
            out as u64,
            1.5f64.to_bits(),
            "the output slot must hold 1.5 as an f64 bit pattern, not an \
             integer 1 or a truncation: got {out:#x}, want {:#x}",
            1.5f64.to_bits()
        );
    }

    /// Arguments reach the WRAPPER, not `poll` -- so a wrapper that forgets to
    /// copy them into their state slots leaves the body reading the allocator's
    /// zeroes. At `Float`, which also proves the seeding store and the body's
    /// reload agree on the register class, not merely on the offset.
    #[test]
    fn an_async_fns_parameters_reach_its_body_through_the_state_object() {
        let out = run_async_fn(
            "async fn add(a: Float, b: Float) -> Float { a + b }\n\
             fn main() { let x = add(1.0, 2.0) }",
            "add",
            &[Arg::Float(1.5), Arg::Float(2.25)],
        );
        assert_eq!(
            out as u64,
            3.75f64.to_bits(),
            "1.5 + 2.25 must reach the body; a zeroed parameter slot yields \
             0.0. Commutative, so `parameter_slots_are_seeded_in_order` \
             covers ordering: got {out:#x}"
        );
    }

    /// Ordering, which a commutative operator cannot see: `a - b` distinguishes
    /// parameter slot 0 from slot 1.
    #[test]
    fn parameter_slots_are_seeded_in_order() {
        let out = run_async_fn(
            "async fn sub(a: Float, b: Float) -> Float { a - b }\n\
             fn main() { let x = sub(1.0, 2.0) }",
            "sub",
            &[Arg::Float(10.0), Arg::Float(2.5)],
        );
        assert_eq!(
            out as u64,
            7.5f64.to_bits(),
            "10.0 - 2.5 = 7.5; reversed slots would give -7.5: got {out:#x}"
        );
    }

    /// A value carried across block boundaries entirely through state slots --
    /// the property Task 6 relies on. A `while` loop's accumulator is written
    /// in the body block and read in the header block, so if the spill were
    /// incomplete this returns garbage or zero rather than the sum.
    #[test]
    fn a_loop_accumulator_survives_the_spill_across_blocks() {
        let out = run_async_fn(
            "async fn total(n: Int) -> Int {\n  \
             let mut i = 0\n  \
             let mut t = 0\n  \
             while i < n { t = t + i\n i = i + 1 }\n  \
             t\n\
             }\n\
             fn main() { let x = total(1) }",
            "total",
            &[Arg::Int(10)],
        );
        assert_eq!(out, 45, "0+1+..+9 = 45");
    }

    /// A heap value built INSIDE the poll function, then reduced to a scalar
    /// output. The state object holds the interpolated `String`'s pointer in a
    /// temp slot while `str_len_chars` is called on it, so this covers three
    /// things no other probe here does: `nova_rt_alloc` running from inside a
    /// generated poll function, a `Ptr`-class value round-tripping through a
    /// state slot, and a runtime call in a poll body.
    ///
    /// A `String` OUTPUT is not asserted directly, because nothing public in
    /// `nova-runtime` decodes a `NovaStr` -- the output slot would only be
    /// readable as an opaque non-zero pointer, which a wrong slot could
    /// accidentally satisfy. Comparing the string inside Nova and returning the
    /// `Bool` checks the value instead, and adds the third distinct output class
    /// (`MirTy::I8`, an 8-byte slot holding one byte) to the two above.
    #[test]
    fn a_heap_value_allocated_inside_the_poll_fn_round_trips_through_a_state_slot() {
        let out = run_async_fn(
            "async fn f(n: Int) -> Bool { let s = \"ab${n}cd\"
 s == \"ab7cd\" }
             fn main() { let x = f(1) }",
            "f",
            &[Arg::Int(7)],
        );
        assert_eq!(
            out & 0xff, 1,
            "the interpolated string must equal \"ab7cd\"; a parameter that did              not reach the body would build \"ab0cd\" and give 0: got {out:#x}"
        );
    }

    /// A unit-returning `async fn` never touches the output slot itself, yet
    /// the executor reads it on completion regardless. What is asserted here is
    /// that the run completes at all: reaching `take_output` means the poll
    /// function returned exactly `POLL_READY` and the executor's unconditional
    /// read of `STATE_SLOT_OUTPUT` stayed inside a state object with the
    /// fewest temp slots any body can have -- the `STATE_MIN_SIZE` case.
    #[test]
    fn a_unit_returning_async_fn_completes_and_reads_a_valid_output_slot() {
        let out = run_async_fn("async fn f() { }\nfn main() { let x = f() }", "f", &[]);
        assert_eq!(out, 0, "the explicit zero the transform stores for unit");
    }
}
