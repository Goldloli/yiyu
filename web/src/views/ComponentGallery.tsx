import { Check, Copy, Search } from "lucide-react";
import { useState } from "react";
import { Button } from "../components/Button";
import { Field } from "../components/Field";
import { IconButton } from "../components/IconButton";
import { SegmentedControl } from "../components/SegmentedControl";
import { SelectMenu } from "../components/SelectMenu";
import { ShortcutRecorder } from "../components/ShortcutRecorder";
import { TitleBar } from "../components/TitleBar";
import { Toggle } from "../components/Toggle";
import { Tooltip } from "../components/Tooltip";
import { platformName } from "../lib/api";

export function ComponentGallery() {
  const [mode, setMode] = useState<"light" | "dark" | "system">("system");
  const [enabled, setEnabled] = useState(true);
  const [language, setLanguage] = useState("zh-Hans");
  const [shortcut, setShortcut] = useState<string | null>("Alt+Shift+Y");

  return (
    <div className="window-shell flex-col">
      <TitleBar title="译羽" subtitle="Component gallery" />
      <main className="min-h-0 flex-1 overflow-y-auto px-8 py-7">
        <div className="mx-auto max-w-[760px]">
          <p className="section-label">Interaction controls</p>
          <h1 className="mt-1 text-[24px] font-semibold">译羽基础组件</h1>
          <p className="mt-2 max-w-[560px] text-[11.5px] leading-5 text-secondary">
            组件画廊用于验证稳定尺寸、键盘焦点、可访问名称与浅深色语义。
          </p>

          <section className="mt-7 grid grid-cols-2 gap-5 border-t pt-5" style={{ borderColor: "var(--line)" }}>
            <div role="group" aria-label="Buttons">
              <p className="mb-1.5 text-[12px] font-semibold">Buttons <span className="font-normal text-tertiary">primary / secondary / danger</span></p>
              <div className="flex flex-wrap gap-2">
                <Button variant="primary" icon={<Check size={13} />}>
                  Primary
                </Button>
                <Button icon={<Copy size={13} />}>Secondary</Button>
                <Button variant="danger">Delete</Button>
                <Button disabled>Disabled</Button>
              </div>
            </div>
            <div role="group" aria-label="Icon buttons">
              <p className="mb-1.5 text-[12px] font-semibold">Icon buttons <span className="font-normal text-tertiary">tooltip + stable target</span></p>
              <div className="flex gap-2">
                <Tooltip label="Search">
                  <IconButton label="Search">
                    <Search size={14} />
                  </IconButton>
                </Tooltip>
                <Tooltip label="Selected">
                  <IconButton label="Selected" active>
                    <Check size={14} />
                  </IconButton>
                </Tooltip>
                <Tooltip label="Copy">
                  <IconButton label="Copy">
                    <Copy size={14} />
                  </IconButton>
                </Tooltip>
              </div>
            </div>
            <Field label="Segmented control">
              <SegmentedControl
                label="Appearance"
                value={mode}
                options={[
                  { value: "system", label: "System" },
                  { value: "light", label: "Light" },
                  { value: "dark", label: "Dark" }
                ]}
                onChange={setMode}
              />
            </Field>
            <Field label="Select menu">
              <SelectMenu
                label="Language"
                value={language}
                options={[
                  { value: "zh-Hans", label: "简体中文" },
                  { value: "en", label: "English" },
                  { value: "ja", label: "日本語" }
                ]}
                onChange={setLanguage}
              />
            </Field>
            <Field label="Switch">
              <div className="flex items-center gap-3">
                <Toggle label="Clipboard fallback" checked={enabled} onChange={setEnabled} />
                <span className="text-[11px] text-secondary">
                  {enabled ? "Enabled" : "Disabled"}
                </span>
              </div>
            </Field>
            <Field label="Shortcut recorder">
              <ShortcutRecorder
                label="Translate selection"
                value={shortcut}
                recordingLabel="Press shortcut"
                clearLabel="Clear shortcut"
                platform={platformName()}
                onChange={setShortcut}
              />
            </Field>
            <Field label="Text area">
              <textarea
                className="control min-h-[92px] w-full resize-none px-3 py-2 text-[11.5px] leading-5"
                defaultValue="Select text, translate it, and keep reading."
              />
            </Field>
            <Field label="Invalid field" error="Please check this value.">
              <input className="control w-full px-3" aria-invalid="true" defaultValue="Invalid" />
            </Field>
            <Field label="Stream text">
              <div className="stream-caret min-h-[92px] rounded-[6px] border border-[var(--line)] bg-[var(--bg-input)] px-3 py-2 text-[11.5px] leading-5">
                译文会在这里流式出现
              </div>
            </Field>
          </section>
        </div>
      </main>
    </div>
  );
}
