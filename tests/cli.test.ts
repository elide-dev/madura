/**
 * CLI-level behaviors the corpus-driven smoke suite does not exercise:
 * drop-in (subcommand-less) passthrough, the `--version` handoff to javac, and
 * the platform-metadata contract — the shipped binary is hermetic (resolves
 * `<exe>/../<arch>/lib/modules`), `$JAVA_HOME` is a fallback, and an explicit
 * `--java-home` overrides both.
 */
import { expect, test } from "bun:test";
import { existsSync, mkdirSync, rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import { MADURA, REPO, run, type Run } from "./harness.ts";

const HELLO = 'public class Hello { public static void main(String[] a) { System.out.println("hi"); } }';

function workdir(name: string): string {
  const dir = join(REPO, "target/cli", name);
  rmSync(dir, { recursive: true, force: true });
  mkdirSync(dir, { recursive: true });
  return dir;
}

/** Spawn the binary from the repo root with an explicit environment. */
async function runWith(args: string[], env: Record<string, string | undefined>): Promise<Run> {
  const proc = Bun.spawn([MADURA, ...args], { cwd: REPO, env, stdout: "pipe", stderr: "pipe" });
  const [stdout, stderr] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
  ]);
  return { exitCode: await proc.exited, stdout, stderr };
}

/** The current environment with JAVA_HOME removed. */
function withoutJavaHome(): Record<string, string | undefined> {
  const { JAVA_HOME, ...rest } = process.env;
  void JAVA_HOME;
  return rest;
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

test("compiles hermetically with JAVA_HOME unset", async () => {
  // Proves the shipped `<arch>/lib/modules` is found binary-relative, with no
  // JDK in the environment at all.
  const dir = workdir("hermetic");
  writeFileSync(join(dir, "Hello.java"), HELLO);
  const out = await runWith([join(dir, "Hello.java"), "-d", join(dir, "out")], withoutJavaHome());
  expect(out.exitCode).toBe(0);
  expect(existsSync(join(dir, "out/Hello.class"))).toBe(true);
});

test("--java-home overrides and rejects a directory without lib/modules", async () => {
  // The dist would otherwise resolve metadata binary-relative, so a bad
  // `--java-home` failing proves the flag takes precedence over both fallbacks.
  const dir = workdir("forced-bad");
  writeFileSync(join(dir, "Hello.java"), HELLO);
  const out = await run(MADURA, [
    "--java-home",
    join(dir, "nope"),
    join(dir, "Hello.java"),
    "-d",
    join(dir, "out"),
  ]);
  expect(out.exitCode).toBe(2);
  expect(out.stderr).toContain("lib/modules");
});

test("--java-home accepts an explicit JDK", async () => {
  const jdk = process.env.JAVA_HOME;
  if (!jdk) return; // nothing valid to point at
  const dir = workdir("forced-ok");
  writeFileSync(join(dir, "Hello.java"), HELLO);
  const out = await runWith(
    ["--java-home", jdk, join(dir, "Hello.java"), "-d", join(dir, "out")],
    withoutJavaHome(),
  );
  expect(out.exitCode).toBe(0);
  expect(existsSync(join(dir, "out/Hello.class"))).toBe(true);
});
