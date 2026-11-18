#!/usr/bin/env node
import { execFileSync } from "node:child_process";

const pid = Number(process.argv[2]);
const samples = Number(process.argv[3] ?? 5);
if (!Number.isInteger(pid) || pid <= 0) {
  console.error("Usage: node scripts/perf-sample.mjs <pid> [samples]");
  process.exit(1);
}

const rows = [];
for (let index = 0; index < samples; index += 1) {
  try {
    const output = execFileSync("ps", ["-o", "rss=,pcpu=", "-p", String(pid)], {
      encoding: "utf8"
    }).trim();
    const [rssKb, cpu] = output.split(/\s+/).map(Number);
    rows.push({ at: Date.now(), rssMb: rssKb / 1024, cpuPercent: cpu });
    console.log(`sample ${index + 1}: rss=${rows.at(-1).rssMb.toFixed(1)}MB cpu=${cpu}%`);
  } catch {
    console.error(`Cannot sample pid ${pid}`);
    process.exit(1);
  }
  if (index < samples - 1) {
    Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 1000);
  }
}

console.log(JSON.stringify({ pid, samples: rows }, null, 2));
