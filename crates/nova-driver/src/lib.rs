//! Compilation pipeline orchestration for Nova.
//!
//! Drives source → lex → parse → resolve → typecheck → MIR →
//! Cranelift (JIT for `nova run`, object emission + native linking for
//! `nova build`), rendering diagnostics from every stage through
//! `nova-diagnostics`. This is the crate behind `nova run`,
//! `nova build`, and `nova check`.

mod link;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use nova_codegen_cranelift::CompiledProgram;
use nova_diagnostics::{render, Diagnostic, FileDb, Severity};

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
/// program is well-formed.
pub fn check_file(path: &Path) -> Result<Outcome<()>> {
    let mut ctx = FrontendContext::load(path)?;
    match ctx.check()? {
        Some(_) => Ok(Outcome::Ok(())),
        None => Ok(Outcome::Failed { errors: ctx.errors }),
    }
}

/// Compile a file to native code via the Cranelift JIT.
pub fn compile_file(path: &Path) -> Result<Outcome<CompiledProgram>> {
    let mir = match lower_to_mir(path)? {
        Outcome::Ok(mir) => mir,
        Outcome::Failed { errors } => return Ok(Outcome::Failed { errors }),
    };
    let program = nova_codegen_cranelift::compile_jit(&mir)
        .context("internal codegen error (this is a compiler bug)")?;
    Ok(Outcome::Ok(program))
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
    let obj_path = output.with_extension(obj_ext);
    std::fs::write(&obj_path, bytes)
        .with_context(|| format!("failed to write {}", obj_path.display()))?;

    let linked = link::link_executable(&obj_path, output);
    let _ = std::fs::remove_file(&obj_path);
    linked?;
    Ok(Outcome::Ok(output.to_path_buf()))
}

/// Run the front end and MIR lowering, rendering any diagnostics.
fn lower_to_mir(path: &Path) -> Result<Outcome<nova_mir::Module>> {
    let mut ctx = FrontendContext::load(path)?;
    let Some(module) = ctx.check()? else {
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
    file_id: nova_diagnostics::FileId,
    source: String,
    errors: usize,
}

impl FrontendContext {
    fn load(path: &Path) -> Result<Self> {
        let source = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let mut db = FileDb::new();
        let file_id = db.add(path.display().to_string(), source.as_str());
        Ok(Self {
            db,
            file_id,
            source,
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

    /// Run lex → parse → resolve → typecheck. Returns the typed module,
    /// or `None` if any stage reported errors.
    fn check(&mut self) -> Result<Option<nova_hir::Module>> {
        let (tokens, lex_errors) = nova_lexer::lex(&self.source, self.file_id);
        let lex_diags: Vec<Diagnostic> = lex_errors
            .iter()
            .map(|e| Diagnostic::error("L0001", e.to_string()).with_primary_label(e.span(), "here"))
            .collect();
        self.render(&lex_diags);

        let (ast, parse_errors) = nova_parser::parse(&tokens, self.file_id);
        let parse_diags: Vec<Diagnostic> = parse_errors
            .iter()
            .map(|e| Diagnostic::error("P0001", e.to_string()).with_primary_label(e.span(), "here"))
            .collect();
        self.render(&parse_diags);

        let Some(ast) = ast else {
            return Ok(None);
        };
        if self.errors > 0 {
            return Ok(None);
        }

        let resolved = nova_resolver::resolve(&ast);
        self.render(&resolved.diagnostics);
        if self.errors > 0 {
            return Ok(None);
        }

        let checked = nova_typeck::check(&ast, &resolved.definitions);
        self.render(&checked.diagnostics);
        if self.errors > 0 {
            return Ok(None);
        }
        Ok(Some(checked.module))
    }
}
