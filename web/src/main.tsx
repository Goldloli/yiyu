import { createRoot } from "react-dom/client";
import "./styles/app.css";
import { App } from "./App";
import { platformName } from "./lib/api";
import { applyMotion, applyTheme, systemTheme } from "./lib/theme";

document.documentElement.dataset.theme = systemTheme();
document.documentElement.dataset.motion = "normal";
document.documentElement.dataset.platform = platformName();
applyTheme("system");
applyMotion("system");

const container = document.getElementById("root");
if (container) {
  createRoot(container).render(<App />);
}
