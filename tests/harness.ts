/**
 * Shared machinery for the smoke suite: it turns `tests/smoke/**\/*.java` into
 * a list of cases, and runs each of them through `madura` and through vanilla
 * `javac`.
 *
 * Everything about a case is read off its source file, so adding one means
 * adding a `.java` file and nothing else:
 *
 * - the class file to expect comes from the `package` declaration plus the
 *   file name, exactly as javac derives it;
 * - a `static ... void main(` makes the case runnable under `java -cp`, and a
 *   sibling `<Name>.out` pins what it should print;
 * - `// madura: <directive>` lines carry the rest — `fail` for sources the
 *   compiler must reject, `releases=8-25` (ranges or a comma-separated list)
 *   to compile the same source once per `--release`.
 *
 * Binaries resolve without any environment setup, preferring what `make all`
 * produces; `MADURA_BIN`, `JAVAC_BIN` and `JAVA_BIN` override.
 */
import { Glob } from "bun";
import { existsSync, readFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";

/** Repository root: this file lives at `<root>/tests`. */
export const REPO = resolve(import.meta.dir, "..");

/** The corpus, and the scratch tree every compile writes into. */
const SMOKE_DIR = "tests/smoke";
const OUT_DIR = "target/smoke";

/** The binary under test: `<root>/madura`, beside its `<arch>/lib/modules`. */
export const MADURA = process.env.MADURA_BIN ?? join(REPO, "target/dist/madura");

/**
 * The reference compiler. A full JDK is preferred over the jlink'd image in
 * `target`, since the claim under test is that madura matches a stock `javac`
 * — but the image is the same compiler and keeps the suite working with no JDK
 * installed at all.
 */
export const JAVAC = which("JAVAC_BIN", ["bin/javac"], "javac");

/** The JVM that runs compiled output, resolved the same way. */
export const JAVA = which("JAVA_BIN", ["bin/java"], "java");

function which(override: string, suffixes: string[], fallback: string): string {
  const explicit = process.env[override];
  if (explicit) return explicit;
  const roots = [process.env.JAVA_HOME].filter((root) => root !== undefined);
  for (const root of roots) {
    for (const suffix of suffixes) {
      const candidate = join(root, suffix);
      if (existsSync(candidate)) return candidate;
    }
  }
  return fallback;
}

if (!existsSync(MADURA)) {
  throw new Error(`no madura binary at ${MADURA} — run \`make all\`, or set MADURA_BIN`);
}

export type Case = {
  /** Stable identifier, and the scratch path each compile writes under. */
  id: string;
  /** Repo-relative source path, e.g. `tests/smoke/simple/Hello.java`. */
  source: string;
  /** Compiler flags this case adds, e.g. `["--release", "21"]`. */
  flags: string[];
  /** Whether both compilers are expected to reject the source. */
  expectFail: boolean;
  /** Class file the compile must produce, relative to its output directory. */
  classFile: string;
  /** Binary class name to run under `java -cp`, or null without a `main`. */
  mainClass: string | null;
  /** Expected stdout from that run, when a sibling `.out` file pins it. */
  expectedStdout: string | null;
};

/** Every case in the corpus, in a stable order. */
export function cases(): Case[] {
  return [...new Glob("**/*.java").scanSync({ cwd: join(REPO, SMOKE_DIR) })]
    .sort()
    .flatMap(expand);
}

/** Read a source file into one case per `--release` it asks for. */
function expand(relative: string): Case[] {
  const source = `${SMOKE_DIR}/${relative}`;
  const text = readFileSync(join(REPO, source), "utf8");
  const stem = relative.slice(0, -".java".length);

  // `simple/Hello.java` in `package simple;` is `simple/Hello.class` — the same
  // layout javac derives, so nothing has to be declared alongside the source.
  const name = stem.slice(stem.lastIndexOf("/") + 1);
  const pkg = text.match(/^\s*package\s+([\w.]+)\s*;/m)?.[1] ?? "";
  const qualified = pkg ? `${pkg}.${name}` : name;
  const outFile = join(REPO, dirname(source), `${name}.out`);

  const shared = {
    source,
    expectFail: directives(text).includes("fail"),
    classFile: `${qualified.replaceAll(".", "/")}.class`,
    mainClass: /\bstatic\s+(?:\w+\s+)*void\s+main\s*\(/.test(text) ? qualified : null,
    expectedStdout: existsSync(outFile) ? readFileSync(outFile, "utf8") : null,
  };

  const releases = releasesOf(text);
  if (releases.length === 0) return [{ id: stem, flags: [], ...shared }];
  return releases.map((release) => ({
    id: `${stem}-${release}`,
    flags: ["--release", `${release}`],
    ...shared,
  }));
}

/** Whitespace-separated tokens from `// madura: <directive> ...` lines. */
function directives(text: string): string[] {
  return [...text.matchAll(/^\s*\/\/\s*madura:\s*(.+)$/gm)].flatMap((match) =>
    match[1]!.trim().split(/\s+/),
  );
}

/** `releases=8-25` or `releases=8,17,25`, expanded and deduplicated. */
function releasesOf(text: string): number[] {
  const spec = directives(text)
    .find((directive) => directive.startsWith("releases="))
    ?.slice("releases=".length);
  if (spec === undefined) return [];

  const releases = new Set<number>();
  for (const part of spec.split(",")) {
    const [from, to] = part.split("-").map(Number);
    if (!Number.isInteger(from) || (to !== undefined && !Number.isInteger(to))) {
      throw new Error(`unparseable releases directive: ${spec}`);
    }
    for (let release = from!; release <= (to ?? from!); release++) releases.add(release);
  }
  return [...releases].sort((a, b) => a - b);
}

/** Where a case's output lands, repo-relative, one tree per compiler and mode. */
export const outDir = (kind: "madura" | "javac" | "check", testCase: Case): string =>
  `${OUT_DIR}/${kind}/${testCase.id}`;

/** `madura compile --release 21 -d … Source.java` */
export const compileArgs = (testCase: Case): string[] => [
  "compile",
  ...testCase.flags,
  "-d",
  outDir("madura", testCase),
  testCase.source,
];

/** The same compile, minus codegen. */
export const checkArgs = (testCase: Case): string[] => [
  "check",
  ...testCase.flags,
  "-d",
  outDir("check", testCase),
  testCase.source,
];

/** What vanilla `javac` is handed, so the only variable is the compiler. */
export const vanillaArgs = (testCase: Case): string[] => [
  ...testCase.flags,
  "-d",
  outDir("javac", testCase),
  testCase.source,
];

export type Run = { exitCode: number; stdout: string; stderr: string };

/** Run a binary from the repo root and collect everything it produced. */
export async function run(bin: string, args: string[]): Promise<Run> {
  const proc = Bun.spawn([bin, ...args], { cwd: REPO, stdout: "pipe", stderr: "pipe" });
  const [stdout, stderr] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
  ]);
  return { exitCode: await proc.exited, stdout, stderr };
}

/**
 * A run's status as a string, carrying its output when it failed. Asserting on
 * this rather than on a bare exit code puts the compiler's diagnostics in the
 * failure diff, where they are useful.
 */
export function status({ exitCode, stdout, stderr }: Run): string {
  return exitCode === 0 ? "exit 0" : `exit ${exitCode}\n${stderr}${stdout}`.trimEnd();
}

/** Class files under a repo-relative directory, relative to it and sorted. */
export function produced(dir: string): string[] {
  const root = join(REPO, dir);
  if (!existsSync(root)) return [];
  return [...new Glob("**/*.class").scanSync({ cwd: root })].sort();
}

/**
 * Every way madura's output for a case differs from vanilla javac's: class
 * files only one of them wrote, and class files whose bytes disagree. An empty
 * list is the whole assertion.
 */
export function bytecodeDifferences(testCase: Case): string[] {
  const mine = outDir("madura", testCase);
  const theirs = outDir("javac", testCase);
  const names = [...new Set([...produced(mine), ...produced(theirs)])].sort();

  return names.flatMap((name) => {
    const a = readIfPresent(join(REPO, mine, name));
    const b = readIfPresent(join(REPO, theirs, name));
    if (a === null) return `${name}: only javac produced it`;
    if (b === null) return `${name}: only madura produced it`;
    if (a.equals(b)) return [];
    return `${name}: differs at byte ${firstDifference(a, b)} (madura ${a.length}B, javac ${b.length}B)`;
  });
}

function readIfPresent(path: string): Buffer | null {
  return existsSync(path) ? readFileSync(path) : null;
}

function firstDifference(a: Buffer, b: Buffer): number {
  const shared = Math.min(a.length, b.length);
  for (let index = 0; index < shared; index++) {
    if (a[index] !== b[index]) return index;
  }
  return shared;
}

/**
 * Run `task` over `items`, a few at a time. Vanilla `javac` spawns a whole JVM
 * and costs ~0.5s each; the suite has dozens of them to do, and they are
 * independent.
 */
export async function inParallel<T, R>(items: T[], task: (item: T) => Promise<R>): Promise<R[]> {
  const results = new Array<R>(items.length);
  let next = 0;
  const worker = async () => {
    for (let index = next++; index < items.length; index = next++) {
      results[index] = await task(items[index]!);
    }
  };
  const width = Math.max(1, Math.min(navigator.hardwareConcurrency, items.length));
  await Promise.all(Array.from({ length: width }, worker));
  return results;
}
