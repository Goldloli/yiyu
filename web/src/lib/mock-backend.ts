import type { Backend, TranslationStartResult } from "./contract";
import { normalizeError } from "./errors";
import {
  CONTRACT_EVENTS,
  type AppError,
  type Bootstrap,
  type HistoryEntry,
  type HistoryFilter,
  type ProviderConfig,
  type ProviderTestResult,
  type ProviderView,
  type Settings,
  type SettingsEnvelope,
  type SettingsPatch,
  type StartTranslationInput,
  type TranslationEvent,
  type TranslationSurface,
  type UpdateSnapshot
} from "./types";

const NOW = Date.now();

const providers: ProviderConfig[] = [
  {
    id: "openai",
    label: "OpenAI",
    baseUrl: "https://api.openai.com/v1",
    model: "gpt-6-luna",
    systemPrompt: null,
    extraHeaders: {},
    isCustom: false,
    requiresApiKey: true
  },
  {
    id: "deepseek",
    label: "DeepSeek",
    baseUrl: "https://api.deepseek.com/v1",
    model: "deepseek-flash",
    systemPrompt: null,
    extraHeaders: {},
    isCustom: false,
    requiresApiKey: true
  },
  {
    id: "openrouter",
    label: "OpenRouter",
    baseUrl: "https://openrouter.ai/api/v1",
    model: "openrouter/auto",
    systemPrompt: null,
    extraHeaders: { "HTTP-Referer": "https://github.com/", "X-Title": "Yiyu" },
    isCustom: false,
    requiresApiKey: true
  },
  {
    id: "zhipu",
    label: "智谱 GLM",
    baseUrl: "https://open.bigmodel.cn/api/paas/v4",
    model: "glm-5.3-flash",
    systemPrompt: null,
    extraHeaders: {},
    isCustom: false,
    requiresApiKey: true
  },
  {
    id: "kimi",
    label: "Kimi",
    baseUrl: "https://api.moonshot.cn/v1",
    model: "kimi-k3.1-pro",
    systemPrompt: null,
    extraHeaders: {},
    isCustom: false,
    requiresApiKey: true
  },
  {
    id: "siliconflow",
    label: "硅基流动",
    baseUrl: "https://api.siliconflow.cn/v1",
    model: "deepseek-ai/DeepSeek-V4.1-Flash",
    systemPrompt: null,
    extraHeaders: {},
    isCustom: false,
    requiresApiKey: true
  },
  {
    id: "ollama",
    label: "Ollama",
    baseUrl: "http://127.0.0.1:11434/v1",
    model: "qwen3.5:4b",
    systemPrompt: null,
    extraHeaders: {},
    isCustom: false,
    requiresApiKey: false
  },
  {
    id: "lmstudio",
    label: "LM Studio",
    baseUrl: "http://127.0.0.1:1234/v1",
    model: "local-model",
    systemPrompt: null,
    extraHeaders: {},
    isCustom: false,
    requiresApiKey: false
  }
];

const languages = [
  ["zh-Hans", "简体中文", "Simplified Chinese"],
  ["zh-Hant", "繁體中文", "Traditional Chinese"],
  ["en", "英语", "English"],
  ["ja", "日语", "Japanese"],
  ["ko", "韩语", "Korean"],
  ["fr", "法语", "French"],
  ["de", "德语", "German"],
  ["es", "西班牙语", "Spanish"],
  ["pt", "葡萄牙语", "Portuguese"],
  ["it", "意大利语", "Italian"],
  ["ru", "俄语", "Russian"],
  ["ar", "阿拉伯语", "Arabic"],
  ["tr", "土耳其语", "Turkish"],
  ["nl", "荷兰语", "Dutch"],
  ["pl", "波兰语", "Polish"],
  ["uk", "乌克兰语", "Ukrainian"],
  ["id", "印度尼西亚语", "Indonesian"],
  ["vi", "越南语", "Vietnamese"],
  ["th", "泰语", "Thai"],
  ["ms", "马来语", "Malay"],
  ["hi", "印地语", "Hindi"],
  ["fa", "波斯语", "Persian"],
  ["he", "希伯来语", "Hebrew"],
  ["sv", "瑞典语", "Swedish"],
  ["da", "丹麦语", "Danish"],
  ["no", "挪威语", "Norwegian"],
  ["fi", "芬兰语", "Finnish"],
  ["cs", "捷克语", "Czech"],
  ["ro", "罗马尼亚语", "Romanian"],
  ["hu", "匈牙利语", "Hungarian"],
  ["el", "希腊语", "Greek"]
].map(([code, nameZh, nameEn]) => ({ code, nameZh, nameEn }));

const sampleHistory: HistoryEntry[] = [
  {
    id: "history-1",
    sourceText:
      "The best translation is the one that disappears into the reading experience.",
    translatedText: "最好的翻译，是让读者忘记它曾经存在。",
    sourceLanguage: "英语",
    targetLanguage: "简体中文",
    providerId: "openai",
    providerLabel: "OpenAI",
    model: "gpt-6-luna",
    favorite: true,
    durationMs: 2840,
    createdAt: NOW - 60_000
  },
  {
    id: "history-2",
    sourceText:
      "小さな道具ほど、毎日の仕事の流れを静かに変えてくれる。",
    translatedText: "越是小巧的工具，越能安静地改变每天的工作流。",
    sourceLanguage: "日语",
    targetLanguage: "简体中文",
    providerId: "deepseek",
    providerLabel: "DeepSeek",
    model: "deepseek-flash",
    favorite: false,
    durationMs: 4120,
    createdAt: NOW - 18 * 60_000
  },
  {
    id: "history-3",
    sourceText: "Lightness is not the absence of features, but the absence of friction.",
    translatedText: "轻盈不是功能的缺席，而是阻力的消失。",
    sourceLanguage: "英语",
    targetLanguage: "简体中文",
    providerId: "openrouter",
    providerLabel: "OpenRouter",
    model: "openrouter/auto",
    favorite: true,
    durationMs: 1920,
    createdAt: NOW - 2 * 60 * 60_000
  },
  {
    id: "history-4",
    sourceText: "少即是多，但少必须刚刚好。",
    translatedText: "Less is more, but less has to be exactly enough.",
    sourceLanguage: "简体中文",
    targetLanguage: "英语",
    providerId: "openai",
    providerLabel: "OpenAI",
    model: "gpt-6-luna",
    favorite: false,
    durationMs: 2200,
    createdAt: NOW - 5 * 60 * 60_000
  },
  {
    id: "history-5",
    sourceText: "Reading should feel like thinking, not like operating a tool.",
    translatedText: "阅读应该像思考一样自然，而不是像在操作一件工具。",
    sourceLanguage: "英语",
    targetLanguage: "简体中文",
    providerId: "zhipu",
    providerLabel: "智谱 GLM",
    model: "glm-5.3-flash",
    favorite: false,
    durationMs: 1630,
    createdAt: NOW - 24 * 60 * 60_000
  },
  {
    id: "history-6",
    sourceText: "Une interface devrait se faire oublier au profit du sens.",
    translatedText: "界面应当隐去自身，把注意力让给意义。",
    sourceLanguage: "法语",
    targetLanguage: "简体中文",
    providerId: "deepseek",
    providerLabel: "DeepSeek",
    model: "deepseek-flash",
    favorite: false,
    durationMs: 3070,
    createdAt: NOW - 2 * 24 * 60 * 60_000
  }
];

function baseSettings(): Settings {
  const onboarding =
    typeof window !== "undefined" &&
    (window.location.hash.includes("/onboarding") ||
      new URLSearchParams(window.location.search).get("onboarding") === "1");
  const configured = !onboarding;
  let appearance: Partial<Pick<Settings, "theme" | "windowGlass" | "backgroundChoice" | "silentStart">> = {};
  try {
    if (typeof localStorage !== "undefined") appearance = JSON.parse(localStorage.getItem("yiyu-mock-appearance") ?? "{}");
  } catch { /* Private browsing may disable storage. */ }
  return {
    version: 1,
    onboardingCompleted: !onboarding,
    providers,
    activeProviderId: onboarding ? null : "openai",
    sourceLanguage: "auto",
    targetLanguage: "zh-Hans",
    fallbackTargetLanguage: "en",
    customTargetLanguage: null,
    systemPrompt: null,
    shortcuts: {
      selection: "Alt+Shift+Y",
      clipboard: "Alt+Shift+V",
      openMain: "Alt+Shift+Space"
    },
    captureClipboardFallback: true,
    autoSaveHistory: true,
    historyLimit: 500,
    maxInputChars: 20_000,
    providerProxy: { mode: "system", url: null },
    updateProxy: { mode: "system", url: null },
    checkUpdates: true,
    theme: appearance.theme ?? "system",
    motion: "system",
    uiLanguage: "system",
    windowGlass: appearance.windowGlass ?? true,
    silentStart: appearance.silentStart ?? false,
    backgroundId: null,
    backgroundChoice: appearance.backgroundChoice ?? null,
    ...(configured ? {} : {})
  };
}

function baseUpdate(): UpdateSnapshot {
  return {
    status: "idle",
    currentVersion: "0.1.0",
    version: null,
    notes: null,
    assetName: null,
    filePath: null,
    downloadedBytes: 0,
    totalBytes: null,
    progress: null,
    bytesPerSecond: null,
    etaSeconds: null,
    error: null
  };
}

type Listener = (payload: unknown) => void;

type MockBackground = { id: string; image: string };

async function mockBackgroundStore(value?: MockBackground | null): Promise<MockBackground | null> {
  if (typeof indexedDB === "undefined") return null;
  return new Promise((resolve, reject) => {
    const request = indexedDB.open("yiyu-mock-appearance", 1);
    request.onupgradeneeded = () => request.result.createObjectStore("background");
    request.onerror = () => reject(request.error);
    request.onsuccess = () => {
      const db = request.result;
      const transaction = db.transaction("background", value === undefined ? "readonly" : "readwrite");
      const store = transaction.objectStore("background");
      const operation = value === undefined ? store.get("current") : value === null ? store.delete("current") : store.put(value, "current");
      let result: MockBackground | null = null;
      operation.onsuccess = () => { result = value === undefined ? (operation.result as MockBackground | undefined) ?? null : value; };
      transaction.oncomplete = () => { db.close(); resolve(result); };
      transaction.onerror = () => { db.close(); reject(transaction.error); };
      transaction.onabort = () => { db.close(); reject(transaction.error); };
    };
  });
}

export class MockBackend implements Backend {
  private settingsRevision = 0;
  private settings: Settings = baseSettings();
  private history: HistoryEntry[] =
    this.settings.onboardingCompleted ? sampleHistory.slice() : [];
  private configuredProviderIds = new Set(
    this.settings.onboardingCompleted ? ["openai", "deepseek", "openrouter"] : []
  );
  private listeners = new Map<string, Set<Listener>>();
  private sessions = new Map<string, number>();
  private popupPinned = false;
  private update = baseUpdate();
  private updateProgressTimer: number | null = null;
  private pendingHistory = new Map<string, HistoryEntry>();
  private background: string | null = null;
  private backgroundLoaded = false;

  private async restoreBackground() {
    if (this.backgroundLoaded) return;
    this.backgroundLoaded = true;
    try {
      const stored = await mockBackgroundStore();
      if (stored) {
        this.background = stored.image;
        this.settings = { ...this.settings, backgroundId: stored.id };
      }
    } catch { /* Browser previews remain usable when storage is unavailable. */ }
  }

  private providerView(provider: ProviderConfig): ProviderView {
    return {
      ...provider,
      configured:
        !provider.requiresApiKey || this.configuredProviderIds.has(provider.id)
    };
  }

  private emit<T>(event: string, payload: T) {
    this.listeners.get(event)?.forEach((listener) => listener(payload));
  }

  async getBootstrap(): Promise<Bootstrap> {
    await this.restoreBackground();
    if (currentScenario() === "popup-permission") {
      window.setTimeout(() => {
        const event: TranslationEvent = {
          sessionId: "capture-permission", surface: "popup", state: "preparing",
          sourceText: "", text: null, direction: null, error: null,
          durationMs: null, historySaved: null, persistenceError: null
        };
        this.emit(CONTRACT_EVENTS.translation, event);
        this.emit(CONTRACT_EVENTS.translation, {
          ...event, state: "failed",
          error: { ...mockError("permission", "Accessibility permission is required"), actions: ["open-settings"] }
        } satisfies TranslationEvent);
      }, 0);
    }
    return {
      app: {
        name: "Yiyu",
        displayName: "译羽",
        version: "0.1.0",
        identifier: "com.yiyu.desktop",
        platform: navigator.platform.includes("Mac") ? "macos" : "windows",
        arch: "aarch64",
        dataDir: "/Users/demo/Library/Application Support/Yiyu"
      },
      settings: {
        revision: this.settingsRevision,
        settings: structuredClone(this.settings)
      },
      settingsRecoveryBackup: currentScenario() === "settings-recovery"
        ? "/demo/settings.invalid-backup.json"
        : null,
      shortcutRegistrationFailed: currentScenario() === "shortcut-conflict",
      providers: this.settings.providers.map((provider) => this.providerView(provider)),
      languages,
      history: this.history.slice(),
      permissions: {
        accessibility: currentScenario() !== "permission-refresh",
        screenRecording: true
      },
      update: structuredClone(this.update)
    };
  }

  async getSettings(): Promise<SettingsEnvelope> {
    return {
      revision: this.settingsRevision,
      settings: structuredClone(this.settings),
      providers: this.settings.providers.map((provider) => this.providerView(provider))
    };
  }

  async patchSettings(
    expectedRevision: number,
    patch: SettingsPatch
  ): Promise<SettingsEnvelope> {
    if (expectedRevision < this.settingsRevision) {
      throw mockError("validation", "Settings revision is stale");
    }
    this.settings = mergeSettingsPatch(this.settings, patch);
    if (patch.theme !== undefined || patch.windowGlass !== undefined || patch.backgroundChoice !== undefined || patch.silentStart !== undefined) {
      try {
        if (typeof localStorage !== "undefined") localStorage.setItem("yiyu-mock-appearance", JSON.stringify({ theme: this.settings.theme, windowGlass: this.settings.windowGlass, backgroundChoice: this.settings.backgroundChoice, silentStart: this.settings.silentStart }));
      } catch { /* The in-memory mock still works when storage is unavailable. */ }
    }
    this.settingsRevision += 1;
    const envelope = await this.getSettings();
    this.emit(CONTRACT_EVENTS.settings, envelope);
    return envelope;
  }

  async getBackground(): Promise<string | null> {
    await this.restoreBackground();
    return currentScenario() === "background-missing" ? null : this.background;
  }

  async getAutostart(): Promise<boolean> {
    return localStorage.getItem("yiyu-mock-autostart") === "true";
  }

  async setAutostart(enabled: boolean): Promise<boolean> {
    localStorage.setItem("yiyu-mock-autostart", String(enabled));
    return enabled;
  }

  async setBackground(expectedRevision: number, data: string): Promise<SettingsEnvelope> {
    if (expectedRevision !== this.settingsRevision) throw mockError("validation", "Settings revision is stale");
    if (data.length > 12 * 1024 * 1024 * 4 / 3 + 4) throw mockError("validation", "Image is larger than 12 MB");
    let bytes: string;
    try { bytes = atob(data); } catch { throw mockError("validation", "Invalid image"); }
    const mime = bytes.startsWith("\x89PNG\r\n\x1a\n") ? "image/png" : bytes.startsWith("\xff\xd8\xff") ? "image/jpeg" : null;
    if (!mime) throw mockError("validation", "Choose a PNG or JPEG image");
    const image = `data:${mime};base64,${data}`;
    const id = crypto.randomUUID();
    await mockBackgroundStore({ id, image });
    this.backgroundLoaded = true;
    this.background = image;
    this.settings = { ...this.settings, backgroundId: id, backgroundChoice: "custom" };
    try { localStorage.setItem("yiyu-mock-appearance", JSON.stringify({ theme: this.settings.theme, windowGlass: this.settings.windowGlass, backgroundChoice: "custom", silentStart: this.settings.silentStart })); } catch { /* in-memory fallback */ }
    this.settingsRevision += 1;
    const envelope = await this.getSettings();
    this.emit(CONTRACT_EVENTS.settings, envelope);
    return envelope;
  }

  async clearBackground(expectedRevision: number): Promise<SettingsEnvelope> {
    if (expectedRevision !== this.settingsRevision) throw mockError("validation", "Settings revision is stale");
    await mockBackgroundStore(null);
    this.backgroundLoaded = true;
    this.background = null;
    this.settings = { ...this.settings, backgroundId: null, backgroundChoice: "none" };
    try { localStorage.setItem("yiyu-mock-appearance", JSON.stringify({ theme: this.settings.theme, windowGlass: this.settings.windowGlass, backgroundChoice: "none", silentStart: this.settings.silentStart })); } catch { /* in-memory fallback */ }
    this.settingsRevision += 1;
    const envelope = await this.getSettings();
    this.emit(CONTRACT_EVENTS.settings, envelope);
    return envelope;
  }

  async saveProviderSecret(providerId: string, secret: string): Promise<void> {
    if (secret.trim()) this.configuredProviderIds.add(providerId);
    else this.configuredProviderIds.delete(providerId);
    this.emit(CONTRACT_EVENTS.settings, await this.getSettings());
  }

  async deleteProviderSecret(providerId: string): Promise<void> {
    this.configuredProviderIds.delete(providerId);
    this.emit(CONTRACT_EVENTS.settings, await this.getSettings());
  }

  async testProvider(
    _provider: ProviderConfig,
    apiKey?: string | null
  ): Promise<ProviderTestResult> {
    const scenario = currentScenario();
    await delay(460);
    if (scenario === "auth-failure") {
      return { ok: false, latencyMs: 412, message: "Invalid API key" };
    }
    if (scenario === "proxy-failure") {
      return { ok: false, latencyMs: 810, message: "Proxy connection failed" };
    }
    if (apiKey === "") {
      return { ok: false, latencyMs: 120, message: "API key is required" };
    }
    return { ok: true, latencyMs: 386, message: "Connection succeeded" };
  }

  async fetchModels(provider: ProviderConfig): Promise<string[]> {
    await delay(350);
    if (provider.id === "openai") return ["gpt-6-luna", "gpt-6.1-terra", "gpt-6.1-sol", "gpt-6-astra"];
    if (provider.id === "deepseek") return ["deepseek-flash", "deepseek-v4-pro"];
    if (provider.id === "openrouter") {
      return ["openrouter/auto", "openai/gpt-6-luna"];
    }
    if (provider.id === "ollama") return ["qwen3.5:4b", "qwen3.5:9b", "qwen3.8:27b"];
    return [provider.model, "custom-model"];
  }

  async startTranslation(input: StartTranslationInput): Promise<TranslationStartResult> {
    const providerId = input.providerId ?? this.settings.activeProviderId;
    if (!providerId) throw mockError("config", "Configure a provider before translating");
    const provider = this.settings.providers.find((item) => item.id === providerId);
    if (!provider) throw mockError("config", "Configure a provider before translating");
    if (provider.requiresApiKey && !this.configuredProviderIds.has(provider.id)) {
      throw mockError("authentication", "API key is not configured");
    }
    if (!input.text.trim()) throw mockError("validation", "Empty text");
    const sessionId = `session-${Date.now()}-${Math.random().toString(16).slice(2)}`;
    this.sessions.set(input.surface, window.setTimeout(() => {}, 0));
    const startEvent: TranslationEvent = {
      sessionId,
      surface: input.surface,
      state: "preparing",
      sourceText: input.text,
      text: null,
      direction: null,
      error: null,
      durationMs: null,
      historySaved: null,
      persistenceError: null
    };
    this.emit(CONTRACT_EVENTS.translation, startEvent);
    const scenario = currentScenario();
    const delayedFailure =
      scenario === "auth-failure"
        ? {
            ...mockError("authentication", "Invalid API key"),
            actions: ["open-settings"]
          }
        : scenario === "proxy-failure"
          ? {
              ...mockError("network", "Proxy connection failed"),
              retryable: true
            }
          : scenario === "empty-result"
            ? {
                ...mockError("emptyResult", "Translation returned an empty result"),
                retryable: true
              }
            : null;
    if (delayedFailure) {
      this.sessions.set(
        input.surface,
        window.setTimeout(() => {
          this.sessions.delete(input.surface);
          this.emit(CONTRACT_EVENTS.translation, {
            ...startEvent,
            state: "failed",
            error: delayedFailure
          } satisfies TranslationEvent);
        }, 180)
      );
      return { sessionId };
    }
    const translated = translateSample(input.text, input.surface);
    const chunks = splitIntoChunks(translated);
    let index = 0;
    let output = "";
    const tick = () => {
      if (this.sessions.get(input.surface) === undefined) return;
      if (scenario === "rate-limit" && index === 1) {
        this.sessions.delete(input.surface);
        const error = mockError("rateLimit", "Provider rate limit reached. Try again shortly.");
        error.retryable = true;
        this.emit(CONTRACT_EVENTS.translation, {
          ...startEvent,
          state: "failed",
          error
        } satisfies TranslationEvent);
        return;
      }
      if (index < chunks.length) {
        output += chunks[index++];
        this.emit(CONTRACT_EVENTS.translation, {
          ...startEvent,
          state: "streaming",
          text: chunks[index - 1]
        } satisfies TranslationEvent);
        this.sessions.set(input.surface, window.setTimeout(tick, 52 + (index % 3) * 12));
        return;
      }
      this.sessions.delete(input.surface);
      const direction = {
        sourceCode: input.text.match(/[一-龥]/) ? "zh-Hans" : "en",
        sourceLabel: input.text.match(/[一-龥]/) ? "简体中文" : "英语",
        targetValue:
          input.targetLanguage ??
          (input.text.match(/[一-龥]/) ? "en" : "zh-Hans"),
        targetLabel:
          input.targetLanguage === "en" || input.text.match(/[一-龥]/) ? "英语" : "简体中文",
        confidence: 0.92,
        uncertain: false
      };
      const nextHistory: HistoryEntry | null = this.settings.autoSaveHistory
        ? {
            id: `history-${Date.now()}`,
            sourceText: input.text,
            translatedText: output,
            sourceLanguage: direction.sourceLabel,
            targetLanguage: direction.targetLabel,
            providerId: provider.id,
            providerLabel: provider.label,
            model: provider.model,
            favorite: false,
            durationMs: 1200,
            createdAt: Date.now()
          }
        : null;
      const persistenceFailure =
        nextHistory && scenario === "history-write-failure"
          ? mockError("storage", "History database is unavailable")
          : null;
      if (persistenceFailure && nextHistory) {
        this.pendingHistory.set(sessionId, nextHistory);
      }
      const completed: TranslationEvent = {
        ...startEvent,
        state: "completed",
        text: output,
        direction,
        durationMs: 1200,
        historySaved: Boolean(nextHistory && !persistenceFailure),
        persistenceError: persistenceFailure
      };
      this.emit(CONTRACT_EVENTS.translation, completed);
      if (nextHistory && !persistenceFailure) {
        this.history.unshift(nextHistory);
        this.history = this.history.slice(0, this.settings.historyLimit);
        this.emit(CONTRACT_EVENTS.history, true);
      }
    };
    this.sessions.set(input.surface, window.setTimeout(tick, 180));
    return { sessionId };
  }

  async cancelTranslation(surface: string): Promise<void> {
    const timer = this.sessions.get(surface);
    if (timer !== undefined) window.clearTimeout(timer);
    this.sessions.delete(surface);
  }

  async captureSelection(): Promise<string> {
    return "The best translation is the one that disappears into the reading experience.";
  }

  async translateClipboard(): Promise<TranslationStartResult> {
    return this.startTranslation({
      surface: "popup",
      text: currentScenario() === "popup-long"
        ? "A thoughtful translation keeps the meaning clear, even across a long passage. ".repeat(10)
        : "Lightness is not the absence of features, but the absence of friction."
    });
  }

  async listHistory(filter: HistoryFilter = {}): Promise<HistoryEntry[]> {
    const query = filter.query?.trim().toLowerCase();
    return this.history.filter((entry) => {
      if (!query) return true;
      return [
        entry.sourceText,
        entry.translatedText,
        entry.providerLabel,
        entry.model
      ].some((value) => value.toLowerCase().includes(query));
    });
  }

  async deleteHistory(id: string): Promise<boolean> {
    const before = this.history.length;
    this.history = this.history.filter((entry) => entry.id !== id);
    this.emit(CONTRACT_EVENTS.history, true);
    return this.history.length !== before;
  }

  async clearHistory(): Promise<{ changed: number }> {
    const before = this.history.length;
    this.history = [];
    this.emit(CONTRACT_EVENTS.history, true);
    return { changed: before - this.history.length };
  }

  async retrySaveHistory(sessionId: string): Promise<HistoryEntry> {
    const entry = this.pendingHistory.get(sessionId);
    if (!entry) {
      throw mockError("storage", "No pending history write");
    }
    this.pendingHistory.delete(sessionId);
    this.history.unshift(entry);
    this.history = this.history.slice(0, this.settings.historyLimit);
    this.emit(CONTRACT_EVENTS.history, true);
    return entry;
  }

  async copyText(text: string): Promise<void> {
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      const area = document.createElement("textarea");
      area.value = text;
      document.body.append(area);
      area.select();
      document.execCommand("copy");
      area.remove();
    }
  }

  async openExternal(url: string): Promise<void> {
    window.open(url, "_blank", "noopener,noreferrer");
  }

  async refreshPermissions(): Promise<Bootstrap["permissions"]> {
    return { accessibility: true, screenRecording: true };
  }

  async requestAccessibilityPermission(): Promise<void> {}
  async openAccessibilitySettings(): Promise<void> {}
  async openMainWindow(): Promise<void> {
    location.hash = "#/main";
  }
  async openSettingsWindow(section?: string): Promise<void> {
    location.hash = section ? `#/settings?section=${encodeURIComponent(section)}` : "#/settings";
  }
  async openOnboardingWindow(): Promise<void> {
    location.hash = "#/onboarding";
  }
  async closePopup(): Promise<void> {
    this.hideMockWindow();
  }
  async setPopupPinned(pinned: boolean): Promise<void> {
    this.popupPinned = pinned;
  }
  async resizePopup(): Promise<void> {}
  async quitApp(): Promise<void> {}

  async checkUpdate(force: boolean): Promise<UpdateSnapshot> {
    this.update = { ...baseUpdate(), status: "checking" };
    this.emit(CONTRACT_EVENTS.update, structuredClone(this.update));
    await delay(800);
    this.update = {
      ...baseUpdate(),
      status: "available",
      version: "0.2.0",
      notes: "Faster stream rendering, clearer provider errors and a smaller bundle.",
      assetName:
        navigator.platform.includes("Mac")
          ? "Yiyu_0.2.0_aarch64.dmg"
          : "Yiyu_0.2.0_x64-setup.exe",
      totalBytes: 7_200_000
    };
    this.emit(CONTRACT_EVENTS.update, structuredClone(this.update));
    return this.update;
  }

  async downloadUpdate(): Promise<UpdateSnapshot> {
    if (this.update.status !== "available") return this.update;
    const total = this.update.totalBytes ?? 7_200_000;
    const started = Date.now();
    this.update = {
      ...this.update,
      status: "downloading",
      totalBytes: total,
      bytesPerSecond: null,
      etaSeconds: null
    };
    return new Promise((resolve) => {
      let downloaded = 0;
      this.updateProgressTimer = window.setInterval(() => {
        downloaded += total / 18;
        const progress = Math.min(100, (downloaded / total) * 100);
        const elapsedSeconds = Math.max(0.1, (Date.now() - started) / 1000);
        const bytesPerSecond = Math.round(Math.min(total, downloaded) / elapsedSeconds);
        this.update = {
          ...this.update,
          downloadedBytes: Math.min(total, downloaded),
          progress,
          bytesPerSecond,
          etaSeconds:
            downloaded >= total
              ? 0
              : Math.ceil((total - downloaded) / bytesPerSecond)
        };
        this.emit(CONTRACT_EVENTS.update, structuredClone(this.update));
        if (progress >= 100) {
          if (this.updateProgressTimer !== null) {
            clearInterval(this.updateProgressTimer);
            this.updateProgressTimer = null;
          }
          this.update = {
            ...this.update,
            status: "ready",
            progress: 100,
            bytesPerSecond: null,
            etaSeconds: 0,
            filePath: "/Users/demo/Library/Application Support/Yiyu/updates/installer"
          };
          this.emit(CONTRACT_EVENTS.update, structuredClone(this.update));
          resolve(this.update);
        }
      }, 140);
    });
  }

  async cancelUpdate(): Promise<void> {
    if (this.updateProgressTimer !== null) {
      clearInterval(this.updateProgressTimer);
      this.updateProgressTimer = null;
    }
    this.update = {
      ...this.update,
      status: "cancelled",
      bytesPerSecond: null,
      etaSeconds: null
    };
    this.emit(CONTRACT_EVENTS.update, structuredClone(this.update));
  }

  async installUpdate(): Promise<void> {}

  async getUpdateSnapshot(): Promise<UpdateSnapshot> {
    return structuredClone(this.update);
  }

  async getDiagnostics(): Promise<Record<string, unknown>> {
    return {
      dataDir: "/Users/demo/Library/Application Support/Yiyu",
      version: "0.1.0",
      platform: "mock"
    };
  }

  on<T>(event: string, handler: (payload: T) => void): () => void {
    const listeners = this.listeners.get(event) ?? new Set<Listener>();
    listeners.add(handler as Listener);
    this.listeners.set(event, listeners);
    return () => {
      listeners.delete(handler as Listener);
    };
  }

  private hideMockWindow() {
    document.documentElement.dataset.mockHidden = "1";
    setTimeout(() => {
      delete document.documentElement.dataset.mockHidden;
    }, 800);
  }
}

function mergeSettingsPatch(settings: Settings, patch: SettingsPatch): Settings {
  const result = structuredClone(settings);
  for (const [key, value] of Object.entries(patch)) {
    (result as unknown as Record<string, unknown>)[key] = value;
  }
  return result;
}

function mockError(kind: string, message: string): AppError {
  return { kind, message, retryable: false, field: null, actions: [] };
}

function delay(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

function splitIntoChunks(text: string): string[] {
  return text
    .split(/(?<=[。！？.!?])\s*/)
    .filter(Boolean)
    .flatMap((sentence) => {
      if (sentence.length <= 16) return [sentence];
      const size = Math.ceil(sentence.length / 2);
      return [sentence.slice(0, size), sentence.slice(size)];
    });
}

function translateSample(text: string, surface: TranslationSurface): string {
  if (surface === "popup") {
    if (currentScenario() === "popup-long") {
      return "清晰的翻译让读者专注于内容，也让长篇文字读起来自然流畅。".repeat(12);
    }
    return "最好的翻译，是消失在阅读体验里的那一个。";
  }
  if (/[一-龥]/.test(text)) {
    return "A small tool works best when it removes friction without asking for attention.";
  }
  const normalized = text.toLowerCase();
  if (normalized.includes("light")) {
    return "轻盈不是功能的缺席，而是阻力的消失。";
  }
  if (normalized.includes("reading")) {
    return "阅读应该像思考一样自然，而不是像在操作一件工具。";
  }
  return "真正自然的技术，是让人忘记自己正在使用技术。";
}

function currentScenario(): string | null {
  if (typeof window === "undefined") return null;
  const hash = window.location.hash;
  const queryIndex = hash.indexOf("?");
  const hashQuery = queryIndex >= 0 ? hash.slice(queryIndex + 1) : "";
  return (
    new URLSearchParams(window.location.search).get("scenario") ??
    new URLSearchParams(hashQuery).get("scenario")
  );
}

export function parseMockError(error: unknown): AppError {
  return normalizeError(error);
}
