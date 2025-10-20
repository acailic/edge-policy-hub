## Monitoring Dashboard Guide

The monitoring dashboard delivers real-time visibility into policy enforcement outcomes, tenant quotas, and compliance audit logs across Edge Policy Hub services.

### Feature Highlights

- **Live Decision Stream** – WebSocket feed of allow/deny decisions from the enforcer with per-tenant filtering.
- **Quota Gauges** – Progress indicators for message and bandwidth quotas with automatic warning and exceedance alerts.
- **Audit Log Explorer** – Search, filter, and export audit logs for investigations or compliance reviews.
- **Desktop Notifications** – Instant alerts for quota thresholds and policy violations via the Tauri notification plugin.
- **Tenant vs. Global Views** – Jump between a global overview and tenant-specific dashboards with consistent tooling.

### Data Architecture

| Service | Endpoint | Purpose |
| --- | --- | --- |
| Enforcer (8181) | `ws://127.0.0.1:8181/v1/stream/decisions` | Broadcast channel of `DecisionEvent` payloads. |
| Audit Store (8182) | `POST /api/audit/logs` | Filtered retrieval of persisted audit log entries. |
| Quota Tracker (8183) | `GET /api/quota/{tenant}` / `POST /api/quota/check` | Current quota metrics and breach detection. |

Every enforcer decision triggers a broadcast to connected subscribers. The UI maintains WebSocket connectivity with exponential backoff and merges streamed data with periodic REST polling.

### Decision Stream Protocol

```ts
const socket = new WebSocket(
  "ws://127.0.0.1:8181/v1/stream/decisions?tenant_id=tenant-a",
);

socket.addEventListener("message", (event) => {
  const payload = JSON.parse(event.data);
  if (payload.type === "decision") {
    console.log(payload.data);
  }
});
```

Messages conform to `{ type: "connected" | "decision" }`. Decision payloads mirror the backend `DecisionEvent` structure (event UUID, tenant, timestamp, `PolicyDecision`, ABAC input, and evaluation metrics).

### Tenant Dashboard Layout (`/tenants/:id/monitor`)

1. **Quota Panel** – Dual gauge cards for message and bandwidth usage, refreshed every five seconds.
2. **Decision Feed** – Scrollable log of the latest decisions with connection status indicator, filters, and pause controls.
3. **Summary Card** – Rolling totals, allow ratio, and mean evaluation latency derived from incoming events.
4. **Audit Log Viewer** – Search by time range, decision, protocol, or free text; supports selection, bulk export (JSON/CSV), and detail modal inspection.
5. **Background Monitors** – Silent `QuotaWarningMonitor` and `PolicyViolationMonitor` components trigger desktop notifications with smart debouncing (5 min for quota warnings, 10 s for violations).

Tabs provide focused experiences for Live Decisions, Audit Logs, and Quota Details while retaining the background alerts.

### Global Dashboard (`/monitor`)

- **Summary Cards** – Counts for total tenants, active tenants, warning states, and exceeded quotas.
- **Quota Table** – Ranked list of tenants with inline progress bars, color-coded status, and quick navigation links.
- **Decision Feed** – Global or tenant-filtered stream for operators watching multiple tenants simultaneously.

### Notification Behaviour

The Tauri notification plugin is initialised in `src-tauri/src/main.rs` and whitelisted via capability entries. The `useNotifications` hook:

- Checks permission on mount and requests it once if missing.
- Provides helpers `notifyQuotaWarning` and `notifyPolicyViolation`.
- Debounces repeated alerts to avoid notification storms.

Notification types:

| Event | Trigger | Message |
| --- | --- | --- |
| Quota Warning | Usage ≥ 80% of limit | `"Quota Warning: {tenant}"` |
| Quota Exceeded | Limit reached | `"Quota Exceeded: {tenant}"` |
| Policy Violation | Decision deny | `"Policy Violation: {tenant}"` |

### Refresh Strategy

- **WebSocket:** Decision events arrive immediately. The client reconnects with exponential backoff (`1s, 2s, 4s… up to 10s`) for up to 20 attempts.
- **Quota Metrics:** Polled via TanStack Query (`refetchInterval` default 5s).
- **Audit Logs:** Refetched manually or automatically every 15s when auto-refresh is enabled.

TanStack Query caches responses and revalidates on focus, ensuring lightweight repeated calls.

### Performance Notes

- The enforcer broadcast channel buffers 1024 events. Lagging clients may drop messages; reconnection restores flow with minimal overhead.
- Quota endpoints are inexpensive; polling at five-second intervals keeps usage data fresh without load concerns.
- Audit log queries respect the configured `limit` and should stay ≤1000 per request to maintain snappy UI interactions.

### Troubleshooting

| Symptom | Resolution |
| --- | --- |
| Decision feed stuck on “Connecting” | Confirm the enforcer is running on port 8181 and not blocked by a firewall. |
| Quota metrics stale | Verify quota-tracker service is active; confirm upstream systems are incrementing quotas. |
| Audit results empty | Validate traffic exists for the tenant and the audit-store service is reachable. |
| No notifications | Ensure OS-level notification permissions are granted for the app. |

### Future Enhancements

- Historical charting for quota consumption trends.
- Custom alert policies (email, webhook integrations).
- Aggregated analytics for top denied actions and latency percentiles.
- Prometheus metrics export for external observability stacks.

Refer back to `services/enforcer/README.md` for low-level WebSocket details and to `apps/tauri-ui/README.md` for operator-oriented usage guidance.
