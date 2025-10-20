# Audit & Quota Architecture

## Overview
Edge Policy Hub captures every access decision and resource consumption event for posterity and enforcement. Two companion services make this possible:

- **Audit Store** — Provides append-only, signed audit logs with tenant isolation and deferred cloud upload.
- **Quota Tracker** — Maintains per-tenant usage counters with persistent backing storage and fast in-memory reads.

Together they deliver compliance-ready observability and deterministic quota enforcement for on-prem and intermittently connected deployments.

## Audit Store
### Responsibilities
- Accept policy decision payloads from proxy-http, bridge-mqtt, and other services.
- Sign each record using HMAC-SHA256 and write it to a tenant-specific SQLite database.
- Expose REST endpoints for querying logs, managing tenants, and driving deferred upload.
- Batch and upload logs to a cloud endpoint when connectivity is available.

### Data Flow
1. Edge service submits a `POST /api/audit/logs` request with decision details.
2. The audit-store computes a canonical digest, signs it, and persists the entry.
3. Entries remain locally accessible via `GET /api/audit/logs` with query parameters (tenant_id, start_time, end_time, decision, protocol, limit).
4. A background `UploadQueue` polls for `uploaded = 0` entries, posts them to the configured endpoint, and then marks them as uploaded.

### Tenant Isolation
- Every tenant has its own subdirectory under `AUDIT_DATA_DIR`.
- `audit.db` stores the append-only log table.
- `tenants.db` tracks registry metadata and configuration.
- `policy_bundles.db` records policy bundle history (versioned rego, status).
- File permissions can be hardened at the directory level to prevent cross-tenant access.

## Quota Tracker
### Responsibilities
- Track per-tenant message counts and bandwidth usage.
- Enforce configurable daily (message) and monthly (bandwidth) limits.
- Persist counters to SQLite on a configurable cadence.
- Provide REST endpoints for increments, limit management, queries, and resets.

### Data Flow
1. Enforcement point calls `POST /api/quota/increment` after a request/message.
2. The quota manager updates in-memory metrics and exposes new totals for policy input.
3. Background persistence flushes the cache to `quotas.db` at fixed intervals.
4. UI or control plane queries `GET /api/quota/:tenant_id` to display usage.

### Automatic Resets
- Message counters reset when the day changes (`YYYY-MM-DD` period key).
- Bandwidth counters reset when the month changes (`YYYY-MM` period key).
- Administrators can trigger manual resets via `POST /api/quota/:tenant_id/reset`.

## Database Schemas
### Audit Store
| Database | Table | Purpose | Key Columns |
| --- | --- | --- | --- |
| `tenants.db` | `tenants` | Tenant registry metadata | `tenant_id`, `status`, `config` |
| `policy_bundles.db` | `policy_bundles` | Policy bundle versions | `bundle_id`, `tenant_id`, `version`, `status` |
| `{tenant}/audit.db` | `audit_logs` | Append-only audit history | `log_id`, `timestamp`, `decision`, `signature`, `uploaded` |

Indexes:
- `idx_audit_tenant_timestamp (tenant_id, timestamp)` accelerates timeline queries.
- `idx_audit_uploaded (uploaded)` optimises deferred upload scans.

### Quota Tracker
| Table | Purpose | Key Columns |
| --- | --- | --- |
| `quota_limits` | Stores message/bandwidth limits per tenant | `tenant_id`, `message_limit`, `bandwidth_limit_bytes` |
| `quota_usage` | Records usage per period and quota type | `tenant_id`, `period`, `quota_type`, `used` |

Unique constraint on `(tenant_id, period, quota_type)` guarantees idempotent updates.

## HMAC Signing
- Algorithm: `HMAC-SHA256` (`SIGNATURE_VERSION = 1`).
- Canonical payload: `log_id|tenant_id|timestamp|decision|protocol|subject_json|action|resource_json|environment_json|policy_version|reason`.
- Secret key: loaded from `AUDIT_HMAC_SECRET` (base64 recommended). Generated automatically if absent; rotate periodically.
- Verification: Recompute canonical string, reapply HMAC, compare using constant-time verification.

## Deferred Upload
### Strategy
- Query per-tenant batches of size `UPLOAD_BATCH_SIZE`.
- POST to `{UPLOAD_ENDPOINT}/tenants/{tenant_id}/audit-logs`.
- Retry on server/network errors; mark entries as uploaded only after successful POST.
- When `UPLOAD_ENDPOINT` is unset, the queue logs a debug message and skips the cycle.

### Offline-First Considerations
- All logs remain on disk until explicitly pruned (future enhancement).
- Upload task never deletes entries; remote consumers must handle duplicates gracefully.
- Failed uploads are retried on the next interval without dropping data.

## Integration Guide
### proxy-http
- After evaluating a request, call `POST /api/audit/logs` with protocol `http`.
- Increment quotas by calling `POST /api/quota/increment` using the response body length as `bytes_sent`.
- Before forwarding a request, optionally call `POST /api/quota/check` to block over-limit tenants.

### bridge-mqtt
- For each published message, send `POST /api/audit/logs` with protocol `mqtt`.
- Call `POST /api/quota/increment` using `message_count=1` and payload size.
- Observe quota responses to reject publishes when the message limit is exceeded.

### enforcer / UI
- Use `GET /api/audit/logs` with query parameters (tenant_id, start_time, end_time, decision, protocol, limit) to render tenant history.
- Surface quota posture via `GET /api/quota/:tenant_id` and `GET /api/quota`.
- Manage tenants via `POST /api/tenants` (audit-store) and adjust limits via `POST /api/quota/limits`.

## Performance Considerations
- **Audit Store**: Connection pooling via DashMap reduces SQLite open/close overhead. Indexes keep query latency predictable as the log grows.
- **Quota Tracker**: In-memory counters avoid hot-path disk hits. Persistence interval can be tuned for durability vs. I/O.
- Both services support `RUST_LOG` tuning and emit structured tracing for observability.

## Monitoring
- Poll `/health` endpoints for liveness.
- Track upload queue success metrics (planned Prometheus integration).
- Monitor persistence logs for failures to flush quota usage or audit uploads.

## Backup & Recovery
- Schedule filesystem snapshots of `data/audit` and `data/quota`.
- For audit logs, copy tenant directories to secure storage; signatures provide integrity verification post-restore.
- For quota data, restore `quotas.db` and restart the service; counters will resume from persisted state.

## Security
- Protect shared secrets (HMAC key, API credentials) using environment injection or secret managers.
- Enforce filesystem permissions to prevent unauthorised reads of tenant databases.
- Deploy network-layer TLS and authentication between edge services and the stores.
- Signatures enable tamper detection; reverify after transferring or restoring audit databases.

## Troubleshooting
- **Uploads stuck**: Ensure `UPLOAD_ENDPOINT` is reachable and authentication is valid; inspect logs for `failed to upload audit batch`.
- **Quota not persisting**: Check background persistence logs; verify `data/quota` is writable.
- **Signature mismatch**: Confirm both producers and consumers use the same `AUDIT_HMAC_SECRET`.
- **Unexpected resets**: Validate system clock and environment variables controlling auto reset.

## Future Work
- External storage drivers (S3, Azure Blob) for audit log upload.
- Distributed quota coordination using CRDTs or a central broker.
- Compression and retention policies for long-term audit storage.
- Prometheus exporters for queue depth, persistence latency, and quota expirations.
