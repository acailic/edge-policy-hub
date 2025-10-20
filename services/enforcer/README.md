# Edge Policy Enforcer Service

OPA-compatible policy enforcement service using Regorus (pure Rust Rego interpreter).

## Features
- Per-tenant policy bundle isolation with separate Engine instances
- REST API: `POST /v1/data/tenants/{tenant_id}/allow`
- WebSocket decision stream: `ws://localhost:8181/v1/stream/decisions`
- Hot-reload support via file watching
- Tenant ID validation for hard multi-tenant boundaries
- p99 < 2ms policy evaluation latency

## Configuration

Environment variables:
- `ENFORCER_HOST` - Server host (default: 127.0.0.1)
- `ENFORCER_PORT` - Server port (default: 8181)
- `BUNDLES_DIR` - Policy bundles directory (default: config/tenants.d)
- `ENABLE_HOT_RELOAD` - Enable file watching (default: true)
- `LOG_LEVEL` - Logging level (default: info)

## Bundle Format

Each tenant has a directory under `config/tenants.d/{tenant_id}/`:

```text
config/tenants.d/
├── tenant_a/
│   ├── policy.rego       # Main policy file
│   ├── data.json         # Optional static data
│   └── metadata.json     # Optional version info
└── tenant_b/
    └── policy.rego
```

Policy files must use package namespace: `package tenants.{tenant_id}`

## API Examples

**Query Policy:**

```bash
curl -X POST http://localhost:8181/v1/data/tenants/tenant_a/allow \
  -H "Content-Type: application/json" \
  -d '{
    "input": {
      "subject": {"tenant_id": "tenant_a", "user_id": "user_123"},
      "action": "read",
      "resource": {"type": "sensor_data", "region": "EU"},
      "environment": {"time": "2025-10-16T14:30:00Z"}
    }
  }'
```

**Response:**

```json
{
  "result": {
    "allow": true,
    "redact": ["pii.email"],
    "reason": "Allowed by data residency policy"
  },
  "metrics": {
    "eval_duration_micros": 1250,
    "tenant_id": "tenant_a"
  }
}
```

## WebSocket Decision Stream

The enforcer exposes a broadcast WebSocket endpoint that streams policy decisions in real time.

- **Endpoint:** `ws://localhost:8181/v1/stream/decisions`
- **Query Parameters:** `tenant_id` (optional) to scope events to a single tenant
- **Message Types:**
  - `{"type": "connected", "message": "Decision stream ready"}`
  - `{"type": "decision", "data": DecisionEvent}`
- **DecisionEvent Fields:**
  - `event_id`: UUID for correlation
  - `tenant_id`: tenant scope for the decision
  - `timestamp`: ISO 8601 timestamp when the decision was evaluated
  - `decision`: `PolicyDecision` payload (allow/redact/reason)
  - `input`: ABAC input supplied to the policy engine
  - `metrics`: evaluation metrics (e.g., `eval_duration_micros`)

The server fans out events through a `tokio::sync::broadcast` channel (capacity 1024). If consumers lag behind the buffer they will drop intermediate events; clients should reconnect if they detect gaps.

### Example WebSocket Client

```ts
const tenantId = "tenant-a";
const url = `ws://127.0.0.1:8181/v1/stream/decisions?tenant_id=${tenantId}`;
const socket = new WebSocket(url);

socket.addEventListener("open", () => {
  console.log("decision stream connected");
});

socket.addEventListener("message", (event) => {
  const payload = JSON.parse(event.data);
  if (payload.type === "decision") {
    console.log("policy decision", payload.data);
  }
});

socket.addEventListener("close", () => {
  console.log("decision stream disconnected");
});
```

### Monitoring Integration

The Tauri desktop UI connects to the decision stream for the real-time monitoring dashboard. Desktop operators receive instant allow/deny updates, quota warnings, and policy violation alerts driven by the streamed `DecisionEvent` payloads.

## Development

```bash
# Build
cargo build --package edge-policy-enforcer

# Run
cargo run --package edge-policy-enforcer

# Test
cargo test --package edge-policy-enforcer
```

Reference the main project `README.md` for overall architecture and multi-tenant isolation strategy.
