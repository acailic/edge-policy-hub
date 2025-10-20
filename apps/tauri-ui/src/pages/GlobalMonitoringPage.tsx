import { useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import { Activity, Globe, Users } from "lucide-react";

import { listTenants, listAllQuotaMetrics } from "../lib/api";
import type { QuotaMetrics } from "../types/monitoring";
import { DecisionStreamFeed } from "../components/DecisionStreamFeed";
import {
  calculatePercentage,
  getQuotaStatus,
} from "../utils/quota-helpers";

interface TenantSummary {
  id: string;
  name: string;
  status: string;
}

export function GlobalMonitoringPage() {
  const [selectedTenantId, setSelectedTenantId] = useState<string | "all">(
    "all",
  );

  const tenantsQuery = useQuery<TenantSummary[]>({
    queryKey: ["tenants", "summary"],
    queryFn: async () => {
      const tenants = await listTenants();
      return tenants.map((tenant) => ({
        id: tenant.tenant_id,
        name: tenant.name,
        status: tenant.status,
      }));
    },
  });

  const quotaQuery = useQuery<QuotaMetrics[]>({
    queryKey: ["all-quota-metrics"],
    queryFn: () => listAllQuotaMetrics(),
    refetchInterval: 10_000,
  });

  const metrics = quotaQuery.data ?? [];

  const warningCount = metrics.filter((metric) => {
    const percentage = Math.max(
      calculatePercentage(metric.message_count, metric.message_limit),
      calculatePercentage(metric.bytes_sent, metric.bandwidth_limit_bytes),
    );
    return percentage >= 80 && percentage < 100;
  }).length;

  const exceededCount = metrics.filter((metric) => {
    return (
      metric.message_count >= metric.message_limit ||
      metric.bytes_sent >= metric.bandwidth_limit_bytes
    );
  }).length;

  const tenantOptions = useMemo(() => {
    const base = [{ label: "All tenants", value: "all" as const }];
    if (!tenantsQuery.data) {
      return base;
    }
    return [
      ...base,
      ...tenantsQuery.data.map((tenant) => ({
        label: `${tenant.name} (${tenant.id})`,
        value: tenant.id,
      })),
    ];
  }, [tenantsQuery.data]);

  return (
    <div className="global-monitoring">
      <header className="global-monitoring__header">
        <div>
          <h1>
            <Globe size={20} /> Global Monitoring
          </h1>
          <p>Real-time overview across all tenants</p>
        </div>
        <select
          value={selectedTenantId}
          onChange={(event) =>
            setSelectedTenantId(event.target.value as "all" | string)
          }
        >
          {tenantOptions.map((option) => (
            <option key={option.value} value={option.value}>
              {option.label}
            </option>
          ))}
        </select>
      </header>

      <section className="global-monitoring__stats">
        <div>
          <span>Total tenants</span>
          <strong>{tenantsQuery.data?.length ?? 0}</strong>
        </div>
        <div>
          <span>Active tenants</span>
          <strong>
            {tenantsQuery.data
              ? tenantsQuery.data.filter((tenant) => tenant.status === "active")
                  .length
              : 0}
          </strong>
        </div>
        <div>
          <span>Warnings</span>
          <strong>{warningCount}</strong>
        </div>
        <div>
          <span>Exceeded</span>
          <strong>{exceededCount}</strong>
        </div>
      </section>

      <section className="global-monitoring__table-section">
        <header>
          <h2>
            <Activity size={18} /> Quota Overview
          </h2>
        </header>
        <table className="global-monitoring__table">
          <thead>
            <tr>
              <th>Tenant</th>
              <th>Message Usage</th>
              <th>Bandwidth Usage</th>
              <th>Status</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {metrics.map((metric) => {
              const messagePercentage = calculatePercentage(
                metric.message_count,
                metric.message_limit,
              );
              const bandwidthPercentage = calculatePercentage(
                metric.bytes_sent,
                metric.bandwidth_limit_bytes,
              );
              const combined = Math.max(messagePercentage, bandwidthPercentage);
              const status = getQuotaStatus(combined);
              return (
                <tr key={metric.tenant_id}>
                  <td>
                    <div className="global-monitoring__tenant">
                      <Users size={16} />
                      <div>
                        <span>{metric.tenant_id}</span>
                        <Link to={`/tenants/${metric.tenant_id}/monitor`}>
                          View dashboard
                        </Link>
                      </div>
                    </div>
                  </td>
                  <td>
                    <div className="global-monitoring__progress">
                      <div
                        style={{ width: `${Math.min(messagePercentage, 100)}%` }}
                        className={`status-${getQuotaStatus(messagePercentage)}`}
                      />
                    </div>
                    <small>{Math.round(messagePercentage)}%</small>
                  </td>
                  <td>
                    <div className="global-monitoring__progress">
                      <div
                        style={{ width: `${Math.min(bandwidthPercentage, 100)}%` }}
                        className={`status-${getQuotaStatus(bandwidthPercentage)}`}
                      />
                    </div>
                    <small>{Math.round(bandwidthPercentage)}%</small>
                  </td>
                  <td>
                    <span className={`global-monitoring__badge status-${status}`}>
                      {status.toUpperCase()}
                    </span>
                  </td>
                  <td>
                    <button
                      type="button"
                      onClick={() => setSelectedTenantId(metric.tenant_id)}
                    >
                      Focus stream
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </section>

      <section className="global-monitoring__stream">
        <DecisionStreamFeed
          tenantId={selectedTenantId === "all" ? undefined : selectedTenantId}
          maxItems={60}
        />
      </section>
    </div>
  );
}
