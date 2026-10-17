//! The VS Code extension's files agree with the compiler (spec
//! `docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md` §7.2,
//! §7.4).

use std::path::PathBuf;

use serde_json::Value;

fn extension() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("tools")
        .join("vscode-nova")
}

fn json(file: &str) -> Value {
    let path = extension().join(file);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

#[test]
fn the_grammar_colours_every_keyword_and_nothing_else() {
    let grammar = json("syntaxes/nova.tmLanguage.json");
    let pattern = grammar["repository"]["keywords"]["match"]
        .as_str()
        .expect("a keywords pattern");
    let inner = pattern
        .strip_prefix("\\b(")
        .and_then(|p| p.strip_suffix(")\\b"))
        .unwrap_or_else(|| panic!("not \\b(…)\\b: {pattern}"));
    let mut listed: Vec<&str> = inner.split('|').collect();
    let mut keywords: Vec<&str> = nova_lexer::KEYWORDS.to_vec();
    listed.sort_unstable();
    keywords.sort_unstable();
    assert_eq!(listed, keywords);
}

/// `component` and `view` are coloured as keywords only before a name and
/// `(`, and before `{`: neither is reserved, so `KEYWORDS` lacks both, and
/// the rule comes before the one for keywords (the 4.4 spec §6).
#[test]
fn the_grammar_colours_component_and_view_where_they_are_keywords() {
    let grammar = json("syntaxes/nova.tmLanguage.json");
    let includes: Vec<&str> = grammar["patterns"]
        .as_array()
        .expect("patterns")
        .iter()
        .filter_map(|p| p["include"].as_str())
        .collect();
    let at = |name: &str| includes.iter().position(|i| *i == name).unwrap();
    assert!(at("#function-names") < at("#contextual-keywords"));
    assert!(at("#contextual-keywords") < at("#keywords"));
    let rules = grammar["repository"]["contextual-keywords"]["patterns"]
        .as_array()
        .expect("contextual keyword rules");
    assert_eq!(
        rules[0]["match"],
        "\\b(component)\\s+([A-Za-z_][A-Za-z0-9_]*)\\s*(?=\\()"
    );
    assert_eq!(
        rules[0]["captures"]["1"]["name"],
        "keyword.other.component.nova"
    );
    assert_eq!(
        rules[0]["captures"]["2"]["name"],
        "entity.name.function.nova"
    );
    assert_eq!(rules[1]["match"], "\\bview(?=\\s*\\{)");
    assert_eq!(rules[1]["name"], "keyword.other.view.nova");
    assert!(!nova_lexer::KEYWORDS.contains(&"component"));
    assert!(!nova_lexer::KEYWORDS.contains(&"view"));
}

#[test]
fn the_extension_has_nova_clis_version_and_its_floor() {
    let package = json("package.json");
    assert_eq!(package["version"], env!("CARGO_PKG_VERSION"));
    assert_eq!(package["engines"]["vscode"], "^1.91.0");
    assert_eq!(package["publisher"], "sakeerin");
    assert_eq!(package["name"], "vscode-nova");
    assert_eq!(package["displayName"], "Nova Language Tools");
    assert_eq!(package["contributes"]["languages"][0]["id"], "nova");
}
