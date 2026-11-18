export function formatRelativeTime(timestamp: number, ui: "zh" | "en"): string {
  const diffMs = Math.max(0, Date.now() - timestamp);
  const minutes = Math.floor(diffMs / 60_000);
  if (minutes < 1) return ui === "en" ? "just now" : "刚刚";
  if (minutes < 60) return ui === "en" ? `${minutes} min ago` : `${minutes} 分钟前`;
  const hours = Math.floor(minutes / 60);
  if (hours < 24) return ui === "en" ? `${hours} hr ago` : `${hours} 小时前`;
  const days = Math.floor(hours / 24);
  if (days < 7) return ui === "en" ? `${days} d ago` : `${days} 天前`;
  return new Intl.DateTimeFormat(ui === "en" ? "en" : "zh-CN", {
    month: "short",
    day: "numeric"
  }).format(timestamp);
}

export function formatBytes(bytes: number): string {
  if (bytes < 1_024) return `${bytes} B`;
  if (bytes < 1_048_576) return `${(bytes / 1_024).toFixed(1)} KB`;
  if (bytes < 1_073_741_824) return `${(bytes / 1_048_576).toFixed(1)} MB`;
  return `${(bytes / 1_073_741_824).toFixed(2)} GB`;
}

export function formatRate(bytesPerSecond: number): string {
  return `${formatBytes(bytesPerSecond)}/s`;
}

export function formatEta(seconds: number, ui: "zh" | "en"): string {
  if (seconds <= 0) return ui === "en" ? "finishing" : "即将完成";
  if (seconds < 60) {
    return ui === "en" ? `${seconds}s left` : `剩余 ${seconds} 秒`;
  }
  const minutes = Math.ceil(seconds / 60);
  return ui === "en" ? `${minutes} min left` : `剩余 ${minutes} 分钟`;
}

export function compactText(value: string, max = 46): string {
  const normalized = value.replace(/\s+/g, " ").trim();
  return normalized.length > max
    ? `${normalized.slice(0, Math.max(0, max - 1))}…`
    : normalized;
}

export function providerInitials(label: string): string {
  const ascii = label.replace(/[^A-Za-z0-9]/g, "");
  if (ascii.length >= 2) return ascii.slice(0, 2).toUpperCase();
  return label.slice(0, 2);
}
