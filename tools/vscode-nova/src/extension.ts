// The Nova extension: the language, its grammar, and a client for
// `nova lsp` (spec docs/superpowers/specs/2026-10-08-phase-3-2-lsp-core-design.md §7).

import * as vscode from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export async function activate(context: vscode.ExtensionContext): Promise<void> {
  context.subscriptions.push(
    vscode.commands.registerCommand("nova.restartServer", async () => {
      await stop();
      await start();
    }),
  );
  await start();
}

export async function deactivate(): Promise<void> {
  await stop();
}

/** The `nova` to run: the `nova.server.path` setting, or `nova` from PATH. */
function serverCommand(): string {
  const configured = vscode.workspace
    .getConfiguration("nova")
    .get<string>("server.path", "");
  return configured.trim() !== "" ? configured : "nova";
}

async function start(): Promise<void> {
  const command = serverCommand();
  const serverOptions: ServerOptions = { command, args: ["lsp"] };
  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "nova" },
      { scheme: "untitled", language: "nova" },
    ],
  };
  // The client id `nova` makes the `nova.trace.server` setting apply.
  client = new LanguageClient("nova", "Nova", serverOptions, clientOptions);
  try {
    await client.start();
  } catch (error) {
    client = undefined;
    void vscode.window.showErrorMessage(
      `Nova: could not start "${command} lsp" (${String(error)}). ` +
        "Set nova.server.path, or install nova: " +
        "cargo install --locked --git https://github.com/Sakeerin/nova nova-cli",
    );
  }
}

async function stop(): Promise<void> {
  if (client !== undefined) {
    const running = client;
    client = undefined;
    await running.stop();
  }
}
