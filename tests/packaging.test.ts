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
  expect(out.stdout).toContain(`version "${VERSION}"`);
  expect(out.stdout).toContain(`sha256 "${SHA}"`);
  expect(out.stdout).toContain(
    `url "https://github.com/elide-dev/madura/releases/download/v${VERSION}/madura-${VERSION}-cosmo-universal.zip"`,
  );
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
