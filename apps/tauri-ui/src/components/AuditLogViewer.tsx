import { useCallback, useEffect, useMemo, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import {
  CheckSquare,
  Download,
  Filter,
  Loader2,
  RefreshCcw,
  Search,
  Square,
} from "lucide-react";

import { queryAuditLogs } from "../lib/api";
import type { AuditLogEntry, AuditLogFilter } from "../types/monitoring";
import { formatTimestamp } from "../utils/date-helpers";
import { AuditLogDetailsModal } from "./AuditLogDetailsModal";

interface AuditLogViewerProps {
  tenantId: string;
  autoRefresh?: boolean;
  refreshInterval?: number;
}

type SortKey = "timestamp" | "decision" | "protocol";
type SortOrder = "asc" | "desc";

const DEFAULT_LIMIT = 100;
const PAGE_SIZE = 25;

export function AuditLogViewer({
  tenantId,
  autoRefresh = false,
  refreshInterval = 10_000,
}: AuditLogViewerProps) {
  const [startTime, setStartTime] = useState<string | undefined>(undefined);
  const [endTime, setEndTime] = useState<string | undefined>(undefined);
  const [decisionFilter, setDecisionFilter] = useState<string>("all");
  const [protocolFilter, setProtocolFilter] = useState<string>("all");
  const [searchTerm, setSearchTerm] = useState("");
  const [limit, setLimit] = useState(DEFAULT_LIMIT);
  const [filters, setFilters] = useState<AuditLogFilter>({
    tenant_id: tenantId,
    limit: DEFAULT_LIMIT,
  });
  const [sortKey, setSortKey] = useState<SortKey>("timestamp");
  const [sortOrder, setSortOrder] = useState<SortOrder>("desc");
  const [visibleCount, setVisibleCount] = useState(PAGE_SIZE);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [selectedLog, setSelectedLog] = useState<AuditLogEntry | null>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const [autoRefreshEnabled, setAutoRefreshEnabled] = useState(autoRefresh);

  useEffect(() => {
    setFilters((current) => ({
      ...current,
      tenant_id: tenantId,
    }));
  }, [tenantId]);

  const query = useQuery({
    queryKey: ["audit-logs", filters],
    queryFn: () => queryAuditLogs(filters),
    refetchInterval: autoRefreshEnabled ? refreshInterval : false,
  });

  useEffect(() => {
    setVisibleCount(PAGE_SIZE);
    setSelectedIds(new Set());
  }, [query.data]);

  const logs = query.data ?? [];

  const filteredLogs = useMemo(() => {
    const term = searchTerm.trim().toLowerCase();
    return logs
      .filter((log) => {
        if (decisionFilter !== "all" && log.decision.toLowerCase() !== decisionFilter) {
          return false;
        }
        if (protocolFilter !== "all" && log.protocol.toLowerCase() !== protocolFilter) {
          return false;
        }
        if (term.length > 0) {
          const haystack = [
            log.action,
            log.decision,
            log.protocol,
            log.reason ?? "",
            JSON.stringify(log.subject),
            JSON.stringify(log.resource),
            JSON.stringify(log.environment),
          ]
            .join(" ")
            .toLowerCase();
          return haystack.includes(term);
        }
        return true;
      })
      .sort((a, b) => {
        const direction = sortOrder === "asc" ? 1 : -1;
        if (sortKey === "timestamp") {
          return (
            (new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()) *
            direction
          );
        }
        if (sortKey === "decision") {
          return a.decision.localeCompare(b.decision) * direction;
        }
        return a.protocol.localeCompare(b.protocol) * direction;
      });
  }, [logs, decisionFilter, protocolFilter, searchTerm, sortKey, sortOrder]);

  const displayedLogs = filteredLogs.slice(0, visibleCount);
  const hasMore = visibleCount < filteredLogs.length;

  const toggleSort = (key: SortKey) => {
    if (sortKey === key) {
      setSortOrder((prev) => (prev === "asc" ? "desc" : "asc"));
    } else {
      setSortKey(key);
      setSortOrder("desc");
    }
  };

  const applyFilters = () => {
    const toIso = (value?: string) =>
      value && value.length > 0 ? new Date(value).toISOString() : undefined;

    setFilters({
      tenant_id: tenantId,
      start_time: toIso(startTime),
      end_time: toIso(endTime),
      decision: decisionFilter === "all" ? undefined : decisionFilter,
      protocol: protocolFilter === "all" ? undefined : protocolFilter,
      limit,
    });
  };

  const clearFilters = () => {
    setStartTime(undefined);
    setEndTime(undefined);
    setDecisionFilter("all");
    setProtocolFilter("all");
    setSearchTerm("");
    setLimit(DEFAULT_LIMIT);
    setFilters({
      tenant_id: tenantId,
      limit: DEFAULT_LIMIT,
    });
  };

  const toggleSelectAll = () => {
    if (displayedLogs.every((log) => selectedIds.has(log.log_id))) {
      setSelectedIds(new Set());
    } else {
      setSelectedIds(new Set(displayedLogs.map((log) => log.log_id)));
    }
  };

  const toggleSelection = (logId: string) => {
    setSelectedIds((current) => {
      const next = new Set(current);
      if (next.has(logId)) {
        next.delete(logId);
      } else {
        next.add(logId);
      }
      return next;
    });
  };

  const exportLogs = useCallback(
    (format: "json" | "csv") => {
      const targetLogs =
        selectedIds.size > 0
          ? filteredLogs.filter((log) => selectedIds.has(log.log_id))
          : filteredLogs;

      if (targetLogs.length === 0) {
        return;
      }

      let content: string;
      let mime: string;
      if (format === "json") {
        content = JSON.stringify(targetLogs, null, 2);
        mime = "application/json";
      } else {
        const headers = [
          "log_id",
          "tenant_id",
          "timestamp",
          "decision",
          "protocol",
          "action",
          "reason",
        ];
        const rows = targetLogs.map((log) =>
          [
            log.log_id,
            log.tenant_id,
            log.timestamp,
            log.decision,
            log.protocol,
            log.action,
            log.reason ?? "",
          ]
            .map((value) => `"${String(value).replace(/"/g, '""')}"`)
            .join(","),
        );
        content = [headers.join(","), ...rows].join("\n");
        mime = "text/csv";
      }

      const blob = new Blob([content], { type: mime });
      const url = URL.createObjectURL(blob);
      const anchor = document.createElement("a");
      anchor.href = url;
      const suffix = format === "json" ? "json" : "csv";
      anchor.download = `audit-logs-${tenantId}.${suffix}`;
      anchor.click();
      URL.revokeObjectURL(url);
    },
    [filteredLogs, selectedIds, tenantId],
  );

  return (
    <section className="audit-log-viewer">
      <header className="audit-log-viewer__header">
        <h3>
          <Filter size={18} /> Audit Logs
        </h3>
        <div className="audit-log-viewer__header-actions">
          <label className="audit-log-viewer__auto-refresh">
            <input
              type="checkbox"
              checked={autoRefreshEnabled}
              onChange={(event) => setAutoRefreshEnabled(event.target.checked)}
            />
            Auto refresh
          </label>
          <button type="button" onClick={() => query.refetch()}>
            <RefreshCcw size={16} /> Refresh
          </button>
        </div>
      </header>

      <div className="audit-log-viewer__filters">
        <div>
          <label htmlFor="start-time">Start time</label>
          <input
            id="start-time"
            type="datetime-local"
            value={startTime ?? ""}
            onChange={(event) =>
              setStartTime(event.target.value ? event.target.value : undefined)
            }
          />
        </div>

        <div>
          <label htmlFor="end-time">End time</label>
          <input
            id="end-time"
            type="datetime-local"
            value={endTime ?? ""}
            onChange={(event) =>
              setEndTime(event.target.value ? event.target.value : undefined)
            }
          />
        </div>

        <div>
          <label htmlFor="decision">Decision</label>
          <select
            id="decision"
            value={decisionFilter}
            onChange={(event) => setDecisionFilter(event.target.value)}
          >
            <option value="all">All</option>
            <option value="allow">Allow</option>
            <option value="deny">Deny</option>
          </select>
        </div>

        <div>
          <label htmlFor="protocol">Protocol</label>
          <select
            id="protocol"
            value={protocolFilter}
            onChange={(event) => setProtocolFilter(event.target.value)}
          >
            <option value="all">All</option>
            <option value="http">HTTP</option>
            <option value="mqtt">MQTT</option>
          </select>
        </div>

        <div className="audit-log-viewer__search">
          <label htmlFor="search">
            <Search size={16} />
          </label>
          <input
            id="search"
            type="search"
            placeholder="Search action, reason, attributes…"
            value={searchTerm}
            onChange={(event) => setSearchTerm(event.target.value)}
          />
        </div>

        <div>
          <label htmlFor="limit">Limit</label>
          <select
            id="limit"
            value={limit}
            onChange={(event) => setLimit(Number(event.target.value))}
          >
            {[50, 100, 250, 500, 1000].map((value) => (
              <option key={value} value={value}>
                {value}
              </option>
            ))}
          </select>
        </div>

        <div className="audit-log-viewer__filter-actions">
          <button type="button" onClick={applyFilters}>
            Apply Filters
          </button>
          <button type="button" onClick={clearFilters}>
            Clear
          </button>
        </div>
      </div>

      <div className="audit-log-viewer__table-wrapper">
        <table className="audit-log-viewer__table">
          <thead>
            <tr>
              <th>
                <button type="button" onClick={toggleSelectAll}>
                  {displayedLogs.length > 0 &&
                  displayedLogs.every((log) => selectedIds.has(log.log_id)) ? (
                    <CheckSquare size={16} />
                  ) : (
                    <Square size={16} />
                  )}
                </button>
              </th>
              <th>
                <button type="button" onClick={() => toggleSort("timestamp")}>
                  Timestamp
                </button>
              </th>
              <th>Tenant</th>
              <th>
                <button type="button" onClick={() => toggleSort("decision")}>
                  Decision
                </button>
              </th>
              <th>
                <button type="button" onClick={() => toggleSort("protocol")}>
                  Protocol
                </button>
              </th>
              <th>Action</th>
              <th>Reason</th>
              <th>Policy Version</th>
              <th />
            </tr>
          </thead>
          <tbody>
            {query.isLoading && (
              <tr>
                <td colSpan={9}>
                  <div className="audit-log-viewer__loading">
                    <Loader2 className="spin" size={18} /> Loading audit logs…
                  </div>
                </td>
              </tr>
            )}

            {!query.isLoading && displayedLogs.length === 0 && (
              <tr>
                <td colSpan={9}>
                  <div className="audit-log-viewer__empty">
                    No audit logs match the current filters.
                  </div>
                </td>
              </tr>
            )}

            {displayedLogs.map((log) => {
              const isSelected = selectedIds.has(log.log_id);
              return (
                <tr
                  key={log.log_id}
                  className={isSelected ? "audit-log-viewer__row--selected" : ""}
                >
                  <td>
                    <button
                      type="button"
                      onClick={() => toggleSelection(log.log_id)}
                      aria-pressed={isSelected}
                    >
                      {isSelected ? <CheckSquare size={16} /> : <Square size={16} />}
                    </button>
                  </td>
                  <td>{formatTimestamp(log.timestamp)}</td>
                  <td>
                    <span className="audit-log-viewer__tenant">{log.tenant_id}</span>
                  </td>
                  <td>
                    <span
                      className={`audit-log-viewer__decision audit-log-viewer__decision--${log.decision.toLowerCase()}`}
                    >
                      {log.decision.toUpperCase()}
                    </span>
                  </td>
                  <td>{log.protocol.toUpperCase()}</td>
                  <td>{log.action}</td>
                  <td title={log.reason ?? ""}>
                    {log.reason ? log.reason.slice(0, 60) : "—"}
                  </td>
                  <td>{log.policy_version ? `v${log.policy_version}` : "—"}</td>
                  <td>
                    <button
                      type="button"
                      onClick={() => {
                        setSelectedLog(log);
                        setModalOpen(true);
                      }}
                    >
                      View
                    </button>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      <footer className="audit-log-viewer__footer">
        <div className="audit-log-viewer__summary">
          <span>Total logs: {filteredLogs.length}</span>
          <span>
            Allow:{" "}
            {filteredLogs.filter((log) => log.decision.toLowerCase() === "allow").length}
          </span>
          <span>
            Deny: {filteredLogs.filter((log) => log.decision.toLowerCase() === "deny").length}
          </span>
        </div>
        <div className="audit-log-viewer__footer-actions">
          <button type="button" onClick={() => exportLogs("json")}>
            <Download size={16} /> Export JSON
          </button>
          <button type="button" onClick={() => exportLogs("csv")}>
            <Download size={16} /> Export CSV
          </button>
          {hasMore && (
            <button
              type="button"
              onClick={() =>
                setVisibleCount((current) => current + PAGE_SIZE)
              }
            >
              Load more
            </button>
          )}
        </div>
      </footer>

      <AuditLogDetailsModal
        log={selectedLog}
        isOpen={modalOpen}
        onClose={() => setModalOpen(false)}
      />
    </section>
  );
}
