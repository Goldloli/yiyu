import {
  ArrowRight,
  Check,
  ChevronLeft,
  Feather,
  KeyRound,
  LoaderCircle,
  MonitorCheck,
  RefreshCw,
  ShieldCheck,
  Wrench
} from "lucide-react";
import { useMemo, useState } from "react";
import { Button } from "../components/Button";
import { Field } from "../components/Field";
import { SelectMenu } from "../components/SelectMenu";
import { TitleBar } from "../components/TitleBar";
import { WindowBackdrop } from "../components/WindowBackdrop";
import { getBackend } from "../lib/api";
import { providerInitials } from "../lib/format";
import { t } from "../lib/i18n";
import { useUiLanguage, useYiyuStore } from "../lib/store";
import type { ProviderConfig, ProviderTestResult } from "../lib/types";

type Step = 0 | 1 | 2;

export function OnboardingWindow() {
  const ui = useUiLanguage();
  const settings = useYiyuStore((state) => state.settings);
  const providers = useYiyuStore((state) => state.providers);
  const permissions = useYiyuStore((state) => state.permissions);
  const patchSettings = useYiyuStore((state) => state.patchSettings);
  const saveSecret = useYiyuStore((state) => state.saveProviderSecret);
  const testProvider = useYiyuStore((state) => state.testProvider);
  const fetchModels = useYiyuStore((state) => state.fetchModels);
  const requestAccessibility = useYiyuStore((state) => state.requestAccessibilityPermission);
  const [step, setStep] = useState<Step>(0);
  const [providerId, setProviderId] = useState(
    settings?.activeProviderId ?? providers[0]?.id ?? "openai"
  );
  const selected = providers.find((provider) => provider.id === providerId) ?? providers[0];
  const [draft, setDraft] = useState<ProviderConfig | null>(selected ?? null);
  const [apiKey, setApiKey] = useState("");
  const [models, setModels] = useState<string[]>([]);
  const [loadingModels, setLoadingModels] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<ProviderTestResult | null>(null);
  const [saving, setSaving] = useState(false);
  const [permissionRequested, setPermissionRequested] = useState(false);

  const chooseProvider = (id: string) => {
    const provider = providers.find((item) => item.id === id);
    if (!provider) return;
    setProviderId(id);
    setDraft(provider);
    setApiKey("");
    setModels([]);
    setTestResult(null);
  };

  const canContinue = Boolean(
    draft &&
      draft.baseUrl.trim() &&
      draft.model.trim() &&
      (!draft.requiresApiKey || apiKey.trim() || providers.find((item) => item.id === draft.id)?.configured)
  );

  const finish = async () => {
    if (!draft || !settings) return;
    setSaving(true);
    try {
      if (apiKey.trim()) await saveSecret(draft.id, apiKey);
      await patchSettings({
        providers: settings.providers.map((provider) =>
          provider.id === draft.id ? draft : provider
        ),
        activeProviderId: draft.id,
        onboardingCompleted: true
      });
      await getBackend().openMainWindow();
    } finally {
      setSaving(false);
    }
  };

  const steps = useMemo(
    () => [
      { title: t("onboarding.provider", undefined, ui), icon: Feather },
      { title: t("onboarding.api", undefined, ui), icon: KeyRound },
      { title: t("onboarding.permission", undefined, ui), icon: ShieldCheck }
    ],
    [ui]
  );

  if (!settings || !draft) return null;

  return (
    <div className="window-shell onboarding-shell flex-col">
      <WindowBackdrop />
      <TitleBar />
      <div className="onboarding-layout glass-surface flex min-h-0 flex-1 flex-col">
        <nav className="onboarding-steps" aria-label={ui === "en" ? "Setup progress" : "配置进度"}>
          {steps.map(({ title }, index) => (
            <div key={title} className="onboarding-step" data-current={step === index || undefined} aria-current={step === index ? "step" : undefined}>
              <span className="onboarding-step-number">{index < step ? <Check size={13} /> : index + 1}</span>
              <span>{title}</span>
            </div>
          ))}
        </nav>
        <main className="onboarding-content flex min-w-0 flex-1 flex-col overflow-y-auto">
          {step === 0 && (
            <div className="fade-in flex min-h-full flex-col">
              <h2 className="mt-2 text-[22px] font-semibold">
                {ui === "en" ? "Choose your model provider" : "选择你的模型服务"}
              </h2>
              <div className="onboarding-provider-grid mt-6 grid grid-cols-2 gap-2">
                {providers.map((provider) => (
                  <button
                    key={provider.id}
                    type="button"
                    className="flex items-center gap-3 rounded-[10px] border px-3 py-3 text-left transition-colors"
                    aria-pressed={provider.id === providerId}
                    style={{
                      borderColor: "var(--line)",
                      background:
                        provider.id === providerId
                          ? "var(--selected-surface)"
                          : "var(--bg-input)"
                    }}
                    onClick={() => chooseProvider(provider.id)}
                  >
                    <span
                      className="grid h-8 w-8 shrink-0 place-items-center rounded-[6px] text-[9px] font-bold"
                      style={{
                        color: "var(--text-secondary)",
                        background: "var(--bg-hover)"
                      }}
                    >
                      {providerInitials(provider.label)}
                    </span>
                    <span className="min-w-0 flex-1">
                      <span className="block text-[11.5px] font-semibold">{provider.label}</span>
                      <span className="mt-0.5 block truncate text-[9.5px] text-tertiary">
                        {provider.model}
                      </span>
                    </span>
                    {provider.id === providerId && (
                      <Check size={14} className="text-secondary" />
                    )}
                  </button>
                ))}
              </div>
              <div className="onboarding-step-actions mt-auto flex justify-end pt-8">
                <Button
                  variant="primary"
                  icon={<ArrowRight size={13} />}
                  onClick={() => setStep(1)}
                >
                  {t("common.next", undefined, ui)}
                </Button>
              </div>
            </div>
          )}

          {step === 1 && (
            <div className="fade-in flex h-full flex-col">
              <h2 className="mt-2 text-[22px] font-semibold">{draft.label}</h2>
              <div className="mt-6 space-y-4">
                <Field label={t("settings.baseUrl", undefined, ui)}>
                  <input
                    className="control mono w-full px-3 text-[11.5px]"
                    value={draft.baseUrl}
                    onChange={(event) =>
                      setDraft({ ...draft, baseUrl: event.target.value })
                    }
                    spellCheck={false}
                  />
                </Field>
                {draft.requiresApiKey && (
                  <Field
                    label={t("settings.apiKey", undefined, ui)}
                    hint={t("settings.savedToKeychain", undefined, ui)}
                  >
                    <input
                      type="password"
                      className="control mono w-full px-3 text-[11.5px]"
                      value={apiKey}
                      placeholder={t("settings.apiKeyPlaceholder", undefined, ui)}
                      onChange={(event) => setApiKey(event.target.value)}
                      autoComplete="off"
                      spellCheck={false}
                    />
                  </Field>
                )}
                <Field label={t("settings.model", undefined, ui)}>
                  <div className="flex gap-2">
                    {models.length > 0 ? (
                      <SelectMenu
                        value={draft.model}
                        options={models.map((model) => ({ value: model, label: model }))}
                        onChange={(model) => setDraft({ ...draft, model })}
                        className="min-w-0 flex-1"
                      />
                    ) : (
                      <input
                        className="control mono min-w-0 flex-1 px-3 text-[11.5px]"
                        value={draft.model}
                        onChange={(event) => setDraft({ ...draft, model: event.target.value })}
                        spellCheck={false}
                      />
                    )}
                    <Button
                      icon={
                        loadingModels ? (
                          <LoaderCircle size={12} className="animate-spin" />
                        ) : (
                          <RefreshCw size={12} />
                        )
                      }
                      disabled={loadingModels}
                      onClick={async () => {
                        setLoadingModels(true);
                        try {
                          setModels(await fetchModels(draft, apiKey || undefined));
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
              </div>
              <div className="mt-4 flex items-center gap-2">
                <Button
                  icon={
                    testing ? (
                      <LoaderCircle size={12} className="animate-spin" />
                    ) : (
                      <Wrench size={12} />
                    )
                  }
                  disabled={testing || !canContinue}
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
                  {testing
                    ? t("settings.testing", undefined, ui)
                    : t("settings.test", undefined, ui)}
                </Button>
                {testResult && (
                  <span
                    className="text-[10.5px]"
                    style={{ color: testResult.ok ? "var(--success)" : "var(--danger)" }}
                  >
                    {testResult.ok
                      ? `${testResult.message} · ${testResult.latencyMs}ms`
                      : testResult.message}
                  </span>
                )}
              </div>
              <div className="mt-auto flex justify-between pt-8">
                <Button
                  icon={<ChevronLeft size={13} />}
                  onClick={() => setStep(0)}
                >
                  {t("common.back", undefined, ui)}
                </Button>
                <Button
                  variant="primary"
                  icon={<ArrowRight size={13} />}
                  disabled={!canContinue || (draft.requiresApiKey && testResult?.ok === false)}
                  onClick={() => setStep(2)}
                >
                  {t("common.next", undefined, ui)}
                </Button>
              </div>
            </div>
          )}

          {step === 2 && (
            <div className="fade-in flex h-full flex-col">
              <h2 className="mt-2 text-[22px] font-semibold">
                {ui === "en"
                  ? "Yiyu needs to see your selection"
                  : "允许译羽读取你的选区"}
              </h2>
              <p className="mt-2 max-w-[520px] text-[11.5px] leading-5 text-secondary">
                {ui === "en"
                  ? "macOS requires Accessibility permission for global selection capture. Yiyu reads the current selection only when you invoke its shortcut."
                  : "macOS 的全局划词需要辅助功能权限。译羽只会在你触发快捷键时读取当前选区。"}
              </p>
              <div className="mt-7 flex items-center gap-4 rounded-[8px] border px-4 py-4" style={{ borderColor: "var(--line)", background: "var(--bg-input)" }}>
                <span
                  className="grid h-10 w-10 place-items-center rounded-[8px]"
                  style={{
                    color: permissions.accessibility
                      ? "var(--accent-fg)"
                      : "var(--text-secondary)",
                    background: permissions.accessibility
                      ? "var(--accent)"
                      : "var(--bg-hover)"
                  }}
                >
                  {permissions.accessibility ? (
                    <Check size={18} />
                  ) : (
                    <MonitorCheck size={18} />
                  )}
                </span>
                <div className="flex-1">
                  <p className="text-[12px] font-semibold">{t("settings.accessibility", undefined, ui)}</p>
                  <p className="mt-0.5 text-[10.5px] text-tertiary">
                    {permissions.accessibility
                      ? t("settings.accessibilityGranted", undefined, ui)
                      : t("settings.accessibilityMissing", undefined, ui)}
                  </p>
                </div>
                {!permissions.accessibility && (
                  <Button
                    onClick={() => {
                      if (permissionRequested) {
                        void getBackend().openAccessibilitySettings();
                      } else {
                        setPermissionRequested(true);
                        void requestAccessibility();
                      }
                    }}
                  >
                    {permissionRequested ? t("settings.openSystemSettings", undefined, ui) : ui === "en" ? "Request permission" : "请求授权"}
                  </Button>
                )}
              </div>
              {permissionRequested && !permissions.accessibility && (
                <p className="mt-3 text-[11.5px] leading-5 text-secondary">
                  {ui === "en"
                    ? "If already enabled in System Settings, remove the old Yiyu entry, add this version, and restart Yiyu if needed."
                    : "若系统中已开启，请移除旧的译羽条目并添加当前版本；必要时重启译羽。"}
                </p>
              )}
              <div className="mt-4 flex gap-2">
                <Button
                  icon={<ChevronLeft size={13} />}
                  onClick={() => setStep(1)}
                >
                  {t("common.back", undefined, ui)}
                </Button>
                <div className="ml-auto flex gap-2">
                  <Button
                    variant="primary"
                    icon={saving ? <LoaderCircle size={12} className="animate-spin" /> : <ArrowRight size={13} />}
                    disabled={saving}
                    onClick={() => void finish()}
                  >
                    {t("onboarding.finish", undefined, ui)}
                  </Button>
                </div>
              </div>
            </div>
          )}
        </main>
      </div>
    </div>
  );
}
