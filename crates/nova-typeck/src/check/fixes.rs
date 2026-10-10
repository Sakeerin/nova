//! The fixes the checker attaches to the errors it raises (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
//! §4; plan decision 3).

use nova_ast as ast;
use nova_ast::item::ImportKind;
use nova_diagnostics::{lines, suggest, Edit, FileId, Fix, Span, Spanned};
use nova_hir::{Ty, TyHead};
use nova_resolver::{DefId, DefKind, Exported, ModuleId, Res, ScopeEntry};

use super::{Checker, FnCtx, MethodRes};

/// How a local the user wrote was bound (spec §4.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Binding {
    /// By a `let`.
    Let,
    /// As a function's, a method's or a closure's parameter, `self`
    /// included.
    Param,
    /// By a pattern or a `for` loop, where `mut` does not parse.
    Other,
}

/// What an unknown name must be at its site (spec §4.2's table; plan
/// decision 6).
#[derive(Clone, Debug)]
pub(super) enum Need {
    /// A value: a function, a constant, or a variant without fields.
    Value,
    /// A call's callee, with the call's argument count.
    Call(usize),
    /// A type: a record or a sum type.
    Type,
    /// A record literal's record.
    Record,
    /// A trait.
    Trait,
    /// `T` in `T::member`; as a call's callee, `call` holds its argument
    /// count.
    Qualifier { member: String, call: Option<usize> },
    /// `T` in the pattern `T::variant` with `arity` fields; `scrutinee` is
    /// the matched value's sum type, when it is known.
    Pattern {
        variant: String,
        arity: usize,
        scrutinee: Option<DefId>,
    },
}

/// The primitive types' names, which "did you mean" offers for a type.
const PRIMITIVES: [&str; 7] = ["Bool", "Bytes", "Char", "Float", "Future", "Int", "String"];

/// The locals `fcx` can see; never the checker's own (`__it` and its kind).
pub(super) fn visible_locals(fcx: &FnCtx) -> Vec<String> {
    let mut names: Vec<String> = fcx
        .scopes
        .iter()
        .flat_map(|scope| scope.keys().cloned())
        .filter(|n| !n.starts_with("__"))
        .collect();
    names.sort();
    names.dedup();
    names
}

/// The sum type of a matched value, when inference knows it.
pub(super) fn scrutinee_sum(fcx: &FnCtx, ty: &Ty) -> Option<DefId> {
    match fcx.icx.apply(ty) {
        Ty::Sum { def_id, .. } => Some(def_id),
        _ => None,
    }
}

impl<'a> Checker<'a> {
    /// "make `x` mutable" (spec §4.1): `mut ` before the name of a local
    /// bound by a `let` or as a parameter. `decl` is that name's span.
    /// Never for `self`, since changing a receiver changes every caller,
    /// nor in a module this project does not own (plan decision 19).
    pub(super) fn mutable_fix(&self, name: &str, decl: Span) -> Option<Fix> {
        if name == "self" || !self.defs.owned(self.cur_module) {
            return None;
        }
        match self.bindings.get(&decl) {
            Some(Binding::Let | Binding::Param) => Some(Fix::new(
                format!("make `{name}` mutable"),
                vec![Edit::insert(decl.start, decl.file, "mut ")],
            )),
            _ => None,
        }
    }

    /// Add `fix`, if there is one, to the diagnostic just pushed, unless
    /// this project does not own the module being checked (spec §3.1; plan
    /// decision 19).
    pub(super) fn push_fix(&mut self, fix: Option<Fix>) {
        if !self.defs.owned(self.cur_module) {
            return;
        }
        if let (Some(fix), Some(d)) = (fix, self.diagnostics.last_mut()) {
            d.fixes.push(fix);
        }
    }

    /// The import and "did you mean" fixes for the unknown `name`, added to
    /// the diagnostic just pushed (spec §4.2, §4.3). `nearby` holds other
    /// names the site could mean: the visible locals, or the type
    /// parameters in scope.
    pub(super) fn name_fixes(&mut self, name: Spanned<String>, need: Need, nearby: Vec<String>) {
        let import = self.import_fix(&name, &need);
        let change = self.did_you_mean(&name, &need, &nearby);
        self.push_fix(import);
        self.push_fix(change);
    }

    /// "import `x` from `m`" (spec §4.2): the one module this one could
    /// import that exports an `x` of the kind `need` names, when importing
    /// it binds `x` nowhere this module already has it.
    fn import_fix(&self, name: &Spanned<String>, need: &Need) -> Option<Fix> {
        let x = name.value.as_str();
        let here = self.cur_module;
        let found: Vec<(&str, ModuleId)> = self
            .defs
            .importable(here)
            .iter()
            .filter(|(_, m)| self.fits(&self.defs.exported(*m, x), need))
            .map(|(import, m)| (import.as_str(), *m))
            .collect();
        let [(import, module)] = found.as_slice() else {
            return None;
        };
        let exported = self.defs.exported(*module, x);
        let bound = self.defs.bound_outside_std(here, x);
        let clash = (exported.value.is_some() && bound.value.is_some())
            || (exported.ty.is_some() && bound.ty.is_some())
            || (exported.trait_def.is_some() && bound.trait_def.is_some());
        if clash {
            return None;
        }
        let edit = self.import_edit(name.span.file, import, x)?;
        Some(Fix::new(
            format!("import `{x}` from `{import}`"),
            vec![edit],
        ))
    }

    /// Where `x` goes in `file`, this module's (spec §4.2, §4.7): into an
    /// `import m::{…}`, or a new line after the last import, or above the
    /// first item.
    fn import_edit(&self, file: FileId, import: &str, x: &str) -> Option<Edit> {
        let here = self.cur_module;
        let items: Vec<&ast::Item> = self
            .file
            .items
            .iter()
            .enumerate()
            .filter(|(i, _)| self.defs.module_of(*i) == here)
            .map(|(_, item)| &item.value)
            .collect();
        let spans: Vec<Span> = self
            .file
            .items
            .iter()
            .enumerate()
            .filter(|(i, _)| self.defs.module_of(*i) == here)
            .map(|(_, item)| item.span)
            .collect();
        let list = items.iter().rev().find_map(|item| match item {
            ast::Item::Import(imp) => match (&imp.kind, imp.path.value.segments.as_slice()) {
                (ImportKind::List(names), [seg]) if seg.value == import => names.last(),
                _ => None,
            },
            _ => None,
        });
        if let Some(last) = list {
            return Some(Edit::insert(last.span.end, file, format!(", {x}")));
        }
        let text = self.sources?.get_source(file)?;
        let line = format!("import {import}::{{{x}}}");
        let last_import = items
            .iter()
            .zip(&spans)
            .filter(|(item, _)| matches!(item, ast::Item::Import(_)))
            .map(|(_, span)| *span)
            .next_back();
        let (at, inserted) = match (last_import, spans.first()) {
            (Some(last), _) => lines::line_after(text, last.end as usize, &line),
            (None, Some(first)) => lines::line_before_item(text, first.start as usize, &line),
            (None, None) => (text.len(), format!("{line}{}", lines::ending(text))),
        };
        Some(Edit::insert(at as u32, file, inserted))
    }

    /// Whether `exported` holds an item of the kind `need` names (spec
    /// §4.2's table; plan decision 6).
    fn fits(&self, exported: &Exported, need: &Need) -> bool {
        let kind = |id: DefId| &self.defs.def(id).kind;
        match need {
            Need::Value => match exported.value {
                Some(Res::Def(id)) => {
                    matches!(kind(id), DefKind::Fn { .. } | DefKind::Const { .. })
                }
                Some(Res::Variant(sum, vi)) => self.variant_arity(sum, vi) == Some(0),
                _ => false,
            },
            Need::Call(n) => match exported.value {
                Some(Res::Def(id)) => self.arity(id) == Some(*n),
                Some(Res::Variant(sum, vi)) => self.variant_arity(sum, vi) == Some(*n),
                _ => false,
            },
            Need::Type => exported
                .ty
                .is_some_and(|id| matches!(kind(id), DefKind::Record { .. } | DefKind::Sum { .. })),
            Need::Record => exported
                .ty
                .is_some_and(|id| matches!(kind(id), DefKind::Record { .. })),
            Need::Trait => exported.trait_def.is_some(),
            Need::Qualifier { member, call } => exported
                .ty
                .is_some_and(|id| self.has_member(id, member, *call)),
            Need::Pattern {
                variant,
                arity,
                scrutinee,
            } => exported.ty.is_some_and(|id| {
                scrutinee.map_or(true, |s| s == id)
                    && self
                        .variant_named(id, variant)
                        .is_some_and(|(_, a)| a == *arity)
            }),
        }
    }

    /// A function's or an extern function's parameter count.
    fn arity(&self, id: DefId) -> Option<usize> {
        match self.defs.def(id).kind {
            DefKind::Fn { .. } => self.sigs.get(&id).map(|s| s.params.len()),
            DefKind::ExternFn { .. } => self
                .externs
                .iter()
                .find(|e| e.def_id == id)
                .map(|e| e.params.len()),
            _ => None,
        }
    }

    /// The field count of variant `vi` of sum type `sum`.
    fn variant_arity(&self, sum: DefId, vi: usize) -> Option<usize> {
        match &self.defs.def(sum).kind {
            DefKind::Sum { variants, .. } => variants.get(vi).map(|v| v.arity),
            _ => None,
        }
    }

    /// Sum type `sum`'s variant `name`: its index and field count.
    fn variant_named(&self, sum: DefId, name: &str) -> Option<(usize, usize)> {
        match &self.defs.def(sum).kind {
            DefKind::Sum { variants, .. } => variants
                .iter()
                .position(|v| v.name == name)
                .map(|i| (i, variants[i].arity)),
            _ => None,
        }
    }

    /// Whether type `id` has `member` for `T::member`: a variant without
    /// fields, or, as a call's callee with `call` arguments, a variant with
    /// that many or an inherent associated function taking them.
    fn has_member(&self, id: DefId, member: &str, call: Option<usize>) -> bool {
        let head = match self.defs.def(id).kind {
            DefKind::Record { .. } => TyHead::Record(id),
            DefKind::Sum { .. } => TyHead::Sum(id),
            _ => return false,
        };
        if let Some((_, arity)) = self.variant_named(id, member) {
            return arity == call.unwrap_or(0);
        }
        let Some(n) = call else {
            return false;
        };
        matches!(
            self.find_assoc_fns(head, member).as_slice(),
            [f] if self.sigs.get(f).is_some_and(|s| s.params.len() == n)
        )
    }

    /// "change `x` to `y`" (spec §4.3), `y` a name of the kind `need` says,
    /// in scope here or in `nearby`.
    fn did_you_mean(&self, name: &Spanned<String>, need: &Need, nearby: &[String]) -> Option<Fix> {
        let mut candidates: Vec<String> = nearby.to_vec();
        for (n, entry) in self.defs.names_in_scope(self.cur_module) {
            if self.offers(need, entry) {
                candidates.push(n);
            }
        }
        if matches!(need, Need::Type | Need::Qualifier { .. }) {
            candidates.extend(PRIMITIVES.iter().map(|p| p.to_string()));
        }
        suggest::change_to(name, candidates.iter().map(String::as_str))
    }

    /// Whether "did you mean" offers a name bound to `entry` where `need`
    /// holds (spec §4.3).
    fn offers(&self, need: &Need, entry: ScopeEntry) -> bool {
        let kind = |id: DefId| &self.defs.def(id).kind;
        match (need, entry) {
            (Need::Value, ScopeEntry::Value(Res::Def(id))) => {
                matches!(kind(id), DefKind::Fn { .. } | DefKind::Const { .. })
            }
            (Need::Value, ScopeEntry::Value(Res::Variant(sum, vi))) => {
                self.variant_arity(sum, vi) == Some(0)
            }
            (Need::Call(_), ScopeEntry::Value(Res::Def(id))) => {
                matches!(kind(id), DefKind::Fn { .. } | DefKind::ExternFn { .. })
            }
            (Need::Call(_), ScopeEntry::Value(Res::Variant(..) | Res::Builtin(_))) => true,
            (Need::Type | Need::Qualifier { .. }, ScopeEntry::Type(_)) => true,
            (Need::Record, ScopeEntry::Type(id)) => matches!(kind(id), DefKind::Record { .. }),
            (Need::Pattern { .. }, ScopeEntry::Type(id)) => matches!(kind(id), DefKind::Sum { .. }),
            (Need::Trait, ScopeEntry::Trait(_)) => true,
            _ => false,
        }
    }

    /// "change `f` to `g`" among the fields of `ty`, a record (spec §4.3).
    pub(super) fn field_fix(&self, ty: &Ty, field: &Spanned<String>) -> Option<Fix> {
        let Ty::Record { def_id, .. } = ty else {
            return None;
        };
        let record = self.records.iter().find(|r| r.def_id == *def_id)?;
        suggest::change_to(field, record.fields.iter().map(|f| f.name.as_str()))
    }

    /// "change `m` to `n`" among the methods a call on `recv` resolves:
    /// its inherent methods and those of every trait implemented for it,
    /// since resolution ignores imports (spec §4.3).
    pub(super) fn method_fix(
        &self,
        recv: &Ty,
        fcx: &FnCtx,
        method: &Spanned<String>,
    ) -> Option<Fix> {
        let trait_methods = |tid: DefId| {
            self.traits
                .iter()
                .filter(move |t| t.def_id == tid)
                .flat_map(|t| t.methods.iter().map(|m| m.name.clone()))
        };
        let mut names: Vec<String> = Vec::new();
        match recv {
            Ty::Param(k) => {
                for &tid in fcx.param_bounds.get(*k as usize).into_iter().flatten() {
                    names.extend(trait_methods(tid));
                }
            }
            _ => {
                if let Some(head) = recv.head() {
                    for imp in self.impls.iter().filter(|i| i.self_head == head) {
                        names.extend(imp.methods.iter().map(|(n, _)| n.clone()));
                        if let Some(tid) = imp.trait_id {
                            names.extend(trait_methods(tid));
                        }
                    }
                }
            }
        }
        names.sort();
        names.dedup();
        names.retain(|n| {
            matches!(
                self.resolve_method_on(recv, fcx, n),
                MethodRes::Inherent(_) | MethodRes::Trait(..)
            )
        });
        suggest::change_to(method, names.iter().map(String::as_str))
    }
}
