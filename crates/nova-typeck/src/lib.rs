//! Type checker and inference for the Nova compiler.
//!
//! Implements the Phase 1 subset of `nova-spec/12-TYPESYSTEM.md`:
//! Hindley-Milner style unification with explicit generics at function
//! boundaries (no let-polymorphism), sum types with Maranget-based
//! exhaustiveness/reachability checking, and desugaring of the AST into typed
//! HIR (`nova-hir`).
//!
//! Literals are typed concretely (`Int` / `Float`), so the spec's numeric
//! defaulting step is a no-op in this implementation.
//!
//! Trait resolution, records, closures and async are later Phase 1/2 steps;
//! encountering them reports an `E0900` "not supported yet" diagnostic
//! rather than panicking.

mod check;
mod infer;
mod usefulness;

pub use check::check;

use nova_hir::Ty;
use nova_resolver::Definitions;

/// Result of type checking one file.
#[derive(Debug)]
pub struct CheckResult {
    /// The typed module. Partially populated when errors were reported;
    /// only meaningful for codegen when `diagnostics` has no errors.
    pub module: nova_hir::Module,
    pub diagnostics: Vec<nova_diagnostics::Diagnostic>,
}

/// Render a type for use in diagnostics, resolving sum-type names.
pub fn display_ty(ty: &Ty, defs: &Definitions) -> String {
    match ty {
        Ty::Int => "Int".to_string(),
        Ty::Float => "Float".to_string(),
        Ty::Bool => "Bool".to_string(),
        Ty::Char => "Char".to_string(),
        Ty::String => "String".to_string(),
        Ty::Unit => "()".to_string(),
        Ty::Fn { params, ret } => {
            let params = params
                .iter()
                .map(|p| display_ty(p, defs))
                .collect::<Vec<_>>()
                .join(", ");
            format!("fn({params}) -> {}", display_ty(ret, defs))
        }
        Ty::Sum { def_id, args } | Ty::Record { def_id, args } => {
            let name = &defs.def(*def_id).name;
            if args.is_empty() {
                name.clone()
            } else {
                let args = args
                    .iter()
                    .map(|a| display_ty(a, defs))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{name}<{args}>")
            }
        }
        Ty::Array(elem) => format!("[{}]", display_ty(elem, defs)),
        Ty::Param(i) => format!("T{i}"),
        Ty::Var(v) => format!("?{v}"),
        Ty::Never => "!".to_string(),
        Ty::Error => "{error}".to_string(),
    }
}
