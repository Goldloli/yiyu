#!/usr/bin/env node
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import path from "node:path";
import { chromium } from "playwright";
import { PNG } from "pngjs";

const root = path.resolve(import.meta.dirname, "../..");
const base = process.env.YIYU_QA_BASE ?? "http://127.0.0.1:1420";
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
const context = await browser.newContext({
  viewport: { width: 1200, height: 800 },
  locale: "zh-CN"
});
const page = await context.newPage();
await page.emulateMedia({ reducedMotion: "reduce" });
const errors = [];

function contrastRatio(rgbA, rgbB) {
  const luminance = (rgb) => rgb.map((value) => value / 255).map((value) => value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4)
    .reduce((sum, value, index) => sum + value * [0.2126, 0.7152, 0.0722][index], 0);
  const [a, b] = [luminance(rgbA), luminance(rgbB)].sort((x, y) => y - x);
  return (a + 0.05) / (b + 0.05);
}

function sampleBackground(kind) {
  const png = new PNG({ width: 120, height: 120 });
  for (let y = 0; y < png.height; y += 1) for (let x = 0; x < png.width; x += 1) {
    const value = kind === "light" || (kind === "contrast" && (Math.floor(x / 20) + Math.floor(y / 20)) % 2) ? 255 : 0;
    const offset = (y * png.width + x) * 4;
    png.data[offset] = value; png.data[offset + 1] = value; png.data[offset + 2] = value; png.data[offset + 3] = 255;
  }
  return PNG.sync.write(png);
}
page.on("pageerror", (error) => errors.push(String(error)));
page.on("console", (message) => {
  if (message.type() === "error") errors.push(message.text());
});

try {
  await page.goto(`${base}/#/main`);
  await page.waitForSelector("[data-window='main']");
  await page.getByRole("textbox", { name: "原文" }).click();
  assert.equal(await page.locator(".translation-input").evaluate((node) => getComputedStyle(node).outlineStyle), "none");
  assert.equal(await page.getByRole("button", { name: "收藏", exact: true }).count(), 0);
  assert.doesNotMatch(await page.locator(".workspace-toolbar").innerText(), /⌘ \+ Enter|Ctrl \+ Enter/);
  await page.getByRole("button", { name: "目标语言" }).click();
  await page.getByRole("option", { name: "自定义语言" }).click();
  const languageDialog = page.getByRole("dialog", { name: "自定义目标语言" });
  await languageDialog.getByRole("textbox", { name: "目标语言" }).fill("韩语");
  await languageDialog.getByRole("button", { name: "保存" }).click();
  assert.match(await page.getByRole("button", { name: "目标语言" }).innerText(), /韩语/);
  await page.getByRole("button", { name: "目标语言" }).click();
  await page.getByRole("option", { name: "简体中文" }).click();
  assert.equal(
    await page.locator("html").getAttribute("data-motion"),
    "reduced"
  );
  await page.locator("textarea").fill(
    "The best translation is the one that disappears into the reading experience."
  );
  await page.getByRole("button", { name: "翻译", exact: true }).click();
  await page.waitForFunction(() =>
    document.body.innerText.includes("阅读应该像思考一样自然")
  );
  assert.match(await page.locator("body").innerText(), /最近翻译|RECENT TRANSLATIONS/);
  await page.getByRole("textbox", { name: "搜索" }).fill("no-such-translation-834");
  await page.getByText("没有匹配的翻译记录").waitFor();
  await page.getByRole("textbox", { name: "搜索" }).fill("Lightness");
  await page.waitForFunction(
    () => document.querySelectorAll(".history-item").length === 1
  );
  const historyItem = page.locator(".history-item").first();
  await historyItem.click();
  assert.match(await page.locator("body").innerText(), /Lightness is not the absence/);
  await page.getByRole("button", { name: "重试", exact: true }).click();
  await page.waitForFunction(() =>
    document.body.innerText.includes("轻盈不是功能的缺席")
  );
  await page.getByRole("button", { name: "复制" }).click();
  await page
    .locator("[aria-live='polite']")
    .getByText("复制", { exact: true })
    .waitFor();
  await page.getByRole("textbox", { name: "搜索" }).fill("");
  await page.waitForFunction(
    () => document.querySelectorAll(".history-item").length >= 6
  );
  const beforeDelete = await page.locator(".history-item").count();
  const itemToDelete = page.locator(".history-item").first();
  await itemToDelete.hover();
  await itemToDelete.getByRole("button", { name: "删除" }).click();
  await page.waitForFunction(
    (count) => document.querySelectorAll(".history-item").length === count,
    beforeDelete - 1
  );
  await page.getByRole("button", { name: "清空历史" }).click();
  const clearDialog = page.getByRole("dialog", { name: "清空所有翻译记录？" });
  await clearDialog.getByRole("button", { name: "取消" }).click();
  assert.equal(await page.locator(".history-item").count(), beforeDelete - 1);
  await page.getByRole("button", { name: "清空历史" }).click();
  await clearDialog.getByRole("button", { name: "清空历史" }).click();
  await page.waitForFunction(
    () => document.querySelectorAll(".history-item").length === 0
  );
  assert.doesNotMatch(await page.locator("body").innerText(), /小さな道具ほど/);

  await page.goto(`${base}/#/main`);
  const longSource = "A longer passage for testing the translation workspace. ".repeat(150);
  await page.getByRole("textbox", { name: "原文" }).fill(longSource);
  await page.getByRole("button", { name: "翻译", exact: true }).click();
  await page.getByRole("button", { name: "停止", exact: true }).click();
  assert.equal(await page.getByRole("textbox", { name: "原文" }).inputValue(), longSource);
  assert.equal(await page.getByRole("button", { name: "翻译", exact: true }).isVisible(), true);

  await page.goto(`${base}/#/main?scenario=history-write-failure`);
  await page.waitForSelector("[data-window='main']");
  await page.locator("textarea").fill("Please translate this sentence.");
  await page.getByRole("button", { name: "翻译", exact: true }).click();
  await page.waitForFunction(() =>
    document.body.innerText.includes("历史未保存")
  );
  await page.getByRole("button", { name: "重试保存" }).click();
  await page.waitForFunction(() =>
    !document.body.innerText.includes("历史未保存")
  );

  await page.goto(`${base}/#/main?scenario=settings-recovery`);
  await page.getByText("设置文件无法读取，原文件已备份至 /demo/settings.invalid-backup.json").waitFor();

  await page.goto(`${base}/#/main?scenario=shortcut-conflict`);
  await page.getByText("快捷键注册失败，请在设置中调整快捷键。").waitFor();
  await page.getByRole("textbox", { name: "原文" }).waitFor({ state: "visible" });

  await page.goto(`${base}/#/settings`);
  await page.waitForSelector("[data-window='settings']");
  await page.getByRole("button", { name: "通用" }).click();
  assert.match(await page.locator("body").innerText(), /通用设置/);
  const autoStart = page.getByRole("switch", { name: "开机启动" });
  const silentStart = page.getByRole("switch", { name: "启动时隐藏主窗口" });
  await autoStart.waitFor({ state: "visible" });
  await page.waitForFunction(() => !document.querySelector('button[aria-label="开机启动"]').disabled);
  await autoStart.click();
  assert.equal(await autoStart.getAttribute("aria-checked"), "true");
  await silentStart.click();
  await page.waitForFunction(() => document.querySelector('button[aria-label="启动时隐藏主窗口"]').getAttribute("aria-checked") === "true");
  await page.reload();
  await page.getByRole("button", { name: "通用", exact: true }).click();
  await page.waitForFunction(() => document.querySelector('button[aria-label="开机启动"]').getAttribute("aria-checked") === "true");
  assert.equal(await silentStart.getAttribute("aria-checked"), "true");
  await autoStart.click();
  await silentStart.click();
  await page.getByRole("button", { name: "模型", exact: true }).click();
  const prompt = page.locator("textarea").first();
  await prompt.fill("Custom prompt");
  await page.getByRole("button", { name: "恢复默认" }).click();
  assert.equal(await prompt.inputValue(), "");
  await page.getByRole("button", { name: "网络", exact: true }).click();
  await page.getByRole("group", { name: "LLM 请求代理" }).getByRole("button", { name: "手动代理" }).click();
  const proxyDraft = page.getByPlaceholder("http://127.0.0.1:7890").first();
  await proxyDraft.fill("http://127.0.0.1:7891");
  await page.getByRole("button", { name: "外观", exact: true }).click();
  await page.getByRole("button", { name: "深色", exact: true }).click();
  await page.getByRole("button", { name: "网络", exact: true }).click();
  assert.equal(await proxyDraft.inputValue(), "http://127.0.0.1:7891");
  await page.getByRole("button", { name: "关于", exact: true }).click();
  assert.match(await page.locator("body").innerText(), /Yiyu|译羽/);
  await page.getByRole("button", { name: "更新", exact: true }).click();
  await page.getByRole("button", { name: "检查更新" }).click();
  await page.waitForFunction(() =>
    document.body.innerText.includes("发现新版本")
  );
  await page.getByRole("button", { name: "下载更新" }).click();
  await page.waitForFunction(() =>
    document.body.innerText.includes("正在下载")
  );
  await page.getByRole("button", { name: "取消" }).click();
  await page.waitForFunction(() =>
    document.body.innerText.includes("检查更新")
  );

  await page.goto(`${base}/#/settings?scenario=auth-failure`);
  await page.waitForSelector("[data-window='settings']");
  const displayName = page.getByLabel("显示名称");
  await displayName.fill("Draft provider");
  await page.getByRole("button", { name: "测试连接" }).click();
  await page.waitForFunction(() =>
    document.body.innerText.includes("Invalid API key")
  );
  assert.equal(await displayName.inputValue(), "Draft provider");
  await page.getByRole("button", { name: "外观", exact: true }).click();
  await page.getByRole("button", { name: "深色", exact: true }).click();
  await page.getByRole("button", { name: "模型", exact: true }).click();
  assert.equal(await displayName.inputValue(), "Draft provider");

  await page.getByRole("button", { name: "外观", exact: true }).click();
  await page.getByRole("button", { name: "浅色", exact: true }).click();
  await page.getByRole("button", { name: "模型", exact: true }).click();
  assert.equal(await displayName.inputValue(), "Draft provider");

  await page.getByRole("button", { name: "添加自定义服务" }).click();
  await page.waitForFunction(() =>
    document.body.innerText.includes("自定义服务")
  );
  await page.getByRole("button", { name: "删除自定义服务" }).click();
  const deleteDialog = page.getByRole("dialog", { name: "删除此服务及其 API Key？" });
  await deleteDialog.getByRole("button", { name: "取消" }).click();
  await page.getByRole("button", { name: "删除自定义服务" }).click();
  await deleteDialog.getByRole("button", { name: "删除", exact: true }).click();
  await page.waitForFunction(() =>
    !document.body.innerText.includes("自定义服务")
  );

  await page.setViewportSize({ width: 720, height: 520 });
  const providerList = page.locator(".provider-list");
  assert.equal(await providerList.evaluate((node) => node.scrollHeight > node.clientHeight), true);
  await page.getByRole("button", { name: /LM Studio/ }).click();
  await page.getByRole("heading", { name: "LM Studio" }).waitFor();
  await page.setViewportSize({ width: 1200, height: 800 });

  await page.goto(`${base}/#/settings?scenario=credential-cleanup-failure`);
  await page.waitForSelector("[data-window='settings']");
  await page.getByRole("button", { name: "添加自定义服务" }).click();
  await page.getByRole("button", { name: "删除自定义服务" }).click();
  await page.getByRole("dialog", { name: "删除此服务及其 API Key？" }).getByRole("button", { name: "删除", exact: true }).click();
  await page.getByText("已删除服务的 API Key 可能仍保存在系统凭据库中。").waitFor();
  await page.getByRole("button", { name: "重试清理" }).click();
  await page.getByText("已删除服务的 API Key 可能仍保存在系统凭据库中。").waitFor({ state: "hidden" });

  await page.goto(`${base}/#/settings?scenario=permission-refresh`);
  await page.getByRole("button", { name: "通用", exact: true }).click();
  await page.getByText("未授权", { exact: true }).waitFor();
  await page.getByRole("button", { name: "打开系统设置" }).click();
  await page.getByText(/移除旧的译羽条目/).waitFor();
  await page.evaluate(() => window.dispatchEvent(new Event("focus")));
  await page.getByText("已授权", { exact: true }).waitFor();
  assert.equal(await page.getByText(/移除旧的译羽条目/).count(), 0);

  await page.setViewportSize({ width: 382, height: 218 });
  await page.goto(`${base}/#/popup`);
  await page.waitForSelector("[data-window='popup']");
  await page.locator(".popup-shell .wallpaper").waitFor();
  await page.getByRole("button", { name: /剪贴板翻译/ }).click();
  await page.waitForFunction(() =>
    document.body.innerText.includes("消失在阅读体验")
  );
  await page.getByRole("button", { name: "重新翻译", exact: true }).waitFor();
  const sourceToggle = page.getByRole("button", { name: "收起原文" });
  assert.equal(await sourceToggle.getAttribute("aria-expanded"), "true");
  await sourceToggle.click();
  assert.equal(await page.getByRole("textbox", { name: "原文" }).count(), 0);
  await page.getByRole("button", { name: "查看原文" }).click();
  assert.equal(await page.getByRole("textbox", { name: "原文" }).isVisible(), true);
  assert.equal(await page.getByRole("button", { name: "复制", exact: true }).isVisible(), true);
  await page.getByRole("button", { name: "固定", exact: true }).click();
  assert.equal(await page.getByRole("button", { name: "取消固定", exact: true }).isVisible(), true);
  await page.getByRole("button", { name: "复制", exact: true }).click();
  await page.getByRole("button", { name: "重新翻译", exact: true }).click();
  await page.getByRole("button", { name: "重新翻译", exact: true }).waitFor();
  await page.keyboard.press("Escape");
  assert.equal(await page.locator("html").getAttribute("data-mock-hidden"), "1");

  await page.goto(`${base}/#/popup?scenario=popup-permission`);
  await page.getByText("需要辅助功能权限才能读取选区。").waitFor();
  await page.getByRole("button", { name: "打开设置" }).click();
  await page.getByRole("heading", { name: "通用设置" }).waitFor();
  await page.goto(`${base}/#/popup?scenario=popup-long`);
  await page.getByRole("button", { name: /剪贴板翻译/ }).click();
  await page.getByRole("button", { name: "重新翻译", exact: true }).waitFor();
  assert.equal(await page.locator(".popup-scroll").evaluate((node) => node.scrollHeight > node.clientHeight), true);
  assert.equal(await page.getByRole("button", { name: "复制", exact: true }).isVisible(), true);
  await page.setViewportSize({ width: 1200, height: 800 });

  for (const [scenario, errorText] of [
    ["auth-failure", "Invalid API key"],
    ["proxy-failure", "Proxy connection failed"],
    ["empty-result", "Translation returned an empty result"]
  ]) {
    await page.goto(`${base}/#/main?scenario=${scenario}`);
    await page.waitForSelector("[data-window='main']");
    const source = page.locator("textarea");
    await source.fill("Keep this source text.");
    await page.getByRole("button", { name: "翻译", exact: true }).click();
    await page.waitForFunction(
      (text) => document.body.innerText.includes(text),
      errorText
    );
    assert.equal(await source.inputValue(), "Keep this source text.");
    assert.match(await page.locator("body").innerText(), /简体中文/);
  }

  await page.goto(`${base}/#/onboarding?scenario=auth-failure`);
  await page.waitForSelector("[data-window='onboarding']");
  await page.getByRole("button", { name: "继续" }).click();
  assert.match(await page.locator("body").innerText(), /API Key|API key/);
  await page.getByPlaceholder("输入 API Key").fill("test-key");
  await page.getByRole("button", { name: "测试连接" }).click();
  await page.getByText("Invalid API key", { exact: true }).waitFor();
  await page.getByRole("button", { name: "返回", exact: true }).click();
  await page.getByRole("button", { name: "继续", exact: true }).click();
  assert.equal(await page.getByPlaceholder("输入 API Key").inputValue(), "test-key");

  await page.goto(`${base}/#/settings`);
  await page.getByRole("button", { name: "快捷键", exact: true }).click();
  assert.match(await page.locator("main").innerText(), /取词|剪贴板/);
  const shortcutRecorder = page.getByRole("button", { name: "划词翻译", exact: true });
  await shortcutRecorder.click();
  assert.equal(await shortcutRecorder.evaluate((element) => document.activeElement === element), true);
  await page.keyboard.press("Alt+Shift+K");
  assert.match(await shortcutRecorder.innerText(), /K/);
  await page.getByRole("button", { name: "保存", exact: true }).click();
  await page.getByRole("button", { name: "外观", exact: true }).click();
  await page.getByRole("button", { name: "海岸微风", exact: true }).click();
  assert.equal(await page.getByRole("button", { name: "海岸微风", exact: true }).getAttribute("aria-pressed"), "true");
  await page.reload();
  await page.getByRole("button", { name: "外观", exact: true }).click();
  assert.equal(await page.getByRole("button", { name: "海岸微风", exact: true }).getAttribute("aria-pressed"), "true");
  await page.getByRole("button", { name: "无背景", exact: true }).click();
  await page.goto(`${base}/#/main`);
  await page.waitForSelector("[data-window='main'] .workspace-panels");
  assert.equal(await page.locator(".wallpaper").count(), 0);
  await page.reload();
  await page.waitForSelector("[data-window='main'] .workspace-panels");
  assert.equal(await page.locator(".wallpaper").count(), 0);
  await page.goto(`${base}/#/settings`);
  await page.getByRole("button", { name: "外观", exact: true }).click();
  const backgroundInput = page.locator('input[type="file"]');
  await backgroundInput.setInputFiles(path.join(root, "web/qa/fixtures/glass-landscape.png"));
  await page.getByRole("img", { name: "背景预览" }).waitFor();
  await page.getByRole("button", { name: "取消", exact: true }).click();
  assert.equal(await page.getByRole("button", { name: "应用", exact: true }).count(), 0);
  await backgroundInput.setInputFiles(path.join(root, "web/qa/fixtures/glass-landscape.png"));
  await page.getByRole("button", { name: "应用", exact: true }).click();
  await page.getByRole("button", { name: "移除", exact: true }).waitFor();
  await page.goto(`${base}/#/main`);
  await page.locator(".wallpaper").waitFor();
  await page.reload();
  await page.locator(".wallpaper").waitFor();
  for (const theme of ["浅色", "深色"]) for (const kind of ["light", "dark", "contrast"]) {
    await page.goto(`${base}/#/settings`);
    await page.getByRole("button", { name: "外观", exact: true }).click();
    await page.getByRole("button", { name: theme, exact: true }).click();
    await page.locator('input[type="file"]').setInputFiles({ name: `${kind}.png`, mimeType: "image/png", buffer: sampleBackground(kind) });
    await page.getByRole("button", { name: "应用", exact: true }).click();
    await page.getByRole("button", { name: "移除", exact: true }).waitFor();
    await page.goto(`${base}/#/main`);
    await page.locator(".wallpaper").waitFor();
    const input = page.locator(".translation-input");
    const box = await input.boundingBox();
    const colors = await input.evaluate((node) => ({ text: getComputedStyle(node).color, placeholder: getComputedStyle(node, "::placeholder").color }));
    const shot = PNG.sync.read(await page.screenshot());
    const x = Math.floor(box.x + box.width / 2), y = Math.floor(box.y + box.height - 30);
    const pixel = (y * shot.width + x) * 4;
    const background = [...shot.data.subarray(pixel, pixel + 3)];
    for (const [name, value] of Object.entries(colors)) {
      const foreground = value.match(/[\d.]+/g).slice(0, 3).map(Number);
      const contrast = contrastRatio(foreground, background);
      assert.ok(contrast >= 4.5, `${theme}/${kind}/${name} contrast ${contrast.toFixed(2)} < 4.5`);
    }
  }
  await page.goto(`${base}/#/main?scenario=background-missing`);
  await page.waitForSelector(".workspace-panels");
  assert.equal(await page.locator(".wallpaper").count(), 0);
  await page.goto(`${base}/#/settings?scenario=background-missing`);
  await page.getByRole("button", { name: "外观", exact: true }).click();
  await page.getByText("已保存的图片不可用，请重新选择或移除。").waitFor();
  await page.getByRole("button", { name: "移除", exact: true }).click();
  await page.goto(`${base}/#/main`);
  await page.waitForSelector(".workspace-panels");
  assert.equal(await page.locator(".wallpaper").count(), 0);

  await page.goto(`${base}/#/gallery`);
  await page.waitForSelector("[data-window='gallery']");
  const iconBox = await page.getByRole("button", { name: "Search" }).boundingBox();
  assert.equal(Math.round(iconBox?.width ?? 0), 30);
  assert.equal(Math.round(iconBox?.height ?? 0), 30);
  assert.equal(await page.getByRole("button", { name: "Disabled", exact: true }).isDisabled(), true);
  assert.equal(await page.getByLabel("Invalid field").getAttribute("aria-invalid"), "true");
  await page.getByRole("button", { name: "Search" }).focus();
  await page.waitForFunction(() => getComputedStyle(document.querySelector(".tooltip-anchor"), "::after").opacity === "1");
  const languageMenu = page.getByRole("button", { name: "Language", exact: true });
  await languageMenu.focus();
  await page.keyboard.press("ArrowDown");
  assert.equal(await languageMenu.getAttribute("aria-expanded"), "true");
  await page.keyboard.press("Escape");
  assert.equal(await languageMenu.getAttribute("aria-expanded"), "false");
  assert.equal(await languageMenu.evaluate((node) => document.activeElement === node), true);

  await page.setViewportSize({ width: 760, height: 500 });
  await page.goto(`${base}/#/main`);
  await page.waitForSelector("[data-window='main']");
  const historyToggle = page.getByRole("button", { name: "显示或隐藏历史" });
  await historyToggle.click();
  assert.equal(await page.locator("aside.sidebar").isVisible(), true);
  await page.keyboard.press("Escape");
  assert.equal(await page.locator("aside.sidebar").isVisible(), false);
  assert.equal(
    await page.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth),
    true
  );
  const toolbarBoxes = await page.locator(".workspace-toolbar button:visible").evaluateAll((buttons) =>
    buttons.map((button) => { const box = button.getBoundingClientRect(); return { left: box.left, right: box.right, top: box.top, bottom: box.bottom }; })
  );
  for (let i = 0; i < toolbarBoxes.length; i += 1) for (let j = i + 1; j < toolbarBoxes.length; j += 1) {
    const a = toolbarBoxes[i], b = toolbarBoxes[j];
    assert.ok(a.right <= b.left + 1 || b.right <= a.left + 1 || a.bottom <= b.top + 1 || b.bottom <= a.top + 1, "toolbar controls overlap");
  }

  assert.equal(errors.length, 0, errors.join("\n"));
  const appearanceContext = await browser.newContext({ colorScheme: "dark", viewport: { width: 980, height: 660 } });
  const appearancePage = await appearanceContext.newPage();
  await appearancePage.goto(`${base}/#/main`, { waitUntil: "domcontentloaded" });
  assert.equal(await appearancePage.locator("html").getAttribute("data-theme"), "dark");
  await appearancePage.waitForSelector(".workspace-panels");
  await appearancePage.emulateMedia({ colorScheme: "light" });
  await appearancePage.waitForFunction(() => document.documentElement.dataset.theme === "light");
  await appearanceContext.close();
  const englishContext = await browser.newContext({ locale: "en-US", viewport: { width: 700, height: 560 } });
  const englishPage = await englishContext.newPage();
  await englishPage.goto(`${base}/#/onboarding`);
  await englishPage.getByRole("heading", { name: "Choose your model provider" }).waitFor();
  const continueBox = await englishPage.getByRole("button", { name: "Continue", exact: true }).boundingBox();
  assert.ok(continueBox && continueBox.y + continueBox.height <= 560, "onboarding action is clipped at minimum size");
  await englishPage.getByRole("button", { name: "Continue", exact: true }).click();
  assert.ok(await englishPage.getByText("API Key", { exact: false }).count() > 0);
  assert.equal(await englishPage.evaluate(() => document.documentElement.scrollWidth <= window.innerWidth), true);
  await englishContext.close();
  console.log("Interaction checks passed");
} finally {
  await context.close();
  await browser.close();
  if (server) {
    try {
      if (process.platform === "win32") server.kill("SIGTERM");
      else process.kill(-server.pid, "SIGTERM");
    } catch (error) {
      if (error.code !== "ESRCH") throw error;
    }
  }
}
