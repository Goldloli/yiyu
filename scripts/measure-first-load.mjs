#!/usr/bin/env node
import fs from "node:fs";
import path from "node:path";
import { gzipSync } from "node:zlib";

const dist = path.resolve(process.argv[2] ?? "web/dist");
const manifestPath = path.join(dist, ".vite", "manifest.json");
const budgetBytes = Number(process.env.YIYU_FIRST_LOAD_BUDGET ?? 175_000);

if (!fs.existsSync(manifestPath)) {
  console.error(`Missing Vite manifest: ${manifestPath}`);
  process.exit(1);
}

const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
const entry = Object.values(manifest).find((item) => item.isEntry);
if (!entry) {
  console.error("No entry chunk found in manifest");
  process.exit(1);
}

const chunks = new Set();
const css = new Set();
function visit(key) {
  if (chunks.has(key)) return;
  chunks.add(key);
  const item = manifest[key];
  if (!item) return;
  if (item.file) {
    if (item.file.endsWith(".css")) css.add(item.file);
    if (item.file.endsWith(".js")) chunks.add(`${key}::js`);
  }
  for (const imported of item.imports ?? []) visit(imported);
}

visit(entry.src ?? Object.keys(manifest).find((key) => manifest[key] === entry));

const files = [
  ...[...chunks]
    .filter((key) => key.endsWith("::js"))
    .map((key) => manifest[key.slice(0, -4)].file),
  ...(entry.css ?? []),
  ...[...css]
];

const unique = [...new Set(files)].filter(Boolean);
let raw = 0;
let gzip = 0;
for (const file of unique) {
  const bytes = fs.readFileSync(path.join(dist, file));
  raw += bytes.length;
  gzip += gzipSync(bytes, { level: 9 }).length;
  console.log(`${file}\t${bytes.length}\t${gzipSync(bytes, { level: 9 }).length}`);
}

console.log(`first-load raw=${raw} gzip=${gzip} budget=${budgetBytes}`);
if (gzip > budgetBytes) {
  console.error(`First-load gzip budget exceeded by ${gzip - budgetBytes} bytes`);
  process.exit(1);
}
