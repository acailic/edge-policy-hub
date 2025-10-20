# API Reference

Edge Policy Hub exposes service-specific APIs for policy enforcement, auditing, and quota management. All endpoints use JSON payloads and are served over HTTP with TLS recommended in production.

## Enforcer Service (Port 8181)

### `POST /v1/data/tenants/{tenant_id}/allow`
- **Description:** Evaluate policy decision for the provided ABAC input.
- **Request Body:**
  ```json
  {
    "input": {
      "subject": {
        "tenant_id": "tenant-a",
        "user_id": "user-123",
        "roles": ["operator"],
        "device_location": "DE"
      },
      "action": "read",
      "resource": {
        "type": "sensor_data",
        "owner_tenant": "tenant-a",
        "region": "EU"
      },
      "environment": {
        "time": "2025-01-15T12:00:00Z",
        "country": "DE",
        "bandwidth_used": 42
      }
    }
  }
  ```
- **Response:**
  ```json
  {
    "result": {
      "allow": true,
      "redact": ["payload.pii.email"],
      "reason": "Allowed by data residency policy"
    },
    "metrics": {
      "evaluation_time_ns": 1720000,
      "engine": "regorus"
    }
  }
  ```
- **Status Codes:** `200 OK`, `403 Forbidden`, `404 Not Found`, `500 Internal Server Error`.
- **Notes:** Include `X-Request-ID` to correlate decisions with audit logs.

### `POST /v1/tenants/{tenant_id}/reload`
- **Description:** Hot-reload tenant policy bundle.
- **Status Codes:** `200 OK` on success, `404 Not Found` if tenant missing.
- **Example:**
  ```bash
  curl -X POST http://localhost:8181/v1/tenants/tenant-a/reload
  ```

### `GET /health`
- **Description:** Service health probe.
- **Response:** `{"status":"healthy","service":"edge-policy-enforcer"}`

### `GET /metrics`
- **Description:** Prometheus-formatted metrics (latency histograms, decision counts).

### `GET /v1/stream/decisions` (WebSocket)
- **Description:** Real-time feed of policy decisions.
- **Parameters:** Optional `tenant_id` to filter stream.
- **Messages:**
  ```json
  {"type": "connected"}
  {"type": "decision", "data": { "tenant_id": "tenant-a", "allow": false, "reason": "Quota exceeded" }}
  ```
- **Example Client (JavaScript):**
  ```js
  const socket = new WebSocket("ws://localhost:8181/v1/stream/decisions?tenant_id=tenant-a");
  socket.onmessage = event => console.log(JSON.parse(event.data));
  ```

## Audit Store Service (Port 8182)

### `POST /api/audit/logs`
- **Description:** Append audit log entry with HMAC-SHA256 signature.
- **Request Body:**
  ```json
  {
    "tenant_id": "tenant-a",
    "decision": "allow",
    "resource": "sensor_data",
    "protocol": "http",
    "payload": {
      "subject": { "tenant_id": "tenant-a" },
      "action": "read",
      "reason": "Allow - EU residency satisfied"
    }
  }
  ```
- **Response:** `{"log_id":"log_01H...", "signature":"base64-hmac"}`

### `GET /api/audit/logs`
- **Description:** Query audit logs with filters.
- **Query Parameters:** `tenant_id`, `start_time`, `end_time`, `decision`, `protocol`, `limit`.
- **Example:** `GET /api/audit/logs?tenant_id=tenant-a&decision=deny&limit=50`

### `POST /api/tenants`
- **Description:** Create tenant record and bootstrap namespaces.
- **Request Body:** Matches tenant configuration schema (see examples).
- **Response:** Newly created tenant JSON.

### `GET /api/tenants/{tenant_id}`
- **Description:** Retrieve tenant configuration.
- **Status Codes:** `200 OK`, `404 Not Found`.

### `PUT /api/tenants/{tenant_id}`
- **Description:** Update tenant metadata or config.
- **Request Body:** Partial tenant config fields.

### `POST /api/bundles`
- **Description:** Register compiled policy bundle metadata.
- **Request Body:** `PolicyBundleRecord` including version, checksum, and storage paths.

### `GET /api/bundles?tenant_id={id}`
- **Description:** List bundles per tenant with status (draft/active).

### `POST /api/bundles/{bundle_id}/activate`
- **Description:** Activate bundle and notify enforcer.

### `GET /health`
- Returns `{"status":"healthy","service":"edge-policy-audit-store"}`.

## Quota Tracker Service (Port 8183)

### `POST /api/quota/increment`
- **Description:** Increment quota counters for a tenant.
- **Request Body:**
  ```json
  {
    "tenant_id": "tenant-a",
    "message_count": 120,
    "bytes_sent": 4096
  }
  ```
- **Response:** `{"metrics":{"message_count":120,"bytes_sent":4096,"updated_at":"2025-01-15T12:00:01Z"}}`

### `GET /api/quota/{tenant_id}`
- **Description:** Fetch current quota metrics (messages, bandwidth).

### `POST /api/quota/check`
- **Description:** Evaluate if tenant has exceeded quotas.
- **Response:** `{"exceeded":false,"quota_type":null,"limit":null,"current":null}`

### `POST /api/quota/limits`
- **Description:** Set or update quota thresholds.
- **Request Body:** `{ "tenant_id": "...", "message_limit": 100000, "bandwidth_limit_gb": 500 }`

### `POST /api/quota/{tenant_id}/reset`
- **Description:** Reset counters (used for daily or monthly rollovers).

### `GET /api/quota`
- **Description:** List quota metrics for all tenants (admin only).

### `GET /health`
- Returns `{"status":"healthy","service":"edge-policy-quota-tracker"}`.

## Authentication and Security

- Services expect mutual TLS or mTLS in production.
- Audit Store endpoints support HMAC signatures for tamper detection.
- Quota Tracker endpoints require service-to-service JWT with `system` role.

## Rate Limiting and Pagination

- Audit log queries default to 100 records per page (`limit` parameter).
- Policy decision endpoint supports up to 500 requests per second per tenant (configurable).

## Error Format

All services return structured errors:
```json
{
  "error": {
    "code": "tenant_not_found",
    "message": "Tenant tenant-unknown not loaded",
    "details": {}
  }
}
```

Refer to the OpenAPI specifications in `docs/openapi/` for machine-readable schemas and code generation support.
