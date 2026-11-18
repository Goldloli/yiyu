import {
  ArrowLeftRight,
  Copy,
  History,
  MessageSquareText,
  PanelLeft,
  Plus,
  RotateCcw,
  Search,
  Settings,
  Square,
  Trash2
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { Button } from "../components/Button";
import { Dialog } from "../components/Dialog";
import { IconButton } from "../components/IconButton";
import { Markdown } from "../components/Markdown";
import { SelectMenu, type SelectOption } from "../components/SelectMenu";
import { TitleBar } from "../components/TitleBar";
import { WindowBackdrop } from "../components/WindowBackdrop";
import { getBackend } from "../lib/api";
import { compactText, formatRelativeTime } from "../lib/format";
import { languageName, t } from "../lib/i18n";
import {
  useUiLanguage,
  useYiyuStore
} from "../lib/store";
import type { HistoryEntry, Language } from "../lib/types";

function ProviderBadge({ id }: { id: string }) {
  const provider = useYiyuStore((state) =>
    state.providers.find((item) => item.id === id)
  );
  if (!provider) return null;
  return (
    <span
      className="history-provider"
      aria-hidden="true"
    >
      <MessageSquareText size={14} />
    </span>
  );
}

function HistorySidebar({
  selectedId,
  onSelect,
  onNew,
  open
}: {
  selectedId: string | null;
  onSelect: (entry: HistoryEntry) => void;
  onNew: () => void;
  open: boolean;
}) {
  const ui = useUiLanguage();
  const history = useYiyuStore((state) => state.history);
  const setHistoryQuery = useYiyuStore((state) => state.setHistoryQuery);
  const deleteHistory = useYiyuStore((state) => state.deleteHistory);
  const clearHistory = useYiyuStore((state) => state.clearHistory);
  const [query, setQuery] = useState("");
  const [confirmClear, setConfirmClear] = useState(false);

  useEffect(() => {
    const timer = window.setTimeout(() => setHistoryQuery(query), 220);
    return () => window.clearTimeout(timer);
  }, [query, setHistoryQuery]);

  return (
    <aside className="sidebar glass-surface flex min-h-0 flex-col" data-open={open ? "true" : "false"}>
      <div className="px-3 pb-2 pt-3">
        <div className="mb-2 flex items-center justify-between gap-2">
          <div>
            <p className="history-title">
              {t("main.recent", undefined, ui)}
            </p>
          </div>
          <Button
            variant="ghost"
            className="!min-h-7 !px-2 text-[11px]"
            icon={<Plus size={13} />}
            onClick={onNew}
          >
            {ui === "en" ? "New" : "新建"}
          </Button>
        </div>
        <div className="relative">
          <Search
            size={13}
            className="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-tertiary"
          />
          <input
            className="control w-full !min-h-[31px] pl-8 pr-2 text-[11.5px]"
            placeholder={t("main.searchHistory", undefined, ui)}
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            aria-label={t("common.search", undefined, ui)}
          />
        </div>
        {history.length > 0 && (
          <div className="mt-1 flex justify-end">
            <IconButton
              label={t("history.clear", undefined, ui)}
              className="!h-7 !w-7"
              onClick={() => setConfirmClear(true)}
            >
              <Trash2 size={12} />
            </IconButton>
          </div>
        )}
      </div>
      <div className="min-h-0 flex-1 overflow-y-auto px-2 pb-3">
        {history.length === 0 ? (
          <div className="grid place-items-center px-5 py-14 text-center">
            <div className="mb-3 grid h-10 w-10 place-items-center rounded-[8px] bg-[var(--bg-hover)] text-tertiary">
              <History size={18} />
            </div>
            <p className="text-[12px] font-medium">{t("main.emptyHistory", undefined, ui)}</p>
          </div>
        ) : (
          <div className="space-y-1">
            {history.map((entry) => (
              <div
                key={entry.id}
                role="button"
                tabIndex={0}
                className="history-item group cursor-pointer"
                data-active={selectedId === entry.id ? "true" : undefined}
                onClick={() => onSelect(entry)}
                onKeyDown={(event) => {
                  if (event.target !== event.currentTarget) return;
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    onSelect(entry);
                  }
                }}
              >
                <div className="mb-1.5 flex items-center gap-2">
                  <ProviderBadge id={entry.providerId} />
                  <span className="min-w-0 flex-1 truncate text-[10.5px] text-tertiary">
                    {entry.sourceLanguage} → {entry.targetLanguage}
                  </span>
                  <span className="shrink-0 text-[10px] text-tertiary">
                    {formatRelativeTime(entry.createdAt, ui)}
                  </span>
                </div>
                <p className="line-clamp-1 text-[11.5px] font-medium leading-5">
                  {compactText(entry.sourceText, 62)}
                </p>
                <p className="mt-0.5 line-clamp-2 text-[10.5px] leading-[1.55] text-secondary">
                  {compactText(entry.translatedText, 80)}
                </p>
                <div className="history-actions pointer-events-none absolute right-1 top-1 flex gap-0.5 opacity-0 transition-opacity group-hover:pointer-events-auto group-hover:opacity-100 group-focus-within:pointer-events-auto group-focus-within:opacity-100">
                  <IconButton
                    label={t("history.delete", undefined, ui)}
                    className="!h-7 !w-7"
                    onClick={(event) => {
                      event.stopPropagation();
                      void deleteHistory(entry.id);
                    }}
                  >
                    <Trash2 size={12} />
                  </IconButton>
                </div>
              </div>
            ))}
          </div>
        )}
      </div>
      {confirmClear && (
        <Dialog title={ui === "en" ? "Clear all translation history?" : "清空所有翻译记录？"} onClose={() => setConfirmClear(false)}>
          <div className="app-dialog-actions">
            <Button onClick={() => setConfirmClear(false)}>{t("common.cancel", undefined, ui)}</Button>
            <Button variant="danger" onClick={() => { setConfirmClear(false); void clearHistory(); }}>
              {ui === "en" ? "Clear history" : "清空历史"}
            </Button>
          </div>
        </Dialog>
      )}
    </aside>
  );
}

function LanguageControls() {
  const ui = useUiLanguage();
  const languages = useYiyuStore((state) => state.languages);
  const settings = useYiyuStore((state) => state.settings);
  const patchSettings = useYiyuStore((state) => state.patchSettings);
  const [customTargetOpen, setCustomTargetOpen] = useState(false);
  const [customTarget, setCustomTarget] = useState("");
  if (!settings) return null;
  const sourceOptions: SelectOption<string>[] = [
    { value: "auto", label: t("main.autoDetect", undefined, ui) },
    ...languages.map((language) => ({
      value: language.code,
      label: languageName(language, ui)
    }))
  ];
  const targetOptions: SelectOption<string>[] = [
    ...languages.map((language) => ({
      value: language.code,
      label: languageName(language, ui)
    })),
    {
      value: "__custom__",
      label: settings.customTargetLanguage || t("main.customTarget", undefined, ui)
    }
  ];
  return (
    <div className="language-controls flex min-w-0 items-center gap-1.5">
      <SelectMenu
        compact
        label={t("main.source", undefined, ui)}
        value={settings.sourceLanguage}
        options={sourceOptions}
        onChange={(value) => void patchSettings({ sourceLanguage: value })}
        className="language-select"
      />
      <IconButton
        label={ui === "en" ? "Swap languages" : "交换语言"}
        className="!h-7 !w-7"
        onClick={() => {
          const source =
            settings.sourceLanguage === "auto"
              ? "en"
              : settings.sourceLanguage;
          const target =
            settings.customTargetLanguage || settings.targetLanguage;
          void patchSettings({
            sourceLanguage: target,
            targetLanguage: source,
            customTargetLanguage: null
          });
        }}
      >
        <ArrowLeftRight size={12} />
      </IconButton>
      <SelectMenu
        compact
        label={t("main.target", undefined, ui)}
        value={settings.customTargetLanguage ? "__custom__" : settings.targetLanguage}
        options={targetOptions}
        onChange={(value) => {
          if (value === "__custom__") {
            setCustomTarget(settings.customTargetLanguage ?? "");
            setCustomTargetOpen(true);
            return;
          }
          void patchSettings({ targetLanguage: value, customTargetLanguage: null });
        }}
        className="language-select target-select"
      />
      {customTargetOpen && (
        <Dialog title={ui === "en" ? "Target language" : "自定义目标语言"} onClose={() => setCustomTargetOpen(false)}>
          <form onSubmit={(event) => {
            event.preventDefault();
            if (!customTarget.trim()) return;
            void patchSettings({ customTargetLanguage: customTarget.trim() });
            setCustomTargetOpen(false);
          }}>
            <input
              autoFocus
              className="control mt-4 w-full px-3"
              aria-label={ui === "en" ? "Target language" : "目标语言"}
              value={customTarget}
              onChange={(event) => setCustomTarget(event.target.value)}
            />
            <div className="app-dialog-actions">
              <Button onClick={() => setCustomTargetOpen(false)}>{t("common.cancel", undefined, ui)}</Button>
              <Button type="submit" variant="primary" disabled={!customTarget.trim()}>{t("common.save", undefined, ui)}</Button>
            </div>
          </form>
        </Dialog>
      )}
    </div>
  );
}

function ModelControl() {
  const ui = useUiLanguage();
  const providers = useYiyuStore((state) => state.providers);
  const settings = useYiyuStore((state) => state.settings);
  const patchSettings = useYiyuStore((state) => state.patchSettings);
  if (!settings) return null;
  return (
    <SelectMenu
      compact
      label={t("settings.model", undefined, ui)}
      value={settings.activeProviderId ?? ""}
      options={providers.map((provider) => ({
        value: provider.id,
        label: `${provider.label} · ${provider.model}`,
        icon: (
          <span
            className="status-dot"
            data-state={provider.configured ? "ready" : "warning"}
          />
        )
      }))}
      onChange={(value) => void patchSettings({ activeProviderId: value })}
      className="model-control"
    />
  );
}

export function MainWindow() {
  const ui = useUiLanguage();
  const settings = useYiyuStore((state) => state.settings);
  const providers = useYiyuStore((state) => state.providers);
  const session = useYiyuStore((state) => state.sessions.main);
  const startTranslation = useYiyuStore((state) => state.startTranslation);
  const cancelTranslation = useYiyuStore((state) => state.cancelTranslation);
  const retryTranslation = useYiyuStore((state) => state.retryTranslation);
  const retrySaveHistory = useYiyuStore((state) => state.retrySaveHistory);
  const copyText = useYiyuStore((state) => state.copyText);
  const patchSettings = useYiyuStore((state) => state.patchSettings);
  const [input, setInput] = useState("");
  const [selectedHistory, setSelectedHistory] = useState<HistoryEntry | null>(null);
  const [showSession, setShowSession] = useState(true);
  const [historyOpen, setHistoryOpen] = useState(false);
  const activeProvider = providers.find(
    (provider) => provider.id === settings?.activeProviderId
  );
  const visibleSession = showSession ? session : null;
  const translationText = useMemo(
    () => selectedHistory?.translatedText ?? visibleSession?.output ?? "",
    [visibleSession?.output, selectedHistory?.translatedText]
  );
  const busy =
    visibleSession?.status === "preparing" || visibleSession?.status === "streaming";

  const submit = async () => {
    setSelectedHistory(null);
    setShowSession(true);
    await startTranslation("main", input, {
      manualSourceLanguage: settings?.sourceLanguage === "auto" ? undefined : settings?.sourceLanguage,
      targetLanguage: settings?.customTargetLanguage || settings?.targetLanguage
    });
  };

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape" && historyOpen) setHistoryOpen(false);
      if ((event.metaKey || event.ctrlKey) && event.key === "Enter") {
        event.preventDefault();
        if (!busy && input.trim() && activeProvider?.configured) void submit();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  });

  if (!settings) return null;

  return (
    <div className="window-shell main-shell flex-col">
      <WindowBackdrop />
      <TitleBar
        actions={
          <>
            <IconButton
              label={t("nav.settings", undefined, ui)}
              onClick={() => void getBackend().openSettingsWindow()}
            >
              <Settings size={15} />
            </IconButton>
          </>
        }
      />
      <div className="main-layout">
        <HistorySidebar
          selectedId={selectedHistory?.id ?? null}
          open={historyOpen}
          onSelect={(entry) => {
            if (busy) void cancelTranslation("main");
            setSelectedHistory(entry);
            setInput(entry.sourceText);
            setShowSession(false);
            setHistoryOpen(false);
          }}
          onNew={() => {
            if (busy) void cancelTranslation("main");
            setSelectedHistory(null);
            setInput("");
            setShowSession(false);
            setHistoryOpen(false);
          }}
        />
        <main className="workspace flex min-w-0 flex-1 flex-col">
          <div className="workspace-toolbar glass-surface">
            <IconButton
              label={ui === "en" ? "Toggle history" : "显示或隐藏历史"}
              className="history-toggle"
              aria-expanded={historyOpen}
              onClick={() => setHistoryOpen((value) => !value)}
            >
              <PanelLeft size={17} />
            </IconButton>
            <LanguageControls />
            <div className="toolbar-end">
              <ModelControl />
            </div>
          </div>

          {!activeProvider?.configured && (
            <div
              className="mx-4 mt-3 flex items-center gap-3 rounded-[7px] border px-3 py-2.5"
              style={{ borderColor: "var(--line)", background: "var(--accent-weak)" }}
            >
              <span className="status-dot" data-state="warning" />
              <p className="flex-1 text-[11.5px]">{t("main.providerMissing", undefined, ui)}</p>
              <Button
                variant="ghost"
                className="text-[11px]"
                onClick={() => void getBackend().openSettingsWindow()}
              >
                {t("main.configure", undefined, ui)}
              </Button>
            </div>
          )}

          <div className="workspace-panels">
            <section className="translation-pane glass-surface flex min-h-0 min-w-0 flex-col">
              <div className="pane-heading">
                <span className="pane-label">{t("main.source", undefined, ui)}</span>
                {input.length >= settings.maxInputChars * 0.8 && (
                  <span className="ml-auto text-[10.5px] text-tertiary">
                    {t("main.characterCount", { current: input.length, max: settings.maxInputChars }, ui)}
                  </span>
                )}
              </div>
              <textarea
                value={input}
                aria-label={t("main.source", undefined, ui)}
                onChange={(event) => {
                  if (busy) void cancelTranslation("main");
                  setInput(event.target.value);
                  setSelectedHistory(null);
                  setShowSession(false);
                }}
                placeholder={t("main.placeholder", undefined, ui)}
                className="translation-input"
                maxLength={settings.maxInputChars}
                spellCheck={false}
              />
            </section>
            <section className="translation-pane glass-surface flex min-h-0 min-w-0 flex-col">
              <div className="pane-heading">
                <span className="pane-label">{t("main.translation", undefined, ui)}</span>
                <div className="ml-auto flex items-center gap-1">
                  {visibleSession?.direction && (
                    <span className="mr-1 text-[10.5px] text-tertiary">
                      {visibleSession.direction.sourceLabel} → {visibleSession.direction.targetLabel}
                    </span>
                  )}
                </div>
              </div>
              <div className="translation-output min-h-0 flex-1 overflow-y-auto">
                {visibleSession?.status === "failed" && visibleSession.error && (
                  <div className="soft-panel mb-3 p-3">
                    <p className="text-[12px] font-semibold text-danger">{visibleSession.error.message}</p>
                    <div className="mt-2 flex gap-2">
                      {visibleSession.error.retryable && (
                        <Button
                          className="!min-h-7 !px-2 text-[10.5px]"
                          icon={<RotateCcw size={12} />}
                          onClick={() => void retryTranslation("main")}
                        >
                          {t("error.retry", undefined, ui)}
                        </Button>
                      )}
                      {visibleSession.error.actions.includes("open-settings") && (
                        <Button
                          className="!min-h-7 !px-2 text-[10.5px]"
                          onClick={() => void getBackend().openSettingsWindow()}
                        >
                          {t("error.openSettings", undefined, ui)}
                        </Button>
                      )}
                    </div>
                  </div>
                )}
                {translationText ? (
                  <div className={visibleSession?.status === "streaming" ? "stream-caret" : ""}>
                    <Markdown>{translationText}</Markdown>
                  </div>
                ) : null}
                {visibleSession?.persistenceError && (
                  <div className="mt-3 flex items-center gap-2 text-[10.5px] text-danger">
                    <p className="min-w-0 flex-1">
                      {t("error.historyUnsaved", undefined, ui)}: {visibleSession.persistenceError.message}
                    </p>
                    <Button
                      className="!min-h-7 !px-2 text-[10.5px]"
                      icon={<RotateCcw size={12} />}
                      onClick={() => void retrySaveHistory("main")}
                    >
                      {t("history.retrySave", undefined, ui)}
                    </Button>
                  </div>
                )}
              </div>
            </section>
          </div>
          <footer className="workspace-actions glass-surface">
            <Button disabled={!translationText} icon={<Copy size={15} />} onClick={() => void copyText(translationText)}>
              {ui === "en" ? "Copy translation" : "复制译文"}
            </Button>
            {!busy && translationText && (
              <Button icon={<RotateCcw size={13} />} onClick={() => void (selectedHistory ? submit() : retryTranslation("main"))}>
                {t("common.retry", undefined, ui)}
              </Button>
            )}
            {busy ? (
              <>
                <span className="ml-auto flex items-center gap-2 text-[11px] text-secondary">
                  <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-[var(--accent)]" />
                  {t("main.translating", undefined, ui)}
                </span>
                <Button icon={<Square size={12} />} onClick={() => void cancelTranslation("main")}>
                  {t("main.stop", undefined, ui)}
                </Button>
              </>
            ) : (
              <Button variant="primary" className="workspace-translate ml-auto" disabled={!input.trim() || !activeProvider?.configured} icon={<MessageSquareText size={15} />} onClick={() => void submit()}>
                {t("main.translate", undefined, ui)}
              </Button>
            )}
          </footer>
        </main>
      </div>
    </div>
  );
}
