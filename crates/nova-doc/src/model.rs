//! From analyses to a site (spec §5 to §7; plan decisions 7 to 9, 13 and
//! 19).

use std::collections::{HashMap, HashSet};
use std::path::Path;

use nova_ast::item::{
    ConstDecl, ExternItem, ImplBlock, Record, TraitDecl, TraitItem, TypeDecl, TypeDef, Visibility,
    WhereBound,
};
use nova_ast::{Item as Ast, Type};
use nova_diagnostics::{Diagnostic, FileDb, FileId, Severity, Span, Spanned};
use nova_driver::{analyze_program, Analysis, DiskSources, Options, Program, Roots, Sources};
use nova_lexer::Token;
use nova_pm::PackageId;
use nova_resolver::index::rank;
use nova_resolver::{Builtin, DefKind, Definitions, ModuleId, Role, Target};

use crate::highlight::highlight;
use crate::markdown::{self, Docs};
use crate::signature::{self, FileText};

/// A program documented: its site, and the warnings its analysis found,
/// with the files they point into.
pub struct Documented {
    /// The site, for [`crate::write_site`].
    pub site: Site,
    /// The files the warnings point into.
    pub db: FileDb,
    /// The analysis's warnings, for the caller to print.
    pub warnings: Vec<Diagnostic>,
}

/// Why a program was not documented.
pub enum Failure {
    /// Its analysis found errors, for the caller to print.
    Diagnostics {
        /// The files the diagnostics point into.
        db: FileDb,
        /// Every diagnostic the analysis found, errors among them.
        diagnostics: Vec<Diagnostic>,
    },
    /// Anything else, in a sentence.
    Other(String),
}

/// A site: its packages, std last (spec §6.1).
pub struct Site {
    pub(crate) packages: Vec<Package>,
    /// The page `--open` opens, from the site's root.
    pub(crate) start: String,
}

/// A documented package.
pub(crate) struct Package {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
    pub std: bool,
    /// The root module first, then the rest by name; std's all by name.
    pub modules: Vec<Module>,
    /// std's alone.
    pub primitives: Vec<Primitive>,
    /// std's alone.
    pub builtins: Vec<BuiltinDoc>,
}

/// A documented module, one page (spec §7.2, §7.3).
pub(crate) struct Module {
    pub name: String,
    /// `package`, or `package::module`: its title and its search path.
    pub qual: String,
    /// Its page, from the site's root.
    pub page: String,
    /// Whether it is its package's root module, whose page is the
    /// package's.
    pub root: bool,
    pub docs: Docs,
    /// In source order.
    pub items: Vec<Item>,
    pub other_impls: Vec<Impl>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ItemKind {
    Record,
    Sum,
    Trait,
    Function,
    Constant,
}

impl ItemKind {
    /// The word its heading and the search index show.
    pub fn word(self) -> &'static str {
        match self {
            ItemKind::Record => "record",
            ItemKind::Sum => "type",
            ItemKind::Trait => "trait",
            ItemKind::Function => "fn",
            ItemKind::Constant => "const",
        }
    }

    /// The section a module page lists it under, and that section's `id`.
    pub fn section(self) -> (&'static str, &'static str) {
        match self {
            ItemKind::Record => ("Records", "records"),
            ItemKind::Sum => ("Sum types", "sum-types"),
            ItemKind::Trait => ("Traits", "traits"),
            ItemKind::Function => ("Functions", "functions"),
            ItemKind::Constant => ("Constants", "constants"),
        }
    }
}

/// A documented item (spec §7.3).
pub(crate) struct Item {
    pub kind: ItemKind,
    pub name: String,
    pub anchor: String,
    /// Its signature's HTML (spec §8).
    pub signature: String,
    pub docs: Docs,
    /// Fields, variants, associated types and methods, as §7.3 orders them.
    pub members: Vec<Member>,
    /// Whether it is a record with fields that are not documented.
    pub hidden_fields: bool,
    /// A record's or sum type's impls; a trait's, by their headers.
    pub impls: Vec<Impl>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MemberKind {
    Field,
    Variant,
    AssocType,
    Method,
}

impl MemberKind {
    /// The word the search index shows.
    pub fn word(self) -> &'static str {
        match self {
            MemberKind::Field => "field",
            MemberKind::Variant => "variant",
            MemberKind::AssocType => "type",
            MemberKind::Method => "method",
        }
    }

    /// The heading a page groups them under.
    pub fn heading(self) -> &'static str {
        match self {
            MemberKind::Field => "Fields",
            MemberKind::Variant => "Variants",
            MemberKind::AssocType => "Associated types",
            MemberKind::Method => "Methods",
        }
    }
}

/// A documented member.
pub(crate) struct Member {
    pub kind: MemberKind,
    pub name: String,
    pub anchor: String,
    pub signature: String,
    pub docs: Docs,
}

/// An impl: its header, its docs and its documented methods (spec §5.2).
pub(crate) struct Impl {
    pub header: String,
    pub docs: Docs,
    pub methods: Vec<Member>,
    /// The type's name, for the search index's paths.
    pub on: String,
}

/// A primitive type's page (spec §7.4).
pub(crate) struct Primitive {
    pub name: &'static str,
    pub page: String,
    pub about: Docs,
    pub impls: Vec<Impl>,
}

/// A builtin function, on std's front page (spec §7.4).
pub(crate) struct BuiltinDoc {
    pub name: &'static str,
    pub signature: String,
    pub docs: Docs,
}

/// The primitive types, with what their pages say first (plan decision
/// 19).
const PRIMITIVES: [(&str, &str); 7] = [
    (
        "Int",
        "A 64-bit signed integer. `+`, `-` and `*` on `Int` wrap on overflow.",
    ),
    ("Float", "A 64-bit floating-point number."),
    ("Bool", "`true` or `false`."),
    ("Char", "A Unicode scalar value."),
    ("String", "A string of UTF-8 text."),
    ("Bytes", "A sequence of bytes."),
    (
        "Future",
        "The work an `async fn` call stands for, which `.await` runs to its value.",
    ),
];

/// The builtin functions every module sees, and their docs (plan decision
/// 19).
const BUILTINS: [(&str, &str); 5] = [
    (
        "println",
        "Writes its argument and a line break to standard output.",
    ),
    (
        "print",
        "Writes its argument to standard output, with no line break.",
    ),
    (
        "eprintln",
        "Writes its argument and a line break to standard error.",
    ),
    (
        "eprint",
        "Writes its argument to standard error, with no line break.",
    ),
    (
        "panic",
        "Stops the program at once: writes `nova: panic: ` and its argument to standard \
         error, and aborts. Under `nova test`, a test marked `@test(should_panic)` expects it.",
    ),
];

/// What std's front page says first.
const STD_ABOUT: &str = "Nova's standard library, which every module sees without an import.";

/// The entry of std's own analysis, which no file holds (plan decision
/// 8).
const STD_ENTRY: &str = "<nova doc>/main.nova";

/// Document the package whose `nova.toml` is in `root` (empty for the
/// current directory), its dependencies and std (spec §5.1).
pub fn document_package(root: &Path) -> Result<Documented, Failure> {
    let (analysis, library) = package_analysis(root, false)?;
    let std = std_analysis()?;
    let mut packages = Ctx::new(&analysis, false, !library)?.packages();
    packages.push(Ctx::new(&std, true, false)?.std_package());
    clash(&packages)?;
    let start = format!("{}/index.html", packages[0].name);
    let warnings = analysis.diagnostics.clone();
    Ok(Documented {
        site: Site { packages, start },
        db: analysis.db,
        warnings,
    })
}

/// A library's own package, as its site documents it, and the analysis it
/// came from; `None` for a package with no library (3.5b spec §4.1). The
/// analysis is a test build's, as `nova test` reads the library (3.5b
/// spec §6.1).
pub(crate) fn library_package(root: &Path) -> Result<Option<(Package, Analysis)>, Failure> {
    let (analysis, library) = package_analysis(root, true)?;
    if !library {
        return Ok(None);
    }
    let packages = Ctx::new(&analysis, false, false)?.packages();
    let name = analysis.graph.as_ref().map(|g| g.root().name.clone());
    let package = packages
        .into_iter()
        .find(|p| Some(&p.name) == name.as_ref());
    Ok(package.map(|p| (p, analysis)))
}

/// std's package, as its site documents it, and std's analysis.
pub(crate) fn std_package() -> Result<(Package, Analysis), Failure> {
    let std = std_analysis()?;
    let package = Ctx::new(&std, true, false)?.std_package();
    Ok((package, std))
}

/// The analysis a package's site reads, of its library when it has one,
/// else of its program (spec §5.1), and whether it is a library. Errors
/// are a failure. Under `tests`, `std/test` is in scope, as in a test
/// build.
fn package_analysis(root: &Path, tests: bool) -> Result<(Analysis, bool), Failure> {
    let mut program = Program::for_package(root, Roots::Check);
    let stem = |p: &Path| p.file_stem().and_then(|s| s.to_str()).map(str::to_string);
    let library = program
        .roots
        .iter()
        .any(|r| stem(&r.path).as_deref() == Some("lib"));
    let keep = if library { "lib" } else { "main" };
    program
        .roots
        .retain(|r| stem(&r.path).as_deref() == Some(keep));
    program.runs = false;
    let options = Options {
        tests,
        index: true,
        module_only: true,
        ..Options::default()
    };
    let analysis = analyze_program(program, &DiskSources, &options)
        .map_err(|e| Failure::Other(format!("cannot read the program: {e}")))?;
    if analysis
        .diagnostics
        .iter()
        .any(|d| d.severity == Severity::Error)
    {
        return Err(Failure::Diagnostics {
            db: analysis.db,
            diagnostics: analysis.diagnostics,
        });
    }
    Ok((analysis, library))
}

/// Document std alone (spec §4.1, `--std`).
pub fn document_std() -> Result<Documented, Failure> {
    let (package, _) = std_package()?;
    Ok(Documented {
        site: Site {
            packages: vec![package],
            start: "std/index.html".to_string(),
        },
        db: FileDb::new(),
        warnings: Vec::new(),
    })
}

/// Serves std's analysis its entry, which no file holds.
struct StdEntry;

impl Sources for StdEntry {
    fn read(&self, path: &Path) -> std::io::Result<String> {
        if path == Path::new(STD_ENTRY) {
            Ok("fn main() {}\n".to_string())
        } else {
            Err(std::io::Error::from(std::io::ErrorKind::NotFound))
        }
    }
}

/// An analysis of std alone, `std/test` included (plan decision 8).
fn std_analysis() -> Result<Analysis, Failure> {
    let options = Options {
        tests: true,
        index: true,
        module_only: true,
        ..Options::default()
    };
    let a = analyze_program(Program::loose(Path::new(STD_ENTRY)), &StdEntry, &options)
        .map_err(|e| Failure::Other(format!("cannot analyse std: {e}")))?;
    if a.diagnostics.iter().any(|d| d.severity == Severity::Error) {
        return Err(Failure::Diagnostics {
            db: a.db,
            diagnostics: a.diagnostics,
        });
    }
    Ok(a)
}

/// Spec §4.2: two packages, or two modules of one package, that would
/// share a directory, names that differ in ASCII case alone included.
fn clash(packages: &[Package]) -> Result<(), Failure> {
    for (k, p) in packages.iter().enumerate() {
        if !p.std && p.name.eq_ignore_ascii_case("std") {
            return Err(Failure::Other(format!(
                "the package `{}` would share std's pages in target/doc/std/, so nova doc \
                 cannot document it",
                p.name
            )));
        }
        if let Some(q) = packages[..k]
            .iter()
            .find(|q| q.name.eq_ignore_ascii_case(&p.name))
        {
            return Err(Failure::Other(format!(
                "the packages `{}` and `{}` would share one directory in target/doc/, so nova \
                 doc cannot document them",
                q.name, p.name
            )));
        }
        // The root module's page is the package's own.
        let modules: Vec<&Module> = p.modules.iter().filter(|m| !m.root).collect();
        for (j, m) in modules.iter().enumerate() {
            if let Some(n) = modules[..j]
                .iter()
                .find(|n| n.name.eq_ignore_ascii_case(&m.name))
            {
                return Err(Failure::Other(format!(
                    "the modules `{}` and `{}` would share one directory in target/doc/{}/, so \
                     nova doc cannot document them",
                    n.qual, m.qual, p.name
                )));
            }
        }
    }
    Ok(())
}

/// Which namespace an item is exported in.
#[derive(Clone, Copy)]
enum Ns {
    Value,
    Type,
    Trait,
}

/// An impl met while building modules, placed once all are built.
struct Pending<'a> {
    module: ModuleId,
    span: Span,
    block: &'a ImplBlock,
}

/// Where an impl goes (spec §5.2).
enum Dest {
    /// With its type: the type's page and anchor.
    Item(String, String),
    /// On a primitive's page.
    Primitive(&'static str),
    /// On its own module's page, under "Implementations on other types".
    Own,
}

/// One analysis, read for its site.
struct Ctx<'a> {
    a: &'a Analysis,
    defs: &'a Definitions,
    file: &'a nova_ast::File,
    /// The program's own modules, which come before std's.
    users: usize,
    /// Whether this is std's own analysis, whose entry has no page.
    std: bool,
    /// Whether the root package documents every item and member (spec
    /// §5.1; plan decision 7).
    program: bool,
    /// Each name's target, by its span (spec §6.3).
    uses: HashMap<Span, Target>,
    /// Each rendered module's file, lexed once.
    texts: HashMap<FileId, FileText<'a>>,
}

impl<'a> Ctx<'a> {
    fn new(a: &'a Analysis, std: bool, program: bool) -> Result<Ctx<'a>, Failure> {
        let incomplete =
            || Failure::Other("the analysis is incomplete; this is a nova doc bug".to_string());
        let defs = a.definitions.as_ref().ok_or_else(incomplete)?;
        let file = a.file.as_ref().ok_or_else(incomplete)?;
        let index = a.index.as_ref().ok_or_else(incomplete)?;
        let mut uses: HashMap<Span, Target> = HashMap::new();
        for o in index.occurrences.iter().filter(|o| o.role == Role::Use) {
            let keep = uses
                .get(&o.span)
                .map_or(true, |t| rank(defs, &o.target) < rank(defs, t));
            if keep {
                uses.insert(o.span, o.target);
            }
        }
        let mut ctx = Ctx {
            a,
            defs,
            file,
            users: a.modules.len(),
            std,
            program,
            uses,
            texts: HashMap::new(),
        };
        for m in 0..defs.module_count() {
            let m = ModuleId(m as u32);
            if !ctx.renders(m) {
                continue;
            }
            if let Some(id) = ctx.module_file(m) {
                if let Some(text) = a.db.get_source(id) {
                    ctx.texts.insert(id, FileText::new(text, id));
                }
            }
        }
        Ok(ctx)
    }

    /// Whether this analysis renders module `m`'s page: std's analysis its
    /// std modules, a project's analysis its own.
    fn renders(&self, m: ModuleId) -> bool {
        ((m.0 as usize) >= self.users) == self.std
    }

    fn std_name(&self, m: ModuleId) -> Option<&str> {
        self.defs.module_name(m).strip_prefix("$std.")
    }

    fn module_file(&self, m: ModuleId) -> Option<FileId> {
        let i = m.0 as usize;
        if i < self.users {
            self.a.modules.get(i).map(|(f, _)| *f)
        } else {
            self.a.db.id_of(&format!("<std/{}>", self.std_name(m)?))
        }
    }

    fn package_id(&self, m: ModuleId) -> Option<PackageId> {
        self.a.module_packages.get(m.0 as usize).copied().flatten()
    }

    fn package_name(&self, m: ModuleId) -> Option<&str> {
        Some(&self.a.graph.as_ref()?.package(self.package_id(m)?).name)
    }

    /// The stem of the root module of `m`'s package (plan decision 9).
    fn root_stem(&self, m: ModuleId) -> &'static str {
        if self.program && self.package_id(m) == Some(PackageId(0)) {
            "main"
        } else {
            "lib"
        }
    }

    /// Module `m`'s page, from the site's root (spec §6.1). Every std
    /// module has one, whichever analysis asks.
    fn page(&self, m: ModuleId) -> Option<String> {
        let i = m.0 as usize;
        if i >= self.users {
            return Some(format!("std/{}/index.html", self.std_name(m)?));
        }
        if self.std {
            return None;
        }
        let stem = self.a.modules.get(i)?.1.file_stem()?.to_str()?;
        let package = self.package_name(m)?;
        Some(if stem == self.root_stem(m) {
            format!("{package}/index.html")
        } else {
            format!("{package}/{stem}/index.html")
        })
    }

    /// Whether module `m` documents every item and member: it is the root
    /// package's, and that package is only a program.
    fn all(&self, m: ModuleId) -> bool {
        self.program && (m.0 as usize) < self.users && self.package_id(m) == Some(PackageId(0))
    }

    fn same_package(&self, a: ModuleId, b: ModuleId) -> bool {
        let std = |m: ModuleId| m.0 as usize >= self.users;
        if std(a) || std(b) {
            std(a) && std(b)
        } else {
            self.package_id(a) == self.package_id(b)
        }
    }

    /// Whether module `m` documents its item `name` (spec §5.1).
    fn shown(&self, m: ModuleId, ns: Ns, name: &str) -> bool {
        if self.all(m) {
            return true;
        }
        let e = self.defs.exported(m, name);
        match ns {
            Ns::Value => e.value.is_some(),
            Ns::Type => e.ty.is_some(),
            Ns::Trait => e.trait_def.is_some(),
        }
    }

    fn member_shown(&self, m: ModuleId, vis: Visibility) -> bool {
        self.all(m) || vis == Visibility::Pub
    }

    /// `target`'s page and anchor from the site's root, when the site
    /// documents it (spec §6.3).
    fn url(&self, target: &Target) -> Option<String> {
        match *target {
            Target::Primitive(p) => Some(format!("std/primitive.{p}.html")),
            Target::Def(id) => {
                let def = self.defs.def(id);
                let item = |item_index: usize, ns: Ns, prefix: &str| {
                    let m = self.defs.module_of(item_index);
                    if !self.shown(m, ns, &def.name) {
                        return None;
                    }
                    Some(format!("{}#{prefix}.{}", self.page(m)?, def.name))
                };
                match &def.kind {
                    DefKind::Fn { item_index } => item(*item_index, Ns::Value, "fn"),
                    DefKind::Const { item_index } => item(*item_index, Ns::Value, "const"),
                    DefKind::Record { item_index } => item(*item_index, Ns::Type, "record"),
                    DefKind::Sum { item_index, .. } => item(*item_index, Ns::Type, "type"),
                    DefKind::Trait { item_index } => item(*item_index, Ns::Trait, "trait"),
                    DefKind::ExternFn { item_index, .. } => {
                        let m = self.defs.module_of(*item_index);
                        if !self.all(m) {
                            return None;
                        }
                        Some(format!("{}#fn.{}", self.page(m)?, def.name))
                    }
                    DefKind::AssocType { trait_def } => {
                        let t = self.url(&Target::Def(*trait_def))?;
                        Some(format!("{t}.type.{}", def.name))
                    }
                    DefKind::Method { .. } => None,
                }
            }
            Target::Variant(sum, vi) => {
                let base = self.url(&Target::Def(sum))?;
                let DefKind::Sum { variants, .. } = &self.defs.def(sum).kind else {
                    return None;
                };
                Some(format!(
                    "{base}.variant.{}",
                    variants.get(vi as usize)?.name
                ))
            }
            Target::Field(rec, fi) => {
                let base = self.url(&Target::Def(rec))?;
                let DefKind::Record { item_index } = self.defs.def(rec).kind else {
                    return None;
                };
                let Ast::Record(r) = &self.file.items.get(item_index)?.value else {
                    return None;
                };
                let field = r.fields.get(fi as usize)?;
                if !self.member_shown(self.defs.module_of(item_index), field.vis) {
                    return None;
                }
                Some(format!("{base}.field.{}", field.name.value))
            }
            _ => None,
        }
    }

    /// `text[start..end]`'s signature for a page at `page` (spec §8).
    fn sig(
        &self,
        ft: &FileText,
        page: &str,
        start: usize,
        end: usize,
        own: Option<Span>,
    ) -> String {
        signature::render(ft, start, end, own, &|span| {
            self.uses
                .get(&span)
                .and_then(|t| self.url(t))
                .map(|url| relative(page, &url))
        })
    }
}

/// `url`, from the site's root, as seen from `page` (plan decision 13).
pub(crate) fn relative(page: &str, url: &str) -> String {
    format!("{}{url}", "../".repeat(page.matches('/').count()))
}

/// Doc lines rendered, their headings moved down `level` levels, keeping
/// their source (3.5b spec §11.1).
fn docs_of(lines: &[Spanned<String>], level: u8) -> Docs {
    markdown::render_source(markdown::join(lines), level)
}

/// The last segment of a path type: what an impl is on.
fn type_name(ty: &Type) -> Option<&Spanned<String>> {
    match ty {
        Type::Path { path, .. } => path.segments.last(),
        _ => None,
    }
}

/// `base`, or `base-2`, `base-3` and so on: the first not yet on its page
/// (spec §6.2).
fn unique(taken: &mut HashSet<String>, base: String) -> String {
    if taken.insert(base.clone()) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}-{n}"))
        .find(|a| taken.insert(a.clone()))
        .expect("an unused id")
}

impl<'a> Ctx<'a> {
    /// Module `m` and its items; its impls go to `pending` (spec §7.3).
    fn module(&self, m: ModuleId, pending: &mut Vec<Pending<'a>>) -> Option<Module> {
        let page = self.page(m)?;
        let ft = self.texts.get(&self.module_file(m)?)?;
        let (name, qual, root) = if m.0 as usize >= self.users {
            let n = self.std_name(m)?.to_string();
            let qual = format!("std::{n}");
            (n, qual, false)
        } else {
            let stem = self
                .a
                .modules
                .get(m.0 as usize)?
                .1
                .file_stem()?
                .to_str()?
                .to_string();
            let package = self.package_name(m)?.to_string();
            let root = stem == self.root_stem(m);
            let qual = if root {
                package
            } else {
                format!("{package}::{stem}")
            };
            (stem, qual, root)
        };
        let mut items = Vec::new();
        for (i, item) in self.file.items.iter().enumerate() {
            if self.defs.module_of(i) != m {
                continue;
            }
            match &item.value {
                Ast::Function(f) if self.shown(m, Ns::Value, &f.name.value) => {
                    items.push(self.function(
                        ft,
                        &page,
                        &f.name,
                        &f.docs,
                        f.return_ty.as_ref(),
                        &f.where_clause,
                    ))
                }
                Ast::Extern(block) if self.all(m) => {
                    for e in &block.items {
                        let ExternItem::Fn(sig) = e;
                        items.push(self.function(
                            ft,
                            &page,
                            &sig.name,
                            &sig.docs,
                            sig.return_ty.as_ref(),
                            &sig.where_clause,
                        ));
                    }
                }
                Ast::Record(r) if self.shown(m, Ns::Type, &r.name.value) => {
                    items.push(self.record(ft, &page, m, r))
                }
                Ast::Type(t)
                    if matches!(t.def, TypeDef::Sum(_))
                        && self.shown(m, Ns::Type, &t.name.value) =>
                {
                    items.push(self.sum(ft, &page, t))
                }
                Ast::Trait(t) if self.shown(m, Ns::Trait, &t.name.value) => {
                    items.push(self.trait_item(ft, &page, t))
                }
                Ast::Const(c) if self.shown(m, Ns::Value, &c.name.value) => {
                    items.push(self.constant(ft, &page, c))
                }
                Ast::Impl(block) => pending.push(Pending {
                    module: m,
                    span: item.span,
                    block,
                }),
                _ => {}
            }
        }
        Some(Module {
            name,
            qual,
            page,
            root,
            docs: markdown::render_source(ft.module_docs(), 1),
            items,
            other_impls: Vec::new(),
        })
    }

    fn function(
        &self,
        ft: &FileText,
        page: &str,
        name: &Spanned<String>,
        docs: &[Spanned<String>],
        ret: Option<&Spanned<Type>>,
        wheres: &[WhereBound],
    ) -> Item {
        let start = signature::decl_start(ft.text, name.span.start as usize);
        let end = signature::fn_end(ft, name.span, ret, wheres);
        Item {
            kind: ItemKind::Function,
            name: name.value.clone(),
            anchor: format!("fn.{}", name.value),
            signature: self.sig(ft, page, start, end, Some(name.span)),
            docs: docs_of(docs, 3),
            members: Vec::new(),
            hidden_fields: false,
            impls: Vec::new(),
        }
    }

    fn record(&self, ft: &FileText, page: &str, m: ModuleId, r: &Record) -> Item {
        let start = signature::decl_start(ft.text, r.name.span.start as usize);
        let end = ft
            .next(r.name.span.end as usize, |t| *t == Token::LBrace)
            .unwrap_or(r.name.span.end as usize);
        let anchor = format!("record.{}", r.name.value);
        let mut members = Vec::new();
        let mut hidden = false;
        for f in &r.fields {
            if !self.member_shown(m, f.vis) {
                hidden = true;
                continue;
            }
            let start = signature::decl_start(ft.text, f.name.span.start as usize);
            members.push(Member {
                kind: MemberKind::Field,
                name: f.name.value.clone(),
                anchor: format!("{anchor}.field.{}", f.name.value),
                signature: self.sig(ft, page, start, f.ty.span.end as usize, Some(f.name.span)),
                docs: docs_of(&f.docs, 4),
            });
        }
        Item {
            kind: ItemKind::Record,
            name: r.name.value.clone(),
            signature: self.sig(ft, page, start, end, Some(r.name.span)),
            anchor,
            docs: docs_of(&r.docs, 3),
            members,
            hidden_fields: hidden,
            impls: Vec::new(),
        }
    }

    fn sum(&self, ft: &FileText, page: &str, t: &TypeDecl) -> Item {
        let start = signature::decl_start(ft.text, t.name.span.start as usize);
        let end = ft
            .next(t.name.span.end as usize, |t| *t == Token::Eq)
            .unwrap_or(t.name.span.end as usize);
        let anchor = format!("type.{}", t.name.value);
        let members = match &t.def {
            TypeDef::Sum(variants) => variants
                .iter()
                .map(|v| {
                    let end = if v.fields.is_empty() {
                        v.name.span.end as usize
                    } else {
                        ft.group_end(v.name.span.end as usize)
                            .unwrap_or(v.name.span.end as usize)
                    };
                    Member {
                        kind: MemberKind::Variant,
                        name: v.name.value.clone(),
                        anchor: format!("{anchor}.variant.{}", v.name.value),
                        signature: self.sig(
                            ft,
                            page,
                            v.name.span.start as usize,
                            end,
                            Some(v.name.span),
                        ),
                        docs: docs_of(&v.docs, 4),
                    }
                })
                .collect(),
            TypeDef::Alias(_) => Vec::new(),
        };
        Item {
            kind: ItemKind::Sum,
            name: t.name.value.clone(),
            signature: self.sig(ft, page, start, end, Some(t.name.span)),
            anchor,
            docs: docs_of(&t.docs, 3),
            members,
            hidden_fields: false,
            impls: Vec::new(),
        }
    }

    fn trait_item(&self, ft: &FileText, page: &str, t: &TraitDecl) -> Item {
        let start = signature::decl_start(ft.text, t.name.span.start as usize);
        let end = ft
            .next(t.name.span.end as usize, |t| *t == Token::LBrace)
            .unwrap_or(t.name.span.end as usize);
        let anchor = format!("trait.{}", t.name.value);
        let mut types = Vec::new();
        let mut methods = Vec::new();
        for item in &t.items {
            match item {
                TraitItem::AssocType { docs, name, bounds } => {
                    let end = bounds.last().map_or(name.span.end, |b| b.span.end) as usize;
                    let start = signature::decl_start(ft.text, name.span.start as usize);
                    types.push(Member {
                        kind: MemberKind::AssocType,
                        name: name.value.clone(),
                        anchor: format!("{anchor}.type.{}", name.value),
                        signature: self.sig(ft, page, start, end, Some(name.span)),
                        docs: docs_of(docs, 4),
                    });
                }
                TraitItem::Required(s) => methods.push(self.method(
                    ft,
                    page,
                    &format!("{anchor}.method"),
                    &s.name,
                    &s.docs,
                    s.return_ty.as_ref(),
                    &s.where_clause,
                )),
                TraitItem::Provided(f) => methods.push(self.method(
                    ft,
                    page,
                    &format!("{anchor}.method"),
                    &f.name,
                    &f.docs,
                    f.return_ty.as_ref(),
                    &f.where_clause,
                )),
            }
        }
        types.extend(methods);
        Item {
            kind: ItemKind::Trait,
            name: t.name.value.clone(),
            signature: self.sig(ft, page, start, end, Some(t.name.span)),
            anchor,
            docs: docs_of(&t.docs, 3),
            members: types,
            hidden_fields: false,
            impls: Vec::new(),
        }
    }

    /// A method, its anchor `<prefix>.<name>`.
    #[allow(clippy::too_many_arguments)]
    fn method(
        &self,
        ft: &FileText,
        page: &str,
        prefix: &str,
        name: &Spanned<String>,
        docs: &[Spanned<String>],
        ret: Option<&Spanned<Type>>,
        wheres: &[WhereBound],
    ) -> Member {
        let start = signature::decl_start(ft.text, name.span.start as usize);
        let end = signature::fn_end(ft, name.span, ret, wheres);
        Member {
            kind: MemberKind::Method,
            name: name.value.clone(),
            anchor: format!("{prefix}.{}", name.value),
            signature: self.sig(ft, page, start, end, Some(name.span)),
            docs: docs_of(docs, 4),
        }
    }

    fn constant(&self, ft: &FileText, page: &str, c: &ConstDecl) -> Item {
        let start = signature::decl_start(ft.text, c.name.span.start as usize);
        Item {
            kind: ItemKind::Constant,
            name: c.name.value.clone(),
            anchor: format!("const.{}", c.name.value),
            signature: self.sig(
                ft,
                page,
                start,
                c.value.span.end as usize,
                Some(c.name.span),
            ),
            docs: docs_of(&c.docs, 3),
            members: Vec::new(),
            hidden_fields: false,
            impls: Vec::new(),
        }
    }
}

impl<'a> Ctx<'a> {
    /// Each impl in `pending`, where spec §5.2 puts it.
    fn place(
        &self,
        modules: &mut [Module],
        primitives: &mut [Primitive],
        pending: Vec<Pending<'a>>,
    ) {
        let mut taken: HashMap<String, HashSet<String>> = HashMap::new();
        for m in modules.iter() {
            let set = taken.entry(m.page.clone()).or_default();
            for item in &m.items {
                set.insert(item.anchor.clone());
                for member in &item.members {
                    set.insert(member.anchor.clone());
                }
            }
        }
        for p in pending {
            let block = p.block;
            let Some(ft) = self.module_file(p.module).and_then(|f| self.texts.get(&f)) else {
                continue;
            };
            let Some(own_page) = self.page(p.module) else {
                continue;
            };
            let on = type_name(&block.ty.value);
            let on_name = on.map_or_else(|| "type".to_string(), |s| s.value.clone());
            let ty = on.and_then(|s| self.uses.get(&s.span)).copied();
            let dest = match ty {
                Some(Target::Def(id)) => {
                    let item_index = match self.defs.def(id).kind {
                        DefKind::Record { item_index } | DefKind::Sum { item_index, .. } => {
                            Some(item_index)
                        }
                        _ => None,
                    };
                    match item_index {
                        Some(ii) if self.same_package(self.defs.module_of(ii), p.module) => {
                            // A type of this package that is not documented
                            // takes its impls with it.
                            let Some(url) = self.url(&Target::Def(id)) else {
                                continue;
                            };
                            let (page, anchor) = url.split_once('#').unwrap_or((url.as_str(), ""));
                            Dest::Item(page.to_string(), anchor.to_string())
                        }
                        _ => Dest::Own,
                    }
                }
                Some(Target::Primitive(name)) if self.std => Dest::Primitive(name),
                _ => Dest::Own,
            };
            let page = match &dest {
                Dest::Item(page, _) => page.clone(),
                Dest::Primitive(name) => format!("std/primitive.{name}.html"),
                Dest::Own => own_page.clone(),
            };
            let header = self.impl_header(ft, &page, p.span, block);
            let mut methods = Vec::new();
            if block.trait_.is_none() {
                let prefix = match &dest {
                    Dest::Item(_, anchor) => format!("{anchor}.method"),
                    Dest::Primitive(_) => "method".to_string(),
                    Dest::Own => format!("impl.{on_name}.method"),
                };
                for f in block
                    .functions
                    .iter()
                    .filter(|f| self.member_shown(p.module, f.vis))
                {
                    let mut m = self.method(
                        ft,
                        &page,
                        &prefix,
                        &f.name,
                        &f.docs,
                        f.return_ty.as_ref(),
                        &f.where_clause,
                    );
                    m.anchor = unique(taken.entry(page.clone()).or_default(), m.anchor);
                    methods.push(m);
                }
            }
            let built = Impl {
                header,
                docs: docs_of(&block.docs, 4),
                methods,
                on: on_name.clone(),
            };
            match &dest {
                Dest::Item(page, anchor) => {
                    if let Some(item) = find_item(modules, page, anchor) {
                        item.impls.push(built);
                    }
                }
                Dest::Primitive(name) => {
                    if let Some(pr) = primitives.iter_mut().find(|pr| pr.name == *name) {
                        pr.impls.push(built);
                    }
                }
                Dest::Own => {
                    if let Some(m) = modules.iter_mut().find(|m| m.page == own_page) {
                        m.other_impls.push(built);
                    }
                }
            }
            // A trait of this package also lists the impl, by its header.
            let tr = block
                .trait_
                .as_ref()
                .and_then(|t| t.value.segments.last())
                .and_then(|s| self.uses.get(&s.span))
                .copied();
            if let Some(Target::Def(t)) = tr {
                if let DefKind::Trait { item_index } = self.defs.def(t).kind {
                    if self.same_package(self.defs.module_of(item_index), p.module) {
                        if let Some(url) = self.url(&Target::Def(t)) {
                            let (page, anchor) = url.split_once('#').unwrap_or((url.as_str(), ""));
                            let header = self.impl_header(ft, page, p.span, block);
                            if let Some(item) = find_item(modules, page, anchor) {
                                item.impls.push(Impl {
                                    header,
                                    docs: Docs::default(),
                                    methods: Vec::new(),
                                    on: on_name,
                                });
                            }
                        }
                    }
                }
            }
        }
    }

    /// An impl's header, `impl … {` without its brace (plan decision 10).
    fn impl_header(&self, ft: &FileText, page: &str, span: Span, block: &ImplBlock) -> String {
        let start = ft
            .next(span.start as usize, |t| *t == Token::Impl)
            .unwrap_or(span.start as usize);
        let end = ft
            .next(start, |t| *t == Token::LBrace)
            .unwrap_or(block.ty.span.end as usize);
        self.sig(ft, page, start, end, None)
    }

    /// A project's packages: the root first, then the rest by name (spec
    /// §7.1).
    fn packages(&self) -> Vec<Package> {
        let Some(graph) = self.a.graph.as_ref() else {
            return Vec::new();
        };
        let mut pending = Vec::new();
        let mut modules = Vec::new();
        let mut owners = Vec::new();
        for i in 0..self.users {
            let m = ModuleId(i as u32);
            let Some(pid) = self.package_id(m) else {
                continue;
            };
            if let Some(module) = self.module(m, &mut pending) {
                modules.push(module);
                owners.push(pid);
            }
        }
        self.place(&mut modules, &mut [], pending);
        let mut packages: Vec<Package> = Vec::new();
        for (module, pid) in modules.into_iter().zip(owners) {
            let gp = graph.package(pid);
            let at = match packages.iter().position(|p| p.name == gp.name) {
                Some(at) => at,
                None => {
                    packages.push(Package {
                        name: gp.name.clone(),
                        version: gp.manifest.package.version.to_string(),
                        description: gp.manifest.package.description.clone(),
                        std: false,
                        modules: Vec::new(),
                        primitives: Vec::new(),
                        builtins: Vec::new(),
                    });
                    packages.len() - 1
                }
            };
            packages[at].modules.push(module);
        }
        for p in &mut packages {
            p.modules
                .sort_by(|a, b| (!a.root, &a.name).cmp(&(!b.root, &b.name)));
        }
        let root = graph.root().name.clone();
        packages.sort_by(|a, b| (a.name != root, &a.name).cmp(&(b.name != root, &b.name)));
        packages
    }

    /// std's package, from std's own analysis (spec §5.3, §7.4).
    fn std_package(&self) -> Package {
        let mut pending = Vec::new();
        let mut modules: Vec<Module> = (self.users..self.defs.module_count())
            .filter_map(|i| self.module(ModuleId(i as u32), &mut pending))
            .collect();
        let mut primitives: Vec<Primitive> = PRIMITIVES
            .iter()
            .map(|&(name, about)| Primitive {
                name,
                page: format!("std/primitive.{name}.html"),
                about: markdown::render(about, 1),
                impls: Vec::new(),
            })
            .collect();
        self.place(&mut modules, &mut primitives, pending);
        modules.sort_by(|a, b| a.name.cmp(&b.name));
        let builtins = Builtin::GLOBAL
            .iter()
            .map(|b| BuiltinDoc {
                name: b.name(),
                signature: highlight(&nova_typeck::builtin_text(*b, self.defs)),
                docs: markdown::render(
                    BUILTINS
                        .iter()
                        .find(|(n, _)| *n == b.name())
                        .map_or("", |(_, d)| *d),
                    3,
                ),
            })
            .collect();
        Package {
            name: "std".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            description: Some(STD_ABOUT.to_string()),
            std: true,
            modules,
            primitives,
            builtins,
        }
    }
}

/// The item at `page`'s `anchor`.
fn find_item<'m>(modules: &'m mut [Module], page: &str, anchor: &str) -> Option<&'m mut Item> {
    modules
        .iter_mut()
        .find(|m| m.page == page)?
        .items
        .iter_mut()
        .find(|i| i.anchor == anchor)
}

/// A small site for the renderer's, the search index's and the writer's
/// tests.
#[cfg(test)]
pub(crate) fn sample_site() -> Site {
    let docs = |text: &str| markdown::render(text, 3);
    let point = Item {
        kind: ItemKind::Record,
        name: "Point".to_string(),
        anchor: "record.Point".to_string(),
        signature: "<span class=\"kw\">pub</span> <span class=\"kw\">record</span> \
                    <span class=\"name\">Point</span>"
            .to_string(),
        docs: docs("A point."),
        members: vec![Member {
            kind: MemberKind::Field,
            name: "x".to_string(),
            anchor: "record.Point.field.x".to_string(),
            signature: "<span class=\"kw\">pub</span> x: \
                        <a href=\"../std/primitive.Int.html\">Int</a>"
                .to_string(),
            docs: docs("Across \"x\"."),
        }],
        hidden_fields: true,
        impls: Vec::new(),
    };
    let module = |name: &str, qual: &str, page: &str, root: bool, items: Vec<Item>| Module {
        name: name.to_string(),
        qual: qual.to_string(),
        page: page.to_string(),
        root,
        docs: markdown::render("A module.", 1),
        items,
        other_impls: Vec::new(),
    };
    let app = Package {
        name: "app".to_string(),
        version: "0.1.0".to_string(),
        description: Some("An app.".to_string()),
        std: false,
        modules: vec![
            module("main", "app", "app/index.html", true, vec![point]),
            module(
                "util",
                "app::util",
                "app/util/index.html",
                false,
                Vec::new(),
            ),
        ],
        primitives: Vec::new(),
        builtins: Vec::new(),
    };
    let std = Package {
        name: "std".to_string(),
        version: "0.2.0".to_string(),
        description: Some(STD_ABOUT.to_string()),
        std: true,
        modules: vec![module(
            "core",
            "std::core",
            "std/core/index.html",
            false,
            Vec::new(),
        )],
        primitives: vec![Primitive {
            name: "Int",
            page: "std/primitive.Int.html".to_string(),
            about: markdown::render("An integer.", 1),
            impls: Vec::new(),
        }],
        builtins: vec![BuiltinDoc {
            name: "println",
            signature: "<span class=\"kw\">fn</span> println(String) -&gt; ()".to_string(),
            docs: markdown::render("Writes a line.", 3),
        }],
    };
    Site {
        packages: vec![app, std],
        start: "app/index.html".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// A fresh, empty directory with a fixed name, so each run replaces the
    /// last.
    fn fresh(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nova-doc-model-{name}"));
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

    fn manifest(name: &str, extra: &str) -> String {
        format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n{extra}")
    }

    fn documented(root: &Path) -> Documented {
        match document_package(root) {
            Ok(d) => d,
            Err(Failure::Diagnostics { db, diagnostics }) => panic!(
                "{}",
                nova_diagnostics::render::render_to_string(&db, &diagnostics)
            ),
            Err(Failure::Other(why)) => panic!("{why}"),
        }
    }

    fn module<'s>(site: &'s Site, page: &str) -> &'s Module {
        site.packages
            .iter()
            .flat_map(|p| &p.modules)
            .find(|m| m.page == page)
            .unwrap_or_else(|| panic!("no page {page}"))
    }

    fn item<'m>(m: &'m Module, anchor: &str) -> &'m Item {
        m.items
            .iter()
            .find(|i| i.anchor == anchor)
            .unwrap_or_else(|| panic!("no {anchor} on {}", m.page))
    }

    const GEOM_LIB: &str = "import utils\n\n/// A point.\npub record Point {\n    /// Across.\n    pub x: Int,\n    y: Int,\n}\n\n/// The area of a square of side `width()`.\npub fn area() -> Int { width() * width() }\n\nfn secret() -> Int { 1 }\n\nrecord Hidden {\n    n: Int,\n}\n\n/// Makes a thing of a private type.\npub fn make() -> Hidden { Hidden { n: 1 } }\n";
    const GEOM_UTILS: &str = "/// Three.\npub fn width() -> Int { 3 }\n";
    const APP_MAIN: &str = "import geom\n\n/// Shows `p`.\nfn show(p: Point) -> String { \"${p.x}\" }\n\n/// A record of the app's own.\nrecord Local {\n    hidden: Int,\n}\n\nfn main() {\n    println(show(Point { x: area(), y: 0 }))\n}\n";

    #[test]
    fn std_is_one_package_of_modules_primitives_and_builtins() {
        let Ok(d) = document_std() else {
            panic!("std is not documented")
        };
        assert_eq!(d.site.packages.len(), 1);
        assert_eq!(d.site.start, "std/index.html");
        let std = &d.site.packages[0];
        assert!(std.std);
        let names: Vec<&str> = std.modules.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "bytes",
                "collections",
                "core",
                "crypto",
                "fmt",
                "fs",
                "http",
                "io",
                "json",
                "log",
                "net",
                "process",
                "strings",
                "sync",
                "task",
                "test",
                "time"
            ]
        );
        let core = module(&d.site, "std/core/index.html");
        let option = item(core, "type.Option");
        let variants: Vec<&str> = option.members.iter().map(|m| m.anchor.as_str()).collect();
        assert_eq!(
            variants,
            ["type.Option.variant.Some", "type.Option.variant.None"]
        );
        assert!(option.impls.iter().any(|i| i
            .methods
            .iter()
            .any(|m| m.anchor == "type.Option.method.unwrap")));
        let string = std
            .primitives
            .iter()
            .find(|p| p.name == "String")
            .expect("a String page");
        assert!(string.impls.iter().any(|i| !i.methods.is_empty()));
        let mut builtins: Vec<&str> = std.builtins.iter().map(|b| b.name).collect();
        builtins.sort_unstable();
        assert_eq!(
            builtins,
            ["eprint", "eprintln", "panic", "print", "println"]
        );
    }

    #[test]
    fn a_program_documents_every_item_and_its_library_only_its_exports() {
        let dir = fresh("program");
        write(
            &dir.join("geom"),
            &[
                ("nova.toml", &manifest("geom", "")),
                ("src/lib.nova", GEOM_LIB),
                ("src/utils.nova", GEOM_UTILS),
            ],
        );
        write(
            &dir.join("app"),
            &[
                (
                    "nova.toml",
                    &manifest("app", "\n[dependencies]\ngeom = { path = \"../geom\" }\n"),
                ),
                ("src/main.nova", APP_MAIN),
            ],
        );
        let d = documented(&dir.join("app"));
        let names: Vec<&str> = d.site.packages.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, ["app", "geom", "std"]);
        assert_eq!(d.site.start, "app/index.html");
        let app = module(&d.site, "app/index.html");
        assert!(app.root);
        let anchors: Vec<&str> = app.items.iter().map(|i| i.anchor.as_str()).collect();
        assert_eq!(anchors, ["fn.show", "record.Local", "fn.main"]);
        assert!(!item(app, "record.Local").hidden_fields);
        let show = item(app, "fn.show");
        assert!(
            show.signature
                .contains("<a href=\"../geom/index.html#record.Point\">Point</a>"),
            "{}",
            show.signature
        );
        assert!(
            show.signature
                .contains("<a href=\"../std/primitive.String.html\">String</a>"),
            "{}",
            show.signature
        );
        let geom = module(&d.site, "geom/index.html");
        let anchors: Vec<&str> = geom.items.iter().map(|i| i.anchor.as_str()).collect();
        assert_eq!(anchors, ["record.Point", "fn.area", "fn.make"]);
        // A private type in a public signature is plain text (mutant 1).
        let make = item(geom, "fn.make");
        assert!(!make.signature.contains("<a "), "{}", make.signature);
        let point = item(geom, "record.Point");
        assert!(point.hidden_fields);
        let fields: Vec<&str> = point.members.iter().map(|m| m.anchor.as_str()).collect();
        assert_eq!(fields, ["record.Point.field.x"]);
        module(&d.site, "geom/utils/index.html");
    }

    const IMPLS: &str = "/// A rectangle.\npub record Rect {\n    pub w: Int,\n    pub h: Int,\n}\n\nimpl Rect {\n    /// Its area.\n    pub fn area(self) -> Int { self.w * self.h }\n\n    fn hidden(self) -> Int { 0 }\n}\n\n/// What can be measured.\npub trait Measure {\n    /// The size.\n    fn size(self) -> Int\n}\n\nimpl Measure for Rect {\n    fn size(self) -> Int { self.area() }\n}\n\nimpl Int {\n    /// Twice it.\n    pub fn doubled(self) -> Int { self * 2 }\n}\n";

    #[test]
    fn an_impl_goes_with_its_type_and_a_trait_lists_its_impls() {
        let dir = fresh("impls");
        write(
            &dir,
            &[
                ("nova.toml", &manifest("shapes", "")),
                ("src/lib.nova", IMPLS),
            ],
        );
        let d = documented(&dir);
        let lib = module(&d.site, "shapes/index.html");
        let rect = item(lib, "record.Rect");
        assert_eq!(rect.impls.len(), 2, "the inherent impl and the trait impl");
        let methods: Vec<&str> = rect
            .impls
            .iter()
            .flat_map(|i| &i.methods)
            .map(|m| m.anchor.as_str())
            .collect();
        assert_eq!(methods, ["record.Rect.method.area"]);
        let measure = item(lib, "trait.Measure");
        assert_eq!(measure.impls.len(), 1);
        assert!(
            measure.impls[0]
                .header
                .contains("<a href=\"../shapes/index.html#record.Rect\">Rect</a>"),
            "{}",
            measure.impls[0].header
        );
        assert_eq!(lib.other_impls.len(), 1, "the impl on Int");
        let doubled: Vec<&str> = lib.other_impls[0]
            .methods
            .iter()
            .map(|m| m.anchor.as_str())
            .collect();
        assert_eq!(doubled, ["impl.Int.method.doubled"]);
    }

    const PAIRS: &str = "/// Two of a kind.\npub record Pair<T> {\n    pub a: T,\n    pub b: T,\n}\n\nimpl Pair<Int> {\n    /// Their sum.\n    pub fn total(self) -> Int { self.a + self.b }\n}\n\nimpl Pair<String> {\n    /// The two, joined.\n    pub fn total(self) -> String { \"${self.a}${self.b}\" }\n}\n";

    #[test]
    fn two_impls_of_one_type_number_a_method_they_share() {
        // Spec §6.2: both impls go on `Pair`'s anchor, so the later
        // `total` is numbered.
        let dir = fresh("pairs");
        write(
            &dir,
            &[
                ("nova.toml", &manifest("pairs", "")),
                ("src/lib.nova", PAIRS),
            ],
        );
        let d = documented(&dir);
        let lib = module(&d.site, "pairs/index.html");
        let methods: Vec<&str> = item(lib, "record.Pair")
            .impls
            .iter()
            .flat_map(|i| &i.methods)
            .map(|m| m.anchor.as_str())
            .collect();
        assert_eq!(
            methods,
            ["record.Pair.method.total", "record.Pair.method.total-2"]
        );
    }

    #[test]
    fn an_error_is_a_failure_that_carries_its_diagnostics() {
        let dir = fresh("error");
        write(
            &dir,
            &[
                ("nova.toml", &manifest("app", "")),
                ("src/main.nova", "fn main() {\n    let x: Int = \"s\"\n}\n"),
            ],
        );
        match document_package(&dir) {
            Err(Failure::Diagnostics { diagnostics, .. }) => {
                assert!(diagnostics.iter().any(|d| d.severity == Severity::Error))
            }
            _ => panic!("a program with an error was documented"),
        }
    }

    #[test]
    fn a_package_named_std_stops_it() {
        let dir = fresh("named-std");
        write(
            &dir,
            &[
                ("nova.toml", &manifest("std", "")),
                ("src/main.nova", "fn main() {}\n"),
            ],
        );
        match document_package(&dir) {
            Err(Failure::Other(why)) => assert!(why.contains("`std`"), "{why}"),
            _ => panic!("a package named std was documented"),
        }
    }

    #[test]
    fn two_modules_whose_names_differ_in_case_alone_stop_it() {
        // Their pages would share a directory on macOS's and Windows's
        // file systems, which ignore case.
        let mut site = sample_site();
        assert!(clash(&site.packages).is_ok());
        let mut util = sample_site().packages.remove(0).modules.remove(1);
        util.name = "Util".to_string();
        util.qual = "app::Util".to_string();
        util.page = "app/Util/index.html".to_string();
        site.packages[0].modules.insert(1, util);
        match clash(&site.packages) {
            Err(Failure::Other(why)) => assert_eq!(
                why,
                "the modules `app::Util` and `app::util` would share one directory in \
                 target/doc/app/, so nova doc cannot document them"
            ),
            _ => panic!("two modules that differ in case alone were documented"),
        }
    }

    #[test]
    fn a_clashing_id_gets_a_number() {
        // Spec §6.2: the later of two equal ids on one page.
        let mut taken = HashSet::new();
        assert_eq!(
            unique(&mut taken, "impl.Vec.method.a".to_string()),
            "impl.Vec.method.a"
        );
        assert_eq!(
            unique(&mut taken, "impl.Vec.method.a".to_string()),
            "impl.Vec.method.a-2"
        );
        assert_eq!(
            unique(&mut taken, "impl.Vec.method.a".to_string()),
            "impl.Vec.method.a-3"
        );
    }

    #[test]
    fn every_builtin_a_module_sees_has_docs() {
        for b in Builtin::GLOBAL.iter() {
            assert!(BUILTINS.iter().any(|(n, _)| *n == b.name()), "{}", b.name());
        }
    }
}
