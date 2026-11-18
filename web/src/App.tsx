import { lazy, Suspense, useEffect, type ReactNode } from "react";
import { ToastStack } from "./components/Toast";
import { TitleBar } from "./components/TitleBar";
import { Button } from "./components/Button";
import { getBackend } from "./lib/api";
import { useYiyuStore } from "./lib/store";
import { watchSystemAppearance, applyMotion, applyTheme } from "./lib/theme";

const MainWindow = lazy(() => import("./views/MainWindow").then((module) => ({ default: module.MainWindow })));
const SettingsWindow = lazy(() => import("./views/SettingsWindow").then((module) => ({ default: module.SettingsWindow })));
const OnboardingWindow = lazy(() => import("./views/OnboardingWindow").then((module) => ({ default: module.OnboardingWindow })));
const PopupWindow = lazy(() => import("./views/PopupWindow").then((module) => ({ default: module.PopupWindow })));
const ComponentGallery = lazy(() => import("./views/ComponentGallery").then((module) => ({ default: module.ComponentGallery })));

type Route = "main" | "settings" | "onboarding" | "popup" | "gallery";

function routeFromHash(): Route {
  if (window.location.hash.startsWith("#/gallery")) return "gallery";
  if (window.location.hash.startsWith("#/settings")) return "settings";
  if (window.location.hash.startsWith("#/onboarding")) return "onboarding";
  if (window.location.hash.startsWith("#/popup")) return "popup";
  return "main";
}

function LoadingWindow({ route }: { route: Route }) {
  return (
    <div className="window-shell flex-col">
      <TitleBar />
      <div className="grid flex-1 place-items-center">
        <div className="flex items-center gap-2 text-[11.5px] text-tertiary">
          <span className="h-2 w-2 animate-pulse rounded-full bg-[var(--accent)]" />
          {route === "popup" ? "Loading translation…" : "Loading Yiyu…"}
        </div>
      </div>
    </div>
  );
}

function FatalWindow({ message }: { message: string }) {
  return (
    <div className="window-shell flex-col">
      <TitleBar />
      <div className="grid flex-1 place-items-center px-10 text-center">
        <div>
          <p className="text-[13px] font-semibold">译羽无法启动</p>
          <p className="mt-2 max-w-[500px] text-[11px] leading-5 text-secondary">{message}</p>
          <Button className="mt-4" variant="primary" onClick={() => window.location.reload()}>
            重新加载
          </Button>
        </div>
      </div>
    </div>
  );
}

export function App() {
  const route = routeFromHash();
  const initialized = useYiyuStore((state) => state.initialized);
  const loading = useYiyuStore((state) => state.loading);
  const fatalError = useYiyuStore((state) => state.fatalError);
  const settings = useYiyuStore((state) => state.settings);
  const app = useYiyuStore((state) => state.app);
  const initialize = useYiyuStore((state) => state.initialize);
  const refreshPermissions = useYiyuStore((state) => state.refreshPermissions);

  useEffect(() => {
    if (!window.location.hash) {
      history.replaceState(null, "", "#/main");
    }
    void initialize();
  }, [initialize]);

  useEffect(() => {
    if (!initialized || (route !== "settings" && route !== "onboarding")) return;
    const refresh = () => { void refreshPermissions().catch(() => {}); };
    window.addEventListener("focus", refresh);
    return () => window.removeEventListener("focus", refresh);
  }, [initialized, refreshPermissions, route]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const modifier = app?.platform === "macos" ? event.metaKey : event.ctrlKey;
      if (modifier && !event.altKey && !event.shiftKey && event.key === ",") {
        event.preventDefault();
        void getBackend().openSettingsWindow();
      }
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [app?.platform]);

  useEffect(() => {
    document.documentElement.dataset.window = route;
    document.body.dataset.window = route;
    if (route !== "popup") {
      document.body.style.background =
        "linear-gradient(to bottom, var(--bg-app), var(--bg-app))";
    } else {
      document.body.style.background = "transparent";
    }
  }, [route]);

  useEffect(() => {
    if (!settings) return;
    const resolveAppearance = () => {
      applyTheme(settings.theme);
      applyMotion(settings.motion);
    };
    const cleanup = watchSystemAppearance(resolveAppearance);
    resolveAppearance();
    return cleanup;
  }, [settings?.theme, settings?.motion]);

  useEffect(() => {
    const onHash = () => window.location.reload();
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);

  let content: ReactNode;
  if (!initialized || loading) {
    content = <LoadingWindow route={route} />;
  } else if (fatalError) {
    content = <FatalWindow message={fatalError.message} />;
  } else if (route === "settings") {
    content = <SettingsWindow />;
  } else if (route === "onboarding") {
    content = <OnboardingWindow />;
  } else if (route === "popup") {
    content = <PopupWindow />;
  } else if (route === "gallery") {
    content = <ComponentGallery />;
  } else {
    content = <MainWindow />;
  }

  return (
    <>
      <Suspense fallback={<LoadingWindow route={route} />}>{content}</Suspense>
      <ToastStack />
    </>
  );
}
