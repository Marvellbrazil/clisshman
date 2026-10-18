"use strict";

const test = require("node:test");
const assert = require("node:assert");
const { spawnSync } = require("node:child_process");
const fs = require("node:fs");
const os = require("node:os");
const path = require("node:path");

const launcherSource = path.join(__dirname, "..", "bin", "clisshman");
const binaryName = process.platform === "win32" ? "clisshman.exe" : "clisshman";

test("both command names use the same npm launcher", () => {
  const manifest = require("../package.json");
  assert.strictEqual(manifest.bin.clisshman, "./bin/clisshman");
  assert.strictEqual(manifest.bin.csm, manifest.bin.clisshman);
});

function fixture(withBinary) {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "clisshman-launcher-"));
  fs.mkdirSync(path.join(root, "bin"));
  fs.copyFileSync(launcherSource, path.join(root, "bin", "clisshman"));
  if (withBinary) {
    const release = path.join(root, "target", "release");
    fs.mkdirSync(release, { recursive: true });
    fs.copyFileSync(process.execPath, path.join(release, binaryName));
  }
  return root;
}

function launch(root, args, prelude = "") {
  const launcher = path.join(root, "bin", "clisshman");
  const script = `${prelude}process.argv.splice(1, 0, ${JSON.stringify(launcher)});require(${JSON.stringify(launcher)});`;
  return spawnSync(process.execPath, ["-e", script, "--", ...args], {
    encoding: "utf8",
  });
}

test("propagates the native binary exit status", (t) => {
  const root = fixture(true);
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const result = launch(root, ["-e", "process.exit(7)"]);
  assert.strictEqual(result.status, 7, result.stderr);
});

test("passes arguments and output through unchanged", (t) => {
  const root = fixture(true);
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const result = launch(root, [
    "-e",
    "console.log(process.argv.slice(1).join('|'))",
    "a b",
    "--json",
  ]);
  assert.strictEqual(result.status, 0, result.stderr);
  assert.strictEqual(result.stdout.trim(), "a b|--json");
});

test("fails with a clear message when the binary is missing", (t) => {
  const root = fixture(false);
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const result = launch(root, ["--version"]);
  assert.strictEqual(result.status, 1);
  assert.match(result.stderr, /binary was not found/);
});

test("rejects unsupported platforms instead of reporting success", (t) => {
  const root = fixture(false);
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const prelude =
    "Object.defineProperty(process,'platform',{value:'aix'});Object.defineProperty(process,'arch',{value:'ppc64'});";
  const result = launch(root, ["--version"], prelude);
  assert.strictEqual(result.status, 1);
  assert.match(result.stderr, /no prebuilt binary is available for aix-ppc64/);
});
