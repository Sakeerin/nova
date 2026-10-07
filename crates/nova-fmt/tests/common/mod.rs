//! Helpers shared by the formatter's tests.

/// Format `input`, expect exactly `expected`, and expect formatting that
/// again to change nothing.
#[allow(dead_code)]
pub fn assert_formats(input: &str, expected: &str) {
    let first = nova_fmt::format(input)
        .unwrap_or_else(|e| panic!("formatting failed: {e:?}\n--- input ---\n{input}"));
    assert_eq!(
        first, expected,
        "\n--- input ---\n{input}\n--- got ---\n{first}"
    );
    let second =
        nova_fmt::format(&first).unwrap_or_else(|e| panic!("formatting the output failed: {e:?}"));
    assert_eq!(second, first, "formatting is not idempotent");
}

/// Expect `source` to be formatted already.
#[allow(dead_code)]
pub fn assert_stable(source: &str) {
    assert_formats(source, source);
}
