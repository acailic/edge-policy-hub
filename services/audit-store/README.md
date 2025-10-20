# Audit Store Service

## Overview
The audit-store service provides append-only, tamper-evident audit logging for the Edge Policy Hub platform. It captures every policy decision issued by edge services, signs each entry with HMAC-SHA256, and persists the logs in tenant-scoped SQLite databases. Designed for offline-first environments, the service batches and uploads logs to a cloud endpoint when connectivity is available while retaining a complete local history for forensic analysis.

## Features
- Tenant-scoped storage that isolates audit data and metadata per tenant.
- HMAC-SHA256 signing with versioned signature metadata for tamper detection.
- Append-only write path with immutable history and signature verification.
- REST API for ingesting decisions, querying history, and managing tenants.
- Deferred upload queue that batches logs and retries on transient failures.
- Configurable retention limits and upload cadence for offline/online scenarios.
- Structured logging with tracing integration for diagnostics and observability.

## Architecture
Each tenant receives a private SQLite database located beneath the configured `data_dir`. Incoming requests are validated, signed, and inserted into the tenant database within a single transaction. An in-memory cache maps tenant identifiers to connection handles to minimize connection churn. A background uploader runs on an interval, querying for log entries marked as `uploaded = 0` and forwarding batches to the cloud endpoint. Upon success the entries are marked as uploaded while still remaining locally for retention and auditing.

The service exposes an Axum-powered REST API, shares a configuration pattern consistent with the other Edge Policy Hub services, and reuses shared workspace dependencies for observability and error handling.

## Configuration
| Environment Variable | Default | Description |
| --- | --- | --- |
| `AUDIT_HOST` | `127.0.0.1` | Address to bind the HTTP listener. |
| `AUDIT_PORT` | `8182` | Port for the HTTP listener. |
| `AUDIT_DATA_DIR` | `data/audit` | Root directory for tenant databases. |
| `AUDIT_HMAC_SECRET` | _generated_ | HMAC key (base64 recommended). Generated automatically if not provided. |
| `ENABLE_DEFERRED_UPLOAD` | `true` | Enables the background upload queue. |
| `UPLOAD_BATCH_SIZE` | `1000` | Number of log entries per upload batch. |
| `UPLOAD_INTERVAL_SECS` | `300` | Interval between upload attempts in seconds. |
| `UPLOAD_ENDPOINT` | _none_ | Remote endpoint for uploading logs. |
| `MAX_LOG_AGE_DAYS` | `90` | Local retention window before archival/cleanup. |
| `LOG_LEVEL` | `info` | Tracing subscriber log level. |

Refer to `.env.example` for a template.

## Database Schemas
### Tenants (`tenants.db`)
- `tenant_id TEXT PRIMARY KEY`
- `name TEXT NOT NULL`
- `status TEXT NOT NULL`
- `created_at TEXT NOT NULL`
- `updated_at TEXT NOT NULL`
- `config TEXT` (JSON blob)

### Policy Bundles (`policy_bundles.db`)
- `bundle_id TEXT PRIMARY KEY`
- `tenant_id TEXT NOT NULL`
- `version INTEGER NOT NULL`
- `rego_code TEXT NOT NULL`
- `metadata TEXT`
- `status TEXT NOT NULL`
- `created_at TEXT NOT NULL`
- `activated_at TEXT`
- `UNIQUE(tenant_id, version)`

### Audit Logs (`{tenant}/audit.db`)
- `log_id TEXT PRIMARY KEY`
- `tenant_id TEXT NOT NULL`
- `timestamp TEXT NOT NULL`
- `decision TEXT NOT NULL`
- `protocol TEXT NOT NULL`
- `subject TEXT NOT NULL` (JSON)
- `action TEXT NOT NULL`
- `resource TEXT NOT NULL` (JSON)
- `environment TEXT NOT NULL` (JSON)
- `policy_version INTEGER`
- `reason TEXT`
- `signature TEXT NOT NULL`
- `uploaded INTEGER DEFAULT 0`
- Indexes on `(tenant_id, timestamp)` and `uploaded`.

## API Endpoints
- `POST /api/audit/logs` — Store a signed audit log entry.
- `GET /api/audit/logs` — Query logs by tenant with query parameters (tenant_id, start_time, end_time, decision, protocol, limit).
- `GET /api/audit/logs/unuploaded` — Retrieve pending logs for upload.
- `POST /api/audit/logs/mark-uploaded` — Mark a batch of logs as uploaded.
- `POST /api/tenants` — Register a tenant in the registry.
- `GET /api/tenants` — List tenants, optionally filtered by status.
- `GET /api/tenants/:tenant_id` — Retrieve tenant metadata.
- `GET /health` — Service health indicator.

All payloads are JSON. The `GET /api/audit/logs` endpoint accepts query parameters instead of a JSON body. See `docs/audit-and-quota.md` for example requests and responses.

## HMAC Signing
Audit entries are serialized into a canonical pipe-delimited string:

```
log_id|tenant_id|timestamp|decision|protocol|subject_json|action|resource_json|environment_json|policy_version|reason
```

The canonical string is hashed with HMAC-SHA256 using the configured secret key. The resulting signature is base64 encoded and stored alongside the entry. Verification recomputes the canonical payload and compares signatures in constant time.

## Deferred Upload
The upload queue runs on a fixed interval, fetching up to `UPLOAD_BATCH_SIZE` logs flagged as `uploaded = 0` per tenant. Successful POSTs to `UPLOAD_ENDPOINT/tenants/{tenant_id}/audit-logs` cause the corresponding records to be marked as uploaded. Errors trigger exponential retries on future intervals without dropping data.

## Integration
- **proxy-http** should call `POST /api/audit/logs` after evaluating policy decisions to record HTTP activity.
- **bridge-mqtt** should submit `protocol = "mqtt"` entries whenever messages are allowed or denied.
- **enforcer** can query audit history for investigative or UI purposes through `GET /api/audit/logs`.

Tenant registration should occur during provisioning through `POST /api/tenants`.

## Security Considerations
- Keep `AUDIT_HMAC_SECRET` private and rotate periodically; treat it as a shared secret between signing and verification agents.
- The append-only schema, HMAC signing, and tenant isolation reduce tampering risk. Enable disk encryption and restrict filesystem permissions for extra protection.
- Add network-layer authentication (mTLS, tokens) at reverse proxies before exposing the API.

## Performance
- SQLite write throughput is sufficient for the expected edge volumes, especially with per-tenant isolation.
- DashMap-backed connection caching minimizes connection churn.
- Indexed queries on tenant/timestamp enable efficient retrievals for dashboards and uploads.
- Upload batching reduces network overhead, while configurable intervals allow tuning for bandwidth or latency preferences.

## Development
```
cargo build -p edge-policy-audit-store
cargo run -p edge-policy-audit-store
cargo test -p edge-policy-audit-store
```

Set environment variables via `.env` or the shell before running locally. Use `RUST_LOG=debug` to enable verbose tracing.

## Future Enhancements
- Pluggable cloud storage adapters (object storage, event streams).
- Signature rotation with key identifiers for forward compatibility.
- Compression of archived audit logs to reduce disk footprint.
- REST API for signature verification and retrieval of historical bundles.
- Integration with metrics exporters (Prometheus) for monitoring upload lag and queue depth.
