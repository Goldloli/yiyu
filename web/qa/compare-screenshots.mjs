#!/usr/bin/env node
import fs from "node:fs/promises";
import path from "node:path";
import pixelmatch from "pixelmatch";
import { PNG } from "pngjs";

const root = path.resolve(import.meta.dirname, "../..");
const artifacts = path.join(root, "web", "qa", "artifacts");
const baselines = path.join(root, "web", "qa", "baselines");
const update = process.argv.includes("--update");
const maxDiffRatio = Number(process.env.YIYU_SCREENSHOT_DIFF_RATIO ?? 0.01);

await fs.mkdir(artifacts, { recursive: true });
await fs.mkdir(baselines, { recursive: true });

const currentFiles = (await fs.readdir(artifacts))
  .filter((file) => file.endsWith(".png") && !file.startsWith("diff-"))
  .sort();
if (currentFiles.length === 0) {
  throw new Error("No screenshots found. Run pnpm qa:screenshots first.");
}

if (update) {
  for (const file of currentFiles) {
    await fs.copyFile(path.join(artifacts, file), path.join(baselines, file));
  }
  console.log(`Updated ${currentFiles.length} screenshot baselines`);
  process.exit(0);
}

const baselineFiles = (await fs.readdir(baselines))
  .filter((file) => file.endsWith(".png"))
  .sort();
if (baselineFiles.length !== currentFiles.length) {
  throw new Error(
    `Screenshot count changed: baseline=${baselineFiles.length}, current=${currentFiles.length}`
  );
}

const failures = [];
for (const file of baselineFiles) {
  const baseline = PNG.sync.read(
    await fs.readFile(path.join(baselines, file))
  );
  const current = PNG.sync.read(
    await fs.readFile(path.join(artifacts, file))
  );
  if (baseline.width !== current.width || baseline.height !== current.height) {
    failures.push(
      `${file}: dimensions ${current.width}x${current.height}, expected ${baseline.width}x${baseline.height}`
    );
    continue;
  }
  const diff = new PNG({ width: current.width, height: current.height });
  const pixels = pixelmatch(
    baseline.data,
    current.data,
    diff.data,
    current.width,
    current.height,
    { threshold: 0.12 }
  );
  const ratio = pixels / (current.width * current.height);
  if (ratio > maxDiffRatio) {
    await fs.writeFile(
      path.join(artifacts, `diff-${file}`),
      PNG.sync.write(diff)
    );
    failures.push(
      `${file}: ${(ratio * 100).toFixed(2)}% pixels differ (limit ${(maxDiffRatio * 100).toFixed(2)}%)`
    );
  }
}

if (failures.length > 0) {
  throw new Error(`Screenshot regression:\n${failures.join("\n")}`);
}
console.log(`Screenshot baselines passed (${baselineFiles.length} images)`);
