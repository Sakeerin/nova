//! Organize imports' block (spec
//! `docs/superpowers/specs/2026-10-10-phase-3-4b-fixes-and-colour-design.md`
//! §6, §9.4).

use nova_fmt::{format, organize, Group, ImportView, TextEdit, Verdict};

/// Every import the project's own, nothing unused.
fn keep(_: &ImportView) -> Verdict {
    Verdict {
        group: Group::Module,
        unused_glob: false,
        unused_names: Vec::new(),
    }
}

/// `text` with `edits` applied, from the last.
fn applied(text: &str, edits: &[TextEdit]) -> String {
    let mut edits = edits.to_vec();
    edits.sort_by_key(|e| e.start);
    let mut out = text.to_string();
    for e in edits.iter().rev() {
        out.replace_range(e.start as usize..e.end as usize, &e.text);
    }
    out
}

/// `text` organized by `judge`; the result is as `nova fmt` prints it, and
/// a second run offers nothing.
#[track_caller]
fn organized(text: &str, judge: &dyn Fn(&ImportView) -> Verdict) -> String {
    let edits = organize(text, judge).unwrap_or_else(|| panic!("nothing to do for {text:?}"));
    let out = applied(text, &edits);
    assert_eq!(organize(&out, judge), None, "a second run changes {out:?}");
    assert_eq!(
        format(&out).unwrap(),
        out.replace("\r\n", "\n"),
        "not as nova fmt prints it"
    );
    out
}

#[test]
fn scattered_imports_gather_at_the_first() {
    assert_eq!(
        organized("import b\n\nfn f() {}\n\nimport a\n", &keep),
        "import a\nimport b\n\nfn f() {}\n"
    );
}

#[test]
fn imports_and_their_lists_are_sorted() {
    assert_eq!(
        organized("import zeta\nimport alpha::{c, a}\n\nfn main() {}\n", &keep),
        "import alpha::{a, c}\nimport zeta\n\nfn main() {}\n"
    );
}

#[test]
fn imports_of_one_module_merge() {
    assert_eq!(
        organized(
            "import m::{b}\nimport m::{a}\nimport n\nimport n\n\nfn main() {}\n",
            &keep
        ),
        "import m::{a, b}\nimport n\n\nfn main() {}\n"
    );
    // A list beside a glob of the same module goes (spec §6.3).
    assert_eq!(
        organized("import m\nimport m::{a}\n\nfn main() {}\n", &keep),
        "import m\n\nfn main() {}\n"
    );
}

#[test]
fn unused_imports_and_names_go() {
    let judge = |v: &ImportView| Verdict {
        group: Group::Module,
        unused_glob: v.first == "n",
        unused_names: match v.first {
            "m" => vec!["b".to_string()],
            "o" => vec!["c".to_string()],
            _ => Vec::new(),
        },
    };
    assert_eq!(
        organized(
            "import m::{a, b}\nimport n\nimport o::{c}\n\nfn main() {}\n",
            &judge
        ),
        "import m::{a}\n\nfn main() {}\n"
    );
}

#[test]
fn comments_travel_with_their_imports_and_a_header_stays() {
    let text = "// header\n\n// about m\nimport m // why m\n// about a\nimport a\n\nfn main() {}\n";
    assert_eq!(
        organized(text, &keep),
        "// header\n\n// about a\nimport a\n// about m\nimport m // why m\n\nfn main() {}\n"
    );
    // An unused import's comments go with it.
    let judge = |v: &ImportView| Verdict {
        group: Group::Module,
        unused_glob: v.first == "a",
        unused_names: Vec::new(),
    };
    assert_eq!(
        organized(text, &judge),
        "// header\n\n// about m\nimport m // why m\n\nfn main() {}\n"
    );
}

#[test]
fn a_doc_comment_travels_with_its_import() {
    assert_eq!(
        organized(
            "/// The shapes.\nimport m\nimport a\n\nfn main() {}\n",
            &keep
        ),
        "import a\n/// The shapes.\nimport m\n\nfn main() {}\n"
    );
}

#[test]
fn two_groups_dependencies_first() {
    let judge = |v: &ImportView| Verdict {
        group: if v.first == "geom" {
            Group::Dependency
        } else {
            Group::Module
        },
        unused_glob: false,
        unused_names: Vec::new(),
    };
    assert_eq!(
        organized(
            "import shapes\nimport geom\nimport app_utils\n\nfn main() {}\n",
            &judge
        ),
        "import geom\n\nimport app_utils\nimport shapes\n\nfn main() {}\n"
    );
}

#[test]
fn organize_removes_every_import_and_the_blank_line_after() {
    // Review Focus 5.
    let unused = |_: &ImportView| Verdict {
        group: Group::Module,
        unused_glob: true,
        unused_names: Vec::new(),
    };
    assert_eq!(
        organized("import m\nimport n\n\nfn main() {}\n", &unused),
        "fn main() {}\n"
    );
    // At the end of the text, the blank line before it goes.
    let b_unused = |v: &ImportView| Verdict {
        group: Group::Module,
        unused_glob: v.first == "b",
        unused_names: Vec::new(),
    };
    assert_eq!(
        organized("import a\n\nfn f() {}\n\nimport b\n", &b_unused),
        "import a\n\nfn f() {}\n"
    );
}

#[test]
fn a_crlf_file_keeps_its_line_endings() {
    assert_eq!(
        organized("import b\r\nimport a\r\n\r\nfn main() {}\r\n", &keep),
        "import a\r\nimport b\r\n\r\nfn main() {}\r\n"
    );
}

#[test]
fn nothing_to_do_or_nothing_parsed_offers_nothing() {
    assert_eq!(
        organize("import a\nimport b\n\nfn main() {}\n", &keep),
        None
    );
    assert_eq!(organize("fn main() {}\n", &keep), None);
    assert_eq!(organize("import a\nfn main( {\n", &keep), None);
}
