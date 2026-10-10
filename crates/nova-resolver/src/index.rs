//! The index of name occurrences the language server reads (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md` §3).
//!
//! The resolver records imports into it and the type checker everything
//! else. Neither reads it: it is written for the server alone.

use std::collections::{HashMap, HashSet};

use nova_diagnostics::{FileId, Span};

use crate::{Builtin, DefId, DefKind, Definitions, ModuleId};

/// Whether an occurrence declares its target or uses it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Declaration,
    Use,
}

/// What a name means (spec §3.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Target {
    /// A function, extern function, const, record, sum type, trait, impl
    /// method or associated type.
    Def(DefId),
    Variant(DefId, u32),
    Field(DefId, u32),
    /// A trait's method, by its index in the checker's list of the trait's
    /// methods.
    TraitMethod(DefId, u32),
    /// A local or parameter, named by its declaration's span.
    Local(Span),
    /// A type parameter, named by its declaration's span.
    TypeParam(Span),
    Module(ModuleId),
    Builtin(Builtin),
    /// An array's `len`.
    BuiltinMethod(&'static str),
    /// `Int`, `Float`, `Bool`, `Char`, `String`, `Bytes` or `Future`.
    Primitive(&'static str),
}

/// How a local was declared, for semantic tokens (spec 3.4b §7.2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalFlags {
    pub parameter: bool,
    pub mutable: bool,
}

/// One name in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Occurrence {
    /// The name's own token, not the expression around it.
    pub span: Span,
    pub role: Role,
    pub target: Target,
    /// A shorthand field `{ x }`, which is a field and a value at once.
    pub shorthand: bool,
}

/// Every name an analysis resolved (spec §3.1).
#[derive(Debug, Clone, Default)]
pub struct Index {
    pub occurrences: Vec<Occurrence>,
    /// Each impl method that implements a trait's method.
    pub implements: Vec<(DefId, (DefId, u32))>,
    /// Each local's type as hover shows it, by declaration span.
    pub types: HashMap<Span, String>,
    /// Each local's declaration span: whether it is a parameter and whether
    /// it is `mut` (spec 3.4b §4.6, §7.2). Recorded with the index only.
    pub locals: HashMap<Span, LocalFlags>,
    /// What `occurrences` holds, so a record is kept once (decision 28).
    seen: HashSet<(Span, Role, Target)>,
}

impl Index {
    /// Record an occurrence, unless the same one is already recorded.
    pub fn record(&mut self, span: Span, role: Role, target: Target) {
        if self.seen.insert((span, role, target)) {
            self.occurrences.push(Occurrence {
                span,
                role,
                target,
                shorthand: false,
            });
        }
    }

    /// Mark every occurrence at `span` as a shorthand field's.
    pub fn mark_shorthand(&mut self, span: Span) {
        for o in self.occurrences.iter_mut().filter(|o| o.span == span) {
            o.shorthand = true;
        }
    }

    pub fn has(&self, span: Span, role: Role, target: Target) -> bool {
        self.seen.contains(&(span, role, target))
    }

    /// Record that `method`, an impl's, implements `trait_method`.
    pub fn implement(&mut self, method: DefId, trait_method: (DefId, u32)) {
        if !self.implements.contains(&(method, trait_method)) {
            self.implements.push((method, trait_method));
        }
    }

    /// Record each of `occurrences`, as [`Index::record`] does.
    pub fn extend(&mut self, occurrences: impl IntoIterator<Item = Occurrence>) {
        for o in occurrences {
            self.record(o.span, o.role, o.target);
            if o.shorthand {
                self.mark_shorthand(o.span);
            }
        }
    }

    /// The occurrence at byte `offset` of `file`: one whose span holds it,
    /// or else one that ends there (spec §3.1). On a shared span the
    /// value's wins, then the type's, then the trait's, then a field's.
    pub fn at(&self, defs: &Definitions, file: FileId, offset: u32) -> Option<&Occurrence> {
        let holding: Vec<&Occurrence> = self
            .occurrences
            .iter()
            .filter(|o| o.span.file == file && o.span.start <= offset && offset < o.span.end)
            .collect();
        let found = if holding.is_empty() {
            self.occurrences
                .iter()
                .filter(|o| o.span.file == file && o.span.end == offset)
                .collect()
        } else {
            holding
        };
        found.into_iter().min_by_key(|o| rank(defs, &o.target))
    }

    /// Every occurrence of `target`, in recording order.
    pub fn of(&self, target: &Target) -> Vec<&Occurrence> {
        self.occurrences
            .iter()
            .filter(|o| &o.target == target)
            .collect()
    }

    pub fn declaration(&self, target: &Target) -> Option<&Occurrence> {
        self.occurrences
            .iter()
            .find(|o| &o.target == target && o.role == Role::Declaration)
    }

    /// `target`'s family (spec §3.1): a trait method and the impl methods
    /// that implement it, the trait's first; any other target alone.
    pub fn family(&self, target: &Target) -> Vec<Target> {
        let trait_method = match target {
            Target::TraitMethod(t, i) => Some((*t, *i)),
            Target::Def(m) => self
                .implements
                .iter()
                .find(|(d, _)| d == m)
                .map(|(_, tm)| *tm),
            _ => None,
        };
        let Some((t, i)) = trait_method else {
            return vec![*target];
        };
        let mut out = vec![Target::TraitMethod(t, i)];
        for (d, tm) in &self.implements {
            if *tm == (t, i) {
                out.push(Target::Def(*d));
            }
        }
        out
    }
}

/// Which of several occurrences on one span a request means: lower first.
pub fn rank(defs: &Definitions, target: &Target) -> u8 {
    match target {
        Target::Field(..) => 3,
        Target::Def(id) => match defs.defs().get(id.0 as usize).map(|d| &d.kind) {
            Some(DefKind::Record { .. } | DefKind::Sum { .. } | DefKind::AssocType { .. }) => 1,
            Some(DefKind::Trait { .. }) => 2,
            _ => 0,
        },
        Target::TypeParam(_) | Target::Primitive(_) => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_diagnostics::FileDb;

    fn files() -> (FileId, FileId) {
        let mut db = FileDb::new();
        (db.add("a.nova", "a"), db.add("b.nova", "b"))
    }

    fn span(file: FileId, start: u32, end: u32) -> Span {
        Span::new(start, end, file)
    }

    #[test]
    fn a_repeated_record_is_kept_once() {
        let (a, _) = files();
        let mut index = Index::default();
        let s = span(a, 4, 9);
        index.record(s, Role::Use, Target::Local(span(a, 0, 1)));
        index.record(s, Role::Use, Target::Local(span(a, 0, 1)));
        index.record(s, Role::Declaration, Target::Local(span(a, 0, 1)));
        assert_eq!(index.occurrences.len(), 2);
        assert!(index.has(s, Role::Use, Target::Local(span(a, 0, 1))));
    }

    #[test]
    fn a_cursor_inside_or_at_the_end_of_a_name_finds_it() {
        let (a, b) = files();
        let defs = Definitions::default();
        let mut index = Index::default();
        let target = Target::Local(span(a, 0, 5));
        index.record(span(a, 10, 15), Role::Use, target);
        for offset in [10, 12, 14, 15] {
            let found = index.at(&defs, a, offset).map(|o| o.span);
            assert_eq!(found, Some(span(a, 10, 15)), "offset {offset}");
        }
        assert_eq!(index.at(&defs, a, 9).map(|o| o.span), None);
        assert_eq!(index.at(&defs, a, 16).map(|o| o.span), None);
        assert_eq!(index.at(&defs, b, 12).map(|o| o.span), None);
    }

    #[test]
    fn a_cursor_between_two_names_finds_the_one_it_touches() {
        // Review Focus 5: `a+b`. Just after `a` is `a`'s end; just before
        // `b` is inside `b`, which wins over an end.
        let (f, _) = files();
        let defs = Definitions::default();
        let mut index = Index::default();
        let a = Target::Local(span(f, 100, 101));
        let b = Target::Local(span(f, 200, 201));
        index.record(span(f, 0, 1), Role::Use, a);
        index.record(span(f, 2, 3), Role::Use, b);
        assert_eq!(index.at(&defs, f, 1).map(|o| o.target), Some(a));
        assert_eq!(index.at(&defs, f, 2).map(|o| o.target), Some(b));
        // `a.b`'s shape: `a` ends where the `.` starts.
        index.record(span(f, 10, 11), Role::Use, a);
        index.record(span(f, 12, 13), Role::Use, b);
        assert_eq!(index.at(&defs, f, 11).map(|o| o.target), Some(a));
        assert_eq!(index.at(&defs, f, 12).map(|o| o.target), Some(b));
    }

    #[test]
    fn at_a_shorthand_the_value_wins() {
        let (f, _) = files();
        let defs = Definitions::default();
        let mut index = Index::default();
        let x = span(f, 20, 21);
        let field = Target::Field(DefId(3), 0);
        let local = Target::Local(span(f, 2, 3));
        index.record(x, Role::Use, field);
        index.record(x, Role::Use, local);
        index.mark_shorthand(x);
        let found = index.at(&defs, f, 20).unwrap();
        assert_eq!(found.target, local);
        assert!(index.occurrences.iter().all(|o| o.shorthand));
    }

    #[test]
    fn of_and_declaration_find_a_targets_occurrences() {
        let (f, _) = files();
        let mut index = Index::default();
        let t = Target::Def(DefId(7));
        index.record(span(f, 0, 3), Role::Declaration, t);
        index.record(span(f, 10, 13), Role::Use, t);
        index.record(span(f, 20, 23), Role::Use, Target::Def(DefId(8)));
        assert_eq!(index.of(&t).len(), 2);
        assert_eq!(index.declaration(&t).map(|o| o.span), Some(span(f, 0, 3)));
        assert!(index.declaration(&Target::Def(DefId(8))).is_none());
    }

    #[test]
    fn a_family_holds_a_trait_method_and_its_impls() {
        let mut index = Index::default();
        index.implement(DefId(10), (DefId(2), 1));
        index.implement(DefId(11), (DefId(2), 1));
        index.implement(DefId(11), (DefId(2), 1));
        index.implement(DefId(12), (DefId(2), 0));
        let family = vec![
            Target::TraitMethod(DefId(2), 1),
            Target::Def(DefId(10)),
            Target::Def(DefId(11)),
        ];
        assert_eq!(index.family(&Target::TraitMethod(DefId(2), 1)), family);
        assert_eq!(index.family(&Target::Def(DefId(11))), family);
        assert_eq!(index.implements.len(), 3);
        assert_eq!(
            index.family(&Target::Def(DefId(99))),
            vec![Target::Def(DefId(99))]
        );
    }

    #[test]
    fn extend_records_through_the_set() {
        let (f, _) = files();
        let mut index = Index::default();
        let o = Occurrence {
            span: span(f, 0, 1),
            role: Role::Use,
            target: Target::Module(ModuleId(1)),
            shorthand: false,
        };
        index.extend([o.clone(), o]);
        assert_eq!(index.occurrences.len(), 1);
    }
}
