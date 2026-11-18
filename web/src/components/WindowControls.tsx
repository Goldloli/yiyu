import { Minus, Square, X } from "lucide-react";
import { isTauriEnvironment, platformName } from "../lib/api";
import { IconButton } from "./IconButton";

export function TrafficLights() {
  return (
    <div className="no-drag absolute left-[14px] top-[17px] flex gap-2">
      <span className="h-3 w-3 rounded-full bg-[#ff5f57] shadow-[inset_0_0_0_0.5px_rgba(0,0,0,0.16)]" />
      <span className="h-3 w-3 rounded-full bg-[#febc2e] shadow-[inset_0_0_0_0.5px_rgba(0,0,0,0.16)]" />
      <span className="h-3 w-3 rounded-full bg-[#28c840] shadow-[inset_0_0_0_0.5px_rgba(0,0,0,0.16)]" />
    </div>
  );
}

export function WindowControls() {
  if (platformName() !== "windows" || !isTauriEnvironment()) return null;
  return (
    <div className="no-drag absolute right-0 top-0 flex h-[var(--titlebar-height)] items-stretch">
      <IconButton
        label="Minimize"
        className="!h-full !w-10 !rounded-none"
        onClick={() => void import("@tauri-apps/api/window").then(({ getCurrentWindow }) => getCurrentWindow().minimize())}
      >
        <Minus size={14} />
      </IconButton>
      <IconButton
        label="Maximize"
        className="!h-full !w-10 !rounded-none"
        onClick={() => void import("@tauri-apps/api/window").then(({ getCurrentWindow }) => getCurrentWindow().toggleMaximize())}
      >
        <Square size={12} />
      </IconButton>
      <IconButton
        label="Close"
        className="!h-full !w-11 !rounded-none hover:!bg-[#d83b3b] hover:!text-white"
        onClick={() => void import("@tauri-apps/api/window").then(({ getCurrentWindow }) => getCurrentWindow().close())}
      >
        <X size={15} />
      </IconButton>
    </div>
  );
}
