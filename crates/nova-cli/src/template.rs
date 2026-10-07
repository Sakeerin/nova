//! The files `nova new` and `nova init` write (spec
//! `docs/superpowers/specs/2026-10-07-phase-3-0-foundations-design.md`
//! §6.3), with `\n` line endings on every system.

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

/// The template for a project called `name`, as (path, text) pairs. `name`
/// has passed `nova_pm::check_name`, so it needs no TOML escaping.
pub fn files(name: &str) -> [(&'static str, String); 4] {
    [
        (
            "nova.toml",
            format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2026\"\n\n\
                 [dependencies]\n"
            ),
        ),
        (".gitignore", "/target\n".to_string()),
        (
            "README.md",
            format!(
                "# {name}\n\n`nova run` builds and runs `src/main.nova`; `nova test` runs its \
                 tests.\n"
            ),
        ),
        ("src/main.nova", MAIN.to_string()),
    ]
}
