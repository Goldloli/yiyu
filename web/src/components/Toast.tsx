import { AlertCircle, CheckCircle2, Info, X, TriangleAlert } from "lucide-react";
import { useUiLanguage, useYiyuStore } from "../lib/store";

export function ToastStack() {
  const notices = useYiyuStore((state) => state.notices);
  const dismiss = useYiyuStore((state) => state.dismissNotice);
  const ui = useUiLanguage();
  return (
    <div
      className="pointer-events-none fixed bottom-4 right-4 z-[100] flex w-[min(340px,calc(100vw-32px))] flex-col gap-2"
      aria-live="polite"
      aria-label={ui === "en" ? "Notifications" : "通知"}
    >
      {notices.map((notice) => {
        const Icon =
          notice.level === "success"
            ? CheckCircle2
            : notice.level === "error"
              ? AlertCircle
              : notice.level === "warning"
                ? TriangleAlert
                : Info;
        return (
          <div
            key={notice.id}
            className="toast-surface pointer-events-auto flex items-start gap-2.5 px-3 py-2.5"
          >
            <Icon
              size={15}
              className={
                notice.level === "success"
                  ? "mt-0.5 shrink-0 text-[var(--success)]"
                  : notice.level === "error"
                    ? "mt-0.5 shrink-0 text-danger"
                    : notice.level === "warning"
                      ? "mt-0.5 shrink-0 text-[var(--warning)]"
                      : "mt-0.5 shrink-0 text-accent"
              }
            />
            <span className="min-w-0 flex-1 text-[11.5px] leading-5">{notice.message}</span>
            <button
              type="button"
              className="icon-button !h-6 !w-6"
              aria-label={ui === "en" ? "Dismiss" : "关闭"}
              onClick={() => dismiss(notice.id)}
            >
              <X size={12} />
            </button>
          </div>
        );
      })}
    </div>
  );
}
