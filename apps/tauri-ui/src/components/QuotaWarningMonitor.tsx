import { useEffect, useRef } from "react";
import { useQuery } from "@tanstack/react-query";

import { checkQuotaStatus } from "../lib/api";
import type { QuotaStatus } from "../types/monitoring";
import { useNotifications } from "../hooks/useNotifications";

interface QuotaWarningMonitorProps {
  tenantId: string;
  checkInterval?: number;
  onQuotaExceeded?: (status: QuotaStatus) => void;
}

const FIVE_MINUTES = 5 * 60 * 1000;

export function QuotaWarningMonitor({
  tenantId,
  checkInterval = 30_000,
  onQuotaExceeded,
}: QuotaWarningMonitorProps) {
  const { notify, notifyQuotaWarning } = useNotifications();
  const lastWarningRef = useRef<number | null>(null);
  const lastExceededRef = useRef<number | null>(null);

  const query = useQuery({
    queryKey: ["quota-status", tenantId],
    queryFn: () => checkQuotaStatus(tenantId),
    refetchInterval: checkInterval,
  });

  useEffect(() => {
    const status = query.data;
    if (!status) {
      return;
    }

    const now = Date.now();
    if (
      status.warning_threshold_reached &&
      (!lastWarningRef.current || now - lastWarningRef.current > FIVE_MINUTES)
    ) {
      const quotaLabel = status.quota_type ?? "Quota";
      notifyQuotaWarning(
        tenantId,
        quotaLabel,
        status.current && status.limit
          ? (status.current / status.limit) * 100
          : 80,
      );
      lastWarningRef.current = now;
    }

    if (
      status.exceeded &&
      (!lastExceededRef.current || now - lastExceededRef.current > FIVE_MINUTES)
    ) {
      const quotaLabel = status.quota_type ?? "Quota";
      const message = status.current && status.limit
        ? `${status.current.toLocaleString()} / ${status.limit.toLocaleString()}`
        : "limit reached";
      notify(`Quota Exceeded: ${tenantId}`, `${quotaLabel} ${message}`);
      lastExceededRef.current = now;
      onQuotaExceeded?.(status);
    }
  }, [query.data, notify, notifyQuotaWarning, onQuotaExceeded, tenantId]);

  return null;
}
