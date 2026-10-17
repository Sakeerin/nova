//! `view { … }` and `component` (RFC 0001; the 4.4 spec
//! `docs/superpowers/specs/2026-10-17-phase-4-4-component-syntax-design.md`
//! §4). A view's elements, text and attributes become the calls and
//! variants of `std/ui`'s API here, as string interpolation becomes calls:
//! no pass rewrites the tree before this one.

use nova_ast as ast;
use nova_ast::{ViewArg, ViewNode};
use nova_diagnostics::{Span, Spanned};
use nova_hir as hir;
use nova_hir::Ty;
use nova_resolver::{DefId, DefKind, ModuleId, Res, Target};

use super::{error_expr, Checker, FnCtx};

/// What a view desugars to: `std/ui`'s two sums, and the two builders of
/// `std/ui/html` it calls or shows.
struct Ui {
    view: DefId,
    attr: DefId,
    /// `on(kind, f)`, which adapts a `fn()` handler to `On`'s `fn(String)`.
    on: DefId,
    /// `text`, which a view's `text(…)` is shown as on hover.
    text: DefId,
}

impl<'a> Checker<'a> {
    /// `std/ui`'s names, which every target compiles (spec 4.3 §7.3), so a
    /// view needs no import of its own.
    fn ui(&self) -> Option<Ui> {
        let module = |name: &str| {
            (0..self.defs.module_count() as u32)
                .map(ModuleId)
                .find(|&m| self.defs.module_name(m) == name)
        };
        let ui = module("$std.ui")?;
        let html = module("$std.ui.html")?;
        let function = |name: &str| match self.defs.resolve_value(html, name) {
            Some(Res::Def(id)) => Some(id),
            _ => None,
        };
        Some(Ui {
            view: self.defs.resolve_type(ui, "View")?,
            attr: self.defs.resolve_type(ui, "Attr")?,
            on: function("on")?,
            text: function("text")?,
        })
    }

    /// A component's return type: `std/ui`'s `View` (§4.1).
    pub(super) fn component_ret(&self) -> Ty {
        match self.ui() {
            Some(ui) => sum(ui.view),
            None => Ty::Error,
        }
    }

    /// `view { child }`: the `View` its one child desugars to (§4.2).
    pub(super) fn check_view(
        &mut self,
        fcx: &mut FnCtx,
        children: &[Spanned<ViewNode>],
        span: Span,
    ) -> hir::Expr {
        let Some(ui) = self.ui() else {
            return error_expr(span);
        };
        if children.len() != 1 {
            self.error(
                "E0094",
                format!("`view` holds one root, found {}", children.len()),
                span,
            );
            for c in children {
                self.view_node(fcx, &ui, c);
            }
            return error_expr(span);
        }
        self.view_node(fcx, &ui, &children[0])
    }

    /// A child: an element, a view's `text(…)`, or an expression of type
    /// `View`.
    fn view_node(&mut self, fcx: &mut FnCtx, ui: &Ui, node: &Spanned<ViewNode>) -> hir::Expr {
        match &node.value {
            ViewNode::Element {
                name,
                args,
                children,
            } => self.element(fcx, ui, name, args, children.as_deref(), node.span),
            ViewNode::Expr(e) => {
                if let Some((word, content)) = text_child(e) {
                    return self.view_text(fcx, ui, word, content, e.span);
                }
                let x = self.check_expr(fcx, e);
                self.expect_ty(fcx, &x, &sum(ui.view), "a view's child");
                x
            }
        }
    }

    /// `text(e)`: `Text(e)` for a string literal, `DynText(|| e)` for
    /// anything else (§4.4).
    fn view_text(
        &mut self,
        fcx: &mut FnCtx,
        ui: &Ui,
        word: &Spanned<String>,
        content: &Spanned<ast::Expr>,
        span: Span,
    ) -> hir::Expr {
        self.note_use(word.span, Target::Def(ui.text));
        if is_str_lit(content) {
            let v = self.check_expr(fcx, content);
            return self.ui_variant(fcx, ui.view, "Text", vec![v], span);
        }
        let (f, ty) = self.reader(fcx, content);
        if !fcx.icx.unify(&ty, &Ty::String) {
            self.error(
                "E0010",
                format!(
                    "text must have type `String`, found `{}`",
                    self.show(&ty, fcx)
                ),
                content.span,
            );
            return error_expr(span);
        }
        self.ui_variant(fcx, ui.view, "DynText", vec![f], span)
    }

    /// An element: a call of its builder with its attributes and its
    /// children (§4.3).
    fn element(
        &mut self,
        fcx: &mut FnCtx,
        ui: &Ui,
        name: &Spanned<String>,
        args: &[ViewArg],
        children: Option<&[Spanned<ViewNode>]>,
        span: Span,
    ) -> hir::Expr {
        let builder = self.element_builder(fcx, ui, name);
        let attrs: Vec<hir::Expr> = args.iter().map(|a| self.view_attr(fcx, ui, a)).collect();
        let kids: Vec<hir::Expr> = children
            .unwrap_or(&[])
            .iter()
            .map(|c| self.view_node(fcx, ui, c))
            .collect();
        let Some((def_id, takes_children)) = builder else {
            return error_expr(span);
        };
        if !takes_children && children.is_some() {
            self.error(
                "E0095",
                format!("`{}` takes no children", name.value),
                name.span,
            );
            return error_expr(span);
        }
        let mut call_args = vec![array(attrs, sum(ui.attr), span)];
        if takes_children {
            call_args.push(array(kids, sum(ui.view), span));
        }
        hir::Expr {
            kind: hir::ExprKind::Call {
                func: hir::Callee::Def(def_id),
                type_args: Vec::new(),
                args: call_args,
            },
            ty: sum(ui.view),
            span,
        }
    }

    /// The function an element names, and whether it takes children: one
    /// that takes `([Attr], [View])`, or `([Attr])`, and returns `View`.
    fn element_builder(
        &mut self,
        fcx: &mut FnCtx,
        ui: &Ui,
        name: &Spanned<String>,
    ) -> Option<(DefId, bool)> {
        let n = name.value.as_str();
        let not_builder = |this: &mut Self| {
            this.error(
                "E0096",
                format!(
                    "`{n}` is not an element builder: one takes `([Attr], [View])` or \
                     `([Attr])` and returns `View`"
                ),
                name.span,
            );
            None
        };
        if fcx.lookup(n).is_some() {
            return not_builder(self);
        }
        let def_id = match self.defs.resolve_value(self.cur_module, n) {
            Some(Res::Def(id)) if matches!(self.defs.def(id).kind, DefKind::Fn { .. }) => id,
            None => {
                if !self.missing_on_target(n, name.span) {
                    self.error(
                        "E0001",
                        format!("cannot find function `{n}` in this scope"),
                        name.span,
                    );
                }
                return None;
            }
            Some(_) => return not_builder(self),
        };
        self.note_use(name.span, Target::Def(def_id));
        let sig = self.sigs.get(&def_id)?.clone();
        let attrs = Ty::Array(Box::new(sum(ui.attr)));
        let views = Ty::Array(Box::new(sum(ui.view)));
        if sig.generics != 0 || sig.ret != sum(ui.view) {
            return not_builder(self);
        }
        if sig.params == [attrs.clone(), views] {
            Some((def_id, true))
        } else if sig.params == [attrs] {
            Some((def_id, false))
        } else {
            not_builder(self)
        }
    }

    /// An element's argument: an `Attr` as it is, or a named one made
    /// into one (§4.3's second table).
    fn view_attr(&mut self, fcx: &mut FnCtx, ui: &Ui, arg: &ViewArg) -> hir::Expr {
        let (name, value) = match arg {
            ViewArg::Positional(e) => {
                let x = self.check_expr(fcx, e);
                self.expect_ty(fcx, &x, &sum(ui.attr), "an element's attribute");
                return x;
            }
            ViewArg::Named { name, value } => (name, value),
        };
        let span = name.span.merge(value.span);
        if let Some(kind) = name.value.strip_prefix("on_") {
            if kind.is_empty() {
                self.error("E0097", "`on_` names no event", name.span);
                return error_expr(span);
            }
            return self.view_handler(fcx, ui, name, kind, value, span);
        }
        let key = str_lit(name.value.replace('_', "-"), name.span);
        let property = name.value == "value";
        if is_str_lit(value) {
            let v = self.check_expr(fcx, value);
            let variant = if property { "Property" } else { "Static" };
            return self.ui_variant(fcx, ui.attr, variant, vec![key, v], span);
        }
        let (f, ty) = self.reader(fcx, value);
        if !fcx.icx.unify(&ty, &Ty::String) {
            self.error(
                "E0097",
                format!(
                    "`{}` takes a `String`, found `{}`",
                    name.value,
                    self.show(&ty, fcx)
                ),
                value.span,
            );
            return error_expr(span);
        }
        let variant = if property { "DynProperty" } else { "Dynamic" };
        self.ui_variant(fcx, ui.attr, variant, vec![key, f], span)
    }

    /// `on_kind: h`: `On("kind", h)` for a `fn(String)`, and `std/ui/html`'s
    /// `on("kind", h)` for a `fn()`.
    fn view_handler(
        &mut self,
        fcx: &mut FnCtx,
        ui: &Ui,
        name: &Spanned<String>,
        kind: &str,
        value: &Spanned<ast::Expr>,
        span: Span,
    ) -> hir::Expr {
        let kind = str_lit(kind.to_string(), name.span);
        let h = self.check_expr(fcx, value);
        match fcx.icx.apply(&h.ty) {
            Ty::Fn { params, ret } if params.is_empty() && fcx.icx.unify(&ret, &Ty::Unit) => {
                hir::Expr {
                    kind: hir::ExprKind::Call {
                        func: hir::Callee::Def(ui.on),
                        type_args: Vec::new(),
                        args: vec![kind, h],
                    },
                    ty: sum(ui.attr),
                    span,
                }
            }
            Ty::Fn { params, ret }
                if params.len() == 1
                    && fcx.icx.unify(&params[0], &Ty::String)
                    && fcx.icx.unify(&ret, &Ty::Unit) =>
            {
                self.ui_variant(fcx, ui.attr, "On", vec![kind, h], span)
            }
            Ty::Error => error_expr(span),
            other => {
                self.error(
                    "E0097",
                    format!(
                        "`{}` takes a handler, `fn()` or `fn(String)`, found `{}`",
                        name.value,
                        self.show(&other, fcx)
                    ),
                    value.span,
                );
                error_expr(span)
            }
        }
    }

    /// `|| value`, which reads `value` again each time it runs, and the type
    /// `value` has.
    fn reader(&mut self, fcx: &mut FnCtx, value: &Spanned<ast::Expr>) -> (hir::Expr, Ty) {
        let f = self.check_closure(fcx, &[], &None, value, value.span);
        let ty = match &f.ty {
            Ty::Fn { ret, .. } => (**ret).clone(),
            _ => Ty::Error,
        };
        (f, ty)
    }

    /// The variant `name` of `std/ui`'s sum `sum_id`, made of `args`.
    fn ui_variant(
        &mut self,
        fcx: &mut FnCtx,
        sum_id: DefId,
        name: &str,
        args: Vec<hir::Expr>,
        span: Span,
    ) -> hir::Expr {
        match self.variant_index(sum_id, name) {
            Some(vi) => self.make_variant(fcx, sum_id, vi, args, span),
            None => error_expr(span),
        }
    }
}

/// A child `text(e)`: the bare name `text` and one argument (§4.4).
fn text_child(e: &Spanned<ast::Expr>) -> Option<(&Spanned<String>, &Spanned<ast::Expr>)> {
    let ast::Expr::Call { callee, args } = &e.value else {
        return None;
    };
    let ast::Expr::Path(path) = &callee.value else {
        return None;
    };
    match (path.segments.as_slice(), args.as_slice()) {
        ([word], [content]) if word.value == "text" => Some((word, content)),
        _ => None,
    }
}

/// A string literal with no interpolation, which a view keeps fixed: a raw
/// string, `r"…"`, or a `StringInterp` of literal parts alone, as the
/// parser makes every other string.
fn is_str_lit(e: &Spanned<ast::Expr>) -> bool {
    match &e.value {
        ast::Expr::Lit(ast::Literal::Str(_)) => true,
        ast::Expr::StringInterp(parts) => {
            parts.iter().all(|p| matches!(p, ast::StringPart::Lit(_)))
        }
        _ => false,
    }
}

fn sum(def_id: DefId) -> Ty {
    Ty::Sum {
        def_id,
        args: Vec::new(),
    }
}

fn str_lit(s: String, span: Span) -> hir::Expr {
    hir::Expr {
        kind: hir::ExprKind::StrLit(s),
        ty: Ty::String,
        span,
    }
}

fn array(elems: Vec<hir::Expr>, elem: Ty, span: Span) -> hir::Expr {
    hir::Expr {
        kind: hir::ExprKind::MakeArray { elems },
        ty: Ty::Array(Box::new(elem)),
        span,
    }
}
