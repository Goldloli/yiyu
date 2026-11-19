import { Check, Eye, EyeOff, KeyRound, LoaderCircle, Plus, RefreshCw, Sparkles, Trash2, Wrench } from "lucide-react";
import { useEffect, useState } from "react";
import { Button } from "../../components/Button";
import { Dialog } from "../../components/Dialog";
import { Field } from "../../components/Field";
import { IconButton } from "../../components/IconButton";
import { SelectMenu } from "../../components/SelectMenu";
import { providerInitials } from "../../lib/format";
import { t } from "../../lib/i18n";
import { RECOMMENDED_MODELS } from "../../lib/model-presets";
import { useUiLanguage, useYiyuStore } from "../../lib/store";
import type { ProviderConfig, ProviderTestResult } from "../../lib/types";
import { PageHeading } from "./SettingsFields";

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

export function ModelsSection({
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
  const retryCredentialCleanup = useYiyuStore((state) => state.retryCredentialCleanup);
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
  const [cleaningCredentials, setCleaningCredentials] = useState(false);

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
        {settings && settings.pendingCredentialCleanup.length > 0 && (
          <div role="status" className="mb-5 flex items-center justify-between gap-3 rounded-lg border px-3 py-2.5 text-[13px]" style={{ borderColor: "var(--line)", background: "var(--bg-hover)" }}>
            <span>{ui === "en" ? "Some removed services may still have API keys in the system credential store." : "已删除服务的 API Key 可能仍保存在系统凭据库中。"}</span>
            <Button disabled={cleaningCredentials} onClick={() => {
              setCleaningCredentials(true);
              void retryCredentialCleanup().finally(() => setCleaningCredentials(false));
            }}>{ui === "en" ? "Retry" : "重试清理"}</Button>
          </div>
        )}
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
