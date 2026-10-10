//! "Did you mean" (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
//! §4.3).

use crate::{Edit, Fix, Spanned};

/// The optimal string alignment distance between `a` and `b`, over Unicode
/// scalar values: an insertion, a deletion, a substitution, or a swap of two
/// adjacent characters each counts one.
// The indices are offsets into three arrays at once, which is what the
// lint's iterator form cannot express.
#[allow(clippy::needless_range_loop)]
pub fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut d = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for (i, row) in d.iter_mut().enumerate() {
        row[0] = i;
    }
    for (j, cell) in d[0].iter_mut().enumerate() {
        *cell = j;
    }
    for i in 1..=a.len() {
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            let mut best = (d[i - 1][j] + 1)
                .min(d[i][j - 1] + 1)
                .min(d[i - 1][j - 1] + cost);
            if i > 1 && j > 1 && a[i - 1] == b[j - 2] && a[i - 2] == b[j - 1] {
                best = best.min(d[i - 2][j - 2] + 1);
            }
            d[i][j] = best;
        }
    }
    d[a.len()][b.len()]
}

/// The candidate to offer for the unknown `name`, other than `name`
/// itself (spec §4.3): one equal to it ignoring ASCII case, at any
/// distance; else the nearest within `max(len, 3) / 3` edits, `len` in
/// characters; ties in byte order.
pub fn closest<'c>(name: &str, candidates: impl IntoIterator<Item = &'c str>) -> Option<&'c str> {
    let bound = name.chars().count().max(3) / 3;
    let mut best: Option<(bool, usize, &'c str)> = None;
    for c in candidates {
        if c == name {
            continue;
        }
        let key = if c.eq_ignore_ascii_case(name) {
            (false, 0, c)
        } else {
            let d = distance(name, c);
            if d > bound {
                continue;
            }
            (true, d, c)
        };
        if best.map_or(true, |b| key < b) {
            best = Some(key);
        }
    }
    best.map(|(_, _, c)| c)
}

/// "change `x` to `y`", `y` the closest of `candidates` (spec §4.3).
pub fn change_to<'c>(
    name: &Spanned<String>,
    candidates: impl IntoIterator<Item = &'c str>,
) -> Option<Fix> {
    let to = closest(&name.value, candidates)?;
    Some(Fix::new(
        format!("change `{}` to `{to}`", name.value),
        vec![Edit::replace(name.span, to)],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FileId, Span};

    #[test]
    fn a_swap_is_one_edit() {
        assert_eq!(distance("cuont", "count"), 1);
        assert_eq!(distance("kitten", "sitting"), 3);
        assert_eq!(distance("", "ab"), 2);
    }

    #[test]
    fn the_nearest_within_the_bound_wins_then_byte_order() {
        assert_eq!(closest("cuont", ["mount", "count"]), Some("count"));
        assert_eq!(closest("ab", ["ac", "aa"]), Some("aa"));
        assert_eq!(closest("count", ["count"]), None);
    }

    #[test]
    fn the_bound_is_a_third_of_the_length() {
        // Five characters: one edit is offered, two are not.
        assert_eq!(closest("abcde", ["abxde"]), Some("abxde"));
        assert_eq!(closest("abcde", ["axxde"]), None);
        // Six characters: two edits are offered.
        assert_eq!(closest("abcdef", ["axxdef"]), Some("axxdef"));
    }

    #[test]
    fn a_name_equal_ignoring_case_is_offered_at_any_distance() {
        assert_eq!(
            closest("HTTPCLIENT", ["HTTPCLIENTS", "HttpClient"]),
            Some("HttpClient")
        );
    }

    #[test]
    fn closest_counts_characters_not_bytes() {
        // Review Focus 4: Thai is three bytes a character. Four characters
        // allow one edit; counted in bytes, twelve would allow four.
        assert_eq!(closest("ราคน", ["ราคา"]), Some("ราคา"));
        assert_eq!(closest("ราคน", ["รากก"]), None);
    }

    #[test]
    fn change_to_replaces_the_name() {
        let name = Spanned::new("cuont".to_string(), Span::new(4, 9, FileId::DUMMY));
        let fix = change_to(&name, ["count"]).unwrap();
        assert_eq!(fix.title, "change `cuont` to `count`");
        assert_eq!(fix.edits, vec![Edit::replace(name.span, "count")]);
    }
}
