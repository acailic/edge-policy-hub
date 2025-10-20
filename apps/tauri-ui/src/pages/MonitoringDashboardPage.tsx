import { useEffect, useMemo, useState } from "react";
import { Link, useParams } from "react-router-dom";
import { useQuery } from "@tanstack/react-query";
import {
  Activity,
  BarChart3,
  FileText,
  LayoutDashboard,
} from "lucide-react";

import { getTenant } from "../lib/api";
import type { Tenant } from "../types/tenant";
import type { DecisionEvent } from "../types/monitoring";
import { DecisionStreamFeed } from "../components/DecisionStreamFeed";
import { QuotaGauges } from "../components/QuotaGauges";
import { AuditLogViewer } from "../components/AuditLogViewer";
import { QuotaWarningMonitor } from "../components/QuotaWarningMonitor";
import { PolicyViolationMonitor } from "../components/PolicyViolationMonitor";

type DashboardTab = "overview" | "decisions" | "audit" | "quotas";

interface DecisionStats {
  total: number;
  allow: number;
  deny: number;
  avgLatency: number;
}

const INITIAL_STATS: DecisionStats = {
  total: 0,
  allow: 0,
  deny: 0,
  avgLatency: 0,
};

export function MonitoringDashboardPage() {
  const params = useParams<{ id: string }>();
  const tenantId = params.id;
  const [activeTab, setActiveTab] = useState<DashboardTab>("overview");
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [decisionStats, setDecisionStats] =
    useState<DecisionStats>(INITIAL_STATS);

  useEffect(() => {
    setDecisionStats(INITIAL_STATS);
  }, [tenantId]);

  const tenantQuery = useQuery<Tenant>({
    queryKey: ["tenant", tenantId],
    queryFn: () => getTenant(tenantId as string),
    enabled: Boolean(tenantId),
  });

  const tenant = tenantQuery.data;

  const statusBadge = useMemo(() => {
    if (!tenant) {
      return null;
    }
    const tone =
      tenant.status === "active" ? "success" : tenant.status === "suspended" ? "warning" : "default";
    return (
      <span className={`monitoring-dashboard__status monitoring-dashboard__status--${tone}`}>
        {tenant.status.toUpperCase()}
      </span>
    );
  }, [tenant]);

  const handleNewDecision = (event: DecisionEvent) => {
    setDecisionStats((prev) => {
      const total = prev.total + 1;
      const allow = prev.allow + (event.decision.allow ? 1 : 0);
      const deny = prev.deny + (event.decision.allow ? 0 : 1);
      const avgLatency =
        (prev.avgLatency * prev.total + event.metrics.eval_duration_micros) /
        total;
      return {
        total,
        allow,
        deny,
        avgLatency,
      };
    });
  };

  if (!tenantId) {
    return (
      <div className="monitoring-dashboard monitoring-dashboard--empty">
        <p>Tenant ID missing in route.</p>
        <Link to="/tenants">Back to tenant list</Link>
      </div>
    );
  }

  if (tenantQuery.isLoading) {
    return (
      <div className="monitoring-dashboard monitoring-dashboard--loading">
        <p>Loading tenant details…</p>
      </div>
    );
  }

  if (tenantQuery.isError || !tenant) {
    return (
      <div className="monitoring-dashboard monitoring-dashboard--error">
        <p>Tenant not found.</p>
        <Link to="/tenants">Back to tenant list</Link>
      </div>
    );
  }

  const allowRatio =
    decisionStats.total > 0
      ? Math.round((decisionStats.allow / decisionStats.total) * 100)
      : 0;

  return (
    <div className="monitoring-dashboard">
      <QuotaWarningMonitor tenantId={tenantId} />
      <PolicyViolationMonitor tenantId={tenantId} />

      <nav className="monitoring-dashboard__breadcrumbs">
        <Link to="/">Home</Link>
        <span>›</span>
        <Link to="/tenants">Tenants</Link>
        <span>›</span>
        <span>{tenant.name}</span>
      </nav>

      <header className="monitoring-dashboard__header">
        <div>
          <h1>{tenant.name}</h1>
          <p>{tenant.tenant_id}</p>
        </div>
        <div className="monitoring-dashboard__header-actions">
          {statusBadge}
          <label>
            <input
              type="checkbox"
              checked={autoRefresh}
              onChange={(event) => setAutoRefresh(event.target.checked)}
            />
            Auto Refresh
          </label>
        </div>
      </header>

      <div className="monitoring-dashboard__tabs">
        <button
          type="button"
          className={activeTab === "overview" ? "active" : ""}
          onClick={() => setActiveTab("overview")}
        >
          <LayoutDashboard size={16} /> Overview
        </button>
        <button
          type="button"
          className={activeTab === "decisions" ? "active" : ""}
          onClick={() => setActiveTab("decisions")}
        >
          <Activity size={16} /> Live Decisions
        </button>
        <button
          type="button"
          className={activeTab === "audit" ? "active" : ""}
          onClick={() => setActiveTab("audit")}
        >
          <FileText size={16} /> Audit Logs
        </button>
        <button
          type="button"
          className={activeTab === "quotas" ? "active" : ""}
          onClick={() => setActiveTab("quotas")}
        >
          <BarChart3 size={16} /> Quota Details
        </button>
      </div>

      {activeTab === "overview" && (
        <section className="monitoring-dashboard__grid">
          <QuotaGauges tenantId={tenantId} />
          <DecisionStreamFeed
            tenantId={tenantId}
            maxItems={20}
            onNewDecision={handleNewDecision}
          />
          <div className="monitoring-dashboard__summary-card">
            <h3>Today&apos;s Summary</h3>
            <ul>
              <li>
                <strong>Total decisions</strong>
                <span>{decisionStats.total}</span>
              </li>
              <li>
                <strong>Allow ratio</strong>
                <span>{allowRatio}%</span>
              </li>
              <li>
                <strong>Average latency</strong>
                <span>
                  {Math.round(decisionStats.avgLatency).toLocaleString()} μs
                </span>
              </li>
            </ul>
          </div>
          <div className="monitoring-dashboard__audit">
            <AuditLogViewer
              tenantId={tenantId}
              autoRefresh={autoRefresh}
              refreshInterval={15_000}
            />
          </div>
        </section>
      )}

      {activeTab === "decisions" && (
        <section className="monitoring-dashboard__panel">
          <DecisionStreamFeed tenantId={tenantId} maxItems={100} />
        </section>
      )}

      {activeTab === "audit" && (
        <section className="monitoring-dashboard__panel">
          <AuditLogViewer
            tenantId={tenantId}
            autoRefresh={autoRefresh}
            refreshInterval={15_000}
          />
        </section>
      )}

      {activeTab === "quotas" && (
        <section className="monitoring-dashboard__panel">
          <QuotaGauges tenantId={tenantId} refreshInterval={5_000} />
        </section>
      )}
    </div>
  );
}
