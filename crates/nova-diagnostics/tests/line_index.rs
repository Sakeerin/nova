//! `LineIndex` (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §5).

use nova_diagnostics::LineIndex;

#[test]
fn ascii_lines_and_columns() {
    let ix = LineIndex::new("ab\ncd");
    assert_eq!(ix.position(0), (0, 0));
    assert_eq!(ix.position(2), (0, 2));
    assert_eq!(ix.position(3), (1, 0));
    assert_eq!(ix.position(4), (1, 1));
    assert_eq!(ix.offset(1, 1), 4);
    assert_eq!(ix.len(), 5);
}

#[test]
fn thai_counts_one_unit_per_character() {
    // Each Thai letter is three bytes of UTF-8 and one UTF-16 unit.
    let ix = LineIndex::new("กข\nx");
    assert_eq!(ix.position(3), (0, 1));
    assert_eq!(ix.position(6), (0, 2));
    assert_eq!(ix.position(7), (1, 0));
    assert_eq!(ix.offset(0, 2), 6);
}

#[test]
fn a_character_above_u_ffff_counts_two_units() {
    // `😀` is four bytes of UTF-8 and a surrogate pair in UTF-16.
    let ix = LineIndex::new("a😀b");
    assert_eq!(ix.position(1), (0, 1));
    assert_eq!(ix.position(5), (0, 3));
    assert_eq!(ix.offset(0, 3), 5);
    // A column inside the pair clamps to the character's start.
    assert_eq!(ix.offset(0, 2), 1);
}

#[test]
fn crlf_ends_a_line_and_its_cr_belongs_to_the_end() {
    let ix = LineIndex::new("a\r\nb\r\n");
    assert_eq!(ix.position(1), (0, 1));
    assert_eq!(ix.position(3), (1, 0));
    assert_eq!(ix.position(4), (1, 1));
    // A column past the line's end clamps to just before its `\r\n`.
    assert_eq!(ix.offset(0, 9), 1);
    assert_eq!(ix.offset(1, 9), 4);
}

#[test]
fn past_the_last_line_clamps_to_the_end_of_the_text() {
    let ix = LineIndex::new("ab\ncd");
    assert_eq!(ix.offset(7, 0), 5);
    assert_eq!(ix.position(99), (1, 2));
}

#[test]
fn every_character_boundary_round_trips() {
    let text = "fn main() {\r\n    let s = \"ก😀\" // é\n}\n";
    let ix = LineIndex::new(text);
    for (offset, c) in text.char_indices() {
        // The `\n` of `\r\n` has no place of its own: it is part of the
        // line's end, which an LSP position reaches only as the `\r`.
        if c == '\n' && text[..offset].ends_with('\r') {
            continue;
        }
        let (line, column) = ix.position(offset as u32);
        assert_eq!(ix.offset(line, column), offset as u32, "offset {offset}");
    }
}
