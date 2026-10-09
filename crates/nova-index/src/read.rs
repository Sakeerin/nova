//! Reading an index's files (spec §3.3), and the resolver's view of them.

use std::path::PathBuf;

use nova_pm::{Candidate, IndexView};

use crate::http::{Http, MAX_INDEX_FILE};
use crate::line::{parse_config, parse_lines, Config};
use crate::location::{index_path, Index, Source};

/// How an index's files are read: from a directory, over HTTP, or through
/// the GitHub API.
pub trait Reader {
    /// The text of the file at `path`, `/`-separated under the index's
    /// root. `Ok(None)` when there is no such file.
    fn file(&mut self, path: &str) -> Result<Option<String>, String>;

    /// The index's `config.json`.
    fn config(&mut self) -> Result<Config, String> {
        match self.file("config.json")? {
            Some(text) => parse_config(&text),
            None => Err("the index has no config.json".to_string()),
        }
    }
}

/// A local index, read from its directory.
pub struct LocalReader {
    pub dir: PathBuf,
}

impl Reader for LocalReader {
    fn file(&mut self, path: &str) -> Result<Option<String>, String> {
        let full = path
            .split('/')
            .fold(self.dir.clone(), |full, part| full.join(part));
        match std::fs::read_to_string(&full) {
            Ok(text) => Ok(Some(text)),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(format!("cannot read {}: {error}", full.display())),
        }
    }
}

/// An index read over HTTP from `base`, its canonical URL.
pub struct HttpReader {
    pub base: String,
    pub http: Http,
}

impl HttpReader {
    pub fn new(base: &str) -> HttpReader {
        HttpReader {
            base: base.to_string(),
            http: Http::new(),
        }
    }
}

impl Reader for HttpReader {
    fn file(&mut self, path: &str) -> Result<Option<String>, String> {
        let url = format!("{}{path}", self.base);
        match self.http.get(&url, MAX_INDEX_FILE)? {
            None => Ok(None),
            Some(bytes) => String::from_utf8(bytes)
                .map(Some)
                .map_err(|_| format!("{url} is not UTF-8")),
        }
    }
}

/// The reader for `index`'s own files: its directory, or HTTP.
pub fn reader_for(index: &Index) -> Box<dyn Reader> {
    match &index.source {
        Source::Local(dir) => Box::new(LocalReader { dir: dir.clone() }),
        Source::Http(base) => Box::new(HttpReader::new(base)),
    }
}

/// The resolver's view of an index through a [`Reader`]. Each name's file
/// is read when the resolver first asks for it, and the notes about lines
/// that do not parse are kept for the command to print.
pub struct View<'a> {
    reader: &'a mut dyn Reader,
    pub notes: Vec<String>,
}

impl<'a> View<'a> {
    pub fn new(reader: &'a mut dyn Reader) -> View<'a> {
        View {
            reader,
            notes: Vec::new(),
        }
    }
}

impl IndexView for View<'_> {
    fn versions(&mut self, name: &str) -> Result<Option<Vec<Candidate>>, String> {
        let path = index_path(name);
        let Some(text) = self.reader.file(&path)? else {
            return Ok(None);
        };
        let (lines, notes) = parse_lines(&text, &path);
        self.notes.extend(notes);
        // `Geom` and `geom` share a file; names compare exactly, as
        // manifests do (spec §3.1).
        let candidates: Vec<Candidate> = lines
            .iter()
            .filter(|line| line.name == name)
            .filter_map(|line| line.candidate().ok())
            .collect();
        Ok((!candidates.is_empty()).then_some(candidates))
    }
}
