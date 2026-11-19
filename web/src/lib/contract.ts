import type {
  Bootstrap,
  HistoryEntry,
  HistoryFilter,
  ProviderConfig,
  ProviderTestResult,
  SettingsEnvelope,
  SettingsPatch,
  StartTranslationInput,
  UpdateSnapshot
} from "./types";

export interface TranslationStartResult {
  sessionId: string;
}

export interface Backend {
  getBootstrap(): Promise<Bootstrap>;
  getSettings(): Promise<SettingsEnvelope>;
  patchSettings(expectedRevision: number, patch: SettingsPatch): Promise<SettingsEnvelope>;
  getBackground(): Promise<string | null>;
  setBackground(expectedRevision: number, data: string): Promise<SettingsEnvelope>;
  clearBackground(expectedRevision: number): Promise<SettingsEnvelope>;
  saveProviderSecret(providerId: string, secret: string): Promise<void>;
  deleteProviderSecret(providerId: string): Promise<void>;
  retryCredentialCleanup(): Promise<SettingsEnvelope>;
  testProvider(provider: ProviderConfig, apiKey?: string | null): Promise<ProviderTestResult>;
  fetchModels(provider: ProviderConfig, apiKey?: string | null): Promise<string[]>;
  startTranslation(input: StartTranslationInput): Promise<TranslationStartResult>;
  cancelTranslation(surface: string): Promise<void>;
  captureSelection(): Promise<string>;
  translateClipboard(): Promise<TranslationStartResult>;
  listHistory(filter?: HistoryFilter): Promise<HistoryEntry[]>;
  deleteHistory(id: string): Promise<boolean>;
  clearHistory(): Promise<{ changed: number }>;
  retrySaveHistory(sessionId: string): Promise<HistoryEntry>;
  copyText(text: string): Promise<void>;
  openExternal(url: string): Promise<void>;
  refreshPermissions(): Promise<Bootstrap["permissions"]>;
  requestAccessibilityPermission(): Promise<void>;
  openAccessibilitySettings(): Promise<void>;
  getAutostart(): Promise<boolean>;
  setAutostart(enabled: boolean): Promise<boolean>;
  openMainWindow(): Promise<void>;
  openSettingsWindow(section?: string): Promise<void>;
  openOnboardingWindow(): Promise<void>;
  closePopup(): Promise<void>;
  setPopupPinned(pinned: boolean): Promise<void>;
  resizePopup(width: number, height: number): Promise<void>;
  quitApp(): Promise<void>;
  checkUpdate(force: boolean): Promise<UpdateSnapshot>;
  downloadUpdate(): Promise<UpdateSnapshot>;
  cancelUpdate(): Promise<void>;
  installUpdate(path: string): Promise<void>;
  getUpdateSnapshot(): Promise<UpdateSnapshot>;
  getDiagnostics(): Promise<Record<string, unknown>>;
  on<T>(event: string, handler: (payload: T) => void): () => void;
}
