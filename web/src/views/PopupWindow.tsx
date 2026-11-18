import {
  ChevronDown,
  Clipboard,
  Copy,
  Pin,
  PinOff,
  RotateCcw,
  Square,
  X
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";
import { Button } from "../components/Button";
import { BrandMark } from "../components/BrandMark";
import { IconButton } from "../components/IconButton";
import { Markdown } from "../components/Markdown";
import { WindowBackdrop } from "../components/WindowBackdrop";
import { getBackend } from "../lib/api";
import { normalizeError } from "../lib/errors";
import { t } from "../lib/i18n";
import { useUiLanguage, useYiyuStore } from "../lib/store";
import type { AppError } from "../lib/types";

export function PopupWindow() {
  const ui = useUiLanguage();
  const session = useYiyuStore((state) => state.sessions.popup);
  const cancelTranslation = useYiyuStore((state) => state.cancelTranslation);
  const retryTranslation = useYiyuStore((state) => state.retryTranslation);
  const retrySaveHistory = useYiyuStore((state) => state.retrySaveHistory);
  const [clipboardError, setClipboardError] = useState<AppError | null>(null);
  const translateClipboard = async () => {
    setClipboardError(null);
    try {
      await getBackend().translateClipboard();
    } catch (error) {
      const normalized = normalizeError(error);
      if (normalized.kind !== "cancelled") setClipboardError(normalized);
    }
  };
  const copyText = useYiyuStore((state) => state.copyText);
  const [sourceOpen, setSourceOpen] = useState(true);
  const [pinned, setPinned] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const busy =
    session?.status === "preparing" || session?.status === "streaming";

  const close = useCallback(() => {
    setPinned(false);
    void cancelTranslation("popup");
    void getBackend().closePopup();
  }, [cancelTranslation]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        close();
      }
    };
    const onBlur = () => {
      if (!pinned) close();
    };
    window.addEventListener("keydown", onKeyDown);
    window.addEventListener("blur", onBlur);
    return () => {
      window.removeEventListener("keydown", onKeyDown);
      window.removeEventListener("blur", onBlur);
    };
    // `pinned` intentionally controls whether a focus loss closes the panel.
  }, [close, pinned]);

  useEffect(() => {
    if (session?.status !== "failed") return;
    setPinned(false);
    void getBackend().setPopupPinned(false);
  }, [session?.status]);

  useEffect(() => {
    const node = rootRef.current;
    if (!node) return;
    let frame = 0;
    const resize = () => {
      cancelAnimationFrame(frame);
      frame = requestAnimationFrame(() => {
        const header = node.querySelector<HTMLElement>(".popup-header");
        const body = node.querySelector<HTMLElement>(".popup-scroll, .popup-empty");
        const footer = node.querySelector<HTMLElement>(".popup-footer");
        let bodyHeight = body?.scrollHeight ?? 0;
        if (body?.classList.contains("popup-scroll")) {
          const style = getComputedStyle(body);
          bodyHeight = parseFloat(style.paddingTop) + parseFloat(style.paddingBottom) +
            Array.from(body.children).reduce((height, child) => {
              const margin = getComputedStyle(child);
              return height + child.getBoundingClientRect().height + parseFloat(margin.marginTop) + parseFloat(margin.marginBottom);
            }, 0);
        }
        const naturalHeight = (header?.offsetHeight ?? 0) + bodyHeight + (footer?.offsetHeight ?? 0) + 2;
        const contentHeight = Math.min(640, Math.max(218, naturalHeight));
        void getBackend().resizePopup(382, contentHeight);
      });
    };
    const observer = new ResizeObserver(resize);
    observer.observe(node);
    resize();
    return () => {
      cancelAnimationFrame(frame);
      observer.disconnect();
    };
  }, [session?.output, session?.status, sourceOpen, session?.error]);

  const togglePin = async () => {
    const next = !pinned;
    setPinned(next);
    await getBackend().setPopupPinned(next);
  };

  return (
    <div className="popup-shell popup-enter" ref={rootRef}>
      <WindowBackdrop />
      <div className="popup-surface">
      <header className="popup-header" data-tauri-drag-region>
        <BrandMark size={24} />
        <div className="min-w-0 flex-1">
          <p className="truncate text-[13px] font-semibold">
            {session?.direction
              ? `${session.direction.sourceLabel} → ${session.direction.targetLabel}`
              : t("app.name", undefined, ui)}
          </p>
        </div>
        <div className="no-drag flex items-center gap-0.5">
          <IconButton
            label={pinned ? t("popup.unpin", undefined, ui) : t("popup.pin", undefined, ui)}
            active={pinned}
            className="!h-7 !w-7"
            onClick={() => void togglePin()}
          >
            {pinned ? <PinOff size={12} /> : <Pin size={12} />}
          </IconButton>
          <IconButton
            label={t("popup.close", undefined, ui)}
            className="!h-7 !w-7"
            onClick={close}
          >
            <X size={12} />
          </IconButton>
        </div>
      </header>

      {!session ? (
        <div className="popup-empty grid min-h-[150px] place-items-center px-8 text-center">
          <div>
            <p role={clipboardError ? "alert" : undefined} className={clipboardError ? "text-[13px] text-danger" : "text-[13px] text-secondary"}>
              {clipboardError?.message ?? t("popup.empty", undefined, ui)}
            </p>
            <Button
              className="mt-3"
              icon={<Clipboard size={12} />}
              onClick={() => void translateClipboard()}
            >
              {t("settings.clipboardShortcut", undefined, ui)}
            </Button>
          </div>
        </div>
      ) : (
        <>
          <div className="popup-scroll">
            {session.sourceText && <section className="popup-source px-3.5 py-3">
              <button
                type="button"
                className="mb-2 flex w-full items-center gap-2 text-left text-[11px] font-semibold text-tertiary"
                aria-expanded={sourceOpen}
                onClick={() => setSourceOpen((value) => !value)}
              >
                {sourceOpen ? t("popup.hideSource", undefined, ui) : t("popup.source", undefined, ui)}
                <ChevronDown
                  size={12}
                  className={`ml-auto transition-transform ${sourceOpen ? "rotate-180" : ""}`}
                />
              </button>
              {sourceOpen && (
                <textarea
                  className="max-h-[126px] min-h-[58px] w-full resize-none bg-transparent text-[13px] leading-[1.65] text-secondary outline-none"
                  value={session.sourceText}
                  aria-label={t("main.source", undefined, ui)}
                  readOnly
                />
              )}
            </section>}
            <section className="popup-result px-4 py-3.5" aria-label={t("main.translation", undefined, ui)}>
              {session.status !== "failed" && <p className="mb-2 text-[12px] font-semibold text-secondary">{t("main.translation", undefined, ui)}</p>}
              {session.status === "failed" && session.error ? (
                <div>
                  <p role="alert" className="text-[13px] font-medium text-danger">
                    {session.error.kind === "permission"
                      ? (ui === "en" ? "Accessibility permission is required to read the selection." : "需要辅助功能权限才能读取选区。")
                      : session.error.message}
                  </p>
                  {session.output && (
                    <div className="mt-2 opacity-70">
                      <Markdown>{session.output}</Markdown>
                    </div>
                  )}
                </div>
              ) : session.output ? (
                <div className={busy ? "stream-caret" : ""}>
                  <Markdown>{session.output}</Markdown>
                </div>
              ) : (
                <div className="flex items-center gap-2 py-3 text-[11px] text-tertiary">
                  <span className="h-1.5 w-1.5 animate-pulse rounded-full bg-[var(--accent)]" />
                  {t("common.loading", undefined, ui)}
                </div>
              )}
            </section>
            {clipboardError && <p role="alert" className="px-4 pb-2 text-[11px] text-danger">{clipboardError.message}</p>}
          </div>
          <footer className="popup-footer">
            <span className="ml-0.5 truncate text-[9.5px] text-tertiary">
              {busy ? t("main.translating", undefined, ui)
                : session.status === "cancelled" ? (ui === "en" ? "Cancelled" : "已取消") : ""}
            </span>
            {busy && (
              <IconButton
                label={t("main.stop", undefined, ui)}
                className="!h-7 !w-7"
                onClick={() => void cancelTranslation("popup")}
              >
                <Square size={11} />
              </IconButton>
            )}
            {!busy && session.sourceText && (
              <IconButton
                label={t("popup.retry", undefined, ui)}
                className="!h-7 !w-7"
                onClick={() => void retryTranslation("popup")}
              >
                <RotateCcw size={12} />
              </IconButton>
            )}
            {session.error?.actions.includes("open-settings") && (
              <Button
                className="!min-h-7 !px-2 text-[10.5px]"
                onClick={() => void getBackend().openSettingsWindow(session.error?.kind === "permission" ? "general" : "models")}
              >
                {t("error.openSettings", undefined, ui)}
              </Button>
            )}
            {session.status === "failed" && !session.sourceText && (
              <Button className="!min-h-7 !px-2 text-[10.5px]" onClick={() => void translateClipboard()}>
                {t("settings.clipboardShortcut", undefined, ui)}
              </Button>
            )}
            {session.persistenceError && (
              <Button
                className="!min-h-7 !px-2 text-[10.5px]"
                onClick={() => void retrySaveHistory("popup")}
              >
                {t("history.retrySave", undefined, ui)}
              </Button>
            )}
            {session.output && (
              <Button
                variant="primary"
                className="ml-auto !min-h-7 !px-2.5 text-[10.5px]"
                icon={<Copy size={11} />}
                onClick={() => void copyText(session.output)}
              >
                {t("common.copy", undefined, ui)}
              </Button>
            )}
          </footer>
        </>
      )}
      </div>
    </div>
  );
}
