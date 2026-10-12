#!/usr/bin/env bash
# Publishes the 19 crates to crates.io in this order (spec docs/superpowers/specs/2026-10-12-phase-3-6a-release-preparation-design.md §5.2).
crates=(
  nova-diagnostics
  nova-std
  nova-ast
  nova-lexer
  nova-runtime
  nova-parser
  nova-pm
  nova-fmt
  nova-index
  nova-resolver
  nova-hir
  nova-mir
  nova-typeck
  nova-codegen-cranelift
  nova-codegen-llvm
  nova-driver
  nova-doc
  nova-lsp
  nova-cli
)
