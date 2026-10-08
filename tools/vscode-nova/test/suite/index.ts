import * as path from "path";
import Mocha from "mocha";

export function run(): Promise<void> {
  const mocha = new Mocha({ ui: "bdd", timeout: 90000 });
  mocha.addFile(path.resolve(__dirname, "smoke.test.js"));
  return new Promise((resolve, reject) => {
    mocha.run((failures) =>
      failures > 0 ? reject(new Error(`${failures} test(s) failed`)) : resolve(),
    );
  });
}
