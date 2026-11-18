#!/usr/bin/env node
import { spawn } from "node:child_process";
import fs from "node:fs/promises";
import path from "node:path";
import { chromium } from "playwright";

const root = path.resolve(import.meta.dirname, "../..");
const artifacts = path.join(root, "web", "qa", "artifacts");
const base = process.env.YIYU_QA_BASE ?? "http://127.0.0.1:1420";
await fs.mkdir(artifacts, { recursive: true });

let server;
try {
  await fetch(base, { signal: AbortSignal.timeout(700) });
} catch {
  server = spawn("pnpm", ["dev:web"], {
    cwd: root,
    stdio: "ignore",
    detached: process.platform !== "win32"
  });
  for (let attempt = 0; attempt < 60; attempt += 1) {
    try {
      await fetch(base, { signal: AbortSignal.timeout(500) });
      break;
    } catch {
      await new Promise((resolve) => setTimeout(resolve, 250));
    }
  }
}

const browser = await chromium.launch({ headless: true });
const failures = [];

async function capture(name, route, viewport, locale, prepare, contextOptions = {}) {
  const context = await browser.newContext({
    viewport,
    deviceScaleFactor: 1,
    locale,
    ...contextOptions
  });
  const page = await context.newPage();
  page.on("pageerror", (error) => failures.push(`${name}: ${error}`));
  page.on("console", (message) => {
    if (message.type() === "error") failures.push(`${name}: ${message.text()}`);
  });
  await page.goto(`${base}/${route}`);
  await page.waitForSelector("[data-window]");
  await page.waitForTimeout(700);
  if (prepare) await prepare(page);
  await page.screenshot({
    path: path.join(artifacts, `${name}.png`),
    fullPage: true,
    animations: "disabled"
  });
  await context.close();
}

await capture("main-zh-light", "#/main", { width: 1200, height: 800 }, "zh-CN");
await capture("main-default-size", "#/main", { width: 980, height: 660 }, "zh-CN");
await capture("main-min-size", "#/main", { width: 760, height: 500 }, "zh-CN");
await capture("main-en-dark", "#/main?theme=dark", { width: 1200, height: 800 }, "en-US", async (page) => {
  await page.evaluate(() => {
    document.documentElement.dataset.theme = "dark";
  });
});
await capture("main-reduced-motion", "#/main", { width: 1200, height: 800 }, "zh-CN", async (page) => {
  if ((await page.locator("html").getAttribute("data-motion")) !== "reduced") {
    failures.push("main-reduced-motion: motion preference was not applied");
  }
}, { reducedMotion: "reduce" });
await capture("main-translated", "#/main", { width: 1200, height: 800 }, "zh-CN", async (page) => {
  await page.locator("textarea").fill(
    "Lightness is not the absence of features, but the absence of friction."
  );
  await page.getByRole("button", { name: "翻译", exact: true }).click();
  await page.waitForFunction(() =>
    document.body.innerText.includes("轻盈不是功能的缺席")
  );
  await page.locator(".stream-caret").waitFor({ state: "hidden" });
});
for (const theme of ["light", "dark"]) {
  await capture(`main-background-${theme}`, "#/main", { width: 1488, height: 1058 }, "zh-CN", async (page) => {
    await page.goto(`${base}/#/settings`);
    await page.getByRole("button", { name: "外观", exact: true }).click();
    await page.getByRole("button", { name: theme === "dark" ? "深色" : "浅色", exact: true }).click();
    await page.locator('input[type="file"]').setInputFiles(path.join(root, "web/qa/fixtures/glass-landscape.png"));
    await page.getByRole("button", { name: "应用", exact: true }).click();
    await page.getByRole("button", { name: "移除", exact: true }).waitFor();
    await page.goto(`${base}/#/main`);
    await page.locator(".wallpaper").waitFor();
    await page.locator(".history-item").first().click();
    await page.mouse.move(1478, 1048);
  });
}
await capture("settings", "#/settings", { width: 860, height: 640 }, "zh-CN");
await capture("settings-appearance", "#/settings", { width: 860, height: 640 }, "zh-CN", async (page) => {
  await page.getByRole("button", { name: "外观", exact: true }).click();
});
await capture("settings-shortcuts", "#/settings", { width: 860, height: 640 }, "zh-CN", async (page) => {
  await page.getByRole("button", { name: "快捷键", exact: true }).click();
});
await capture("settings-general-dark", "#/settings", { width: 860, height: 640 }, "zh-CN", async (page) => {
  await page.getByRole("button", { name: "通用", exact: true }).click();
}, { colorScheme: "dark" });
await capture("settings-min", "#/settings", { width: 720, height: 520 }, "zh-CN");
await capture("onboarding", "#/onboarding", { width: 780, height: 640 }, "zh-CN");
await capture("onboarding-dark", "#/onboarding", { width: 780, height: 640 }, "zh-CN", undefined, { colorScheme: "dark" });
await capture("onboarding-min", "#/onboarding", { width: 700, height: 560 }, "en-US");
await capture("components", "#/gallery", { width: 900, height: 700 }, "zh-CN");
await capture("dialog-clear-history", "#/main", { width: 980, height: 660 }, "zh-CN", async (page) => {
  await page.getByRole("button", { name: "清空历史" }).click();
  await page.getByRole("dialog", { name: "清空所有翻译记录？" }).waitFor();
});
await capture("popup-permission", "#/popup?scenario=popup-permission", { width: 382, height: 240 }, "zh-CN", async (page) => {
  await page.getByText("需要辅助功能权限才能读取选区。").waitFor();
});
await capture("popup", "#/popup", { width: 382, height: 320 }, "zh-CN", async (page) => {
  await page.getByRole("button", { name: /剪贴板翻译/ }).click();
  await page.waitForFunction(() =>
    document.body.innerText.includes("消失在阅读体验里")
  );
  await page.getByRole("button", { name: "重新翻译", exact: true }).waitFor();
});
await capture("popup-dark", "#/popup", { width: 382, height: 320 }, "zh-CN", async (page) => {
  await page.getByRole("button", { name: /剪贴板翻译/ }).click();
  await page.getByRole("button", { name: "重新翻译", exact: true }).waitFor();
}, { colorScheme: "dark" });
await capture("popup-min", "#/popup", { width: 320, height: 520 }, "zh-CN", async (page) => {
  await page.getByRole("button", { name: /剪贴板翻译/ }).click();
  await page.getByRole("button", { name: "重新翻译", exact: true }).waitFor();
});
await capture("popup-max", "#/popup", { width: 560, height: 680 }, "zh-CN", async (page) => {
  await page.getByRole("button", { name: /剪贴板翻译/ }).click();
  await page.getByRole("button", { name: "重新翻译", exact: true }).waitFor();
});
await capture("error-rate-limit", "#/main?scenario=rate-limit", { width: 1200, height: 800 }, "zh-CN", async (page) => {
  await page.locator("textarea").fill("Please translate this sentence.");
  await page.getByRole("button", { name: "翻译", exact: true }).click();
  await page.waitForFunction(() =>
    document.body.innerText.includes("限流") ||
    document.body.innerText.includes("rate limit")
  );
});

await browser.close();
if (server) {
  try {
    if (process.platform === "win32") server.kill("SIGTERM");
    else process.kill(-server.pid, "SIGTERM");
  } catch (error) {
    if (error.code !== "ESRCH") throw error;
  }
}

if (failures.length) {
  console.error(failures.join("\n"));
  process.exit(1);
}
console.log(`Screenshots written to ${artifacts}`);
