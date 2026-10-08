// Runs the smoke test in a downloaded VS Code, pinned to the extension's
// floor, 1.91.0 (decision 12), against the freshly built debug `nova`.

import * as fs from "fs";
import * as path from "path";
import { runTests } from "@vscode/test-electron";

async function main(): Promise<void> {
  // out/test -> out -> tools/vscode-nova -> tools -> the repository.
  const extensionDevelopmentPath = path.resolve(__dirname, "..", "..");
  const repo = path.resolve(extensionDevelopmentPath, "..", "..");
  const nova = path.join(
    repo,
    "target",
    "debug",
    process.platform === "win32" ? "nova.exe" : "nova",
  );
  if (!fs.existsSync(nova)) {
    throw new Error(`build nova first (cargo build -p nova-cli): ${nova} is missing`);
  }
  const workspace = path.join(extensionDevelopmentPath, "test", "fixture");
  // The fixture's settings point the extension at that `nova`; the file is
  // git-ignored.
  fs.mkdirSync(path.join(workspace, ".vscode"), { recursive: true });
  fs.writeFileSync(
    path.join(workspace, ".vscode", "settings.json"),
    JSON.stringify({ "nova.server.path": nova }, null, 2),
  );
  await runTests({
    version: "1.91.0",
    extensionDevelopmentPath,
    extensionTestsPath: path.resolve(__dirname, "suite", "index"),
    launchArgs: [workspace, "--disable-extensions"],
  });
}

main().catch((error) => {
  console.error(error);
  process.exit(1);
});
