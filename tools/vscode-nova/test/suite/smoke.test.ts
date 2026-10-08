// The extension's smoke test (spec §7.6): the language, a diagnostic, a
// completion and a formatted document, through the real `nova lsp`.

import * as assert from "assert";
import * as path from "path";
import * as vscode from "vscode";

// out/test/suite -> the extension -> test/fixture.
const fixture = path.resolve(__dirname, "..", "..", "..", "test", "fixture");

function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/** Retry `attempt` until it returns a value, for at most 60 s. */
async function eventually<T>(what: string, attempt: () => Promise<T | undefined>): Promise<T> {
  const deadline = Date.now() + 60000;
  for (;;) {
    const value = await attempt();
    if (value !== undefined) {
      return value;
    }
    if (Date.now() > deadline) {
      throw new Error(`timed out waiting for ${what}`);
    }
    await sleep(500);
  }
}

describe("the Nova extension", () => {
  it("gives .nova files the language nova", async () => {
    const doc = await vscode.workspace.openTextDocument(path.join(fixture, "src", "main.nova"));
    assert.strictEqual(doc.languageId, "nova");
  });

  it("shows the planted error's diagnostic", async () => {
    const uri = vscode.Uri.file(path.join(fixture, "src", "main.nova"));
    await vscode.window.showTextDocument(await vscode.workspace.openTextDocument(uri));
    const diagnostics = await eventually("a diagnostic", async () => {
      const found = vscode.languages.getDiagnostics(uri);
      return found.length > 0 ? found : undefined;
    });
    assert.ok(
      diagnostics.some((d) => d.code === "E0010"),
      JSON.stringify(diagnostics),
    );
  });

  it("completes a record field after a dot", async () => {
    const uri = vscode.Uri.file(path.join(fixture, "src", "complete.nova"));
    const doc = await vscode.workspace.openTextDocument(uri);
    const at = doc.positionAt(doc.getText().indexOf("p.") + 2);
    const labels = await eventually("a completion of x", async () => {
      const list = await vscode.commands.executeCommand<vscode.CompletionList>(
        "vscode.executeCompletionItemProvider",
        uri,
        at,
        ".",
      );
      const names = list.items.map((i) => (typeof i.label === "string" ? i.label : i.label.label));
      return names.includes("x") ? names : undefined;
    });
    assert.ok(labels.includes("x"), labels.join(", "));
  });

  it("formats a document", async () => {
    const uri = vscode.Uri.file(path.join(fixture, "src", "unformatted.nova"));
    const doc = await vscode.workspace.openTextDocument(uri);
    const edits = await eventually("a formatting edit", async () => {
      const found = await vscode.commands.executeCommand<vscode.TextEdit[]>(
        "vscode.executeFormatDocumentProvider",
        uri,
        { tabSize: 4, insertSpaces: true },
      );
      return found !== undefined && found.length > 0 ? found : undefined;
    });
    // VS Code shrinks the server's one whole-document edit to minimal ones,
    // so check the text they make, applied to the unchanged document from
    // its end backwards.
    let text = doc.getText();
    const fromTheEnd = [...edits].sort(
      (a, b) => doc.offsetAt(b.range.start) - doc.offsetAt(a.range.start),
    );
    for (const edit of fromTheEnd) {
      text =
        text.slice(0, doc.offsetAt(edit.range.start)) +
        edit.newText +
        text.slice(doc.offsetAt(edit.range.end));
    }
    assert.strictEqual(text, 'fn main() { println("hi") }\n');
  });
});
