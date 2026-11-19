import { create } from "zustand";
import { getBackend } from "./api";
import { normalizeError } from "./errors";
import { resolveUiLanguage, t } from "./i18n";
import { applyGlass, applyMotion, applyTheme } from "./theme";
import {
  CONTRACT_EVENTS,
  type AppError,
  type Bootstrap,
  type HistoryEntry,
  type Language,
  type Notice,
  type PermissionStatus,
  type ProviderConfig,
  type ProviderTestResult,
  type ProviderView,
  type Settings,
  type SettingsEnvelope,
  type SettingsPatch,
  type StartTranslationInput,
  type TranslationEvent,
  type TranslationSession,
  type TranslationSurface,
  type UpdateSnapshot
} from "./types";

type SessionMap = Partial<Record<TranslationSurface, TranslationSession>>;

interface YiyuState {
  initialized: boolean;
  loading: boolean;
  fatalError: AppError | null;
  app: Bootstrap["app"] | null;
  settings: Settings | null;
  settingsRevision: number;
  providers: ProviderView[];
  languages: Language[];
  history: HistoryEntry[];
  permissions: PermissionStatus;
  update: UpdateSnapshot | null;
  sessions: SessionMap;
  notices: Notice[];
  historyQuery: string;
  subscribeCleanup: (() => void)[];
  initialize: () => Promise<void>;
  applySettings: (envelope: SettingsEnvelope) => void;
  patchSettings: (patch: SettingsPatch) => Promise<SettingsEnvelope>;
  saveProviderSecret: (providerId: string, secret: string) => Promise<void>;
  deleteProviderSecret: (providerId: string) => Promise<void>;
  retryCredentialCleanup: () => Promise<void>;
  testProvider: (
    provider: ProviderConfig,
    apiKey?: string | null
  ) => Promise<ProviderTestResult>;
  fetchModels: (provider: ProviderConfig, apiKey?: string | null) => Promise<string[]>;
  startTranslation: (
    surface: TranslationSurface,
    text: string,
    options?: Partial<
      Pick<StartTranslationInput, "manualSourceLanguage" | "targetLanguage" | "fallbackTargetLanguage">
    >
  ) => Promise<void>;
  cancelTranslation: (surface: TranslationSurface) => Promise<void>;
  retryTranslation: (surface: TranslationSurface) => Promise<void>;
  retrySaveHistory: (surface: TranslationSurface) => Promise<void>;
  copyText: (text: string) => Promise<void>;
  setHistoryQuery: (query: string) => void;
  refreshHistory: () => Promise<void>;
  deleteHistory: (id: string) => Promise<void>;
  clearHistory: () => Promise<void>;
  refreshPermissions: () => Promise<void>;
  requestAccessibilityPermission: () => Promise<void>;
  checkUpdate: (force?: boolean) => Promise<void>;
  downloadUpdate: () => Promise<void>;
  cancelUpdate: () => Promise<void>;
  installUpdate: () => Promise<void>;
  notify: (
    level: Notice["level"],
    message: string,
    code?: string,
    ttl?: number
  ) => void;
  dismissNotice: (id: number) => void;
}

const initialPermissions: PermissionStatus = {
  accessibility: false,
  screenRecording: true
};

let noticeId = 0;
const pendingTranslationRequests = new Map<TranslationSurface, symbol>();

function applyAppearance(settings: Settings) {
  applyTheme(settings.theme);
  applyMotion(settings.motion);
  applyGlass(settings.windowGlass);
}

export function applyTranslationEvent(
  state: YiyuState,
  event: TranslationEvent
): Partial<YiyuState> {
  const current = state.sessions[event.surface];
  const startsNewSession = event.state === "preparing" && event.sessionId !== "" &&
    current?.sessionId !== event.sessionId;
  if (
    current &&
    current.sessionId !== "pending" &&
    current.sessionId !== event.sessionId &&
    event.sessionId !== "" &&
    !startsNewSession
  ) {
    return {};
  }
  if (event.sessionId === "" && !current) {
    return {};
  }
  if (
    current?.status === "cancelled" &&
    current.sessionId === event.sessionId &&
    event.state !== "preparing"
  ) {
    return {};
  }
  const base: TranslationSession =
    (startsNewSession ? undefined : current) ??
    {
      sessionId: event.sessionId,
      surface: event.surface,
      sourceText: "",
      status: "preparing",
      output: "",
      direction: null,
      error: null,
      durationMs: null,
      historySaved: null,
      persistenceError: null
    };
  const next: TranslationSession = {
    ...base,
    sessionId: event.sessionId || base.sessionId,
    sourceText: event.sourceText ?? base.sourceText,
    status: event.state,
    output:
      event.state === "streaming"
        ? base.output + (event.text ?? "")
        : event.text ?? base.output,
    direction: event.direction ?? base.direction,
    error: event.error,
    durationMs: event.durationMs ?? base.durationMs,
    historySaved: event.historySaved ?? base.historySaved,
    persistenceError: event.persistenceError ?? base.persistenceError
  };
  return { sessions: { ...state.sessions, [event.surface]: next } };
}

export const useYiyuStore = create<YiyuState>((set, get) => ({
  initialized: false,
  loading: false,
  fatalError: null,
  app: null,
  settings: null,
  settingsRevision: 0,
  providers: [],
  languages: [],
  history: [],
  permissions: initialPermissions,
  update: null,
  sessions: {},
  notices: [],
  historyQuery: "",
  subscribeCleanup: [],

  initialize: async () => {
    if (get().initialized || get().loading) return;
    set({ loading: true, fatalError: null });
    const backend = getBackend();
    const cleanup = [
      backend.on<TranslationEvent>(CONTRACT_EVENTS.translation, (event) => {
        set((state) => applyTranslationEvent(state, event) as YiyuState);
      }),
      backend.on<SettingsEnvelope>(CONTRACT_EVENTS.settings, (envelope) => {
        get().applySettings(envelope);
      }),
      backend.on<unknown>(CONTRACT_EVENTS.history, () => {
        void get().refreshHistory();
      }),
      backend.on<PermissionStatus>(CONTRACT_EVENTS.permission, (permissions) => {
        set({ permissions });
      }),
      backend.on<UpdateSnapshot>(CONTRACT_EVENTS.update, (update) => {
        set({ update });
      }),
    ];
    set({ subscribeCleanup: cleanup });
    try {
      const bootstrap = await backend.getBootstrap();
      applyAppearance(bootstrap.settings.settings);
      document.documentElement.dataset.platform =
        bootstrap.app.platform === "macos" ? "macos" : "windows";
      set({
        initialized: true,
        loading: false,
        app: bootstrap.app,
        settings: bootstrap.settings.settings,
        settingsRevision: bootstrap.settings.revision,
        providers: bootstrap.providers,
        languages: bootstrap.languages,
        history: bootstrap.history,
        permissions: bootstrap.permissions,
        update: bootstrap.update
      });
      if (bootstrap.settingsRecoveryBackup && !window.location.hash.startsWith("#/popup")) {
        const ui = resolveUiLanguage(bootstrap.settings.settings.uiLanguage);
        const message = ui === "en"
          ? `Settings could not be read. The original file was backed up to ${bootstrap.settingsRecoveryBackup}`
          : `设置文件无法读取，原文件已备份至 ${bootstrap.settingsRecoveryBackup}`;
        get().notify("warning", message, "settings-recovery", 0);
      }
      if (bootstrap.shortcutRegistrationFailed && !window.location.hash.startsWith("#/popup")) {
        const ui = resolveUiLanguage(bootstrap.settings.settings.uiLanguage);
        const message = ui === "en"
          ? "Could not register shortcuts. Change them in settings."
          : "快捷键注册失败，请在设置中调整快捷键。";
        get().notify("warning", message, "shortcut-registration", 0);
      }
    } catch (error) {
      set({
        initialized: true,
        loading: false,
        fatalError: normalizeError(error)
      });
    }
  },

  applySettings: (envelope) => {
    if (envelope.revision < get().settingsRevision) return;
    applyAppearance(envelope.settings);
    set({
      settings: envelope.settings,
      settingsRevision: envelope.revision,
      providers: envelope.providers
    });
  },

  patchSettings: async (patch) => {
    const current = get();
    if (!current.settings) throw new Error("settings not initialized");
    try {
      const envelope = await getBackend().patchSettings(
        current.settingsRevision,
        patch
      );
      get().applySettings(envelope);
      return envelope;
    } catch (error) {
      const normalized = normalizeError(error);
      get().notify("error", normalized.message, normalized.kind);
      throw normalized;
    }
  },

  saveProviderSecret: async (providerId, secret) => {
    try {
      await getBackend().saveProviderSecret(providerId, secret);
      get().notify(
        "success",
        get().settings &&
          resolveUiLanguage(get().settings!.uiLanguage) === "en"
          ? "API key saved"
          : "API Key 已保存",
        "api-key-saved"
      );
    } catch (error) {
      const normalized = normalizeError(error);
      get().notify("error", normalized.message, normalized.kind);
      throw normalized;
    }
  },

  deleteProviderSecret: async (providerId) => {
    await getBackend().deleteProviderSecret(providerId);
  },

  retryCredentialCleanup: async () => {
    try {
      const envelope = await getBackend().retryCredentialCleanup();
      get().applySettings(envelope);
      if (envelope.settings.pendingCredentialCleanup.length > 0) {
        const en = resolveUiLanguage(envelope.settings.uiLanguage) === "en";
        get().notify("error", en ? "Could not remove the saved API keys" : "无法清理系统中的 API Key", "credential-cleanup");
      }
    } catch (error) {
      const normalized = normalizeError(error);
      get().notify("error", normalized.message, normalized.kind);
    }
  },

  testProvider: async (provider, apiKey) => {
    return getBackend().testProvider(provider, apiKey);
  },

  fetchModels: async (provider, apiKey) => {
    return getBackend().fetchModels(provider, apiKey);
  },

  startTranslation: async (surface, text, options = {}) => {
    if (!text.trim()) {
      get().notify(
        "warning",
        get().settings &&
          resolveUiLanguage(get().settings!.uiLanguage) === "en"
          ? "Enter some text first"
          : "请先输入文本",
        "empty-text"
      );
      return;
    }
    const requestId = Symbol(surface);
    pendingTranslationRequests.set(surface, requestId);
    const placeholder: TranslationSession = {
      sessionId: "pending",
      surface,
      sourceText: text,
      status: "preparing",
      output: "",
      direction: null,
      error: null,
      durationMs: null,
      historySaved: null,
      persistenceError: null
    };
    set((state) => ({
      sessions: { ...state.sessions, [surface]: placeholder }
    }));
    try {
      const result = await getBackend().startTranslation({
        surface,
        text,
        ...options
      });
      if (pendingTranslationRequests.get(surface) !== requestId) return;
      set((state) => {
        const current = state.sessions[surface];
        if (!current || (current.sessionId !== "pending" && current.sessionId !== result.sessionId)) return {};
        return {
          sessions: {
            ...state.sessions,
            [surface]: {
              ...current,
              sessionId: result.sessionId,
              status: current.status === "preparing" ? "preparing" : current.status
            }
          }
        };
      });
    } catch (error) {
      const normalized = normalizeError(error);
      if (pendingTranslationRequests.get(surface) !== requestId || normalized.kind === "cancelled") return;
      set((state) => ({
        sessions: {
          ...state.sessions,
          [surface]: {
            ...placeholder,
            sessionId: `local-${Date.now()}`,
            status: "failed",
            error: normalized
          }
        }
      }));
      get().notify("error", normalized.message, normalized.kind);
    } finally {
      if (pendingTranslationRequests.get(surface) === requestId) {
        pendingTranslationRequests.delete(surface);
      }
    }
  },

  cancelTranslation: async (surface) => {
    const cancelledSession = get().sessions[surface];
    pendingTranslationRequests.delete(surface);
    await getBackend().cancelTranslation(surface);
    set((state) => {
      const current = state.sessions[surface];
      const isSameSession = current && cancelledSession && (
        current === cancelledSession ||
        (cancelledSession.sessionId !== "pending" && current.sessionId === cancelledSession.sessionId)
      );
      return current && isSameSession
        ? {
            sessions: {
              ...state.sessions,
              [surface]: { ...current, status: "cancelled" }
            }
          }
        : {};
    });
  },

  retryTranslation: async (surface) => {
    const current = get().sessions[surface];
    if (current) {
      await get().startTranslation(surface, current.sourceText);
    }
  },

  retrySaveHistory: async (surface) => {
    try {
      const sessionId = get().sessions[surface]?.sessionId;
      if (!sessionId) throw new Error("No translation to save");
      await getBackend().retrySaveHistory(sessionId);
      set((state) => {
        const current = state.sessions[surface];
        if (!current || current.sessionId !== sessionId) return {};
        return {
          sessions: {
            ...state.sessions,
            [surface]: {
              ...current,
              historySaved: true,
              persistenceError: null
            }
          }
        };
      });
      await get().refreshHistory();
    } catch (error) {
      const normalized = normalizeError(error);
      get().notify("error", normalized.message, "history-retry");
    }
  },

  copyText: async (text) => {
    await getBackend().copyText(text);
    const settings = get().settings;
    const ui = settings ? resolveUiLanguage(settings.uiLanguage) : "zh";
    get().notify("success", t("common.copy", undefined, ui), "copied", 1400);
  },

  setHistoryQuery: (historyQuery) => {
    set({ historyQuery });
    void get().refreshHistory();
  },

  refreshHistory: async () => {
    try {
      const history = await getBackend().listHistory({
        query: get().historyQuery || null,
        limit: 200
      });
      set({ history });
    } catch (error) {
      get().notify("error", normalizeError(error).message, "history-load");
    }
  },

  deleteHistory: async (id) => {
    await getBackend().deleteHistory(id);
    await get().refreshHistory();
  },

  clearHistory: async () => {
    const result = await getBackend().clearHistory();
    await get().refreshHistory();
    const ui = get().settings
      ? resolveUiLanguage(get().settings!.uiLanguage)
      : "zh";
    get().notify(
      "success",
      `${result.changed} ${ui === "en" ? "entries removed" : "条记录已清除"}`,
      "history-cleared"
    );
  },

  refreshPermissions: async () => {
    const permissions = await getBackend().refreshPermissions();
    set({ permissions });
  },

  requestAccessibilityPermission: async () => {
    await getBackend().requestAccessibilityPermission();
  },

  checkUpdate: async (force = false) => {
    try {
      const update = await getBackend().checkUpdate(force);
      set({ update });
    } catch (error) {
      get().notify("error", normalizeError(error).message, "update-check");
    }
  },

  downloadUpdate: async () => {
    try {
      await getBackend().downloadUpdate();
    } catch (error) {
      get().notify("error", normalizeError(error).message, "update-download");
    }
  },

  cancelUpdate: async () => {
    await getBackend().cancelUpdate();
  },

  installUpdate: async () => {
    const path = get().update?.filePath;
    if (!path) return;
    try {
      await getBackend().installUpdate(path);
    } catch (error) {
      get().notify("error", normalizeError(error).message, "update-install");
    }
  },

  notify: (level, message, code = "notice", ttl = 3600) => {
    const id = ++noticeId;
    set((state) => ({
      notices: [...state.notices, { id, level, message, code }]
    }));
    if (ttl > 0) {
      window.setTimeout(() => get().dismissNotice(id), ttl);
    }
  },

  dismissNotice: (id) => {
    set((state) => ({
      notices: state.notices.filter((notice) => notice.id !== id)
    }));
  }
}));

export function useUiLanguage(): "zh" | "en" {
  return useYiyuStore((state) =>
    state.settings ? resolveUiLanguage(state.settings.uiLanguage) : "zh"
  );
}
