//! Renders `Diagnostic` values to stderr using `codespan-reporting`.

use codespan_reporting::{
    diagnostic::{Diagnostic as CsDiag, Label as CsLabel, Severity as CsSeverity},
    files::SimpleFiles,
    term::{
        self,
        termcolor::{ColorChoice, StandardStream},
        Config,
    },
};

use crate::{Diagnostic, FileDb, Severity};

/// Render a slice of diagnostics to stderr.
pub fn emit_all(db: &FileDb, diagnostics: &[Diagnostic]) {
    let mut files: SimpleFiles<&str, &str> = SimpleFiles::new();
    let mut id_map = std::collections::HashMap::new();

    // Build codespan file db from our FileDb.
    for (raw_id, diag) in diagnostics.iter().enumerate() {
        let _ = raw_id;
        for label in &diag.labels {
            let fid = label.span.file;
            if let std::collections::hash_map::Entry::Vacant(e) = id_map.entry(fid) {
                if let (Some(name), Some(src)) = (db.get_name(fid), db.get_source(fid)) {
                    let cs_id = files.add(name, src);
                    e.insert(cs_id);
                }
            }
        }
    }

    let writer = StandardStream::stderr(ColorChoice::Auto);
    let config = Config::default();

    for diag in diagnostics {
        let cs_severity = match diag.severity {
            Severity::Error => CsSeverity::Error,
            Severity::Warning => CsSeverity::Warning,
            Severity::Note => CsSeverity::Note,
            Severity::Help => CsSeverity::Help,
        };

        let labels: Vec<_> = diag
            .labels
            .iter()
            .filter_map(|l| {
                let cs_file = *id_map.get(&l.span.file)?;
                let range = l.span.as_range();
                let cs_label = if l.primary {
                    CsLabel::primary(cs_file, range)
                } else {
                    CsLabel::secondary(cs_file, range)
                };
                Some(cs_label.with_message(l.message.clone()))
            })
            .collect();

        let cs_diag = CsDiag::new(cs_severity)
            .with_code(&diag.code)
            .with_message(&diag.message)
            .with_labels(labels)
            .with_notes(diag.notes.clone());

        term::emit(&mut writer.lock(), &config, &files, &cs_diag)
            .expect("failed to write diagnostic");
    }
}

/// Render diagnostics to a `String` (useful for tests and snapshot testing).
pub fn render_to_string(db: &FileDb, diagnostics: &[Diagnostic]) -> String {
    use codespan_reporting::term::termcolor::NoColor;
    use std::io::Write;

    let mut files: SimpleFiles<&str, &str> = SimpleFiles::new();
    let mut id_map = std::collections::HashMap::new();

    for diag in diagnostics {
        for label in &diag.labels {
            let fid = label.span.file;
            if let std::collections::hash_map::Entry::Vacant(e) = id_map.entry(fid) {
                if let (Some(name), Some(src)) = (db.get_name(fid), db.get_source(fid)) {
                    let cs_id = files.add(name, src);
                    e.insert(cs_id);
                }
            }
        }
    }

    let mut buf = Vec::new();
    let mut writer = NoColor::new(&mut buf);
    let config = Config::default();

    for diag in diagnostics {
        let cs_severity = match diag.severity {
            Severity::Error => CsSeverity::Error,
            Severity::Warning => CsSeverity::Warning,
            Severity::Note => CsSeverity::Note,
            Severity::Help => CsSeverity::Help,
        };

        let labels: Vec<_> = diag
            .labels
            .iter()
            .filter_map(|l| {
                let cs_file = *id_map.get(&l.span.file)?;
                let range = l.span.as_range();
                let cs_label = if l.primary {
                    CsLabel::primary(cs_file, range)
                } else {
                    CsLabel::secondary(cs_file, range)
                };
                Some(cs_label.with_message(l.message.clone()))
            })
            .collect();

        let cs_diag = CsDiag::new(cs_severity)
            .with_code(&diag.code)
            .with_message(&diag.message)
            .with_labels(labels)
            .with_notes(diag.notes.clone());

        term::emit(&mut writer, &config, &files, &cs_diag).expect("failed to render diagnostic");
    }

    let _ = writer.flush();
    String::from_utf8(buf).expect("diagnostic output is not valid UTF-8")
}
