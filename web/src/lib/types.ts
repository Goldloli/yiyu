export type ThemePreference = "system" | "light" | "dark";
export type MotionPreference = "system" | "reduce" | "normal";
export type UiLanguage = "system" | "zh" | "en";
export type ProxyMode = "system" | "direct" | "manual";
export type TranslationSurface = "main" | "popup";
export type BackgroundChoice = "none" | "alpine-lake" | "spring-valley" | "coast" | "custom";

export interface ProviderConfig {
  id: string;
  label: string;
  baseUrl: string;
  model: string;
  systemPrompt: string | null;
  extraHeaders: Record<string, string>;
  isCustom: boolean;
  requiresApiKey: boolean;
}

export interface ProviderView extends ProviderConfig {
  configured: boolean;
}

export interface ProxyConfig {
  mode: ProxyMode;
  url: string | null;
}

export interface ShortcutConfig {
  selection: string | null;
  clipboard: string | null;
  openMain: string | null;
}

export interface Settings {
  version: number;
  onboardingCompleted: boolean;
  providers: ProviderConfig[];
  activeProviderId: string | null;
  sourceLanguage: string;
  targetLanguage: string;
  fallbackTargetLanguage: string;
  customTargetLanguage: string | null;
  systemPrompt: string | null;
  shortcuts: ShortcutConfig;
  captureClipboardFallback: boolean;
  autoSaveHistory: boolean;
  historyLimit: number;
  maxInputChars: number;
  providerProxy: ProxyConfig;
  updateProxy: ProxyConfig;
  checkUpdates: boolean;
  theme: ThemePreference;
  motion: MotionPreference;
  uiLanguage: UiLanguage;
  windowGlass: boolean;
  silentStart: boolean;
  backgroundId: string | null;
  backgroundChoice: BackgroundChoice | null;
  pendingCredentialCleanup: string[];
}

export interface SettingsSnapshot {
  revision: number;
  settings: Settings;
}

export interface SettingsPatch {
  onboardingCompleted?: boolean;
  providers?: ProviderConfig[];
  activeProviderId?: string | null;
  sourceLanguage?: string;
  targetLanguage?: string;
  fallbackTargetLanguage?: string;
  customTargetLanguage?: string | null;
  systemPrompt?: string | null;
  shortcuts?: ShortcutConfig;
  captureClipboardFallback?: boolean;
  autoSaveHistory?: boolean;
  historyLimit?: number;
  maxInputChars?: number;
  providerProxy?: ProxyConfig;
  updateProxy?: ProxyConfig;
  checkUpdates?: boolean;
  theme?: ThemePreference;
  motion?: MotionPreference;
  uiLanguage?: UiLanguage;
  windowGlass?: boolean;
  silentStart?: boolean;
  backgroundChoice?: BackgroundChoice;
}

export interface Language {
  code: string;
  nameZh: string;
  nameEn: string;
}

export interface LanguageDirection {
  sourceCode: string | null;
  sourceLabel: string;
  targetValue: string;
  targetLabel: string;
  confidence: number;
  uncertain: boolean;
}

export interface AppError {
  kind: string;
  message: string;
  retryable: boolean;
  field: string | null;
  actions: string[];
}

export interface PermissionStatus {
  accessibility: boolean;
  screenRecording: boolean;
}

export interface HistoryEntry {
  id: string;
  sourceText: string;
  translatedText: string;
  sourceLanguage: string;
  targetLanguage: string;
  providerId: string;
  providerLabel: string;
  model: string;
  favorite: boolean;
  durationMs: number;
  createdAt: number;
}

export interface HistoryFilter {
  query?: string | null;
  limit?: number | null;
  offset?: number | null;
}

export interface AppInfo {
  name: string;
  displayName: string;
  version: string;
  identifier: string;
  platform: string;
  arch: string;
  dataDir: string;
}

export interface UpdateSnapshot {
  status:
    | "idle"
    | "checking"
    | "upToDate"
    | "noRelease"
    | "available"
    | "downloading"
    | "verifying"
    | "ready"
    | "cancelled"
    | "error";
  currentVersion: string;
  version: string | null;
  notes: string | null;
  assetName: string | null;
  filePath: string | null;
  downloadedBytes: number;
  totalBytes: number | null;
  progress: number | null;
  bytesPerSecond: number | null;
  etaSeconds: number | null;
  error: AppError | null;
}

export interface Bootstrap {
  app: AppInfo;
  settings: SettingsSnapshot;
  settingsRecoveryBackup: string | null;
  shortcutRegistrationFailed: boolean;
  providers: ProviderView[];
  languages: Language[];
  history: HistoryEntry[];
  permissions: PermissionStatus;
  update: UpdateSnapshot;
}

export interface SettingsEnvelope {
  revision: number;
  settings: Settings;
  providers: ProviderView[];
}

export interface StartTranslationInput {
  surface: TranslationSurface;
  text: string;
  providerId?: string | null;
  manualSourceLanguage?: string | null;
  targetLanguage?: string | null;
  fallbackTargetLanguage?: string | null;
}

export type TranslationState =
  | "idle"
  | "preparing"
  | "streaming"
  | "completed"
  | "cancelled"
  | "failed";

export interface TranslationEvent {
  sessionId: string;
  surface: TranslationSurface;
  state: TranslationState;
  sourceText: string | null;
  text: string | null;
  direction: LanguageDirection | null;
  error: AppError | null;
  durationMs: number | null;
  historySaved: boolean | null;
  persistenceError: AppError | null;
}

export interface TranslationSession {
  sessionId: string;
  surface: TranslationSurface;
  sourceText: string;
  status: TranslationState;
  output: string;
  direction: LanguageDirection | null;
  error: AppError | null;
  durationMs: number | null;
  historySaved: boolean | null;
  persistenceError: AppError | null;
}

export interface ProviderTestResult {
  ok: boolean;
  latencyMs: number;
  message: string;
}

export interface Notice {
  id: number;
  level: "info" | "success" | "warning" | "error";
  code: string;
  message: string;
}

export const CONTRACT_COMMANDS = {
  getBootstrap: "get_bootstrap",
  getSettings: "get_settings",
  patchSettings: "patch_settings",
  getBackground: "get_background",
  setBackground: "set_background",
  clearBackground: "clear_background",
  saveProviderSecret: "save_provider_secret",
  deleteProviderSecret: "delete_provider_secret",
  retryCredentialCleanup: "retry_credential_cleanup",
  testProvider: "test_provider_connection",
  fetchModels: "fetch_provider_models",
  translateStart: "translate_start",
  translateCancel: "translate_cancel",
  captureSelection: "capture_selection",
  translateClipboard: "translate_clipboard",
  listHistory: "list_history",
  deleteHistory: "delete_history",
  clearHistory: "clear_history",
  retrySaveHistory: "retry_save_history",
  copyText: "copy_text",
  openExternal: "open_external",
  refreshPermissions: "refresh_permissions",
  requestAccessibility: "request_accessibility_permission",
  openAccessibilitySettings: "open_accessibility_settings",
  getAutostart: "get_autostart",
  setAutostart: "set_autostart",
  openMain: "open_main_window",
  openSettings: "open_settings_window",
  openOnboarding: "open_onboarding_window",
  closePopup: "close_popup",
  setPopupPinned: "set_popup_pinned",
  resizePopup: "resize_popup_window",
  quit: "quit_app",
  checkUpdate: "check_update",
  downloadUpdate: "download_update",
  cancelUpdate: "cancel_update_download",
  installUpdate: "install_update",
  getUpdateSnapshot: "get_update_snapshot",
  getDiagnostics: "get_diagnostics"
} as const;

export const CONTRACT_EVENTS = {
  translation: "translation://event",
  settings: "settings://changed",
  settingsNavigate: "settings://navigate",
  history: "history://changed",
  permission: "capture://permission",
  update: "update://state",
} as const;
