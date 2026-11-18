import { ImagePlus, Trash2 } from "lucide-react";
import { useRef, useState } from "react";
import { getBackend } from "../lib/api";
import { BACKGROUNDS, MAX_BACKGROUND_BYTES, readImageData, useBackgroundImage } from "../lib/background";
import { normalizeError } from "../lib/errors";
import { useUiLanguage, useYiyuStore } from "../lib/store";
import { Button } from "./Button";

export function BackgroundSettings() {
  const ui = useUiLanguage();
  const input = useRef<HTMLInputElement>(null);
  const settings = useYiyuStore((state) => state.settings);
  const applySettings = useYiyuStore((state) => state.applySettings);
  const patchSettings = useYiyuStore((state) => state.patchSettings);
  const notify = useYiyuStore((state) => state.notify);
  const { customImage, unavailable, choice } = useBackgroundImage();
  const [file, setFile] = useState<File | null>(null);
  const [preview, setPreview] = useState<string | null>(null);
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);

  const choose = async (candidate: File | undefined) => {
    if (!candidate) return;
    setError("");
    if (candidate.size > MAX_BACKGROUND_BYTES || !["image/png", "image/jpeg"].includes(candidate.type)) {
      setError(ui === "en" ? "Choose a PNG or JPEG up to 12 MB." : "请选择不超过 12 MB 的 PNG 或 JPEG 图片。");
      return;
    }
    const probe = new Image();
    try {
      const url = `data:${candidate.type};base64,${await readImageData(candidate)}`;
      await new Promise<void>((resolve, reject) => {
        probe.onload = () => resolve();
        probe.onerror = () => reject(new Error("invalid image"));
        probe.src = url;
      });
      if (probe.naturalWidth > 8192 || probe.naturalHeight > 8192 || probe.naturalWidth * probe.naturalHeight > 32_000_000) {
        throw new Error("image dimensions exceed the supported limit");
      }
      setFile(candidate);
      setPreview(url);
    } catch {
      setError(ui === "en" ? "The image is damaged or exceeds the supported dimensions." : "图片无法读取或尺寸超出限制（最长边 8192 像素，最多 3200 万像素）。");
    }
  };

  const save = async () => {
    if (!file) return;
    setSaving(true);
    setError("");
    try {
      const data = await readImageData(file);
      const revision = useYiyuStore.getState().settingsRevision;
      const envelope = await getBackend().setBackground(revision, data);
      applySettings(envelope);
      setFile(null);
      setPreview(null);
      notify("success", ui === "en" ? "Background applied" : "背景已应用", "background-saved");
    } catch (reason) {
      setError(normalizeError(reason).message);
    } finally {
      setSaving(false);
    }
  };

  const clear = async () => {
    setSaving(true);
    setError("");
    try {
      const revision = useYiyuStore.getState().settingsRevision;
      applySettings(await getBackend().clearBackground(revision));
      setFile(null);
      setPreview(null);
      notify("success", ui === "en" ? "Background removed" : "背景已移除", "background-removed");
    } catch (reason) {
      setError(normalizeError(reason).message);
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="background-settings">
      <div className="background-gallery-heading">
        <p className="settings-row-title">{ui === "en" ? "Background" : "背景图片"}</p>
      </div>
      <div className="background-gallery" role="group" aria-label={ui === "en" ? "Background" : "背景图片"}>
        {BACKGROUNDS.map((item) => (
          <button type="button" key={item.id} className="background-option" aria-pressed={choice === item.id} disabled={saving} onClick={() => void patchSettings({ backgroundChoice: item.id })}>
            <span className="background-thumbnail"><img src={item.image} alt="" /></span>
            <span>{ui === "en" ? item.nameEn : item.name}</span>
          </button>
        ))}
        <button type="button" className="background-option" aria-pressed={choice === "none"} disabled={saving} onClick={() => void patchSettings({ backgroundChoice: "none" })}>
          <span className="background-thumbnail background-none"><span /></span>
          <span>{ui === "en" ? "None" : "无背景"}</span>
        </button>
        {settings?.backgroundId && customImage && <button type="button" className="background-option" aria-pressed={choice === "custom"} disabled={saving} onClick={() => void patchSettings({ backgroundChoice: "custom" })}>
          <span className="background-thumbnail"><img src={customImage} alt="" /></span>
          <span>{ui === "en" ? "My image" : "我的图片"}</span>
        </button>}
      </div>
      <div className="background-custom">
        {preview && <div className="background-preview"><img src={preview} alt={ui === "en" ? "Background preview" : "背景预览"} /></div>}
        <div className="min-w-0 flex-1">
        <p className="text-[12px] font-medium">{ui === "en" ? "Custom background" : "自定义背景"}</p>
        <p className="mt-1 text-[11px] leading-5 text-tertiary">{ui === "en" ? "PNG or JPEG, up to 12 MB. Stored only on this device." : "支持 PNG、JPEG，最大 12 MB。图片仅保存在本机。"}</p>
        {unavailable && !preview && <p className="mt-1 text-[11px] text-danger" role="status">{ui === "en" ? "The saved image is unavailable. Choose another or remove it." : "已保存的图片不可用，请重新选择或移除。"}</p>}
        {error && <p className="mt-1 text-[11px] text-danger" role="alert">{error}</p>}
        <div className="mt-3 flex flex-wrap gap-2">
          <input ref={input} type="file" accept="image/png,image/jpeg" className="sr-only" aria-label={ui === "en" ? "Choose background image" : "选择背景图片"} onChange={(event) => void choose(event.target.files?.[0])} />
          <Button icon={<ImagePlus size={13} />} disabled={saving} onClick={() => { if (input.current) { input.current.value = ""; input.current.click(); } }}>{ui === "en" ? "Choose image" : "选择图片"}</Button>
          {file && <Button variant="primary" disabled={saving} onClick={() => void save()}>{ui === "en" ? "Apply" : "应用"}</Button>}
          {file && <Button variant="ghost" disabled={saving} onClick={() => { if (input.current) input.current.value = ""; setFile(null); setPreview(null); setError(""); }}>{ui === "en" ? "Cancel" : "取消"}</Button>}
          {settings?.backgroundId && !file && <Button variant="ghost" icon={<Trash2 size={13} />} disabled={saving} onClick={() => void clear()}>{ui === "en" ? "Remove" : "移除"}</Button>}
        </div>
      </div>
      </div>
    </div>
  );
}
