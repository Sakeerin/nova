//! `nova parse <file>` — parse a Nova source file and print the AST.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Args;
use nova_diagnostics::{render, FileDb};
use nova_lexer::lex;
use nova_parser::parse;

#[derive(Args)]
pub struct ParseCmd {
    /// Path to the Nova source file to parse.
    file: PathBuf,

    /// Print the full AST debug representation.
    #[arg(long)]
    ast: bool,

    /// Print only errors (exit 1 if any).
    #[arg(long)]
    check: bool,
}

pub fn run(cmd: ParseCmd) -> Result<()> {
    let source = std::fs::read_to_string(&cmd.file)
        .with_context(|| format!("failed to read {}", cmd.file.display()))?;

    let mut db = FileDb::new();
    let file_id = db.add(cmd.file.display().to_string(), source.as_str());

    let (tokens, lex_errors) = lex(&source, file_id);

    // Render lex errors
    let lex_diags: Vec<_> = lex_errors
        .iter()
        .map(|e| {
            nova_diagnostics::Diagnostic::error("L0001", e.to_string())
                .with_primary_label(e.span(), "here")
        })
        .collect();
    if !lex_diags.is_empty() {
        render::emit_all(&db, &lex_diags);
    }

    let (ast, parse_errors) = parse(&tokens, file_id);

    // Render parse errors
    let parse_diags: Vec<_> = parse_errors
        .iter()
        .map(|e| {
            nova_diagnostics::Diagnostic::error("P0001", e.to_string())
                .with_primary_label(e.span(), "here")
        })
        .collect();
    if !parse_diags.is_empty() {
        render::emit_all(&db, &parse_diags);
    }

    let total_errors = lex_errors.len() + parse_errors.len();

    if cmd.ast {
        if let Some(tree) = &ast {
            println!("{:#?}", tree);
        }
    }

    if total_errors == 0 {
        if !cmd.check {
            let item_count = ast.as_ref().map(|f| f.items.len()).unwrap_or(0);
            println!(
                "Parsed {} — {} top-level item{}",
                cmd.file.display(),
                item_count,
                if item_count == 1 { "" } else { "s" }
            );
        }
        Ok(())
    } else {
        anyhow::bail!(
            "{} error{} found",
            total_errors,
            if total_errors == 1 { "" } else { "s" }
        )
    }
}
