import { useEffect, useState } from "react";
import { getBackend } from "./api";
import { useYiyuStore } from "./store";
import alpineLake from "../assets/backgrounds/alpine-lake.jpg";
import springValley from "../assets/backgrounds/spring-valley.jpg";
import coast from "../assets/backgrounds/coast.jpg";
import type { BackgroundChoice } from "./types";

export const MAX_BACKGROUND_BYTES = 12 * 1024 * 1024;
export const BACKGROUNDS = [
  { id: "alpine-lake", name: "山湖清晨", nameEn: "Alpine lake", image: alpineLake },
  { id: "spring-valley", name: "春日山谷", nameEn: "Spring valley", image: springValley },
  { id: "coast", name: "海岸微风", nameEn: "Coastal breeze", image: coast }
] as const;

export function useBackgroundImage() {
  const id = useYiyuStore((state) => state.settings?.backgroundId);
  const savedChoice = useYiyuStore((state) => state.settings?.backgroundChoice);
  const choice: BackgroundChoice = savedChoice ?? (id ? "custom" : "alpine-lake");
  const [customImage, setCustomImage] = useState<string | null>(null);
  const [unavailable, setUnavailable] = useState(false);
  useEffect(() => {
    let cancelled = false;
    setCustomImage(null);
    setUnavailable(false);
    if (id) {
      void getBackend().getBackground().then((value) => {
        if (!cancelled) {
          setCustomImage(value);
          setUnavailable(!value);
        }
      }).catch(() => {
        if (!cancelled) setUnavailable(true);
      });
    }
    return () => { cancelled = true; };
  }, [id]);
  const image = choice === "custom" ? customImage : BACKGROUNDS.find((item) => item.id === choice)?.image ?? null;
  return { image, customImage, unavailable: choice === "custom" && unavailable, choice };
}

export function readImageData(file: File): Promise<string> {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.onerror = () => reject(reader.error ?? new Error("Cannot read image"));
    reader.onload = () => {
      const value = typeof reader.result === "string" ? reader.result.split(",")[1] : null;
      if (value) resolve(value);
      else reject(new Error("Cannot read image"));
    };
    reader.readAsDataURL(file);
  });
}
