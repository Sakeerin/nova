//! Go to definition and find references (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4a-navigation-design.md`
//! §5.2, §5.3).

use std::cell::{OnceCell, RefCell};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use lsp_types as lsp;
use nova_diagnostics::{FileId, LineIndex, Span};
use nova_driver::Analysis;
use nova_resolver::Target;

use crate::analysis::Answer;
use crate::{convert, std_cache, uri};

/// Turns one analysis's spans into locations (plan decision 18). Std's
/// cache is made ready at most once per `Locator`, which a request makes
/// one of, and each file's lines are indexed once.
pub struct Locator<'a> {
    a: &'a Analysis,
    uri_of: &'a dyn Fn(&Path) -> String,
    std_dir: OnceCell<Option<PathBuf>>,
    lines: RefCell<HashMap<FileId, LineIndex>>,
}

impl<'a> Locator<'a> {
    pub fn new(a: &'a Analysis, uri_of: &'a dyn Fn(&Path) -> String) -> Locator<'a> {
        Locator {
            a,
            uri_of,
            std_dir: OnceCell::new(),
            lines: RefCell::new(HashMap::new()),
        }
    }

    /// Where a file is on disk: a std module's place in std's cache, or the
    /// real path of anything else, so `app/../geom` reads as `geom` (plan
    /// decision 8).
    pub fn path_of(&self, file: FileId) -> Option<PathBuf> {
        let name = self.a.db.get_name(file)?;
        match name.strip_prefix("<std/").and_then(|s| s.strip_suffix('>')) {
            Some(short) => {
                let dir = self.std_dir.get_or_init(std_cache::ensure).as_ref()?;
                Some(dir.join(format!("{short}.nova")))
            }
            None => Some(nova_pm::real_path(Path::new(name))),
        }
    }

    /// `span`'s range in its file.
    pub fn range(&self, span: Span) -> Option<lsp::Range> {
        let source = self.a.db.get_source(span.file)?;
        let mut lines = self.lines.borrow_mut();
        let index = lines
            .entry(span.file)
            .or_insert_with(|| LineIndex::new(source));
        Some(convert::range(index, span.start, span.end))
    }

    /// The URI `uri_of` gives a file.
    pub fn uri(&self, file: FileId) -> Option<lsp::Uri> {
        uri::parse(&(self.uri_of)(&self.path_of(file)?))
    }

    pub fn location(&self, span: Span) -> Option<lsp::Location> {
        Some(lsp::Location {
            uri: self.uri(span.file)?,
            range: self.range(span)?,
        })
    }
}

/// The definition of the name at byte `offset` (spec §5.2).
pub fn definition(
    answer: &Answer,
    offset: u32,
    uri_of: &dyn Fn(&Path) -> String,
) -> Option<lsp::Location> {
    let a = &answer.analysis;
    let index = a.index.as_ref()?;
    let o = index.at(a.definitions.as_ref()?, answer.file, offset)?;
    let locator = Locator::new(a, uri_of);
    match o.target {
        Target::Builtin(_) | Target::BuiltinMethod(_) | Target::Primitive(_) => None,
        Target::Module(m) => {
            let (file, _) = a.modules.get(m.0 as usize)?;
            locator.location(Span::new(0, 0, *file))
        }
        target => locator.location(index.declaration(&target)?.span),
    }
}
