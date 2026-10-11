//! The search index (spec §10.1; plan decision 14).

use crate::model::{Impl, Site};

/// One entry: name, kind, path, URL from the site's root, and summary.
type Entry = [String; 5];

fn entry(name: &str, kind: &str, path: String, url: String, summary: &str) -> Entry {
    [
        name.to_string(),
        kind.to_string(),
        path,
        url,
        summary.to_string(),
    ]
}

/// An impl's documented methods, under `path`.
fn methods(entries: &mut Vec<Entry>, path: &str, page: &str, i: &Impl) {
    for m in &i.methods {
        entries.push(entry(
            &m.name,
            m.kind.word(),
            format!("{path}::{}", m.name),
            format!("{page}#{}", m.anchor),
            &m.docs.summary,
        ));
    }
}

/// `search-index.js`: `window.NOVA_SEARCH`, one entry per line (spec
/// §10.1; plan decision 14).
pub(crate) fn index(site: &Site) -> String {
    let mut entries: Vec<Entry> = Vec::new();
    for p in &site.packages {
        entries.push(entry(
            &p.name,
            "package",
            p.name.clone(),
            format!("{}/index.html", p.name),
            p.description.as_deref().unwrap_or(""),
        ));
        for m in &p.modules {
            if !m.root {
                entries.push(entry(
                    &m.name,
                    "module",
                    m.qual.clone(),
                    m.page.clone(),
                    &m.docs.summary,
                ));
            }
            for it in &m.items {
                let path = format!("{}::{}", m.qual, it.name);
                entries.push(entry(
                    &it.name,
                    it.kind.word(),
                    path.clone(),
                    format!("{}#{}", m.page, it.anchor),
                    &it.docs.summary,
                ));
                for mem in &it.members {
                    entries.push(entry(
                        &mem.name,
                        mem.kind.word(),
                        format!("{path}::{}", mem.name),
                        format!("{}#{}", m.page, mem.anchor),
                        &mem.docs.summary,
                    ));
                }
                for i in &it.impls {
                    methods(&mut entries, &path, &m.page, i);
                }
            }
            for i in &m.other_impls {
                methods(&mut entries, &format!("{}::{}", m.qual, i.on), &m.page, i);
            }
        }
        for pr in &p.primitives {
            let path = format!("std::{}", pr.name);
            entries.push(entry(
                pr.name,
                "primitive",
                path.clone(),
                pr.page.clone(),
                &pr.about.summary,
            ));
            for i in &pr.impls {
                methods(&mut entries, &path, &pr.page, i);
            }
        }
        for b in &p.builtins {
            entries.push(entry(
                b.name,
                "fn",
                format!("std::{}", b.name),
                format!("std/index.html#fn.{}", b.name),
                &b.docs.summary,
            ));
        }
    }
    entries.sort_by(|a, b| (&a[2], &a[1], &a[3]).cmp(&(&b[2], &b[1], &b[3])));
    let lines: Vec<String> = entries
        .iter()
        .map(|e| serde_json::to_string(e).expect("strings serialize"))
        .collect();
    format!("window.NOVA_SEARCH = [\n{}\n];\n", lines.join(",\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::sample_site;

    #[test]
    fn the_index_has_an_entry_per_item_and_member_sorted_and_escaped() {
        let index = index(&sample_site());
        assert!(index.starts_with("window.NOVA_SEARCH = [\n"), "{index}");
        assert!(index.ends_with("\n];\n"), "{index}");
        let lines: Vec<&str> = index.lines().collect();
        assert!(lines.contains(&"[\"Point\",\"record\",\"app::Point\",\"app/index.html#record.Point\",\"A point.\"],"), "{index}");
        assert!(lines.contains(&"[\"x\",\"field\",\"app::Point::x\",\"app/index.html#record.Point.field.x\",\"Across \\\"x\\\".\"],"), "{index}");
        assert!(
            lines.contains(
                &"[\"util\",\"module\",\"app::util\",\"app/util/index.html\",\"A module.\"],"
            ),
            "{index}"
        );
        assert!(
            lines.iter().any(|l| l
                .starts_with("[\"println\",\"fn\",\"std::println\",\"std/index.html#fn.println\"")),
            "{index}"
        );
        let paths: Vec<&str> = lines[1..lines.len() - 1]
            .iter()
            .map(|l| l.split('"').nth(5).unwrap())
            .collect();
        let mut sorted = paths.clone();
        sorted.sort_unstable();
        assert_eq!(paths, sorted);
    }
}
