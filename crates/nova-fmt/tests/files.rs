//! Files: their line endings, their final newline and `.editorconfig`
//! (spec `docs/superpowers/specs/2026-10-07-phase-3-1-formatter-design.md`
//! §5.1, §7.3).

use std::path::{Path, PathBuf};

use nova_fmt::{format_file, format_text, FileError, FormatError, LineEnding};

const UNFORMATTED: &str = "fn main() {\nprintln(\"hi\")\n}\n";
const FORMATTED: &str = "fn main() { println(\"hi\") }\n";

/// A fresh directory for one test, under the system temp directory. Its
/// `.editorconfig` says `root = true` before `sections`, so that one above
/// the temp directory cannot reach the test.
fn fresh_dir(name: &str, sections: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("nova-fmt-file-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create the test directory");
    std::fs::write(
        dir.join(".editorconfig"),
        format!("root = true\n{sections}"),
    )
    .expect("write .editorconfig");
    dir
}

fn write(dir: &Path, name: &str, text: impl AsRef<[u8]>) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, text).expect("write the test file");
    path
}

fn crlf(text: &str) -> String {
    text.replace('\n', "\r\n")
}

#[test]
fn line_endings_come_from_the_first_line_break() {
    assert_eq!(LineEnding::of("a\r\nb\n"), LineEnding::Crlf);
    assert_eq!(LineEnding::of("a\nb\r\n"), LineEnding::Lf);
    assert_eq!(LineEnding::of("no break"), LineEnding::Lf);
}

#[test]
fn a_crlf_file_stays_crlf_and_an_lf_file_lf() {
    let dir = fresh_dir("endings", "");
    let file = write(&dir, "crlf.nova", crlf(UNFORMATTED));
    assert_eq!(format_file(&file).unwrap().formatted, crlf(FORMATTED));
    let file = write(&dir, "lf.nova", UNFORMATTED);
    assert_eq!(format_file(&file).unwrap().formatted, FORMATTED);
}

#[test]
fn an_already_formatted_file_is_unchanged() {
    let dir = fresh_dir("unchanged", "");
    let file = write(&dir, "main.nova", FORMATTED);
    let formatted = format_file(&file).unwrap();
    assert!(!formatted.changed(), "{formatted:?}");
    let file = write(&dir, "main.nova", UNFORMATTED);
    assert!(format_file(&file).unwrap().changed());
}

#[test]
fn a_file_with_no_line_break_gets_lf_and_one_final_newline() {
    let dir = fresh_dir("no-break", "");
    let file = write(&dir, "main.nova", "fn main() {}");
    assert_eq!(format_file(&file).unwrap().formatted, "fn main() {}\n");
}

#[test]
fn editorconfig_end_of_line_overrides_the_files_own() {
    let dir = fresh_dir("end-of-line-lf", "[*.nova]\nend_of_line = lf\n");
    let file = write(&dir, "main.nova", crlf(UNFORMATTED));
    assert_eq!(format_file(&file).unwrap().formatted, FORMATTED);
    let dir = fresh_dir("end-of-line-crlf", "[*.nova]\nend_of_line = crlf\n");
    let file = write(&dir, "main.nova", UNFORMATTED);
    assert_eq!(format_file(&file).unwrap().formatted, crlf(FORMATTED));
}

#[test]
fn insert_final_newline_false_keeps_the_files_last_line_as_it_was() {
    let dir = fresh_dir("final-newline", "[*]\ninsert_final_newline = false\n");
    let file = write(&dir, "without.nova", "fn main() {\nprintln(\"hi\")\n}");
    assert_eq!(
        format_file(&file).unwrap().formatted,
        "fn main() { println(\"hi\") }"
    );
    let file = write(&dir, "with.nova", UNFORMATTED);
    assert_eq!(format_file(&file).unwrap().formatted, FORMATTED);
}

#[test]
fn a_multi_line_string_keeps_its_files_line_ending() {
    let dir = fresh_dir("string", "");
    let text = crlf("fn main() {\n    let s = \"a\nb\"\n    println(s)\n}\n");
    let file = write(&dir, "main.nova", &text);
    assert_eq!(format_file(&file).unwrap().formatted, text);
}

#[test]
fn a_file_that_is_not_utf8_or_starts_with_a_byte_order_mark_is_refused() {
    let dir = fresh_dir("refused", "");
    let file = write(&dir, "bytes.nova", [0x66u8, 0x6e, 0xff]);
    assert!(matches!(format_file(&file), Err(FileError::NotUtf8)));
    let file = write(&dir, "bom.nova", "\u{feff}fn main() {}\n");
    assert!(matches!(
        format_file(&file),
        Err(FileError::Format(FormatError::Syntax { .. }))
    ));
}

#[test]
fn format_text_keeps_the_inputs_line_ending() {
    assert_eq!(format_text(&crlf(UNFORMATTED)).unwrap(), crlf(FORMATTED));
    assert_eq!(format_text(UNFORMATTED).unwrap(), FORMATTED);
}

// === Phase 3.2: an editor's buffer (spec
// docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §6.5) ===

#[test]
fn format_buffer_applies_the_editorconfig_that_governs_its_path() {
    let dir = fresh_dir(
        "buffer",
        "\n[*.nova]\nend_of_line = crlf\n\n[keep.nova]\ninsert_final_newline = false\n",
    );
    // Neither file exists: the buffer is the text, and the path only
    // chooses the `.editorconfig` sections.
    let crlf = nova_fmt::format_buffer(&dir.join("main.nova"), UNFORMATTED).unwrap();
    assert_eq!(crlf, FORMATTED.replace('\n', "\r\n"));
    let keep =
        nova_fmt::format_buffer(&dir.join("keep.nova"), "fn main() {\nprintln(\"hi\")\n}").unwrap();
    assert_eq!(keep, "fn main() { println(\"hi\") }");
}

#[test]
fn format_buffer_keeps_a_buffers_crlf() {
    let dir = fresh_dir("buffer-crlf", "");
    let out = nova_fmt::format_buffer(&dir.join("main.nova"), &UNFORMATTED.replace('\n', "\r\n"))
        .unwrap();
    assert_eq!(out, FORMATTED.replace('\n', "\r\n"));
}

#[test]
fn format_buffer_refuses_a_syntax_error() {
    let dir = fresh_dir("buffer-error", "");
    let err = nova_fmt::format_buffer(&dir.join("main.nova"), "fn main( {\n").unwrap_err();
    assert!(matches!(err, FormatError::Syntax { .. }), "{err:?}");
}
