## Getting Started with Edge Policy Hub

### Introduction

This guide walks through installation, tenant creation, policy authoring, testing, deployment, and monitoring. Estimated time: **30 minutes**.

**Prerequisites**
- Gateway or workstation with 4+ GB RAM (Linux, Windows, or macOS)
- Rust toolchain (if building from source)
- Optional: Docker + Docker Compose

### Step 1: Installation

#### Option A: Native Installation (Recommended)

**Linux**
```bash
wget https://github.com/acailic/edge-policy-hub/releases/latest/download/edge-policy-hub_1.0.0_amd64.deb
sudo dpkg -i edge-policy-hub_1.0.0_amd64.deb
sudo systemctl enable --now edge-policy-hub.target
```

**Windows**
1. Download `Edge_Policy_Hub_1.0.0_x64_en-US.msi`
2. Run installer with administrative privileges
3. Services start automatically (enforcer, proxy, audit-store, quota-tracker)

**macOS**
1. Download `Edge_Policy_Hub_1.0.0_x64.dmg`
2. Drag **Edge Policy Hub** to Applications
3. Launch from Applications folder or Spotlight

#### Option B: Docker Compose

```bash
cd infra/docker
cp .env.example .env
# Update .env with upstream URLs and secrets
docker compose up -d
```

#### Verify Installation

```bash
# Health endpoints
curl http://localhost:8181/health   # Enforcer
curl http://localhost:8182/health   # Audit Store
curl http://localhost:8183/health   # Quota Tracker

# Systemd status
sudo systemctl status edge-policy-hub.target

# Docker status
docker compose ps
```

### Step 2: Launch Desktop UI

- **Linux/Windows:** Launch “Edge Policy Hub” from applications menu or run `edge-policy-ui`.
- **macOS:** Open from Applications (grant notification permissions when prompted).
- The UI connects to local services on ports 8181–8183.

### Step 3: Create Your First Tenant

1. Navigate to **Tenants** → **Add New Tenant**.
2. Fill the form:
   - Tenant ID: `my-first-tenant`
   - Name: `My First Tenant`
   - Message Limit: `50000`
   - Bandwidth Limit: `100`
   - Data Residency: Select region (`EU`, `US`, `APAC`)
   - Enable PII Redaction if needed.
3. Click **Create**. The tenant appears with status **Active**.

**Behind the Scenes**
- Audit Store stores tenant metadata.
- Quota Tracker seeds limit counters.
- Enforcer allocates namespace for bundles.

### Step 4: Write a Policy

1. Open **Policies** for your tenant.
2. Click **Load Template** → choose *Data Residency (EU-only)*.
3. Customize:
   ```dsl
   allow read sensor_data if
     subject.tenant_id == "my-first-tenant" and
     resource.region == "EU" and
     subject.device_location in ["DE", "FR", "NL"]
   ```
4. Click **Compile** (Ctrl/Cmd+S). Compilation completes within milliseconds.
5. Inspect generated Rego on the right panel.

### Step 5: Test the Policy

1. Click **Test Policy** to open the simulator.
2. Provide ABAC attributes:
   - Subject: tenant_id `my-first-tenant`, user_id `user-123`, device_location `DE`, clearance_level `2`
   - Action: `read`
   - Resource: type `sensor_data`, region `EU`, owner_tenant `my-first-tenant`
   - Environment: country `DE`
3. Click **Test Policy**.
4. Review result:
   - ✅ **ALLOW** with reason *Allowed by data residency policy*
   - Evaluation time ~1.5 ms
5. Repeat test with `device_location = "US"` to confirm denial.

### Step 6: Deploy Policy

1. Review compiled Rego (ensure package `tenants.my_first_tenant`).
2. Add metadata (Version `1.0.0`, Author, Description).
3. Click **Deploy as Draft**.
4. Test in staging via HTTP proxy or MQTT bridge.
5. Once validated, click **Deploy & Activate** with confirmation.

**Internals**
- Policy bundle stored in Audit Store with versioning.
- Rego file placed under enforcer bundle directory.
- Enforcer hot-reloads without downtime.

### Step 7: Monitor Enforcement

1. Open **Monitoring Dashboard**.
2. Observe live decisions via WebSocket stream (allow/deny cards).
3. Inspect quota gauges (messages, bandwidth). Colors change near thresholds.
4. Review audit logs (filter by protocol, decision). Export for compliance if needed.

### Step 8: Validate Workflows

**HTTP Proxy**
```bash
curl -H "X-Tenant-ID: my-first-tenant" http://localhost:8080/api/sensor-data
```

**MQTT Bridge**
```bash
mosquitto_pub -h localhost -p 1883 \
  -i "my-first-tenant/device-1" \
  -t "my-first-tenant/sensors/temp" \
  -m '{"value":22.5,"location":"DE"}'
```

Monitor the dashboard for new decision events and quota updates.

### Next Steps

1. **Policy Versioning:** Explore version history and rollbacks.
2. **Multi-Tenant Scenarios:** Create additional tenants with custom policies.
3. **Cost Guardrails:** Configure quotas and test enforcement.
4. **Field Redaction:** Add `redact` directives for PII fields.
5. **TLS/mTLS:** Configure certificates for proxy and MQTT endpoints.

### Production Hardening

1. Review [Deployment Guide](deployment.md) for best practices.
2. Configure TLS certificates and secure secrets.
3. Set up monitoring/alerting (Prometheus, Grafana).
4. Enable backups for SQLite databases.
5. Run full `make verify` before release.

### Common Issues

| Issue | Resolution |
|-------|------------|
| Services not starting | Check ports (use `lsof -i :8181`), inspect `logs/` directory, confirm permissions |
| Policy not enforcing | Ensure bundle activated, check enforcer logs for reload errors, test in simulator |
| UI cannot connect | Verify services running, firewall allows localhost, restart Tauri app |
| MQTT failures | Confirm ports 1883/1884 free, ensure TLS certificates configured if required |

### Support

- Documentation: `docs/`
- Issues: [GitHub Issues](https://github.com/acailic/edge-policy-hub/issues)
- Discussions: [GitHub Discussions](https://github.com/acailic/edge-policy-hub/discussions)

Continue with the [Testing Guide](testing-guide.md) for validation workflows and CI integration.
