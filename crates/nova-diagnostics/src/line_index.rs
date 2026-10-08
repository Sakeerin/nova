//! Byte offsets to LSP positions and back (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §5).
//!
//! The compiler's spans are byte offsets. The Language Server Protocol
//! counts a position as a zero-based line and a column in UTF-16 code units,
//! so a character above U+FFFF is two columns, and a Thai letter, three bytes
//! of UTF-8, is one.

/// The start of every line in a text.
#[derive(Debug, Clone)]
pub struct LineIndex {
    text: String,
    /// The byte offset of each line's first character; `starts[0]` is 0.
    starts: Vec<u32>,
}

impl LineIndex {
    /// Index `text`. `\n` ends a line, and so does `\r\n`, whose `\r` is
    /// part of the line's end.
    pub fn new(text: &str) -> LineIndex {
        let mut starts = vec![0];
        for (i, b) in text.bytes().enumerate() {
            if b == b'\n' {
                starts.push(i as u32 + 1);
            }
        }
        LineIndex {
            text: text.to_owned(),
            starts,
        }
    }

    /// The text's length in bytes.
    pub fn len(&self) -> u32 {
        self.text.len() as u32
    }

    /// Whether the text is empty.
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// The zero-based line and UTF-16 column of byte `offset`. An offset
    /// past the end is the end of the text.
    pub fn position(&self, offset: u32) -> (u32, u32) {
        let offset = offset.min(self.len());
        let line = match self.starts.binary_search(&offset) {
            Ok(line) => line,
            Err(next) => next - 1,
        };
        let start = self.starts[line] as usize;
        let mut column = 0u32;
        for (i, c) in self.text[start..].char_indices() {
            if start + i >= offset as usize {
                break;
            }
            column += c.len_utf16() as u32;
        }
        (line as u32, column)
    }

    /// The byte offset of zero-based `line` and UTF-16 `column`.
    ///
    /// Out-of-range positions clamp:
    /// - a column past the line's end goes to the line's end, before its
    ///   `\r\n` or `\n`;
    /// - a line past the last goes to the end of the text;
    /// - a column inside a surrogate pair goes to its character's start.
    pub fn offset(&self, line: u32, column: u32) -> u32 {
        let Some(&start) = self.starts.get(line as usize) else {
            return self.len();
        };
        let end = self.line_end(line as usize);
        let mut units = 0u32;
        for (i, c) in self.text[start as usize..end].char_indices() {
            let next = units + c.len_utf16() as u32;
            if next > column {
                return start + i as u32;
            }
            units = next;
        }
        end as u32
    }

    /// The byte offset where `line`'s text ends, before its line break.
    fn line_end(&self, line: usize) -> usize {
        let start = self.starts[line] as usize;
        let mut end = self
            .starts
            .get(line + 1)
            .map_or(self.text.len(), |&s| s as usize);
        let bytes = self.text.as_bytes();
        if end > start && bytes[end - 1] == b'\n' {
            end -= 1;
        }
        if end > start && bytes[end - 1] == b'\r' {
            end -= 1;
        }
        end
    }
}
