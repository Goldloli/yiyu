#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = path.resolve(import.meta.dirname, "..");
const version = JSON.parse(
  fs.readFileSync(path.join(root, "package.json"), "utf8")
).version;
const expected = {
  macOS: `Yiyu_${version}_aarch64.dmg`,
  windows: `Yiyu_${version}_x64-setup.exe`,
  checksums: "SHA256SUMS.txt"
};
const assetsDir = process.argv[2] ? path.resolve(process.argv[2]) : null;

if (!assetsDir) {
  for (const [platform, name] of Object.entries(expected)) {
    console.log(`${platform}: ${name}`);
  }
  console.log("Release asset dry-run passed");
  process.exit(0);
}

if (!fs.existsSync(assetsDir) || !fs.statSync(assetsDir).isDirectory()) {
  throw new Error(`Release asset directory does not exist: ${assetsDir}`);
}

const files = fs
  .readdirSync(assetsDir, { withFileTypes: true })
  .filter((entry) => entry.isFile())
  .map((entry) => entry.name);

for (const [platform, name] of Object.entries(expected)) {
  const matches = files.filter((file) => file === name);
  if (matches.length !== 1) {
    throw new Error(
      `${platform} asset must appear exactly once: expected ${name}, found ${matches.length}`
    );
  }
}

const installerFiles = files.filter(
  (file) => file.endsWith(".dmg") || file.endsWith(".exe")
);
if (
  installerFiles.length !== 2 ||
  !installerFiles.includes(expected.macOS) ||
  !installerFiles.includes(expected.windows)
) {
  throw new Error(
    `Unexpected installer assets: ${installerFiles.join(", ") || "(none)"}`
  );
}

console.log(
  `Release assets validated for ${version}: ${expected.macOS}, ${expected.windows}, ${expected.checksums}`
);
