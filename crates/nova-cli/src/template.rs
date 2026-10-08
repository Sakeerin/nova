//! The files `nova new` and `nova init` write (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
//! §6.3, and for a library
//! `docs/superpowers/specs/2026-10-08-phase-3-3a-local-packages-design.md`
//! §5.5), with `\n` line endings on every system.

const MAIN: &str = concat!(
    "fn greeting() -> String {\n",
    "    \"Hello, Nova!\"\n",
    "}\n",
    "\n",
    "fn main() {\n",
    "    println(greeting())\n",
    "}\n",
    "\n",
    "@test\n",
    "fn greeting_says_hello() {\n",
    "    assert_eq(greeting(), \"Hello, Nova!\")\n",
    "}\n",
);

const LIB: &str = concat!(
    "pub fn greeting() -> String {\n",
    "    \"Hello, Nova!\"\n",
    "}\n",
    "\n",
    "@test\n",
    "fn greeting_says_hello() {\n",
    "    assert_eq(greeting(), \"Hello, Nova!\")\n",
    "}\n",
);

/// What a template makes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `src/main.nova`.
    Program,
    /// `src/lib.nova`, and a test in `tests/`.
    Library,
}

/// The template of `kind` for a project called `name`, as (path, text)
/// pairs. `name` has passed `nova_pm::check_name`, so it needs no TOML
/// escaping.
pub fn files(name: &str, kind: Kind) -> Vec<(String, String)> {
    let mut files = vec![
        (
            "nova.toml".to_string(),
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n\
                 [dependencies]\n"
            ),
        ),
        (".gitignore".to_string(), "/target\n".to_string()),
    ];
    match kind {
        Kind::Program => {
            files.push((
                "README.md".to_string(),
                format!(
                    "# {name}\n\n`nova run` builds and runs `src/main.nova`; `nova test` runs \
                     its tests.\n"
                ),
            ));
            files.push(("src/main.nova".to_string(), MAIN.to_string()));
        }
        Kind::Library => {
            // `_test` keeps the file's name from ever being the import name,
            // which would be E0004 (spec 3.3a §5.5).
            let import = nova_pm::import_name(name);
            files.push((
                "README.md".to_string(),
                format!(
                    "# {name}\n\nA library: other packages import it as `import {import}`. \
                     `nova test` runs the tests in `src/lib.nova` and `tests/`.\n"
                ),
            ));
            files.push(("src/lib.nova".to_string(), LIB.to_string()));
            files.push((
                format!("tests/{import}_test.nova"),
                format!(
                    "import {import}\n\n@test\nfn greeting_is_public() {{\n    \
                     assert_eq(greeting(), \"Hello, Nova!\")\n}}\n"
                ),
            ));
        }
    }
    files
}
