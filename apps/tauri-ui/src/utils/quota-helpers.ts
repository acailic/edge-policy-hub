export function calculatePercentage(used: number, limit: number): number {
  if (limit <= 0) {
    return 0;
  }
  const percentage = (used / limit) * 100;
  return Number.isFinite(percentage) ? percentage : 0;
}

export function formatBytes(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unitIndex = 0;

  while (value >= 1024 && unitIndex < units.length - 1) {
    value /= 1024;
    unitIndex += 1;
  }

  return `${value.toFixed(value >= 10 ? 0 : 2)} ${units[unitIndex]}`;
}

export function formatBytesToGB(bytes: number): number {
  if (bytes <= 0) {
    return 0;
  }
  const gigabytes = bytes / (1024 ** 3);
  return Math.round(gigabytes * 100) / 100;
}

export function getQuotaColor(percentage: number): string {
  if (percentage >= 90) {
    return "#ef4444";
  }
  if (percentage >= 70) {
    return "#f59e0b";
  }
  return "#10b981";
}

export function getQuotaStatus(
  percentage: number,
): "ok" | "warning" | "critical" {
  if (percentage >= 95) {
    return "critical";
  }
  if (percentage >= 80) {
    return "warning";
  }
  return "ok";
}

export function isQuotaWarning(percentage: number): boolean {
  return percentage >= 80 && percentage < 100;
}

export function isQuotaExceeded(percentage: number): boolean {
  return percentage >= 100;
}

export function formatQuotaMessage(
  quotaType: string,
  used: number,
  limit: number,
  unit: string,
): string {
  const percentage = calculatePercentage(used, limit);
  const formattedUsed =
    unit === "bytes" ? formatBytes(used) : used.toLocaleString();
  const formattedLimit =
    unit === "bytes" ? formatBytes(limit) : limit.toLocaleString();

  const label = unit === "bytes" ? "" : ` ${unit}`;

  return `${quotaType}: ${formattedUsed} / ${formattedLimit}${label} (${Math.round(
    percentage,
  )}%)`;
}
