//! Name resolution for the Nova compiler.
//!
//! Phase 1 scope: a single-file module. The resolver walks the top-level
//! items of a parsed [`nova_ast::File`] and produces a [`Definitions`] table
//! mapping every top-level name (functions, sum types, sum-type variants,
//! constants) to a stable [`DefId`]. Duplicate definitions are reported as
//! `E0002` diagnostics.
//!
//! Local (block-scoped) variable resolution happens during type checking,
//! where scoping interacts with inference; this crate owns the *item-level*
//! namespace and the shared [`DefId`] / [`Res`] vocabulary used by the rest
//! of the pipeline.

use indexmap::IndexMap;
use nova_ast::item::{ExternItem, Import, ImportKind, TypeDef};
use nova_ast::{File, Item};
use nova_diagnostics::{Diagnostic, FileId, Span};
use rustc_hash::FxHashMap;

/// A stable identifier for a top-level definition within a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DefId(pub u32);

/// A compiler-provided builtin function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Builtin {
    /// `println(s: String)` — write a line to stdout.
    Println,
    /// `print(s: String)` — write to stdout without a trailing newline.
    Print,
    /// `panic(msg: String)` — abort the program with a message.
    Panic,
}

impl Builtin {
    /// The source-level name of the builtin.
    pub fn name(self) -> &'static str {
        match self {
            Builtin::Println => "println",
            Builtin::Print => "print",
            Builtin::Panic => "panic",
        }
    }

    /// All builtins injected into every module's scope.
    pub const ALL: [Builtin; 3] = [Builtin::Println, Builtin::Print, Builtin::Panic];
}

/// What kind of definition a [`DefId`] refers to.
#[derive(Debug, Clone)]
pub enum DefKind {
    /// A function; the payload is the index of the item in `File::items`.
    Fn { item_index: usize },
    /// A sum type declaration with its variants.
    Sum {
        item_index: usize,
        variants: Vec<VariantDef>,
    },
    /// A record (struct) declaration.
    Record { item_index: usize },
    /// A constant; payload is the item index.
    Const { item_index: usize },
    /// A trait declaration.
    Trait { item_index: usize },
    /// A method: an `ast::Function` living inside an impl block, or a
    /// trait's default-method body. `method_index` selects it within the
    /// owner (impl `functions`, or the trait's provided items).
    Method {
        item_index: usize,
        method_index: usize,
        owner: MethodOwner,
    },
    /// An `extern` function declaration (a C-ABI import, no Nova body).
    /// `fn_index` selects the declaration within the extern block's `items`.
    ExternFn { item_index: usize, fn_index: usize },
}

/// Where a [`DefKind::Method`] lives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MethodOwner {
    /// A method defined in an `impl` block (`item_index` → the impl item).
    Impl,
    /// A default-method body in a `trait` declaration (`item_index` → the
    /// trait item, `method_index` → its `functions`-style provided list).
    TraitDefault,
}

/// One variant of a sum type.
#[derive(Debug, Clone)]
pub struct VariantDef {
    pub name: String,
    pub span: Span,
    /// Number of payload fields (types live in the AST, indexed by the parent).
    pub arity: usize,
}

/// A single top-level definition.
#[derive(Debug, Clone)]
pub struct Def {
    pub name: String,
    pub span: Span,
    pub kind: DefKind,
}

/// What a name in expression position resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Res {
    /// A top-level definition.
    Def(DefId),
    /// Variant `variant_index` of the sum type `DefId`.
    Variant(DefId, usize),
    /// A compiler builtin.
    Builtin(Builtin),
}

/// Index of a module within a compiled program.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ModuleId(pub u32);

/// One module's visible namespaces: its own items (public and private), the
/// implicit `std/core` module, and names it imports. Resolution is performed
/// relative to a module.
#[derive(Debug, Default)]
struct ModuleScope {
    name: String,
    values: FxHashMap<String, Res>,
    types: FxHashMap<String, DefId>,
    traits: FxHashMap<String, DefId>,
}

/// The item-level namespaces of a whole program (one or more modules).
///
/// `defs` and `DefId`s are global; name lookups are **module-relative** — a
/// name is resolved in the scope of the module that owns the item currently
/// being checked (see [`Definitions::module_of`]).
#[derive(Debug, Default)]
pub struct Definitions {
    defs: Vec<Def>,
    modules: Vec<ModuleScope>,
    /// Merged-item-index → owning module.
    item_module: Vec<u32>,
}

impl Definitions {
    /// All collected definitions, indexable by `DefId.0`.
    pub fn defs(&self) -> &[Def] {
        &self.defs
    }

    /// Look up a definition by id.
    pub fn def(&self, id: DefId) -> &Def {
        &self.defs[id.0 as usize]
    }

    /// The module that owns the item at `item_index` in the merged file.
    pub fn module_of(&self, item_index: usize) -> ModuleId {
        ModuleId(self.item_module.get(item_index).copied().unwrap_or(0))
    }

    /// Number of modules in the program.
    pub fn module_count(&self) -> usize {
        self.modules.len()
    }

    /// The name of a module.
    pub fn module_name(&self, module: ModuleId) -> &str {
        self.modules
            .get(module.0 as usize)
            .map(|m| m.name.as_str())
            .unwrap_or("")
    }

    /// Resolve a name in *value* position, relative to `module`.
    pub fn resolve_value(&self, module: ModuleId, name: &str) -> Option<Res> {
        self.modules
            .get(module.0 as usize)
            .and_then(|m| m.values.get(name).copied())
    }

    /// Resolve a name in *type* position, relative to `module`.
    pub fn resolve_type(&self, module: ModuleId, name: &str) -> Option<DefId> {
        self.modules
            .get(module.0 as usize)
            .and_then(|m| m.types.get(name).copied())
    }

    /// Resolve a name in *trait* position, relative to `module`.
    pub fn resolve_trait(&self, module: ModuleId, name: &str) -> Option<DefId> {
        self.modules
            .get(module.0 as usize)
            .and_then(|m| m.traits.get(name).copied())
    }

    /// Iterate all method definitions as `(DefId, item_index, method_index,
    /// owner)`.
    pub fn methods(&self) -> impl Iterator<Item = (DefId, usize, usize, MethodOwner)> + '_ {
        self.defs
            .iter()
            .enumerate()
            .filter_map(|(i, d)| match d.kind {
                DefKind::Method {
                    item_index,
                    method_index,
                    owner,
                } => Some((DefId(i as u32), item_index, method_index, owner)),
                _ => None,
            })
    }

    /// Iterate all function definitions as `(DefId, item_index)`.
    pub fn functions(&self) -> impl Iterator<Item = (DefId, usize)> + '_ {
        self.defs
            .iter()
            .enumerate()
            .filter_map(|(i, d)| match d.kind {
                DefKind::Fn { item_index } => Some((DefId(i as u32), item_index)),
                _ => None,
            })
    }

    /// Iterate all `extern` function declarations as
    /// `(DefId, item_index, fn_index)` (fn_index into the extern block's items).
    pub fn extern_functions(&self) -> impl Iterator<Item = (DefId, usize, usize)> + '_ {
        self.defs
            .iter()
            .enumerate()
            .filter_map(|(i, d)| match d.kind {
                DefKind::ExternFn {
                    item_index,
                    fn_index,
                } => Some((DefId(i as u32), item_index, fn_index)),
                _ => None,
            })
    }
}

/// A source module: a parsed file plus its module name (file stem).
pub struct ModuleSource<'a> {
    pub name: String,
    pub file: &'a File,
}

/// The public exports of one module (its `pub` items), used to resolve imports.
#[derive(Default)]
struct Exports {
    values: FxHashMap<String, Res>,
    types: FxHashMap<String, DefId>,
    traits: FxHashMap<String, DefId>,
}

fn push_def(defs: &mut Vec<Def>, def: Def) -> DefId {
    let id = DefId(defs.len() as u32);
    defs.push(def);
    id
}

/// Output of [`resolve`]: the namespace table, the merged file whose
/// `item_index`es the definitions refer to (the input plus the implicit
/// `std/core` module), and any diagnostics. Downstream stages must type-check
/// against this `file`, not the original input, since `item_index`es index
/// into it.
#[derive(Debug)]
pub struct ResolveResult {
    pub definitions: Definitions,
    pub file: File,
    pub diagnostics: Vec<Diagnostic>,
}

/// Output of [`resolve_program`]: the merged file, its namespaces, and any
/// diagnostics. The merged file's `item_index`es are what `Definitions` refers
/// to, so downstream stages consume both together.
pub struct ProgramResolution {
    pub file: File,
    pub definitions: Definitions,
    pub diagnostics: Vec<Diagnostic>,
}

/// Collect the item-level namespace of a single file.
///
/// Reports `E0002` for duplicate definitions in the same namespace. Imports
/// and `module` declarations are accepted but ignored in Phase 1 (single-file
/// compilation); traits/impls/records are collected in later Phase 1 steps.
pub fn resolve(file: &File) -> ResolveResult {
    let sources = [ModuleSource {
        name: "main".to_string(),
        file,
    }];
    let prog = resolve_program(&sources, FileId::DUMMY);
    ResolveResult {
        definitions: prog.definitions,
        file: prog.file,
        diagnostics: prog.diagnostics,
    }
}

/// Resolve a multi-module program: collect each module's item namespace,
/// enforce `pub` visibility across modules, wire up `import`s, and merge all
/// items into one file for whole-program compilation downstream.
///
/// `std_core_file` is the [`FileId`] the caller registered [`STD_CORE_SRC`]
/// under. Only the driver owns a `FileDb`, so the id cannot be invented here;
/// threading it through means a syntax error in `std/core` is reported
/// against a real file instead of [`FileId::DUMMY`]. The single-module
/// [`resolve`] wrapper passes `FileId::DUMMY` since it has no `FileDb` to
/// register into.
///
/// `item_index`es in the returned [`ProgramResolution::file`] are global
/// (module items concatenated in `modules` order, plus the implicit
/// `std/core` module last).
pub fn resolve_program(modules: &[ModuleSource], std_core_file: FileId) -> ProgramResolution {
    let mut definitions = Definitions::default();
    let mut diagnostics = Vec::new();
    let mut merged = Vec::new();

    // Compile the implicit `std/core` module so `Option`/`Result` and their
    // variants get real DefIds and sum layouts (and are then glob-imported
    // into every module below). It goes *last* so user modules keep their
    // indices — module 0 stays the first user module. `std/core` ships with
    // the compiler, so a parse failure here is a compiler bug — but it is
    // reported against a real file so it is debuggable, and `None` means the
    // implicit module is skipped entirely rather than silently empty.
    let (std_core, std_core_diags) = std_core_module(std_core_file);
    diagnostics.extend(std_core_diags);
    let all: Vec<ModuleSource> = modules
        .iter()
        .map(|m| ModuleSource {
            name: m.name.clone(),
            file: m.file,
        })
        .chain(std_core.iter().map(|file| ModuleSource {
            name: STD_CORE_NAME.to_string(),
            file,
        }))
        .collect();
    let std_core_mid = std_core.is_some().then(|| all.len() - 1);

    let mut exports: Vec<Exports> = Vec::new();

    // A scope per module, seeded with the compiler builtins.
    for m in &all {
        let mut scope = ModuleScope {
            name: m.name.clone(),
            ..Default::default()
        };
        for b in Builtin::ALL {
            scope.values.insert(b.name().to_string(), Res::Builtin(b));
        }
        definitions.modules.push(scope);
        exports.push(Exports::default());
    }

    // Pass 1: collect each module's own definitions into its scope + exports.
    for (mid, m) in all.iter().enumerate() {
        let mut first_value: IndexMap<String, Span> = IndexMap::new();
        let mut first_type: IndexMap<String, Span> = IndexMap::new();
        for item in &m.file.items {
            let item_index = merged.len();
            definitions.item_module.push(mid as u32);
            collect_item(
                &mut definitions.defs,
                &mut definitions.modules[mid],
                &mut exports[mid],
                &mut first_value,
                &mut first_type,
                &mut diagnostics,
                item_index,
                &item.value,
            );
            merged.push(item.clone());
        }
    }

    // Pass 2: resolve `import`s, binding other modules' public names.
    let by_name: FxHashMap<&str, usize> = all
        .iter()
        .enumerate()
        .map(|(i, m)| (m.name.as_str(), i))
        .collect();
    for (mid, m) in all.iter().enumerate() {
        for item in &m.file.items {
            if let Item::Import(imp) = &item.value {
                resolve_import(
                    &mut definitions,
                    &exports,
                    &by_name,
                    mid,
                    imp,
                    &mut diagnostics,
                );
            }
        }
    }

    // Glob `std/core`'s public names into every user module — LAST, so it is
    // the lowest-priority binding: a name the module already defines or imports
    // wins (no conflict), and only otherwise-unbound names fall through to
    // `std/core`. A program may thus define or import its own `Option`/`Result`.
    if let Some(mid) = std_core_mid {
        import_std_core(&mut definitions, &exports[mid], mid);
    }

    ProgramResolution {
        file: File { items: merged },
        definitions,
        diagnostics,
    }
}

/// The `std/core` source, compiled as an implicit module. Embedded at build
/// time so the compiler is self-contained; the path is relative to this file.
pub const STD_CORE_SRC: &str = include_str!("../../../std/core/lib.nova");

/// Module name of the implicit `std/core`. Not a valid identifier, so it can
/// never collide with a user module or be named in an `import`.
const STD_CORE_NAME: &str = "$std.core";

/// Lex and parse the implicit `std/core` module. Its source ships with the
/// compiler, so any failure is a compiler bug — but it is reported against
/// `file_id` so it is debuggable rather than silently dropped. Returns `None`
/// when parsing fails outright (nothing to merge in as a module); the caller
/// still gets the diagnostics either way.
fn std_core_module(file_id: FileId) -> (Option<File>, Vec<Diagnostic>) {
    let (tokens, lex_errors) = nova_lexer::lex(STD_CORE_SRC, file_id);
    let mut diags: Vec<Diagnostic> = lex_errors
        .iter()
        .map(|e| Diagnostic::error("L0001", e.to_string()).with_primary_label(e.span(), "here"))
        .collect();
    let (ast, parse_errors) = nova_parser::parse(&tokens, file_id);
    diags.extend(
        parse_errors.iter().map(|e| {
            Diagnostic::error("P0001", e.to_string()).with_primary_label(e.span(), "here")
        }),
    );
    (ast, diags)
}

/// Bind `std/core`'s public names into every other module's scope, leaving
/// any name the module already defines untouched (user items shadow
/// `std/core`).
fn import_std_core(definitions: &mut Definitions, std_core_exports: &Exports, std_core_mid: usize) {
    let values: Vec<(String, Res)> = std_core_exports
        .values
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    let types: Vec<(String, DefId)> = std_core_exports
        .types
        .iter()
        .map(|(k, v)| (k.clone(), *v))
        .collect();
    for mid in 0..definitions.modules.len() {
        if mid == std_core_mid {
            continue;
        }
        let scope = &mut definitions.modules[mid];
        for (n, r) in &values {
            scope.values.entry(n.clone()).or_insert(*r);
        }
        for (n, id) in &types {
            scope.types.entry(n.clone()).or_insert(*id);
        }
    }
}

/// Collect one item's definitions into `scope` (and `exp` if `pub`), pushing
/// global `Def`s into `defs`.
#[allow(clippy::too_many_arguments)]
fn collect_item(
    defs: &mut Vec<Def>,
    scope: &mut ModuleScope,
    exp: &mut Exports,
    first_value: &mut IndexMap<String, Span>,
    first_type: &mut IndexMap<String, Span>,
    diagnostics: &mut Vec<Diagnostic>,
    item_index: usize,
    item: &Item,
) {
    match item {
        Item::Function(f) => {
            let name = f.name.value.clone();
            let span = f.name.span;
            let id = push_def(
                defs,
                Def {
                    name: name.clone(),
                    span,
                    kind: DefKind::Fn { item_index },
                },
            );
            insert_value(
                scope,
                exp,
                first_value,
                diagnostics,
                name,
                span,
                Res::Def(id),
                is_pub(f.vis),
            );
        }
        Item::Type(t) => match &t.def {
            TypeDef::Sum(variants) => {
                let name = t.name.value.clone();
                let span = t.name.span;
                let pubv = is_pub(t.vis);
                let variant_defs: Vec<VariantDef> = variants
                    .iter()
                    .map(|v| VariantDef {
                        name: v.name.value.clone(),
                        span: v.name.span,
                        arity: v.fields.len(),
                    })
                    .collect();
                let id = push_def(
                    defs,
                    Def {
                        name: name.clone(),
                        span,
                        kind: DefKind::Sum {
                            item_index,
                            variants: variant_defs,
                        },
                    },
                );
                insert_type(scope, exp, first_type, diagnostics, name, span, id, pubv);
                // Variants live in the value namespace and inherit the type's
                // visibility, so `Some(x)` / `Circle(1.0)` resolve unprefixed.
                for (vi, v) in variants.iter().enumerate() {
                    insert_value(
                        scope,
                        exp,
                        first_value,
                        diagnostics,
                        v.name.value.clone(),
                        v.name.span,
                        Res::Variant(id, vi),
                        pubv,
                    );
                }
            }
            TypeDef::Alias(_) => {
                diagnostics.push(unsupported(
                    t.name.span,
                    "type aliases are not supported yet in the Phase 1 compiler",
                ));
            }
        },
        Item::Const(c) => {
            let name = c.name.value.clone();
            let span = c.name.span;
            let id = push_def(
                defs,
                Def {
                    name: name.clone(),
                    span,
                    kind: DefKind::Const { item_index },
                },
            );
            insert_value(
                scope,
                exp,
                first_value,
                diagnostics,
                name,
                span,
                Res::Def(id),
                is_pub(c.vis),
            );
        }
        Item::Record(r) => {
            let name = r.name.value.clone();
            let span = r.name.span;
            let id = push_def(
                defs,
                Def {
                    name: name.clone(),
                    span,
                    kind: DefKind::Record { item_index },
                },
            );
            insert_type(
                scope,
                exp,
                first_type,
                diagnostics,
                name,
                span,
                id,
                is_pub(r.vis),
            );
        }
        Item::Trait(t) => {
            let name = t.name.value.clone();
            let span = t.name.span;
            let pubv = is_pub(t.vis);
            let id = push_def(
                defs,
                Def {
                    name: name.clone(),
                    span,
                    kind: DefKind::Trait { item_index },
                },
            );
            if scope.traits.contains_key(&name) {
                diagnostics.push(
                    Diagnostic::error("E0002", format!("duplicate trait `{name}`"))
                        .with_primary_label(span, "redefined here"),
                );
            } else {
                scope.traits.insert(name.clone(), id);
                if pubv {
                    exp.traits.insert(name, id);
                }
            }
            // Default-method bodies become their own method defs (global).
            for (method_index, ti) in t.items.iter().enumerate() {
                if let nova_ast::item::TraitItem::Provided(f) = ti {
                    push_def(
                        defs,
                        Def {
                            name: format!("{}::{}$default", t.name.value, f.name.value),
                            span: f.name.span,
                            kind: DefKind::Method {
                                item_index,
                                method_index,
                                owner: MethodOwner::TraitDefault,
                            },
                        },
                    );
                }
            }
        }
        Item::Impl(i) => {
            // Impls are globally coherent — collected program-wide, resolved by
            // type rather than by name, so they are not bound into any scope.
            let self_name = type_full_name(&i.ty.value);
            for (method_index, f) in i.functions.iter().enumerate() {
                let mangled = match &i.trait_ {
                    Some(tr) => format!("{}.{}.{}", self_name, path_tail(&tr.value), f.name.value),
                    None => format!("{}.{}", self_name, f.name.value),
                };
                push_def(
                    defs,
                    Def {
                        name: mangled,
                        span: f.name.span,
                        kind: DefKind::Method {
                            item_index,
                            method_index,
                            owner: MethodOwner::Impl,
                        },
                    },
                );
            }
        }
        Item::Extern(block) => {
            // Each declaration binds as a callable value under its (unmangled)
            // C symbol name. ABI / signature validity is checked in typeck.
            // Extern fns are module-private (the grammar has no `pub` on them).
            for (fn_index, ext_item) in block.items.iter().enumerate() {
                let ExternItem::Fn(sig) = ext_item;
                let name = sig.name.value.clone();
                let span = sig.name.span;
                let id = push_def(
                    defs,
                    Def {
                        name: name.clone(),
                        span,
                        kind: DefKind::ExternFn {
                            item_index,
                            fn_index,
                        },
                    },
                );
                insert_value(
                    scope,
                    exp,
                    first_value,
                    diagnostics,
                    name,
                    span,
                    Res::Def(id),
                    false,
                );
            }
        }
        // `import`s are handled in pass 2; `module` declarations carry no items.
        Item::Import(_) | Item::Module(_) => {}
    }
}

/// Bind another module's public names into the importing module's scope.
fn resolve_import(
    definitions: &mut Definitions,
    exports: &[Exports],
    by_name: &FxHashMap<&str, usize>,
    mid: usize,
    imp: &Import,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let span = imp.path.span;
    // Only single-segment module imports are supported. A qualified or nested
    // path (`a::b`) would otherwise silently drop its leading segments and bind
    // an unrelated module; reject it explicitly like `import ... as`.
    if imp.path.value.segments.len() > 1 {
        diagnostics.push(unsupported(
            span,
            "qualified or nested import paths (`a::b`) are not supported yet; \
             import a top-level module by its name",
        ));
        return;
    }
    let target_name = imp
        .path
        .value
        .segments
        .last()
        .map(|s| s.value.as_str())
        .unwrap_or("");
    let Some(&target) = by_name.get(target_name) else {
        diagnostics.push(
            Diagnostic::error("E0001", format!("cannot find module `{target_name}`"))
                .with_primary_label(span, "no such module"),
        );
        return;
    };
    if target == mid {
        diagnostics.push(
            Diagnostic::error("E0001", format!("module `{target_name}` imports itself"))
                .with_primary_label(span, "self-import"),
        );
        return;
    }
    match &imp.kind {
        ImportKind::Simple => {
            // Glob: bring all of the target module's public names into scope.
            let names: Vec<(String, Res)> = exports[target]
                .values
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect();
            for (n, r) in names {
                bind_value(&mut definitions.modules[mid], diagnostics, n, span, r);
            }
            let tys: Vec<(String, DefId)> = exports[target]
                .types
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect();
            for (n, id) in tys {
                bind_type(&mut definitions.modules[mid], diagnostics, n, span, id);
            }
            let trs: Vec<(String, DefId)> = exports[target]
                .traits
                .iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect();
            for (n, id) in trs {
                bind_trait(&mut definitions.modules[mid], diagnostics, n, span, id);
            }
        }
        ImportKind::List(names) => {
            for n in names {
                let name = &n.value;
                let mut found = false;
                if let Some(r) = exports[target].values.get(name).copied() {
                    bind_value(
                        &mut definitions.modules[mid],
                        diagnostics,
                        name.clone(),
                        n.span,
                        r,
                    );
                    found = true;
                }
                if let Some(id) = exports[target].types.get(name).copied() {
                    bind_type(
                        &mut definitions.modules[mid],
                        diagnostics,
                        name.clone(),
                        n.span,
                        id,
                    );
                    found = true;
                }
                if let Some(id) = exports[target].traits.get(name).copied() {
                    bind_trait(
                        &mut definitions.modules[mid],
                        diagnostics,
                        name.clone(),
                        n.span,
                        id,
                    );
                    found = true;
                }
                if !found {
                    diagnostics.push(
                        Diagnostic::error(
                            "E0001",
                            format!("`{name}` is not a public item of module `{target_name}`"),
                        )
                        .with_primary_label(n.span, "not found or not `pub`"),
                    );
                }
            }
        }
        ImportKind::Alias(_) => {
            diagnostics.push(unsupported(
                span,
                "`import ... as` aliases are not supported yet",
            ));
        }
    }
}

fn is_pub(vis: nova_ast::item::Visibility) -> bool {
    matches!(vis, nova_ast::item::Visibility::Pub)
}

#[allow(clippy::too_many_arguments)]
fn insert_value(
    scope: &mut ModuleScope,
    exp: &mut Exports,
    first: &mut IndexMap<String, Span>,
    diagnostics: &mut Vec<Diagnostic>,
    name: String,
    span: Span,
    res: Res,
    is_pub: bool,
) {
    if scope.values.contains_key(&name) {
        let mut diag = Diagnostic::error("E0002", format!("duplicate definition of `{name}`"))
            .with_primary_label(span, "redefined here");
        if let Some(prev) = first.get(&name) {
            diag = diag.with_secondary_label(*prev, "first defined here");
        } else {
            diag = diag.with_note(format!("`{name}` is a compiler builtin"));
        }
        diagnostics.push(diag);
        return;
    }
    first.insert(name.clone(), span);
    if is_pub {
        exp.values.insert(name.clone(), res);
    }
    scope.values.insert(name, res);
}

#[allow(clippy::too_many_arguments)]
fn insert_type(
    scope: &mut ModuleScope,
    exp: &mut Exports,
    first: &mut IndexMap<String, Span>,
    diagnostics: &mut Vec<Diagnostic>,
    name: String,
    span: Span,
    id: DefId,
    is_pub: bool,
) {
    if scope.types.contains_key(&name) {
        let mut diag = Diagnostic::error("E0002", format!("duplicate definition of type `{name}`"))
            .with_primary_label(span, "redefined here");
        if let Some(prev) = first.get(&name) {
            diag = diag.with_secondary_label(*prev, "first defined here");
        }
        diagnostics.push(diag);
        return;
    }
    first.insert(name.clone(), span);
    if is_pub {
        exp.types.insert(name.clone(), id);
    }
    scope.types.insert(name, id);
}

/// Bind an imported name into a module's value namespace, or report a conflict.
fn bind_value(
    scope: &mut ModuleScope,
    diagnostics: &mut Vec<Diagnostic>,
    name: String,
    span: Span,
    res: Res,
) {
    if scope.values.contains_key(&name) {
        diagnostics.push(
            Diagnostic::error(
                "E0002",
                format!("`{name}` is already defined or imported in this module"),
            )
            .with_primary_label(span, "conflicting import"),
        );
        return;
    }
    scope.values.insert(name, res);
}

fn bind_type(
    scope: &mut ModuleScope,
    diagnostics: &mut Vec<Diagnostic>,
    name: String,
    span: Span,
    id: DefId,
) {
    if scope.types.contains_key(&name) {
        diagnostics.push(
            Diagnostic::error(
                "E0002",
                format!("type `{name}` is already defined or imported in this module"),
            )
            .with_primary_label(span, "conflicting import"),
        );
        return;
    }
    scope.types.insert(name, id);
}

fn bind_trait(
    scope: &mut ModuleScope,
    diagnostics: &mut Vec<Diagnostic>,
    name: String,
    span: Span,
    id: DefId,
) {
    if scope.traits.contains_key(&name) {
        diagnostics.push(
            Diagnostic::error(
                "E0002",
                format!("trait `{name}` is already defined or imported in this module"),
            )
            .with_primary_label(span, "conflicting import"),
        );
        return;
    }
    scope.traits.insert(name, id);
}

fn unsupported(span: Span, msg: &str) -> Diagnostic {
    Diagnostic::error("E0900", msg).with_primary_label(span, "not supported yet")
}

/// A short textual head for a type, used only to build unique method
/// symbol names (semantic resolution happens later in the type checker).
/// A textual name for an impl self type that includes its type arguments, so
/// that two impls sharing a head but differing in arguments (`Pair<Int, Bool>`
/// vs `Pair<Int, Int>`) yield distinct method names and never collide when
/// monomorphized. Uses only `_`/`.`-style separators to stay linker-safe.
/// A generic parameter contributes its source name (`Box<T>` → `Box_T`); a
/// concrete instantiation still gets a distinct mangled instance via type
/// arguments at monomorphization.
fn type_full_name(ty: &nova_ast::Type) -> String {
    match ty {
        nova_ast::Type::Path { path, args } => {
            let head = path_tail(path);
            if args.is_empty() {
                head
            } else {
                let parts: Vec<String> = args.iter().map(|a| type_full_name(&a.value)).collect();
                format!("{head}_{}", parts.join("_"))
            }
        }
        nova_ast::Type::Array(elem) => format!("Arr_{}", type_full_name(&elem.value)),
        _ => "anon".to_string(),
    }
}

/// The last segment of a path (`a::b::c` → `c`).
fn path_tail(path: &nova_ast::Path) -> String {
    path.segments
        .last()
        .map(|s| s.value.clone())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use nova_diagnostics::FileId;
    use nova_lexer::lex;
    use nova_parser::parse;

    fn resolve_src(src: &str) -> ResolveResult {
        let file_id = FileId::DUMMY;
        let (tokens, lex_errors) = lex(src, file_id);
        assert!(lex_errors.is_empty(), "lex errors: {lex_errors:?}");
        let (ast, parse_errors) = parse(&tokens, file_id);
        assert!(parse_errors.is_empty(), "parse errors: {parse_errors:?}");
        resolve(&ast.expect("no AST produced"))
    }

    fn parse_file(src: &str) -> nova_ast::File {
        let (tokens, _) = lex(src, FileId::DUMMY);
        let (ast, errs) = parse(&tokens, FileId::DUMMY);
        assert!(errs.is_empty(), "parse errors: {errs:?}");
        ast.expect("no AST")
    }

    /// Resolve a two-module program: `main` plus `lib`.
    fn resolve_two(main_src: &str, lib_src: &str) -> ProgramResolution {
        let main = parse_file(main_src);
        let lib = parse_file(lib_src);
        let sources = [
            ModuleSource {
                name: "main".to_string(),
                file: &main,
            },
            ModuleSource {
                name: "lib".to_string(),
                file: &lib,
            },
        ];
        resolve_program(&sources, FileId::DUMMY)
    }

    fn error_codes(diags: &[Diagnostic]) -> Vec<&str> {
        diags.iter().map(|d| d.code.as_str()).collect()
    }

    #[test]
    fn import_binds_public_names_across_modules() {
        let p = resolve_two(
            "import lib::{add}\nfn main() { let x = add(1, 2) }\n",
            "pub fn add(a: Int, b: Int) -> Int { a + b }\nfn secret() -> Int { 0 }\n",
        );
        assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
        // `add` is visible in module 0 (main) after the import.
        assert!(matches!(
            p.definitions.resolve_value(ModuleId(0), "add"),
            Some(Res::Def(_))
        ));
    }

    #[test]
    fn importing_a_private_item_is_rejected() {
        let p = resolve_two(
            "import lib::{secret}\nfn main() { }\n",
            "pub fn add() -> Int { 0 }\nfn secret() -> Int { 0 }\n",
        );
        assert!(
            error_codes(&p.diagnostics).contains(&"E0001"),
            "{:?}",
            p.diagnostics
        );
    }

    #[test]
    fn dangling_import_reports_missing_module() {
        let p = resolve_two(
            "import nope::{x}\nfn main() { }\n",
            "pub fn add() -> Int { 0 }\n",
        );
        assert!(
            error_codes(&p.diagnostics).contains(&"E0001"),
            "{:?}",
            p.diagnostics
        );
    }

    #[test]
    fn private_item_is_not_visible_to_other_modules() {
        let p = resolve_two(
            "import lib\nfn main() { }\n",
            "pub fn add() -> Int { 0 }\nfn secret() -> Int { 0 }\n",
        );
        assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
        // Glob import brings `add` but not `secret`.
        assert!(p.definitions.resolve_value(ModuleId(0), "add").is_some());
        assert!(p.definitions.resolve_value(ModuleId(0), "secret").is_none());
    }

    #[test]
    fn same_name_in_two_modules_does_not_collide() {
        // Each module has its own `helper`; no duplicate-definition error.
        // (Codegen keeps them distinct via DefId-mangled symbols; see the
        // `modules_same_name_functions_dispatch_correctly` CLI test.)
        let p = resolve_two(
            "fn helper() -> Int { 1 }\nfn main() { }\n",
            "pub fn helper() -> Int { 2 }\n",
        );
        assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
    }

    #[test]
    fn qualified_import_path_is_rejected() {
        // `import lib::extra` is a qualified/nested path; it must be reported,
        // not silently resolved to the last segment's module.
        let p = resolve_two(
            "import lib::extra\nfn main() { }\n",
            "pub fn add() -> Int { 0 }\n",
        );
        assert!(
            error_codes(&p.diagnostics).contains(&"E0900"),
            "{:?}",
            p.diagnostics
        );
    }

    #[test]
    fn std_core_option_result_are_in_scope() {
        // Option/Result and their variants resolve with no import or definition.
        let r = resolve_src("fn main() { }\n");
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        assert!(r.definitions.resolve_type(ModuleId(0), "Option").is_some());
        assert!(r.definitions.resolve_type(ModuleId(0), "Result").is_some());
        assert!(matches!(
            r.definitions.resolve_value(ModuleId(0), "Some"),
            Some(Res::Variant(_, 0))
        ));
        assert!(matches!(
            r.definitions.resolve_value(ModuleId(0), "None"),
            Some(Res::Variant(_, 1))
        ));
        assert!(matches!(
            r.definitions.resolve_value(ModuleId(0), "Ok"),
            Some(Res::Variant(_, 0))
        ));
        assert!(matches!(
            r.definitions.resolve_value(ModuleId(0), "Err"),
            Some(Res::Variant(_, 1))
        ));
    }

    #[test]
    fn extern_fn_resolves_as_callable_extern_def() {
        let r = resolve_src("extern \"C\" { fn sqrt(x: Float) -> Float }\nfn main() { }\n");
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let Some(Res::Def(id)) = r.definitions.resolve_value(ModuleId(0), "sqrt") else {
            panic!("sqrt should resolve to a Def");
        };
        assert!(matches!(
            r.definitions.def(id).kind,
            DefKind::ExternFn { .. }
        ));
    }

    #[test]
    fn user_type_shadows_std_core() {
        // A user-defined `Option` shadows std/core's without an E0002 clash:
        // the module's own definition wins over the soft std/core import.
        let r = resolve_src("type Option<T> = | Present(T) | Absent\nfn main() { }\n");
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let opt = r
            .definitions
            .resolve_type(ModuleId(0), "Option")
            .expect("Option resolves");
        // It is the user's two-variant `Option`, not std/core's.
        assert!(matches!(
            r.definitions.def(opt).kind,
            DefKind::Sum { ref variants, .. } if variants.len() == 2 && variants[0].name == "Present"
        ));
    }

    #[test]
    fn importing_a_std_core_name_from_a_module_is_allowed() {
        // A user module may export a name coinciding with a std/core name; a
        // glob import of it binds (shadowing the soft std/core import), not a
        // spurious E0002.
        let p = resolve_two(
            "import lib\nfn main() { }\n",
            "pub type Status = | Ok | Fail\n",
        );
        assert!(p.diagnostics.is_empty(), "{:?}", p.diagnostics);
        // `Ok` resolves (to the imported Status variant), not rejected.
        assert!(p.definitions.resolve_value(ModuleId(0), "Ok").is_some());
    }

    #[test]
    fn collects_functions_and_sum_types() {
        let r = resolve_src(
            "type Shape = | Circle(Int) | Empty\n\
             fn area(s: Shape) -> Int { 0 }\n\
             fn main() { }\n",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        assert!(matches!(
            r.definitions.resolve_value(ModuleId(0), "area"),
            Some(Res::Def(_))
        ));
        assert!(matches!(
            r.definitions.resolve_value(ModuleId(0), "Circle"),
            Some(Res::Variant(_, 0))
        ));
        assert!(matches!(
            r.definitions.resolve_value(ModuleId(0), "Empty"),
            Some(Res::Variant(_, 1))
        ));
        assert!(r.definitions.resolve_type(ModuleId(0), "Shape").is_some());
        assert!(matches!(
            r.definitions.resolve_value(ModuleId(0), "println"),
            Some(Res::Builtin(Builtin::Println))
        ));
    }

    #[test]
    fn collects_records() {
        let r = resolve_src("record Point { x: Float, y: Float }\nfn main() { }\n");
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        let id = r
            .definitions
            .resolve_type(ModuleId(0), "Point")
            .expect("Point resolves");
        assert!(matches!(r.definitions.def(id).kind, DefKind::Record { .. }));
    }

    #[test]
    fn collects_traits_and_impl_methods() {
        let r = resolve_src(
            "record P { v: Int }\n\
             trait Show { fn name(self) -> String\n fn shout(self) -> String { self.name() } }\n\
             impl Show for P { fn name(self) -> String { \"p\" } }\n\
             impl P { fn get(self) -> Int { self.v } }\n\
             fn main() { }\n",
        );
        assert!(r.diagnostics.is_empty(), "{:?}", r.diagnostics);
        assert!(r.definitions.resolve_trait(ModuleId(0), "Show").is_some());
        // One default (shout), one trait-impl method (name), one inherent (get).
        let methods = r.definitions.methods().count();
        assert_eq!(methods, 3, "expected 3 method defs");
    }

    #[test]
    fn duplicate_function_reports_e0002() {
        let r = resolve_src("fn f() { }\nfn f() { }\n");
        assert_eq!(r.diagnostics.len(), 1);
        assert_eq!(r.diagnostics[0].code, "E0002");
    }

    #[test]
    fn shadowing_builtin_reports_e0002() {
        let r = resolve_src("fn println() { }\n");
        assert_eq!(r.diagnostics.len(), 1);
        assert_eq!(r.diagnostics[0].code, "E0002");
    }
}
