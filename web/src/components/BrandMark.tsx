import brandIcon from "../assets/yiyu-icon.png";

export function BrandMark({ size = 26 }: { size?: number }) {
  return (
    <span className="brand-mark" style={{ width: size, height: size }} aria-hidden="true">
      <img src={brandIcon} alt="" draggable={false} />
    </span>
  );
}
