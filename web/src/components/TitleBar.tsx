import type { ReactNode } from "react";
import { isTauriEnvironment, platformName } from "../lib/api";
import { BrandMark } from "./BrandMark";
import { TrafficLights, WindowControls } from "./WindowControls";

export function TitleBar({
  title,
  subtitle,
  actions
}: {
  title?: string;
  subtitle?: string;
  actions?: ReactNode;
}) {
  const mock = !isTauriEnvironment();
  const platform = platformName();
  return (
    <header data-tauri-drag-region className="titlebar relative z-20 flex shrink-0 items-center gap-2 px-4">
      {mock && platform === "macos" && <TrafficLights />}
      <div className="brand-lockup">
        <BrandMark size={27} />
        <span className="brand-name">{title ?? "译羽"}</span>
        {subtitle && <span className="brand-subtitle">{subtitle}</span>}
      </div>
      {actions && (
        <div className="no-drag ml-auto flex items-center gap-1.5">{actions}</div>
      )}
      {!actions && <div className="ml-auto" />}
      <WindowControls />
    </header>
  );
}
