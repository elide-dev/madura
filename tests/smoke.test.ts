/**
 * The smoke suite: every case in `tests/smoke` becomes two tests, each named
 * after the command line it runs.
 *
 * `compile` passes when madura exits like vanilla `javac`, writes the class
 * file its `package` implies, produces bytecode identical to javac's byte for
 * byte, and — for the sources that have a `main` — yields a class the JVM
 * runs. `check` passes when it reaches the same verdict as that compile while
 * writing nothing at all.
 *
 * See `tests/harness.ts` for how a `.java` file turns into cases.
 */
import { beforeAll, expect, test } from "bun:test";
import { existsSync, rmSync } from "node:fs";
import { join } from "node:path";

import {
  bytecodeDifferences,
  cases,
  checkArgs,
  compileArgs,
  inParallel,
  JAVA,
  JAVAC,
  MADURA,
  outDir,
  produced,
  REPO,
  run,
  status,
  vanillaArgs,
  type Case,
  type Run,
} from "./harness.ts";

const CASES = cases();

/** Vanilla `javac`'s verdict for each case, by id. */
const vanilla = new Map<string, Run>();

beforeAll(async () => {
  // One reference tree per case, built up front: the JVM spawns are the slow
  // part of this suite and they are all independent of each other.
  rmSync(join(REPO, "target/smoke"), { recursive: true, force: true });
  const results = await inParallel(CASES, (testCase) => run(JAVAC, vanillaArgs(testCase)));
  CASES.forEach((testCase, index) => vanilla.set(testCase.id, results[index]!));
}, 600_000);

/** What `check` left behind. Nothing, including the output directory itself. */
function residue(dir: string): string[] {
  if (!existsSync(join(REPO, dir))) return [];
  return [`created ${dir}`, ...produced(dir).map((name) => `wrote ${name}`)];
}

for (const testCase of CASES) {
  test(`madura ${compileArgs(testCase).join(" ")}`, async () => {
    rmSync(join(REPO, outDir("madura", testCase)), { recursive: true, force: true });
    const compiled = await run(MADURA, compileArgs(testCase));
    const reference = vanilla.get(testCase.id)!;

    if (testCase.expectFail) {
      expect(status(compiled)).not.toBe("exit 0");
      expect(compiled.exitCode).toBe(reference.exitCode);
      expect(bytecodeDifferences(testCase)).toEqual([]);
      return;
    }

    expect(status(compiled)).toBe("exit 0");
    expect(status(reference)).toBe("exit 0");
    expect(produced(outDir("madura", testCase))).toContain(testCase.classFile);
    expect(bytecodeDifferences(testCase)).toEqual([]);

    // Only the sources with a `main` are worth running; for everything else the
    // bytecode comparison above is the stronger statement anyway.
    if (testCase.mainClass === null) return;
    const executed = await run(JAVA, ["-cp", outDir("madura", testCase), testCase.mainClass]);
    expect(status(executed)).toBe("exit 0");
    if (testCase.expectedStdout !== null) {
      expect(executed.stdout.trim()).toBe(testCase.expectedStdout.trim());
    }
  });

  test(`madura ${checkArgs(testCase).join(" ")}`, async () => {
    rmSync(join(REPO, outDir("check", testCase)), { recursive: true, force: true });
    const checked = await run(MADURA, checkArgs(testCase));

    // Same verdict as the compile, and no bytecode: that is the whole feature.
    expect(checked.exitCode).toBe(vanilla.get(testCase.id)!.exitCode);
    if (!testCase.expectFail) expect(status(checked)).toBe("exit 0");
    expect(residue(outDir("check", testCase))).toEqual([]);
  });
}

// A corpus that silently stopped being discovered would make every assertion
// above vacuous.
test("the corpus is discovered", () => {
  expect(CASES.length).toBeGreaterThan(0);
  expect(new Set(CASES.map((testCase: Case) => testCase.id)).size).toBe(CASES.length);
});
