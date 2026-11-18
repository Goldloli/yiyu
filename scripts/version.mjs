#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";

const root = path.resolve(import.meta.dirname, "..");
const paths = {
  package: path.join(root, "package.json"),
  tauri: path.join(root, "src-tauri", "tauri.conf.json"),
  cargo: path.join(root, "Cargo.toml"),
  web: path.join(root, "web", "src", "lib", "constants.ts")
};

function readJson(file) {
  return JSON.parse(fs.readFileSync(file, "utf8"));
}

function cargoVersion() {
  const text = fs.readFileSync(paths.cargo, "utf8");
  const workspacePackage = text.match(/\[workspace\.package\]([\s\S]*?)(?:\n\[|$)/);
  if (!workspacePackage) throw new Error("missing [workspace.package] in Cargo.toml");
  const match = workspacePackage[1].match(/^version\s*=\s*"([^"]+)"/m);
  if (!match) throw new Error("missing workspace package version in Cargo.toml");
  return match[1];
}

function webVersion() {
  const text = fs.readFileSync(paths.web, "utf8");
  const match = text.match(/APP_VERSION\s*=\s*"([^"]+)"/);
  if (!match) throw new Error("missing APP_VERSION in web/src/lib/constants.ts");
  return match[1];
}

function currentVersions() {
  return {
    package: readJson(paths.package).version,
    tauri: readJson(paths.tauri).version,
    cargo: cargoVersion(),
    web: webVersion()
  };
}

function nextVersion(current, action) {
  if (!["major", "minor", "patch"].includes(action)) return action;
  const [major, minor, patch] = current.split(".").map(Number);
  if (![major, minor, patch].every(Number.isInteger)) {
    throw new Error(`cannot bump non-semver version: ${current}`);
  }
  if (action === "major") return `${major + 1}.0.0`;
  if (action === "minor") return `${major}.${minor + 1}.0`;
  return `${major}.${minor}.${patch + 1}`;
}

function writeVersion(version) {
  const packageJson = readJson(paths.package);
  packageJson.version = version;
  fs.writeFileSync(paths.package, `${JSON.stringify(packageJson, null, 2)}\n`);

  const tauri = readJson(paths.tauri);
  tauri.version = version;
  fs.writeFileSync(paths.tauri, `${JSON.stringify(tauri, null, 2)}\n`);

  const cargo = fs.readFileSync(paths.cargo, "utf8").replace(
    /(\[workspace\.package\][\s\S]*?^version\s*=\s*")[^"]+("\s*$)/m,
    `$1${version}$2`
  );
  fs.writeFileSync(paths.cargo, cargo);

  const web = fs
    .readFileSync(paths.web, "utf8")
    .replace(/APP_VERSION\s*=\s*"[^"]+"/, `APP_VERSION = "${version}"`);
  fs.writeFileSync(paths.web, web);
}

function check() {
  const versions = currentVersions();
  if (new Set(Object.values(versions)).size !== 1) {
    for (const [file, version] of Object.entries(versions)) {
      console.error(`${file}: ${version}`);
    }
    process.exitCode = 1;
    return;
  }
  console.log(`Version consistent: ${versions.package}`);
}

const args = process.argv.slice(2);
if (args.length === 0) {
  check();
} else if (args[0] === "--bump" || args[0] === "--set") {
  const version = nextVersion(readJson(paths.package).version, args[1]);
  if (!/^\d+\.\d+\.\d+$/.test(version)) {
    throw new Error(`invalid semver: ${version}`);
  }
  writeVersion(version);
  check();
} else {
  throw new Error(`unknown arguments: ${args.join(" ")}`);
}
