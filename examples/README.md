# Examples Directory

This directory contains sample tenant configurations and policy examples demonstrating real-world use cases for Edge Policy Hub.

## Tenant Configurations

**tenant-eu-manufacturing.json**
- Use case: Industrial IoT in European manufacturing facilities
- Key features: High message limits (100k/day), strict EU data residency, GDPR compliance
- Quota: 500 GB/month bandwidth for sensor data
- Compliance: GDPR with 7-year retention (2555 days)

**tenant-us-retail.json**
- Use case: Retail point-of-sale and inventory systems in United States
- Key features: PCI-DSS compliance, business hours restrictions, VPN requirement
- Quota: 100 GB/month bandwidth (standard retail workloads)
- Compliance: PCI-DSS for payment data

**tenant-apac-logistics.json**
- Use case: Real-time GPS tracking for logistics fleet in Asia-Pacific
- Key features: Very high message limits (200k/day), multi-country operations, mobile network support
- Quota: 1 TB/month bandwidth for GPS telemetry
- Compliance: Data sovereignty (no cross-border transfers)

## Policy Examples

**eu-manufacturing-policy.dsl**
- Enforces EU data residency for sensor data
- Requires clearance level 2+ for reads, 3+ for writes
- Includes bandwidth quota check (500 GB limit)
- Demonstrates multi-condition policies with geo validation

**us-retail-policy.dsl**
- Enforces PCI-DSS compliance for payment data
- Restricts access to business hours (9 AM - 5 PM Eastern)
- Role-based access (cashiers read, admins write)
- Includes risk score check for fraud prevention

**apac-logistics-policy.dsl**
- MQTT-specific policy for GPS telemetry
- Enforces data sovereignty (APAC countries only)
- High message quota (200k/day)
- Device type filtering (only GPS trackers)

**combined-production-policy.dsl**
- Production-ready policy combining all guardrails
- Tenant isolation + data residency + cost control + risk-based access
- Demonstrates best practices for complex policies
- Suitable as template for new tenants

## Using Examples

### Import Tenant Configuration

```bash
# Using Tauri UI
1. Navigate to Tenants → Add New
2. Click "Import from JSON"
3. Select tenant configuration file
4. Review and customize settings
5. Click Create

# Using API directly
curl -X POST http://localhost:8182/api/tenants \
  -H "Content-Type: application/json" \
  -d @examples/tenants/tenant-eu-manufacturing.json
```

### Deploy Sample Policy

```bash
# Using Tauri UI
1. Navigate to Tenant → Manage Policies
2. Click "Load Template"
3. Select policy file (e.g., eu-manufacturing-policy.dsl)
4. Review and customize for your tenant
5. Click Compile to validate
6. Click Deploy & Activate

# Using policy-dsl CLI (if available)
cd libs/policy-dsl
cargo run --example compile_example ../../examples/policies/eu-manufacturing-policy.dsl
```

## Customization Guide

### Tenant Configuration
1. Change `tenant_id` to your organization identifier
2. Adjust `quotas` based on expected traffic volume
3. Set `data_residency` to required regions
4. Configure `compliance` for your industry
5. Customize `network_config` for your infrastructure

### Policy Files
1. Replace tenant IDs in conditions with your tenant
2. Adjust allowed countries/regions for your geography
3. Modify quota limits to match tenant configuration
4. Add or remove conditions based on requirements
5. Test policies thoroughly before deploying to production

## Testing Examples

Before deploying to production:

1. **Compile Policy**
   - Use Tauri UI Policy Builder to compile DSL
   - Check for syntax errors and validation warnings
   - Review generated Rego code

2. **Test with Simulator**
   - Use Test Simulator in Tauri UI
   - Create sample ABAC inputs matching your scenario
   - Verify allow/deny decisions
   - Exercise edge cases (quota limits, geo boundaries)

3. **Deploy as Draft**
   - Deploy policy without activating
   - Test workflows in staging environment
   - Monitor decision stream for unexpected denials
   - Activate only after validation

## Best Practices

1. **Start with Templates** – Use sample policies as a foundation
2. **Test Incrementally** – Add one condition at a time and re-test
3. **Document Changes** – Add comments explaining business logic
4. **Version Policies** – Track versions for rollback capability
5. **Monitor Enforcement** – Watch decision stream after deployment
6. **Review Audit Logs** – Check for unexpected denials or anomalies

## Additional Resources

- [Policy DSL Reference](../docs/policy-dsl-reference.md) – Complete syntax guide
- [Deployment Guide](../docs/deployment.md) – Production deployment procedures
- [API Reference](../docs/api-reference.md) – REST API documentation
- [Rego Bundles Guide](../docs/rego-bundles.md) – Advanced Rego patterns

Consult the main project README for architecture overview and getting started instructions.
