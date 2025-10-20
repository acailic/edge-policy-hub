import { useEffect, useMemo, useRef } from "react";
import { useQuery } from "@tanstack/react-query";
import { Activity, HardDrive, RefreshCcw } from "lucide-react";

import { getQuotaMetrics } from "../lib/api";
import type { QuotaMetrics } from "../types/monitoring";
import { useNotifications } from "../hooks/useNotifications";
import { QuotaProgressBar } from "./QuotaProgressBar";
import {
  calculatePercentage,
  formatBytesToGB,
} from "../utils/quota-helpers";

interface QuotaGaugesProps {
  tenantId: string;
  refreshInterval?: number;
}

const DEFAULT_REFRESH_INTERVAL = 5_000;
const WARNING_THRESHOLD = 80;

export function QuotaGauges({
  tenantId,
  refreshInterval = DEFAULT_REFRESH_INTERVAL,
}: QuotaGaugesProps) {
  const { notifyQuotaWarning } = useNotifications();
  const notificationState = useRef({
    messageWarned: false,
    bandwidthWarned: false,
  });

  const query = useQuery<QuotaMetrics>({
    queryKey: ["quota-metrics", tenantId],
    queryFn: () => getQuotaMetrics(tenantId),
    refetchInterval: refreshInterval,
  });

  const metrics = query.data;

  const messagePercentage = useMemo(() => {
    if (!metrics) {
      return 0;
    }
    return calculatePercentage(metrics.message_count, metrics.message_limit);
  }, [metrics]);

  const bandwidthPercentage = useMemo(() => {
    if (!metrics) {
      return 0;
    }
    return calculatePercentage(
      metrics.bytes_sent,
      metrics.bandwidth_limit_bytes,
    );
  }, [metrics]);

  useEffect(() => {
    if (!metrics) {
      return;
    }

    if (
      messagePercentage >= WARNING_THRESHOLD &&
      !notificationState.current.messageWarned
    ) {
      notificationState.current.messageWarned = true;
      notifyQuotaWarning(tenantId, "Messages", messagePercentage);
    }

    if (messagePercentage < WARNING_THRESHOLD) {
      notificationState.current.messageWarned = false;
    }

    if (
      bandwidthPercentage >= WARNING_THRESHOLD &&
      !notificationState.current.bandwidthWarned
    ) {
      notificationState.current.bandwidthWarned = true;
      notifyQuotaWarning(tenantId, "Bandwidth", bandwidthPercentage);
    }

    if (bandwidthPercentage < WARNING_THRESHOLD) {
      notificationState.current.bandwidthWarned = false;
    }
  }, [
    metrics,
    tenantId,
    messagePercentage,
    bandwidthPercentage,
    notifyQuotaWarning,
  ]);

  if (query.isLoading) {
    return (
      <section className="quota-gauges quota-gauges--loading">
        <p>Loading quota metrics…</p>
      </section>
    );
  }

  if (query.isError || !metrics) {
    return (
      <section className="quota-gauges quota-gauges--error">
        <p>Failed to load quota metrics</p>
        <button type="button" onClick={() => query.refetch()}>
          <RefreshCcw size={16} /> Retry
        </button>
      </section>
    );
  }

  const messageRemaining =
    metrics.message_limit - metrics.message_count > 0
      ? metrics.message_limit - metrics.message_count
      : 0;
  const bandwidthRemainingGb =
    formatBytesToGB(
      metrics.bandwidth_limit_bytes - metrics.bytes_sent > 0
        ? metrics.bandwidth_limit_bytes - metrics.bytes_sent
        : 0,
    );

  return (
    <section className="quota-gauges">
      <div className="quota-gauges__heading">
        <h3>
          <Activity size={18} /> Quota Usage
        </h3>
        <button
          type="button"
          className="quota-gauges__refresh"
          onClick={() => query.refetch()}
          title="Refresh quota metrics"
        >
          <RefreshCcw size={16} />
        </button>
      </div>

      <div className="quota-gauges__grid">
        <div className="quota-gauges__card">
          <header>
            <Activity size={20} />
            <span>MQTT Messages</span>
          </header>
          <QuotaProgressBar
            label="Message Quota"
            percentage={messagePercentage}
            current={metrics.message_count}
            limit={metrics.message_limit}
            unit="messages"
          />
          <footer>
            <span>
              Remaining: {messageRemaining.toLocaleString()} messages
            </span>
            <span>Period: {metrics.period}</span>
          </footer>
        </div>

        <div className="quota-gauges__card">
          <header>
            <HardDrive size={20} />
            <span>Bandwidth Usage</span>
          </header>
          <QuotaProgressBar
            label="Bandwidth"
            percentage={bandwidthPercentage}
            current={metrics.bytes_sent}
            limit={metrics.bandwidth_limit_bytes}
            unit="bytes"
          />
          <footer>
            <span>{bandwidthRemainingGb.toFixed(2)} GB remaining</span>
            <span>Last reset: {new Date(metrics.last_reset).toLocaleString()}</span>
          </footer>
        </div>
      </div>
    </section>
  );
}
