import { Cloud, Database, Download, Globe2, Info, Keyboard, Palette, RefreshCw, SlidersHorizontal } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { Button } from "../components/Button";
import { Field } from "../components/Field";
import { SegmentedControl } from "../components/SegmentedControl";
import { ShortcutRecorder } from "../components/ShortcutRecorder";
import { TitleBar } from "../components/TitleBar";
import { Toggle } from "../components/Toggle";
import { WindowBackdrop } from "../components/WindowBackdrop";
import { getBackend, platformName } from "../lib/api";
import { formatBytes, formatEta, formatRate } from "../lib/format";
import { t } from "../lib/i18n";
import { useUiLanguage, useYiyuStore } from "../lib/store";
import { CONTRACT_EVENTS, type ProxyConfig } from "../lib/types";
import { AppearanceSection, GeneralSection } from "./settings/GeneralSections";
import { ModelsSection } from "./settings/ModelsSection";
import { PageHeading, PropertyRow } from "./settings/SettingsFields";

type SettingsSection = "models" | "general" | "appearance" | "shortcuts" | "network" | "updates" | "about";

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
  proxy,
  setProxy
}: {
  title: string;
  proxy: ProxyConfig;
  setProxy: (proxy: ProxyConfig) => void;
}) {
  const ui = useUiLanguage();
  return (
    <div className="border-t py-4" style={{ borderColor: "var(--line)" }}>
      <div className="mb-3 flex items-center justify-between gap-4">
        <p className="text-[13px] font-medium">{title}</p>
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
        proxy={providerProxy}
        setProxy={setProviderProxy}
      />
      <ProxyEditor
        title={t("settings.updateProxy", undefined, ui)}
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
