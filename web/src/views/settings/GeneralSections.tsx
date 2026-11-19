import { useEffect, useState } from "react";
import { BackgroundSettings } from "../../components/BackgroundSettings";
import { Button } from "../../components/Button";
import { Field } from "../../components/Field";
import { SegmentedControl } from "../../components/SegmentedControl";
import { SelectMenu } from "../../components/SelectMenu";
import { Toggle } from "../../components/Toggle";
import { getBackend } from "../../lib/api";
import { languageName, t } from "../../lib/i18n";
import { useUiLanguage, useYiyuStore } from "../../lib/store";
import { PageHeading, PropertyRow } from "./SettingsFields";

export function AppearanceSection() {
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

export function GeneralSection() {
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
        <p className="section-label mb-2">{ui === "en" ? "Translation" : "翻译"}</p>
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
        <div className="mt-3 border-t" style={{ borderColor: "var(--line)" }}>
          <PropertyRow
            title={t("settings.maxInput", undefined, ui)}
            description="1,000–200,000"
            control={
              <input
                type="number"
                aria-label={t("settings.maxInput", undefined, ui)}
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
                aria-label={t("settings.historyLimit", undefined, ui)}
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
        </div>
      </div>
      <div className="mb-6">
        <p className="section-label mb-2">{t("settings.capture", undefined, ui)}</p>
        <div className="border-t" style={{ borderColor: "var(--line)" }}>
          <PropertyRow
            title={t("settings.clipboardFallback", undefined, ui)}
            description={
              ui === "en"
                ? "Used only when selection cannot be read directly. New copies are preserved."
                : "无法直接取词时临时复制，并保留之后手动复制的内容。"
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
