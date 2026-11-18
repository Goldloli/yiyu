import { describe, expect, it } from "vitest";
import { MockBackend } from "../src/lib/mock-backend";
import {
  CONTRACT_COMMANDS,
  CONTRACT_EVENTS,
  type ProviderConfig,
  type TranslationEvent
} from "../src/lib/types";

const expectedCommands = {
  getBootstrap: "get_bootstrap",
  getSettings: "get_settings",
  patchSettings: "patch_settings",
  getBackground: "get_background",
  setBackground: "set_background",
  clearBackground: "clear_background",
  saveProviderSecret: "save_provider_secret",
  deleteProviderSecret: "delete_provider_secret",
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

describe("IPC contract", () => {
  it("keeps command identifiers unique, snake_case and stable", () => {
    expect(CONTRACT_COMMANDS).toEqual(expectedCommands);
    const names = Object.values(CONTRACT_COMMANDS);
    expect(new Set(names).size).toBe(names.length);
    expect(names.every((name) => /^([a-z0-9]+_)*[a-z0-9]+$/.test(name))).toBe(
      true
    );
  });

  it("keeps every backend method represented by the mock", () => {
    const backend = new MockBackend();
    const methods = Object.getOwnPropertyNames(
      Object.getPrototypeOf(backend)
    );
    const expectedMethods = [
      "cancelTranslation",
      "cancelUpdate",
      "captureSelection",
      "checkUpdate",
      "clearHistory",
      "clearBackground",
      "closePopup",
      "copyText",
      "deleteHistory",
      "deleteProviderSecret",
      "downloadUpdate",
      "fetchModels",
      "getBootstrap",
      "getAutostart",
      "getBackground",
      "getDiagnostics",
      "getSettings",
      "getUpdateSnapshot",
      "installUpdate",
      "listHistory",
      "on",
      "openExternal",
      "openAccessibilitySettings",
      "openMainWindow",
      "openOnboardingWindow",
      "openSettingsWindow",
      "patchSettings",
      "quitApp",
      "refreshPermissions",
      "requestAccessibilityPermission",
      "resizePopup",
      "retrySaveHistory",
      "saveProviderSecret",
      "setPopupPinned",
      "setBackground",
      "setAutostart",
      "startTranslation",
      "testProvider",
      "translateClipboard"
    ];
    for (const method of expectedMethods) {
      expect(methods).toContain(method);
    }
  });

  it("publishes managed background changes and rejects stale revisions", async () => {
    const backend = new MockBackend();
    const events: unknown[] = [];
    backend.on(CONTRACT_EVENTS.settings, (event) => events.push(event));
    const png = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+nmhUAAAAASUVORK5CYII=";
    const applied = await backend.setBackground(0, png);
    expect(applied.settings.backgroundId).toBeTruthy();
    expect(await backend.getBackground()).toBe(`data:image/png;base64,${png}`);
    await expect(backend.clearBackground(0)).rejects.toBeTruthy();
    expect(await backend.getBackground()).toBeTruthy();
    const cleared = await backend.clearBackground(applied.revision);
    expect(cleared.settings.backgroundId).toBeNull();
    expect(await backend.getBackground()).toBeNull();
    expect(events).toHaveLength(2);
  });

  it("serializes provider and translation event fields with camelCase keys", () => {
    const provider: ProviderConfig = {
      id: "test",
      label: "Test",
      baseUrl: "https://example.com/v1",
      model: "test-model",
      systemPrompt: null,
      extraHeaders: {},
      isCustom: false,
      requiresApiKey: true
    };
    expect(Object.keys(provider).sort()).toEqual(
      [
        "id",
        "label",
        "baseUrl",
        "model",
        "systemPrompt",
        "extraHeaders",
        "isCustom",
        "requiresApiKey"
      ].sort()
    );

    const event: TranslationEvent = {
      sessionId: "session",
      surface: "main",
      state: "streaming",
      sourceText: "hello",
      text: "你好",
      direction: null,
      error: null,
      durationMs: null,
      historySaved: null,
      persistenceError: null
    };
    expect(Object.keys(event).sort()).toEqual(
      [
        "sessionId",
        "surface",
        "state",
        "sourceText",
        "text",
        "direction",
        "error",
        "durationMs",
        "historySaved",
        "persistenceError"
      ].sort()
    );
    expect(CONTRACT_EVENTS).toEqual({
      translation: "translation://event",
      settings: "settings://changed",
      settingsNavigate: "settings://navigate",
      history: "history://changed",
      permission: "capture://permission",
      update: "update://state",
    });
  });
});
