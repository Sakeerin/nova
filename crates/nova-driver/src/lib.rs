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
        let modules = self.load_modules()?;
        if self.errors > 0 {
            return Ok(None);
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
