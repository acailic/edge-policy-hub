# Deployment Guide

## Overview

This guide covers production deployment of Edge Policy Hub on edge gateways and remote sites.

## Deployment Architectures

### Single-Node Gateway (Recommended for MVP)

- All services on one gateway (4–16 GB RAM).
- Local SQLite databases for audit and quota.
- Offline-first operation.
- Suitable for small/medium sites, remote offices, and edge locations.
- Recommended deployment method: native installation (systemd/Windows Service) or Docker Compose.

### Multi-Node Cluster (Future Roadmap)

- Enforcer, audit-store, quota-tracker on control plane nodes.
- Proxy-http and bridge-mqtt on data plane nodes.
- Shared storage for policy bundles (NFS/Ceph).
- Ideal for large sites and high availability requirements.
- Deployment method: Kubernetes/Helm or Docker Swarm (future enhancement).

---

## Production Deployment Checklist

### Pre-Deployment

- [ ] Hardware sizing: verify gateway meets minimum requirements (4 GB RAM, 2 cores, 10 GB disk).
- [ ] Network planning: document required ports and firewall rules.
- [ ] TLS certificates: generate or obtain certificates for TLS/mTLS.
- [ ] HMAC secret: generate strong secret for audit log signing.
- [ ] Backup strategy: plan data backup and retention.
- [ ] Monitoring: prepare external monitoring (Prometheus, Grafana).
- [ ] Disaster recovery: document recovery procedures.

### Deployment Steps

1. Install operating system (Ubuntu 22.04 LTS or Windows Server 2022 recommended).
2. Apply security hardening (firewall, SSH keys, disable unnecessary services).
3. Install Edge Policy Hub (native installer or Docker Compose).
4. Configure TLS and client authentication.
5. Generate secrets (HMAC, TLS keys) and store securely.
6. Configure services via environment variables.
7. Start services and enable auto-start.
8. Create tenants in the desktop UI.
9. Deploy ABAC policies.
10. Test policy enforcement end-to-end.
11. Configure monitoring dashboards and alerts.
12. Document configuration and customizations.

### Post-Deployment

- [ ] Verify health checks (`/health` endpoints).
- [ ] Test policy enforcement (HTTP, MQTT).
- [ ] Review system logs for errors or warnings.
- [ ] Validate quota tracking.
- [ ] Test offline mode behavior.
- [ ] Perform initial data backup.
- [ ] Finalize deployment documentation.

---

## Production Configuration

### Security Hardening

**TLS/mTLS**

- Enable TLS for all external services (proxy-http, bridge-mqtt).
- Enable mTLS for tenant authentication.
- Use strong cipher suites (prefer TLS 1.3).
- Rotate certificates every 90 days.

**Secrets Management**

- Store HMAC secret in `/etc/edge-policy-hub/hmac-secret` with mode 0600.
- Consider secure enclaves or TPM for enhanced protection.
- Rotate secrets periodically and document procedures.

**Network Isolation**

- Bind internal services (enforcer, audit-store, quota-tracker) to `127.0.0.1`.
- Bind external services to specific interfaces or VLANs.
- Enforce firewall rules for allowed traffic.
- Consider network segmentation for multi-tenant deployments.

**User Permissions**

- Run services as the dedicated `edge-policy` user.
- Enable systemd hardening features (`NoNewPrivileges`, `ProtectSystem`, `PrivateTmp`).
- Restrict file permissions (0750 for data directories, 0600 for secrets).

### Performance Tuning

**Resource Limits (systemd)**

```ini
[Service]
MemoryMax=512M
CPUQuota=100%
TasksMax=1024
```

**Resource Limits (Docker)**

```yaml
deploy:
  resources:
    limits:
      cpus: '1.0'
      memory: 512M
```

**Database Optimization**

- Enable WAL mode for SQLite (`PRAGMA journal_mode=WAL`).
- Set `PRAGMA synchronous=NORMAL` for balanced durability/performance.
- Schedule periodic `VACUUM` to reclaim disk space.

**Network Tuning**

- Increase TCP limits for high-throughput deployments.
- Configure keepalive for long-lived connections.
- Use connection pooling in upstream services.

### High Availability

**Service Redundancy**

- Use systemd restart policies (`Restart=on-failure`).
- Leverage Docker health checks for automated restarts.
- Monitor services and automate remediation.

**Data Redundancy**

- Perform regular backups of SQLite databases.
- Replicate to secondary storage (NFS/S3) where possible.
- Validate restoration procedures regularly.

**Network Redundancy**

- Configure redundant network interfaces.
- Provide backup upstream connections.
- Employ DNS-based load balancing for remote tenants.

---

## Monitoring and Observability

### Logging

**Linux (systemd)**

- Logs available via `journalctl -u edge-policy-hub.target -f`.
- Tune retention in `/etc/systemd/journald.conf`.
- Forward to central syslog or SIEM if required.

**Windows**

- Logs stored in `C:\Program Files\Edge Policy Hub\logs\`.
- Configure WinSW log rotation via XML configuration.
- Optionally forward to Windows Event Log.

**Docker**

- Logs collected through `docker-compose logs`.
- Use `json-file` driver with rotation.
- Forward to Loki/ELK/Fluentd for aggregation.

### Metrics (Future Enhancements)

- Expose Prometheus metrics from each service.
- Publish request count, latency, quota usage.
- Build Grafana dashboards for operations teams.

### Health Checks

- All services expose `/health` endpoints.
- Integrate with external monitoring (Nagios, Zabbix, Pingdom).
- Configure alerts for service degradation.

### Alerting

- Desktop UI can surface quota warnings and policy violations.
- Plan email/SMS/Slack integrations for operational alerts.
- Consider PagerDuty or Opsgenie for on-call escalation.

---

## Backup and Recovery

### Backup Strategy

**Datasets to Back Up**

1. Audit databases (`/var/lib/edge-policy-hub/data/audit/`).
2. Quota databases (`/var/lib/edge-policy-hub/data/quota/`).
3. Policy bundles (`/var/lib/edge-policy-hub/config/tenants.d/`).
4. Configuration (`/etc/edge-policy-hub/`).
5. HMAC secret (`/etc/edge-policy-hub/hmac-secret`).

**Frequency**

- Audit databases: hourly (append-only).
- Quota databases: daily.
- Policy bundles and configuration: on change.

**Example Backup Script (Linux)**

```bash
#!/bin/bash
BACKUP_DIR=/backup/edge-policy-hub/$(date +%Y%m%d-%H%M%S)
mkdir -p "$BACKUP_DIR"

sudo systemctl stop edge-policy-hub.target

sudo tar czf "$BACKUP_DIR/data.tar.gz" /var/lib/edge-policy-hub
sudo tar czf "$BACKUP_DIR/config.tar.gz" /etc/edge-policy-hub

sudo systemctl start edge-policy-hub.target

# Optional: upload to remote storage
# rclone copy "$BACKUP_DIR" remote:edge-policy-backups/
```

**Docker Volume Backup**

```bash
docker run --rm -v audit-data:/data -v $(pwd):/backup alpine tar czf /backup/audit-backup.tar.gz /data
```

### Recovery Procedures

```bash
sudo systemctl stop edge-policy-hub.target
sudo tar xzf backup/data.tar.gz -C /
sudo tar xzf backup/config.tar.gz -C /
sudo chown -R edge-policy:edge-policy /var/lib/edge-policy-hub
sudo systemctl start edge-policy-hub.target
```

- Validate recovery by checking service status and running health checks.
- Aim for **RTO < 1 hour** and **RPO < 1 hour** with hourly backups.

---

## Scaling Considerations

### Vertical Scaling

- Add RAM and CPU for larger policy sets and higher throughput.
- Migrate to SSDs for improved database performance.
- Single-node deployment scales to ~10,000 requests/second with tuning.

### Horizontal Scaling (Future)

- Deploy multiple enforcer instances with shared policy bundles.
- Use distributed quota tracking (e.g., Redis, etcd).
- Replicate audit logs to PostgreSQL/CockroachDB.
- Load balance proxy-http and bridge-mqtt across nodes.

**Kubernetes Roadmap**

- Helm charts for installation.
- StatefulSets for stateful services (audit-store, quota-tracker).
- Deployments for stateless services (enforcer, proxy-http, bridge-mqtt).
- Persistent Volume Claims for data persistence.
- Horizontal Pod Autoscaler policies.

---

## Compliance and Auditing

### Audit Log Retention

- Local retention default: 90 days (configurable).
- Cloud retention for compliance (e.g., 7 years for GDPR).
- Enable deferred upload with `ENABLE_DEFERRED_UPLOAD=true`.

### Compliance Requirements

**GDPR**

- Comprehensive audit logs for data access decisions.
- Policy rules enforce data residency and privacy controls.
- Honour right-to-erasure requests by deleting tenant data.
- Support data portability via CSV/JSON export.

**HIPAA (Future)**

- Encrypt data in transit and at rest.
- Implement strict access controls and authentication.
- Maintain PHI access audit trails.
- Establish Business Associate Agreements (BAA) where applicable.

### Audit Verification

```bash
# Verify log integrity (future endpoint)
curl -X POST http://localhost:8182/api/audit/verify \
  -H "Content-Type: application/json" \
  -d '{"tenant_id":"tenant-a","log_id":"log-123"}'
```

```bash
# Export tenant audit logs
curl -X POST http://localhost:8182/api/audit/logs \
  -H "Content-Type: application/json" \
  -d '{"tenant_id":"tenant-a","limit":10000}' \
  | jq -r '.logs[] | [.timestamp, .decision, .action, .reason] | @csv' > audit-export.csv
```

Refer to `docs/audit-and-quota.md` for deeper insight into audit pipeline design.

---

## Operational Runbook

1. **Monitoring**
   - Dashboards for quota consumption, policy decisions, and service health.
   - Alerts on repeated policy denies, quota exhaustion, or service failures.
2. **Maintenance**
   - Rotate TLS certificates and secrets regularly.
   - Apply system updates and security patches.
   - Review audit logs for anomalous activity.
3. **Incident Response**
   - Document escalation paths.
   - Keep recovery procedures validated and tested.
   - Maintain contact information for on-call engineers.

---

## References

- [Installation Guide](installation.md)
- [Audit and Quota Documentation](audit-and-quota.md)
- [Docker Deployment Guide](../infra/docker/README.md)
- [Main Project README](../README.md)
