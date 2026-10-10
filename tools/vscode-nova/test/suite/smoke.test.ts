// The extension's smoke test (spec §7.6; 3.4a §7; 3.4b §8): the language, a
// diagnostic, a completion, a formatted document, a hover, a definition,
// semantic tokens and a quick fix, through the real `nova lsp`.

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

  it("hovers over a call with its signature and doc", async () => {
    const uri = vscode.Uri.file(path.join(fixture, "src", "navigate.nova"));
    const doc = await vscode.workspace.openTextDocument(uri);
    const at = doc.positionAt(doc.getText().indexOf("area(") + 1);
    const text = await eventually("a hover", async () => {
      const hovers = await vscode.commands.executeCommand<vscode.Hover[]>(
        "vscode.executeHoverProvider",
        uri,
        at,
      );
      const joined = (hovers ?? [])
        .flatMap((h) => h.contents.map((c) => (typeof c === "string" ? c : c.value)))
        .join("\n");
      return joined.includes("fn area") ? joined : undefined;
    });
    assert.ok(text.includes("pub fn area(side: Int) -> Int"), text);
    assert.ok(text.includes("The area of a square."), text);
  });

  it("goes to a definition in another module", async () => {
    const uri = vscode.Uri.file(path.join(fixture, "src", "navigate.nova"));
    const doc = await vscode.workspace.openTextDocument(uri);
    const at = doc.positionAt(doc.getText().indexOf("area(") + 1);
    const found = await eventually("a definition", async () => {
      const locations = await vscode.commands.executeCommand<
        (vscode.Location | vscode.LocationLink)[]
      >("vscode.executeDefinitionProvider", uri, at);
      return locations !== undefined && locations.length > 0 ? locations : undefined;
    });
    const first = found[0];
    const target = "targetUri" in first ? first.targetUri : first.uri;
    const range =
      "targetUri" in first ? (first.targetSelectionRange ?? first.targetRange) : first.range;
    assert.ok(target.fsPath.endsWith("shapes.nova"), target.fsPath);
    assert.strictEqual(range.start.line, 1);
    assert.strictEqual(range.start.character, 7);
  });

  it("colours a call as a function with a semantic token", async () => {
    const uri = vscode.Uri.file(path.join(fixture, "src", "navigate.nova"));
    await vscode.workspace.openTextDocument(uri);
    // One `eventually`, which asks for the legend and the tokens in each
    // attempt: two would wait up to 120 s, past mocha's 90 s timeout.
    const types = await eventually("a function token", async () => {
      const legend = await vscode.commands.executeCommand<vscode.SemanticTokensLegend>(
        "vscode.provideDocumentSemanticTokensLegend",
        uri,
      );
      const found = await vscode.commands.executeCommand<vscode.SemanticTokens>(
        "vscode.provideDocumentSemanticTokens",
        uri,
      );
      if (legend === undefined || found === undefined) {
        return undefined;
      }
      const fn = legend.tokenTypes.indexOf("function");
      for (let i = 3; i < found.data.length; i += 5) {
        if (found.data[i] === fn) {
          return legend.tokenTypes;
        }
      }
      return undefined;
    });
    assert.ok(types.includes("function"), types.join(", "));
  });

  it("offers a quick fix for a planted error", async () => {
    const uri = vscode.Uri.file(path.join(fixture, "src", "fix.nova"));
    const doc = await vscode.workspace.openTextDocument(uri);
    const at = doc.positionAt(doc.getText().indexOf("x = 2"));
    const titles = await eventually("a quick fix", async () => {
      const found = await vscode.commands.executeCommand<(vscode.CodeAction | vscode.Command)[]>(
        "vscode.executeCodeActionProvider",
        uri,
        new vscode.Range(at, at),
      );
      const names = (found ?? []).map((a) => a.title);
      return names.includes("Make `x` mutable") ? names : undefined;
    });
    assert.ok(titles.includes("Make `x` mutable"), titles.join(", "));
  });
});
