import { useBackgroundImage } from "../lib/background";

export function WindowBackdrop() {
  const { image } = useBackgroundImage();
  return image ? <div aria-hidden="true" className="wallpaper" style={{ backgroundImage: `url("${image}")` }} /> : null;
}
