import type { Backend, TranslationStartResult } from "./contract";
import { CONTRACT_COMMANDS } from "./types";

async function invoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke: call } = await import("@tauri-apps/api/core");
  return call<T>(command, args);
}

export function createTauriBackend(): Backend {
  return {
    getBootstrap: () => invoke(CONTRACT_COMMANDS.getBootstrap),
    getSettings: () => invoke(CONTRACT_COMMANDS.getSettings),
    patchSettings: (expectedRevision, patch) =>
      invoke(CONTRACT_COMMANDS.patchSettings, { expectedRevision, patch }),
    getBackground: () => invoke(CONTRACT_COMMANDS.getBackground),
    setBackground: (expectedRevision, data) =>
      invoke(CONTRACT_COMMANDS.setBackground, { expectedRevision, data }),
    clearBackground: (expectedRevision) =>
      invoke(CONTRACT_COMMANDS.clearBackground, { expectedRevision }),
    saveProviderSecret: (providerId, secret) =>
      invoke(CONTRACT_COMMANDS.saveProviderSecret, { providerId, secret }),
    deleteProviderSecret: (providerId) =>
      invoke(CONTRACT_COMMANDS.deleteProviderSecret, { providerId }),
    testProvider: (provider, apiKey) =>
      invoke(CONTRACT_COMMANDS.testProvider, { provider, apiKey }),
    fetchModels: (provider, apiKey) =>
      invoke(CONTRACT_COMMANDS.fetchModels, { provider, apiKey }),
    startTranslation: (input) =>
      invoke<TranslationStartResult>(CONTRACT_COMMANDS.translateStart, { input }),
    cancelTranslation: (surface) =>
      invoke(CONTRACT_COMMANDS.translateCancel, { surface }),
    captureSelection: () => invoke(CONTRACT_COMMANDS.captureSelection),
    translateClipboard: () => invoke(CONTRACT_COMMANDS.translateClipboard),
    listHistory: (filter) => invoke(CONTRACT_COMMANDS.listHistory, { filter }),
    deleteHistory: (id) => invoke(CONTRACT_COMMANDS.deleteHistory, { id }),
    clearHistory: () => invoke(CONTRACT_COMMANDS.clearHistory),
    retrySaveHistory: (sessionId) => invoke(CONTRACT_COMMANDS.retrySaveHistory, { sessionId }),
    copyText: (text) => invoke(CONTRACT_COMMANDS.copyText, { text }),
    openExternal: (url) => invoke(CONTRACT_COMMANDS.openExternal, { url }),
    refreshPermissions: () => invoke(CONTRACT_COMMANDS.refreshPermissions),
    requestAccessibilityPermission: () =>
      invoke(CONTRACT_COMMANDS.requestAccessibility),
    openAccessibilitySettings: () => invoke(CONTRACT_COMMANDS.openAccessibilitySettings),
    getAutostart: () => invoke(CONTRACT_COMMANDS.getAutostart),
    setAutostart: (enabled) => invoke(CONTRACT_COMMANDS.setAutostart, { enabled }),
    openMainWindow: () => invoke(CONTRACT_COMMANDS.openMain),
    openSettingsWindow: (section) => invoke(CONTRACT_COMMANDS.openSettings, { section }),
    openOnboardingWindow: () => invoke(CONTRACT_COMMANDS.openOnboarding),
    closePopup: () => invoke(CONTRACT_COMMANDS.closePopup),
    setPopupPinned: (pinned) => invoke(CONTRACT_COMMANDS.setPopupPinned, { pinned }),
    resizePopup: (width, height) =>
      invoke(CONTRACT_COMMANDS.resizePopup, { width, height }),
    quitApp: () => invoke(CONTRACT_COMMANDS.quit),
    checkUpdate: (force) => invoke(CONTRACT_COMMANDS.checkUpdate, { force }),
    downloadUpdate: () => invoke(CONTRACT_COMMANDS.downloadUpdate),
    cancelUpdate: () => invoke(CONTRACT_COMMANDS.cancelUpdate),
    installUpdate: (path) => invoke(CONTRACT_COMMANDS.installUpdate, { path }),
    getUpdateSnapshot: () => invoke(CONTRACT_COMMANDS.getUpdateSnapshot),
    getDiagnostics: () => invoke(CONTRACT_COMMANDS.getDiagnostics),
    on<T>(event: string, handler: (payload: T) => void) {
      let unlisten: (() => void) | null = null;
      let cancelled = false;
      void import("@tauri-apps/api/event")
        .then(async ({ listen }) => {
          if (cancelled) return () => {};
          return listen<T>(event, ({ payload }) => handler(payload));
        })
        .then((un) => {
          if (cancelled) un();
          else unlisten = un;
        })
        .catch(() => {});
      return () => {
        cancelled = true;
        unlisten?.();
      };
    }
  };
}
