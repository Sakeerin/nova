//! `nova parse <file>` — parse a Nova source file and print the AST.
//!
//! The hidden `--dump-tokens FILE…` prints each file's token dump instead,
//! what the self-hosted lexer's `selfhost tokens` prints (spec
//! `docs/superpowers/specs/2026-10-30-phase-5-1-the-lexer-the-parser-and-the-ast-design.md`
//! §3.3, ADR 0051).

use std::ffi::{OsStr, OsString};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::builder::{OsStringValueParser, TypedValueParser};
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

    /// Print the token dump of FILE and of every further file given, for
    /// the self-hosted lexer's differential test. Hidden: it carries no
    /// stability promise.
    #[arg(long, hide = true, conflicts_with_all = ["ast", "check"])]
    dump_tokens: bool,

    /// The files after FILE, which only `--dump-tokens` takes. Taken as
    /// they are, an empty one included, so that without the flag `run`
    /// refuses them in clap's words, and with it an empty one is a file
    /// that is not there, as N reads it.
    #[arg(hide = true, value_parser = OsStringValueParser::new().map(PathBuf::from))]
    more: Vec<PathBuf>,
}

pub fn run(cmd: ParseCmd) -> Result<()> {
    if cmd.dump_tokens {
        let files: Vec<&Path> = std::iter::once(cmd.file.as_path())
            .chain(cmd.more.iter().map(PathBuf::as_path))
            .collect();
        let code = dump_tokens(&files)?;
        if code != 0 {
            std::process::exit(code);
        }
        return Ok(());
    }
    if let Some(extra) = cmd.more.first() {
        // What clap said before `more` existed, without naming the hidden
        // flag: its "unexpected argument" error, built as clap builds it,
        // so a terminal shows it in the same colours, with its usage line
        // naming the command as clap does (`parse_bin_name`).
        use clap::error::{ContextKind, ContextValue, ErrorKind};
        let cli = <crate::Cli as clap::CommandFactory>::command();
        let mut parse = cli
            .find_subcommand("parse")
            .expect("parse is a subcommand")
            .clone()
            .bin_name(parse_bin_name(std::env::args_os().next()));
        let usage = parse.render_usage();
        let mut error = clap::Error::new(ErrorKind::UnknownArgument).with_cmd(&parse);
        error.insert(
            ContextKind::InvalidArg,
            ContextValue::String(extra.display().to_string()),
        );
        error.insert(ContextKind::Usage, ContextValue::StyledStr(usage));
        error.exit();
    }
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
            e.notes().into_iter().fold(
                nova_diagnostics::Diagnostic::error("P0001", e.to_string())
                    .with_primary_label(e.span(), "here"),
                |d, n| d.with_note(n),
            )
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

/// The name clap's usage line gives `nova parse`, from the binary's first
/// argument `arg0`, as clap names it: that argument's file name, then
/// `parse`, so a renamed `nova` prints its own; or `parse` alone when the
/// argument has no file name in UTF-8 (`""`, `/`, `..`, bytes that are not
/// UTF-8) or there is none.
fn parse_bin_name(arg0: Option<OsString>) -> String {
    let name = arg0
        .as_deref()
        .and_then(|arg0| Path::new(arg0).file_name())
        .and_then(OsStr::to_str);
    match name {
        Some(name) => format!("{name} parse"),
        None => "parse".to_string(),
    }
}

/// `--dump-tokens`: for each file, `== FILE` (the path as given), then its
/// token dump; for a file that cannot be read, `== FILE` then `IOERR` and
/// its kind, after which the rest are dumped and the exit code is 2.
fn dump_tokens(files: &[&Path]) -> Result<i32> {
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    let mut code = 0;
    for file in files {
        writeln!(out, "== {}", file.display())?;
        match std::fs::read_to_string(file) {
            Ok(source) => out.write_all(nova_lexer::dump_tokens(&source).as_bytes())?,
            Err(e) => {
                writeln!(out, "IOERR {}", io_kind_name(e.kind()))?;
                code = 2;
            }
        }
    }
    out.flush()?;
    Ok(code)
}

/// The kind as std/io's `IoErrorKind` names it, so N's dump, whose reads
/// fail with those kinds, prints the same word: the seven kinds it has, and
/// `Other` for every other.
fn io_kind_name(kind: std::io::ErrorKind) -> &'static str {
    use std::io::ErrorKind;
    match kind {
        ErrorKind::NotFound => "NotFound",
        ErrorKind::PermissionDenied => "PermissionDenied",
        ErrorKind::AlreadyExists => "AlreadyExists",
        ErrorKind::InvalidData => "InvalidData",
        ErrorKind::Interrupted => "Interrupted",
        ErrorKind::TimedOut => "TimedOut",
        ErrorKind::ConnectionRefused => "ConnectionRefused",
        _ => "Other",
    }
}
