/**
 * CLI-level behaviors that the corpus-driven smoke suite does not exercise:
 * drop-in (subcommand-less) passthrough, the `--version` handoff to javac, and
 * the environment contract — madura resolves its platform metadata from
 * `$JAVA_HOME` and fails loudly when it is absent.
 */
import { expect, test } from "bun:test";
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { MADURA, REPO, run } from "./harness.ts";

const HELLO = 'public class Hello { public static void main(String[] a) { System.out.println("hi"); } }';

function workdir(name: string): string {
  const dir = join(REPO, "target/cli", name);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  return dir;
}

test("--version prints javac's version", async () => {
  const out = await run(MADURA, ["--version"]);
  expect(out.exitCode).toBe(0);
  expect(`${out.stdout}${out.stderr}`).toContain("javac");
});

test("passthrough (no subcommand) compiles like javac", async () => {
  const dir = workdir("passthrough");
  writeFileSync(join(dir, "Hello.java"), HELLO);
  const out = await run(MADURA, [join(dir, "Hello.java"), "-d", join(dir, "out")]);
  expect(out.exitCode).toBe(0);
  expect(existsSync(join(dir, "out/Hello.class"))).toBe(true);
});

test("missing JAVA_HOME is an environment error (exit 2)", async () => {
  const dir = workdir("no-java-home");
  writeFileSync(join(dir, "Hello.java"), HELLO);
  const { JAVA_HOME, ...env } = process.env;
  void JAVA_HOME;
  const proc = Bun.spawn([MADURA, "Hello.java", "-d", join(dir, "out")], {
    cwd: REPO,
    env,
    stdout: "pipe",
    stderr: "pipe",
  });
  const stderr = await new Response(proc.stderr).text();
  expect(await proc.exited).toBe(2);
  expect(stderr).toContain("JAVA_HOME");
});
