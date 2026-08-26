/**
 * Packaging contracts. `scripts/homebrew-formula.sh` runs exactly once per
 * release, inside the job that publishes the tap — the worst possible place to
 * discover a broken heredoc — so its output is checked here instead: valid
 * Ruby, carrying the version, URL and checksum it was handed.
 *
 * Deliberately does not import `./harness.ts`: that module refuses to load
 * without a built binary, and none of this needs one.
 */
import { expect, test } from "bun:test";
import { resolve } from "node:path";

const REPO = resolve(import.meta.dir, "..");
const RENDER = resolve(REPO, "scripts/homebrew-formula.sh");

const VERSION = "9.8.7";
const SHA = "a".repeat(64);

async function render(args: string[]): Promise<{ exitCode: number; stdout: string; stderr: string }> {
  const proc = Bun.spawn([RENDER, ...args], { cwd: REPO, stdout: "pipe", stderr: "pipe" });
  const [stdout, stderr] = await Promise.all([
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
  ]);
  return { exitCode: await proc.exited, stdout, stderr };
}

test("the formula describes the release it was rendered for", async () => {
  const out = await render([VERSION, SHA]);
  expect(out.exitCode).toBe(0);
  expect(out.stdout).toContain(`sha256 "${SHA}"`);
  expect(out.stdout).toContain(
    `url "https://github.com/elide-dev/madura/releases/download/v${VERSION}/madura-${VERSION}-cosmo-universal.zip"`,
  );
  // The URL is the *only* statement of the version: Homebrew scans it from
  // there, and `brew audit --strict` rejects a `version` line that repeats what
  // the URL already says. Re-adding one would pass style and fail audit on the
  // tap, which is where nobody is watching.
  expect(out.stdout).not.toContain("version \"");
});

test("rendering is silent, so nothing in the formula was shell-expanded", async () => {
  // The heredoc that emits the formula is unquoted — it has to be, to
  // interpolate the version, URL and checksum — so bash expands anything inside
  // it that looks expandable. A backtick in a *comment* within that heredoc is
  // run as a command and its output substituted, silently corrupting the
  // formula: exit status stays 0, the result is still valid Ruby, and the only
  // evidence is `command not found` on stderr. Which is why this asserts on
  // stderr rather than on any particular line of output.
  const out = await render([VERSION, SHA]);
  expect(out.exitCode).toBe(0);
  expect(out.stderr).toBe("");
});

test("the formula keeps the binary beside its platform metadata", async () => {
  // The distribution only works as a unit: madura resolves `lib/modules`
  // relative to the real path of its executable. A formula that did
  // `bin.install "madura.com"` would strand the binary away from `lib/`, so the
  // libexec-plus-symlink shape is the contract, not a style choice.
  const out = await render([VERSION, SHA]);
  expect(out.stdout).toContain("libexec.install");
  expect(out.stdout).toContain('bin.install_symlink libexec/"madura.com" => "madura"');
});

test("the formula's license covers every license the binary carries", async () => {
  // Homebrew's `license` field describes the installed files, and the installed
  // files are a compound artifact: OpenJDK compiled ahead of time, with the
  // Kotlin stdlib and Cosmopolitan Libc linked in. Listing only the most
  // restrictive term would make the tap disagree with NOTICE.md, so every term
  // is pinned here — the failure mode is silent metadata rot, not a broken
  // install.
  const out = await render([VERSION, SHA]);
  for (const spdx of ["0BSD", "Apache-2.0", "ISC"]) {
    expect(out.stdout).toContain(`"${spdx}"`);
  }
  expect(out.stdout).toContain('{ "GPL-2.0-only" => { with: "Classpath-exception-2.0" } }');
  // `all_of`, not `any_of`: redistribution must satisfy all four, and the two
  // read alike at a glance while meaning opposite things.
  expect(out.stdout).toContain("license all_of: [");
});

test("the rendered formula is valid Ruby", async () => {
  const formula = (await render([VERSION, SHA])).stdout;
  const check = Bun.spawn(["ruby", "-c"], { stdin: "pipe", stdout: "pipe", stderr: "pipe" });
  check.stdin.write(formula);
  await check.stdin.end();
  const [code, stderr] = await Promise.all([check.exited, new Response(check.stderr).text()]);
  expect(`${code} ${stderr}`).toBe("0 ");
});

test("a malformed checksum is rejected rather than published", async () => {
  // The digest reaches this script from `awk` over SHA256SUMS; an empty or
  // truncated match must fail the release job, not ship a formula Homebrew
  // will reject at install time for everyone.
  const out = await render([VERSION, "not-a-digest"]);
  expect(out.exitCode).not.toBe(0);
  expect(out.stderr).toContain("not a sha256 digest");
  expect(out.stdout).toBe("");
});

test("both arguments are required", async () => {
  const out = await render([VERSION]);
  expect(out.exitCode).not.toBe(0);
  expect(out.stdout).toBe("");
});
