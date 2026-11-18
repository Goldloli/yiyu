import {
  Check,
  Cloud,
  Database,
  Download,
  Eye,
  EyeOff,
  Globe2,
  Info,
  KeyRound,
  Keyboard,
  Palette,
  Laptop,
  LoaderCircle,
  Moon,
  Plus,
  RefreshCw,
  SlidersHorizontal,
  Sparkles,
  Sun,
  Trash2,
  Wrench
} from "lucide-react";
import {
  useEffect,
  useMemo,
  useState,
  type ReactNode
} from "react";
import { Button } from "../components/Button";
import { Dialog } from "../components/Dialog";
import { BackgroundSettings } from "../components/BackgroundSettings";
import { Field } from "../components/Field";
import { IconButton } from "../components/IconButton";
import { SegmentedControl } from "../components/SegmentedControl";
import { SelectMenu } from "../components/SelectMenu";
import { ShortcutRecorder } from "../components/ShortcutRecorder";
import { TitleBar } from "../components/TitleBar";
import { WindowBackdrop } from "../components/WindowBackdrop";
import { Toggle } from "../components/Toggle";
import { getBackend, platformName } from "../lib/api";
import {
  formatBytes,
  formatEta,
  formatRate,
  providerInitials
} from "../lib/format";
import { languageName, t } from "../lib/i18n";
import { RECOMMENDED_MODELS } from "../lib/model-presets";
import { useUiLanguage, useYiyuStore } from "../lib/store";
import type {
  ProviderConfig,
  ProviderTestResult,
  ProxyConfig,
  SettingsPatch
} from "../lib/types";
import { CONTRACT_EVENTS } from "../lib/types";

type SettingsSection = "models" | "general" | "appearance" | "shortcuts" | "network" | "updates" | "about";

function PageHeading({
  title,
  action
}: {
  title: string;
  action?: ReactNode;
}) {
  return (
    <div className="mb-5 flex items-start gap-4">
      <div className="min-w-0 flex-1">
        <h1 className="text-[20px] font-semibold tracking-[-0.01em]">{title}</h1>
      </div>
      {action}
    </div>
  );
}

function PropertyRow({
  title,
  description,
  control
}: {
  title: string;
  description?: string;
  control: ReactNode;
}) {
  return (
    <div className="property-row">
      <div className="min-w-0">
        <p className="text-[13px] font-medium">{title}</p>
        {description && (
          <p className="mt-0.5 text-[13px] leading-4 text-tertiary">{description}</p>
        )}
      </div>
      <div className="shrink-0">{control}</div>
    </div>
  );
}

function ProviderList({
  selected,
  onSelect,
  onAdd
}: {
  selected: string;
  onSelect: (id: string) => void;
  onAdd: () => void;
}) {
  const ui = useUiLanguage();
  const providers = useYiyuStore((state) => state.providers);
  const activeId = useYiyuStore((state) => state.settings?.activeProviderId);
  return (
    <div className="provider-list" style={{ borderColor: "var(--line)" }}>
      <div className="mb-3 flex items-center justify-between">
        <span className="section-label">{t("settings.modelsTitle", undefined, ui)}</span>
        <IconButton
          label={ui === "en" ? "Add custom provider" : "添加自定义服务"}
          className="!h-7 !w-7"
          onClick={onAdd}
        >
          <Plus size={13} />
        </IconButton>
      </div>
      <div className="space-y-1">
        {providers.map((provider) => (
          <button
            type="button"
            key={provider.id}
            className="flex w-full items-center gap-2.5 rounded-[7px] border px-2.5 py-2 text-left transition-colors"
            style={{
              borderColor: "transparent",
              background: provider.id === selected ? "var(--selected-surface)" : "transparent"
            }}
            onClick={() => onSelect(provider.id)}
          >
            <span
              className="grid h-7 w-7 shrink-0 place-items-center rounded-[6px] text-[11px] font-bold"
              style={{
                color: "var(--text-secondary)",
                background: "var(--bg-hover)"
              }}
            >
              {providerInitials(provider.label)}
            </span>
            <span className="min-w-0 flex-1">
              <span className="flex items-center gap-1.5">
                <span className="truncate text-[13px] font-medium">{provider.label}</span>
                {provider.id === activeId && (
                  <span className="provider-active" role="img" aria-label={t("settings.active", undefined, ui)} title={t("settings.active", undefined, ui)} />
                )}
              </span>
              <span className="mt-0.5 block truncate text-[12px] text-tertiary">
                {provider.model}
              </span>
            </span>
          </button>
        ))}
      </div>
    </div>
  );
}

function ModelsSection({
  selected,
  setSelected
}: {
  selected: string;
  setSelected: (id: string) => void;
}) {
  const ui = useUiLanguage();
  const providers = useYiyuStore((state) => state.providers);
  const settings = useYiyuStore((state) => state.settings);
  const patchSettings = useYiyuStore((state) => state.patchSettings);
  const saveSecret = useYiyuStore((state) => state.saveProviderSecret);
  const deleteSecret = useYiyuStore((state) => state.deleteProviderSecret);
  const testProvider = useYiyuStore((state) => state.testProvider);
  const fetchModels = useYiyuStore((state) => state.fetchModels);
  const provider = providers.find((item) => item.id === selected) ?? providers[0];
  const [draft, setDraft] = useState<ProviderConfig | null>(provider ?? null);
  const [apiKey, setApiKey] = useState("");
  const [visibleKey, setVisibleKey] = useState(false);
  const [models, setModels] = useState<string[]>([]);
  const [loadingModels, setLoadingModels] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<ProviderTestResult | null>(null);
  const [saving, setSaving] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);

  useEffect(() => {
    if (!provider) return;
    setDraft(provider);
    setApiKey("");
    setModels([]);
    setTestResult(null);
  }, [provider?.id]);

  if (!provider || !draft) return null;
  const currentView = providers.find((item) => item.id === draft.id) ?? provider;

  const applyDraft = async () => {
    setSaving(true);
    try {
      if (apiKey.trim()) await saveSecret(draft.id, apiKey);
      if (settings?.activeProviderId === null && draft.requiresApiKey) {
        await patchSettings({ activeProviderId: draft.id });
      }
      await patchSettings({
        providers: providers.map((item) =>
          item.id === draft.id ? draft : stripViewFields(item)
        ),
        activeProviderId: settings?.activeProviderId ?? draft.id
      });
    } finally {
      setSaving(false);
    }
  };

  const deleteProvider = async () => {
    if (!settings || !draft.isCustom) return;
    const remaining = providers
      .filter((item) => item.id !== draft.id)
      .map((item) => stripViewFields(item));
    if (remaining.length === 0) return;
    const nextActive =
      settings.activeProviderId === draft.id
        ? remaining.find((item) => !item.requiresApiKey || providers.find((view) => view.id === item.id)?.configured)?.id
          ?? remaining[0].id
        : settings.activeProviderId;
    setSaving(true);
    try {
      await patchSettings({
        providers: remaining,
        activeProviderId: nextActive
      });
      setSelected(nextActive ?? remaining[0].id);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="flex min-h-0 flex-1">
      <ProviderList
        selected={selected}
        onSelect={setSelected}
        onAdd={() => {
          const id = `custom-${crypto.randomUUID?.() ?? Date.now()}`;
          const custom: ProviderConfig = {
            id,
            label: ui === "en" ? "Custom provider" : "自定义服务",
            baseUrl: "https://api.example.com/v1",
            model: "model-name",
            systemPrompt: null,
            extraHeaders: {},
            isCustom: true,
            requiresApiKey: true
          };
          void patchSettings({ providers: [...providers.map(stripViewFields), custom] }).then(() => {
            setSelected(id);
          });
        }}
      />
      <div className="min-w-0 flex-1 overflow-y-auto pl-6">
        <PageHeading
          title={draft.label}
          action={
            <div className="flex items-center gap-2">
              <Button
                variant="secondary"
                icon={
                  settings?.activeProviderId === draft.id ? (
                    <Check size={12} />
                  ) : (
                    <Sparkles size={12} />
                  )
                }
                onClick={() => void patchSettings({ activeProviderId: draft.id })}
              >
                {settings?.activeProviderId === draft.id
                  ? t("settings.activeBadge", undefined, ui)
                  : t("settings.active", undefined, ui)}
              </Button>
            </div>
          }
        />
        <div className="space-y-4 pb-8">
          <Field label={ui === "en" ? "Display name" : "显示名称"}>
            <input
              className="control w-full px-3"
              value={draft.label}
              onChange={(event) => setDraft({ ...draft, label: event.target.value })}
            />
          </Field>
          <Field label={t("settings.baseUrl", undefined, ui)}>
            <input
              className="control mono w-full px-3 text-[13px]"
              value={draft.baseUrl}
              onChange={(event) => setDraft({ ...draft, baseUrl: event.target.value })}
              spellCheck={false}
            />
          </Field>
          <Field
            label={t("settings.apiKey", undefined, ui)}
            hint={
              !draft.requiresApiKey
                ? (ui === "en" ? "Optional; leave blank to keep any saved key." : "选填；留空时保留已有密钥。")
                : currentView.configured
                ? t("settings.apiKeySaved", undefined, ui)
                : t("settings.unconfigured", undefined, ui)
            }
          >
            <div className="flex gap-2">
              <div className="relative min-w-0 flex-1">
                <KeyRound size={13} className="absolute left-2.5 top-1/2 -translate-y-1/2 text-tertiary" />
                <input
                  className="control mono w-full px-8 text-[13px]"
                  type={visibleKey ? "text" : "password"}
                  value={apiKey}
                  placeholder={
                    currentView.configured && draft.requiresApiKey ? "••••••••••••" : t("settings.apiKeyPlaceholder", undefined, ui)
                  }
                  onChange={(event) => setApiKey(event.target.value)}
                  autoComplete="off"
                  spellCheck={false}
                />
                <IconButton
                  label={visibleKey ? (ui === "en" ? "Hide key" : "隐藏 Key") : (ui === "en" ? "Show key" : "显示 Key")}
                  className="!absolute right-1 top-0.5 !h-7 !w-7"
                  onClick={() => setVisibleKey((value) => !value)}
                >
                  {visibleKey ? <EyeOff size={12} /> : <Eye size={12} />}
                </IconButton>
              </div>
              {(currentView.configured || !draft.requiresApiKey) && (
                <Button
                  variant="danger"
                  icon={<Trash2 size={12} />}
                  onClick={() => void deleteSecret(draft.id)}
                >
                  {t("common.delete", undefined, ui)}
                </Button>
              )}
            </div>
            <p className="mt-1.5 text-[13px] text-tertiary">{t("settings.savedToKeychain", undefined, ui)}</p>
          </Field>
          <Field label={t("settings.model", undefined, ui)}>
            <div className="flex gap-2">
              {models.length > 0 ? (
                <SelectMenu
                  value={draft.model}
                  options={[...new Set([draft.model, ...models])].map((model) => ({ value: model, label: model }))}
                  onChange={(model) => setDraft({ ...draft, model })}
                  className="min-w-0 flex-1"
                />
              ) : (
                <input
                  className="control mono min-w-0 flex-1 px-3 text-[13px]"
                  value={draft.model}
                  onChange={(event) => setDraft({ ...draft, model: event.target.value })}
                  spellCheck={false}
                />
              )}
              <Button
                icon={loadingModels ? <LoaderCircle size={12} className="animate-spin" /> : <RefreshCw size={12} />}
                disabled={loadingModels}
                onClick={async () => {
                  setLoadingModels(true);
                  try {
                    const fetched = await fetchModels(draft, apiKey || undefined);
                    setModels(fetched);
                  } catch (error) {
                    setTestResult({
                      ok: false,
                      latencyMs: 0,
                      message: (error as { message?: string }).message ?? "Model list failed"
                    });
                  } finally {
                    setLoadingModels(false);
                  }
                }}
              >
                {t("settings.fetchModels", undefined, ui)}
              </Button>
            </div>
          </Field>
          {RECOMMENDED_MODELS[draft.id] && (
            <div className="model-recommendation">
              <span>{ui === "en" ? "Suggested model" : "推荐模型"} · {RECOMMENDED_MODELS[draft.id]}</span>
              <button type="button" onClick={() => { setModels([]); setDraft({ ...draft, model: RECOMMENDED_MODELS[draft.id] }); }}>
                {ui === "en" ? "Use" : "使用"}
              </button>
            </div>
          )}
          <div>
            <Field
              label={t("settings.systemPrompt", undefined, ui)}
              hint={t("settings.promptHint", undefined, ui)}
            >
              <textarea
                className="control mono min-h-[104px] w-full resize-y px-3 py-2 text-[13px] leading-5"
                value={draft.systemPrompt ?? ""}
                placeholder={t("settings.promptPlaceholder", undefined, ui)}
                onChange={(event) =>
                  setDraft({
                    ...draft,
                    systemPrompt: event.target.value || null
                  })
                }
              />
            </Field>
            {draft.systemPrompt && (
              <div className="mt-1.5 flex justify-end">
                <Button
                  variant="ghost"
                  className="!min-h-7 !px-2 text-[13px]"
                  onClick={() => setDraft({ ...draft, systemPrompt: null })}
                >
                  {t("settings.restorePrompt", undefined, ui)}
                </Button>
              </div>
            )}
          </div>
          <div className="flex items-center gap-2 border-t pt-4" style={{ borderColor: "var(--line)" }}>
            <Button
              icon={
                testing ? (
                  <LoaderCircle size={12} className="animate-spin" />
                ) : (
                  <Wrench size={12} />
                )
              }
              disabled={testing}
              onClick={async () => {
                setTesting(true);
                setTestResult(null);
                try {
                  setTestResult(await testProvider(draft, apiKey || undefined));
                } finally {
                  setTesting(false);
                }
              }}
            >
              {testing ? t("settings.testing", undefined, ui) : t("settings.test", undefined, ui)}
            </Button>
            {draft.isCustom && (
              <Button variant="danger" icon={<Trash2 size={12} />} disabled={saving} onClick={() => setConfirmDelete(true)}>
                {t("settings.deleteProvider", undefined, ui)}
              </Button>
            )}
            {testResult && (
              <span
                className="text-[13px]"
                style={{ color: testResult.ok ? "var(--success)" : "var(--danger)" }}
              >
                {testResult.ok
                  ? `${testResult.message} · ${testResult.latencyMs}ms`
                  : testResult.message}
              </span>
            )}
            <Button
              variant="primary"
              className="ml-auto"
              disabled={saving}
              onClick={() => void applyDraft()}
            >
              {t("common.save", undefined, ui)}
            </Button>
          </div>
        </div>
      </div>
      {confirmDelete && (
        <Dialog title={ui === "en" ? "Delete this provider and its API key?" : "删除此服务及其 API Key？"} onClose={() => setConfirmDelete(false)}>
          <div className="app-dialog-actions">
            <Button onClick={() => setConfirmDelete(false)}>{t("common.cancel", undefined, ui)}</Button>
            <Button variant="danger" onClick={() => { setConfirmDelete(false); void deleteProvider(); }}>
              {t("common.delete", undefined, ui)}
            </Button>
          </div>
        </Dialog>
      )}
    </div>
  );
}

function stripViewFields(provider: ProviderConfig & { configured?: boolean }): ProviderConfig {
  const { configured: _configured, ...config } = provider;
  return config;
}

function AppearanceSection() {
  const ui = useUiLanguage();
  const settings = useYiyuStore((state) => state.settings);
  const patchSettings = useYiyuStore((state) => state.patchSettings);
  if (!settings) return null;
  return (
    <div className="settings-section">
      <PageHeading title={ui === "en" ? "Appearance" : "外观"} />
      <div className="mb-6 border-t" style={{ borderColor: "var(--line)" }}>
        <PropertyRow
          title={t("settings.theme", undefined, ui)}
          control={
            <SegmentedControl
              label={t("settings.theme", undefined, ui)}
              value={settings.theme}
              onChange={(theme) => void patchSettings({ theme })}
              options={[
                { value: "system", label: t("settings.system", undefined, ui) },
                { value: "light", label: t("settings.light", undefined, ui) },
                { value: "dark", label: t("settings.dark", undefined, ui) }
              ]}
            />
          }
        />
        <BackgroundSettings />
        <PropertyRow
          title={ui === "en" ? "Glass effect" : "玻璃效果"}
          control={<Toggle checked={settings.windowGlass} onChange={(windowGlass) => void patchSettings({ windowGlass })} label={ui === "en" ? "Glass effect" : "玻璃效果"} />}
        />
        <PropertyRow
          title={t("settings.motion", undefined, ui)}
          control={
            <SegmentedControl
              label={t("settings.motion", undefined, ui)}
              value={settings.motion}
              onChange={(motion) => void patchSettings({ motion })}
              options={[
                { value: "system", label: t("settings.system", undefined, ui) },
                { value: "reduce", label: t("settings.reduceMotion", undefined, ui) },
                { value: "normal", label: t("settings.normalMotion", undefined, ui) }
              ]}
            />
          }
        />
      </div>
    </div>
  );
}

function GeneralSection() {
  const ui = useUiLanguage();
  const settings = useYiyuStore((state) => state.settings);
  const languages = useYiyuStore((state) => state.languages);
  const permissions = useYiyuStore((state) => state.permissions);
  const patchSettings = useYiyuStore((state) => state.patchSettings);
  const notify = useYiyuStore((state) => state.notify);
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [autostartPending, setAutostartPending] = useState(false);
  const [permissionRequested, setPermissionRequested] = useState(false);
  const [historyLimit, setHistoryLimit] = useState(String(settings?.historyLimit ?? 500));
  const [maxInput, setMaxInput] = useState(String(settings?.maxInputChars ?? 20_000));

  useEffect(() => { if (settings) setHistoryLimit(String(settings.historyLimit)); }, [settings?.historyLimit]);
  useEffect(() => { if (settings) setMaxInput(String(settings.maxInputChars)); }, [settings?.maxInputChars]);
  useEffect(() => {
    let active = true;
    const refresh = () => {
      void getBackend().getAutostart().then((value) => { if (active) setAutostart(value); }).catch(() => { if (active) setAutostart(null); });
    };
    refresh();
    window.addEventListener("focus", refresh);
    return () => { active = false; window.removeEventListener("focus", refresh); };
  }, []);

  if (!settings) return null;
  const targetOptions = languages.map((language) => ({
    value: language.code,
    label: languageName(language, ui)
  }));

  return (
    <div className="min-w-0 flex-1 overflow-y-auto pl-1">
      <PageHeading title={t("settings.generalTitle", undefined, ui)} />
      <div className="mb-6 border-t" style={{ borderColor: "var(--line)" }}>
        <PropertyRow
          title={ui === "en" ? "Launch at login" : "开机启动"}
          control={<Toggle label={ui === "en" ? "Launch at login" : "开机启动"} checked={autostart ?? false} disabled={autostart === null || autostartPending} onChange={(enabled) => {
            setAutostartPending(true);
            void getBackend().setAutostart(enabled).then(setAutostart).catch((error) => notify("error", String(error?.message ?? error))).finally(() => setAutostartPending(false));
          }} />}
        />
        <PropertyRow
          title={ui === "en" ? "Hide main window on launch" : "启动时隐藏主窗口"}
          control={<Toggle label={ui === "en" ? "Hide main window on launch" : "启动时隐藏主窗口"} checked={settings.silentStart} onChange={(silentStart) => void patchSettings({ silentStart })} />}
        />
      </div>
      <div className="mb-6 border-t" style={{ borderColor: "var(--line)" }}>
        <PropertyRow
          title={t("settings.uiLanguage", undefined, ui)}
          control={
            <SegmentedControl
              label={t("settings.uiLanguage", undefined, ui)}
              value={settings.uiLanguage}
              onChange={(uiLanguage) => void patchSettings({ uiLanguage })}
              options={[
                { value: "system", label: t("settings.system", undefined, ui) },
                { value: "zh", label: "简体中文" },
                { value: "en", label: "English" }
              ]}
            />
          }
        />
      </div>
      <div className="mb-6">
        <p className="section-label mb-2">{t("settings.targetDefaults", undefined, ui)}</p>
        <div className="grid grid-cols-2 gap-4 border-t pt-3" style={{ borderColor: "var(--line)" }}>
          <Field label={t("main.target", undefined, ui)}>
            <SelectMenu
              value={settings.targetLanguage}
              options={targetOptions}
              onChange={(targetLanguage) => void patchSettings({ targetLanguage, customTargetLanguage: null })}
            />
          </Field>
          <Field label={t("settings.fallback", undefined, ui)}>
            <SelectMenu
              value={settings.fallbackTargetLanguage}
              options={targetOptions}
              onChange={(fallbackTargetLanguage) => void patchSettings({ fallbackTargetLanguage })}
            />
          </Field>
        </div>
      </div>
      <div className="mb-6">
        <p className="section-label mb-2">{t("settings.history", undefined, ui)}</p>
        <div className="border-t" style={{ borderColor: "var(--line)" }}>
          <PropertyRow
            title={t("settings.autoSave", undefined, ui)}
            control={
              <Toggle
                label={t("settings.autoSave", undefined, ui)}
                checked={settings.autoSaveHistory}
                onChange={(autoSaveHistory) => void patchSettings({ autoSaveHistory })}
              />
            }
          />
          <PropertyRow
            title={t("settings.historyLimit", undefined, ui)}
            description="10–10,000"
            control={
              <input
                type="number"
                className="control w-[100px] px-2 text-right text-[13px]"
                min={10}
                max={10_000}
                value={historyLimit}
                onChange={(event) => setHistoryLimit(event.target.value)}
                onBlur={() => {
                  const value = Number(historyLimit);
                  if (Number.isFinite(value) && value >= 10 && value <= 10_000) {
                    void patchSettings({ historyLimit: value });
                  } else {
                    setHistoryLimit(String(settings.historyLimit));
                  }
                }}
              />
            }
          />
          <PropertyRow
            title={t("settings.maxInput", undefined, ui)}
            description="1,000–200,000"
            control={
              <input
                type="number"
                className="control w-[100px] px-2 text-right text-[13px]"
                min={1_000}
                max={200_000}
                inputMode="numeric"
                value={maxInput}
                onChange={(event) => setMaxInput(event.target.value)}
                onBlur={() => {
                  const value = Number(maxInput);
                  if (Number.isFinite(value) && value >= 1_000 && value <= 200_000) {
                    void patchSettings({ maxInputChars: value });
                  } else {
                    setMaxInput(String(settings.maxInputChars));
                  }
                }}
              />
            }
          />
        </div>
      </div>
      <div className="mb-6">
        <p className="section-label mb-2">{t("settings.capture", undefined, ui)}</p>
        <div className="border-t" style={{ borderColor: "var(--line)" }}>
          <PropertyRow
            title={t("settings.clipboardFallback", undefined, ui)}
            description={
              ui === "en"
                ? "Used only when native accessibility selection reading fails. The clipboard is restored."
                : "仅在原生 Accessibility 取词失败时使用，并恢复原剪贴板内容。"
            }
            control={
              <Toggle
                label={t("settings.clipboardFallback", undefined, ui)}
                checked={settings.captureClipboardFallback}
                onChange={(captureClipboardFallback) =>
                  void patchSettings({ captureClipboardFallback })
                }
              />
            }
          />
          <PropertyRow
            title={t("settings.accessibility", undefined, ui)}
            description={permissionRequested && !permissions.accessibility
              ? ui === "en"
                ? "If already enabled in System Settings, remove the old Yiyu entry, add this version, and restart Yiyu if needed."
                : "若系统中已开启，请移除旧的译羽条目并添加当前版本；必要时重启译羽。"
              : undefined}
            control={
              <div className="flex items-center gap-2">
                <span className="status-dot" data-state={permissions.accessibility ? "ready" : "warning"} />
                <span className="text-[13px]">
                  {permissions.accessibility
                    ? t("settings.accessibilityGranted", undefined, ui)
                    : t("settings.accessibilityMissing", undefined, ui)}
                </span>
                {!permissions.accessibility && (
                  <Button
                    className="!min-h-7 !px-2 text-[13px]"
                    onClick={() => {
                      setPermissionRequested(true);
                      void getBackend().openAccessibilitySettings().catch((error) => notify("error", String(error?.message ?? error)));
                    }}
                  >
                    {t("settings.openSystemSettings", undefined, ui)}
                  </Button>
                )}
              </div>
            }
          />
        </div>
      </div>

    </div>
  );
}

function ShortcutsSection() {
  const ui = useUiLanguage();
  const settings = useYiyuStore((state) => state.settings);
  const patchSettings = useYiyuStore((state) => state.patchSettings);
  const [shortcuts, setShortcuts] = useState(settings?.shortcuts);
  useEffect(() => { if (settings) setShortcuts(settings.shortcuts); }, [settings?.shortcuts.selection, settings?.shortcuts.clipboard, settings?.shortcuts.openMain]);
  if (!settings || !shortcuts) return null;
  return (
    <div className="settings-section shortcuts-section">
      <PageHeading title={ui === "en" ? "Shortcuts" : "快捷键"} />
      <div className="mb-6">
        <div className="shortcut-list border-t pt-3" style={{ borderColor: "var(--line)" }}>
          <Field label={t("settings.selectionShortcut", undefined, ui)}>
            <ShortcutRecorder
              label={t("settings.selectionShortcut", undefined, ui)}
              value={shortcuts.selection ?? null}
              recordingLabel={t("settings.recordShortcut", undefined, ui)}
              clearLabel={t("settings.clearShortcut", undefined, ui)}
              platform={platformName()}
              onChange={(selection) => setShortcuts({ ...shortcuts, selection })}
            />
          </Field>
          <Field label={t("settings.clipboardShortcut", undefined, ui)}>
            <ShortcutRecorder
              label={t("settings.clipboardShortcut", undefined, ui)}
              value={shortcuts.clipboard ?? null}
              recordingLabel={t("settings.recordShortcut", undefined, ui)}
              clearLabel={t("settings.clearShortcut", undefined, ui)}
              platform={platformName()}
              onChange={(clipboard) => setShortcuts({ ...shortcuts, clipboard })}
            />
          </Field>
          <Field label={t("settings.mainShortcut", undefined, ui)}>
            <ShortcutRecorder
              label={t("settings.mainShortcut", undefined, ui)}
              value={shortcuts.openMain ?? null}
              recordingLabel={t("settings.recordShortcut", undefined, ui)}
              clearLabel={t("settings.clearShortcut", undefined, ui)}
              platform={platformName()}
              onChange={(openMain) => setShortcuts({ ...shortcuts, openMain })}
            />
          </Field>
        </div>
        <div className="mt-3 flex justify-end">
          <Button onClick={() => void patchSettings({ shortcuts })}>
            {t("common.save", undefined, ui)}
          </Button>
        </div>
      </div>
    </div>
  );
}


function ProxyEditor({
  title,
  description,
  proxy,
  setProxy
}: {
  title: string;
  description: string;
  proxy: ProxyConfig;
  setProxy: (proxy: ProxyConfig) => void;
}) {
  const ui = useUiLanguage();
  return (
    <div className="border-t py-4" style={{ borderColor: "var(--line)" }}>
      <div className="mb-3 flex items-center justify-between gap-4">
        <div>
          <p className="text-[13px] font-medium">{title}</p>
          <p className="mt-0.5 text-[13px] leading-4 text-tertiary">{description}</p>
        </div>
        <SegmentedControl
          label={title}
          value={proxy.mode}
          onChange={(mode) => setProxy({ ...proxy, mode })}
          options={[
            { value: "system", label: t("settings.system", undefined, ui) },
            { value: "direct", label: t("settings.direct", undefined, ui) },
            { value: "manual", label: t("settings.manual", undefined, ui) }
          ]}
        />
      </div>
      {proxy.mode === "manual" && (
        <Field label={t("settings.proxyUrl", undefined, ui)} hint={t("settings.proxyHint", undefined, ui)}>
          <input
            className="control mono w-full px-3 text-[13px]"
            value={proxy.url ?? ""}
            placeholder="http://127.0.0.1:7890"
            onChange={(event) => setProxy({ ...proxy, url: event.target.value || null })}
            spellCheck={false}
          />
        </Field>
      )}
    </div>
  );
}

function NetworkSection() {
  const ui = useUiLanguage();
  const settings = useYiyuStore((state) => state.settings);
  const patchSettings = useYiyuStore((state) => state.patchSettings);
  const [providerProxy, setProviderProxy] = useState<ProxyConfig>(
    settings?.providerProxy ?? { mode: "system", url: null }
  );
  const [updateProxy, setUpdateProxy] = useState<ProxyConfig>(
    settings?.updateProxy ?? { mode: "system", url: null }
  );
  useEffect(() => {
    setProviderProxy(settings?.providerProxy ?? { mode: "system", url: null });
  }, [settings?.providerProxy.mode, settings?.providerProxy.url]);
  useEffect(() => {
    setUpdateProxy(settings?.updateProxy ?? { mode: "system", url: null });
  }, [settings?.updateProxy.mode, settings?.updateProxy.url]);
  if (!settings) return null;
  return (
    <div className="min-w-0 flex-1 overflow-y-auto pl-1">
      <PageHeading title={t("settings.networkTitle", undefined, ui)} />
      <ProxyEditor
        title={t("settings.providerProxy", undefined, ui)}
        description={
          ui === "en"
            ? "Used for model requests. localhost endpoints stay direct."
            : "用于模型请求，localhost 地址始终直连。"
        }
        proxy={providerProxy}
        setProxy={setProviderProxy}
      />
      <ProxyEditor
        title={t("settings.updateProxy", undefined, ui)}
        description={
          ui === "en"
            ? "Used only for GitHub metadata, checksums and installer downloads."
            : "仅用于 GitHub 元数据、校验文件与安装包下载。"
        }
        proxy={updateProxy}
        setProxy={setUpdateProxy}
      />
      <div className="flex justify-end border-t pt-4" style={{ borderColor: "var(--line)" }}>
        <Button
          variant="primary"
          onClick={() => void patchSettings({ providerProxy, updateProxy })}
        >
          {t("common.save", undefined, ui)}
        </Button>
      </div>
    </div>
  );
}

function UpdateSection() {
  const ui = useUiLanguage();
  const settings = useYiyuStore((state) => state.settings);
  const app = useYiyuStore((state) => state.app);
  const update = useYiyuStore((state) => state.update);
  const checkUpdate = useYiyuStore((state) => state.checkUpdate);
  const downloadUpdate = useYiyuStore((state) => state.downloadUpdate);
  const cancelUpdate = useYiyuStore((state) => state.cancelUpdate);
  const installUpdate = useYiyuStore((state) => state.installUpdate);
  const patchSettings = useYiyuStore((state) => state.patchSettings);
  if (!settings || !app) return null;
  const busy =
    update?.status === "checking" ||
    update?.status === "downloading" ||
    update?.status === "verifying";
  return (
    <div className="min-w-0 flex-1 overflow-y-auto pl-1">
      <PageHeading title={t("settings.updatesTitle", undefined, ui)} />
      <div className="border-y py-5" style={{ borderColor: "var(--line)" }}>
        <div className="flex items-center gap-3">
          <div className="grid h-10 w-10 place-items-center rounded-[8px] bg-[var(--accent-weak)] text-accent">
            <Download size={18} />
          </div>
          <div>
            <p className="text-[13px] font-semibold">
              {update?.status === "available"
                ? t("settings.updateAvailable", { version: update.version ?? "" }, ui)
                : update?.status === "upToDate"
                  ? t("settings.upToDate", undefined, ui)
                  : update?.status === "noRelease"
                    ? t("settings.noRelease", undefined, ui)
                  : update?.status === "ready"
                    ? t("settings.readyToInstall", undefined, ui)
                    : update?.status === "error"
                      ? update.error?.message
                      : t("settings.currentVersion", undefined, ui)}
            </p>
            <p className="mt-0.5 text-[13px] text-tertiary">
              Yiyu {app.version} · {app.platform === "macos" ? "Apple Silicon" : "x64"}
            </p>
          </div>
          <div className="ml-auto flex items-center gap-2">
            {update?.status === "available" && (
              <Button variant="primary" onClick={() => void downloadUpdate()}>
                {t("settings.download", undefined, ui)}
              </Button>
            )}
            {update?.status === "ready" && (
              <Button variant="primary" onClick={() => void installUpdate()}>
                {t("settings.install", undefined, ui)}
              </Button>
            )}
            {update?.status === "downloading" && (
              <Button onClick={() => void cancelUpdate()}>
                {t("common.cancel", undefined, ui)}
              </Button>
            )}
            {!busy && update?.status !== "ready" && (
              <Button
                icon={<RefreshCw size={12} />}
                onClick={() => void checkUpdate(true)}
              >
                {t("settings.checkNow", undefined, ui)}
              </Button>
            )}
          </div>
        </div>
        {(update?.status === "downloading" || update?.status === "verifying") && (
          <div className="mt-4">
            <div className="mb-1.5 flex items-center justify-between text-[13px] text-tertiary">
              <span>
                {update.status === "verifying"
                  ? t("settings.verifying", undefined, ui)
                  : `${t("settings.downloading", undefined, ui)} ${Math.round(update.progress ?? 0)}%`}
              </span>
              <span>
                {formatBytes(update.downloadedBytes)}
                {update.totalBytes ? ` / ${formatBytes(update.totalBytes)}` : ""}
                {update.bytesPerSecond
                  ? ` · ${formatRate(update.bytesPerSecond)}`
                  : ""}
                {update.etaSeconds !== null
                  ? ` · ${formatEta(update.etaSeconds, ui)}`
                  : ""}
              </span>
            </div>
            <div className="h-1.5 overflow-hidden rounded-full bg-[var(--bg-hover)]">
              <div
                className="h-full rounded-full bg-[var(--accent)] transition-[width]"
                style={{ width: `${update.progress ?? 0}%` }}
              />
            </div>
          </div>
        )}
        {update?.notes && (
          <div className="mt-4 max-h-56 overflow-y-auto border-t pt-3 text-[13px] leading-5 text-secondary" style={{ borderColor: "var(--line)" }}>
            {update.notes}
          </div>
        )}
      </div>
      <div className="mt-5">
        <PropertyRow
          title={t("settings.checkUpdates", undefined, ui)}
          control={
            <Toggle
              label={t("settings.checkUpdates", undefined, ui)}
              checked={settings.checkUpdates}
              onChange={(checkUpdates) => void patchSettings({ checkUpdates })}
            />
          }
        />
      </div>
    </div>
  );
}

function AboutSection() {
  const ui = useUiLanguage();
  const app = useYiyuStore((state) => state.app);
  if (!app) return null;
  return (
    <div className="min-w-0 flex-1 overflow-y-auto pl-1">
      <PageHeading title={t("settings.aboutTitle", undefined, ui)} />
      <div className="border-y py-6" style={{ borderColor: "var(--line)" }}>
        <div className="mt-5 grid grid-cols-2 gap-3 text-[13px]">
          <div className="soft-panel px-3 py-2.5">
            <p className="text-tertiary">{t("settings.currentVersion", undefined, ui)}</p>
            <p className="mt-1 font-medium">{app.version}</p>
          </div>
          <div className="soft-panel px-3 py-2.5">
            <p className="text-tertiary">License</p>
            <p className="mt-1 font-medium">MIT</p>
          </div>
        </div>
        <div className="mt-4 flex items-center gap-2 text-[13px] text-tertiary">
          <Database size={12} />
          <span className="mono truncate">{app.dataDir}</span>
        </div>
      </div>
    </div>
  );
}

const SECTIONS: {
  id: SettingsSection;
  icon: typeof SlidersHorizontal;
}[] = [
  { id: "general", icon: SlidersHorizontal },
  { id: "appearance", icon: Palette },
  { id: "models", icon: Cloud },
  { id: "shortcuts", icon: Keyboard },
  { id: "network", icon: Globe2 },
  { id: "updates", icon: Download },
  { id: "about", icon: Info }
];

function navLabel(section: SettingsSection, ui: "zh" | "en"): string {
  return {
    models: t("nav.providers", undefined, ui),
    general: t("nav.general", undefined, ui),
    appearance: ui === "en" ? "Appearance" : "外观",
    shortcuts: ui === "en" ? "Shortcuts" : "快捷键",
    network: t("nav.network", undefined, ui),
    updates: t("nav.updates", undefined, ui),
    about: t("nav.about", undefined, ui)
  }[section];
}

export function SettingsWindow() {
  const ui = useUiLanguage();
  const providers = useYiyuStore((state) => state.providers);
  const activeId = useYiyuStore((state) => state.settings?.activeProviderId);
  const [section, setSection] = useState<SettingsSection>(() => {
    const requested = new URLSearchParams(window.location.hash.split("?")[1]).get("section");
    return SECTIONS.some(({ id }) => id === requested) ? requested as SettingsSection : "models";
  });
  const [selectedProvider, setSelectedProvider] = useState(
    activeId ?? providers[0]?.id ?? "openai"
  );

  useEffect(() => {
    if (!providers.some((provider) => provider.id === selectedProvider)) {
      setSelectedProvider(activeId ?? providers[0]?.id ?? "openai");
    }
  }, [activeId, providers, selectedProvider]);

  useEffect(() => getBackend().on<string>(CONTRACT_EVENTS.settingsNavigate, (requested) => {
    if (SECTIONS.some(({ id }) => id === requested)) setSection(requested as SettingsSection);
  }), []);

  const sectionTitle = useMemo(() => {
    return navLabel(section, ui);
  }, [section, ui]);

  return (
    <div className="window-shell settings-shell flex-col">
      <WindowBackdrop />
      <TitleBar title={t("settings.title", undefined, ui)} />
      <div className="settings-layout">
        <aside className="settings-sidebar glass-surface flex shrink-0 flex-col px-2.5 py-3" aria-label={sectionTitle}>
          <div className="space-y-1">
            {SECTIONS.map(({ id, icon: Icon }) => (
              <button
                key={id}
                type="button"
                className="settings-nav-item"
                aria-current={section === id ? "page" : undefined}
                onClick={() => setSection(id)}
              >
                <Icon size={14} />
                {navLabel(id, ui)}
              </button>
            ))}
          </div>
        </aside>
        <main className="settings-content glass-surface flex min-h-0 min-w-0 flex-1 px-6 py-5">
          <div className={`min-h-0 min-w-0 flex-1 ${section === "models" ? "flex" : "hidden"}`}>
            <ModelsSection selected={selectedProvider} setSelected={setSelectedProvider} />
          </div>
          <div className={`min-h-0 min-w-0 flex-1 ${section === "general" ? "flex" : "hidden"}`}><GeneralSection /></div>
          <div className={`min-h-0 min-w-0 flex-1 ${section === "appearance" ? "flex" : "hidden"}`}><AppearanceSection /></div>
          <div className={`min-h-0 min-w-0 flex-1 ${section === "shortcuts" ? "flex" : "hidden"}`}><ShortcutsSection /></div>
          <div className={`min-h-0 min-w-0 flex-1 ${section === "network" ? "flex" : "hidden"}`}><NetworkSection /></div>
          <div className={`min-h-0 min-w-0 flex-1 ${section === "updates" ? "flex" : "hidden"}`}><UpdateSection /></div>
          <div className={`min-h-0 min-w-0 flex-1 ${section === "about" ? "flex" : "hidden"}`}><AboutSection /></div>
        </main>
      </div>
    </div>
  );
}
