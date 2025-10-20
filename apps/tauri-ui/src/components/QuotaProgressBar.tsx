import { AlertTriangle } from "lucide-react";

import {
  formatBytes,
  formatBytesToGB,
  getQuotaColor,
  getQuotaStatus,
  isQuotaExceeded,
  isQuotaWarning,
} from "../utils/quota-helpers";

interface QuotaProgressBarProps {
  label: string;
  percentage: number;
  current: number;
  limit: number;
  unit: string;
  showWarning?: boolean;
}

export function QuotaProgressBar({
  label,
  percentage,
  current,
  limit,
  unit,
  showWarning = true,
}: QuotaProgressBarProps) {
  const color = getQuotaColor(percentage);
  const status = getQuotaStatus(percentage);
  const remaining = Math.max(limit - current, 0);

  const formattedCurrent =
    unit === "bytes" ? formatBytes(current) : current.toLocaleString();
  const formattedLimit =
    unit === "bytes" ? formatBytes(limit) : limit.toLocaleString();
  const formattedRemaining =
    unit === "bytes"
      ? formatBytes(remaining)
      : remaining.toLocaleString(undefined, {
          maximumFractionDigits: 2,
        });

  const displayPercentage = Math.round(percentage * 10) / 10;

  return (
    <div className={`quota-progress quota-progress--${status}`}>
      <div className="quota-progress__header">
        <span className="quota-progress__label">{label}</span>
        <span className="quota-progress__value">
          {displayPercentage.toFixed(1)}%
        </span>
      </div>
      <div className="quota-progress__bar">
        <div
          className={`quota-progress__bar-fill quota-progress__bar-fill--${status} ${
            isQuotaExceeded(percentage) ? "quota-progress__bar-fill--pulse" : ""
          }`}
          style={{ width: `${Math.min(percentage, 100)}%`, backgroundColor: color }}
        />
      </div>
      <div className="quota-progress__metrics">
        <span>
          {formattedCurrent} / {formattedLimit}{" "}
          {unit === "bytes" ? "" : unit}
        </span>
        <span>{formattedRemaining} left</span>
      </div>
      {showWarning && isQuotaWarning(percentage) && (
        <div className="quota-progress__warning">
          <AlertTriangle size={16} />
          <span>
            Approaching limit. {unit === "bytes"
              ? `${formatBytesToGB(limit)} GB total`
              : `${limit.toLocaleString()} ${unit}`}
          </span>
        </div>
      )}
    </div>
  );
}
