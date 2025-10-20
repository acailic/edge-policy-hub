import type { AbacInput } from "../types/abac";

export const SAMPLE_POLICIES: Record<string, string> = {
  data_residency: `# EU Data Residency Policy
# Ensures that EU-classified resources can only be accessed from EU locations

allow read sensor_data if subject.tenant_id == "tenant-eu" and resource.region == "EU" and subject.device_location in ["DE", "FR", "NL", "BE", "IT", "ES"]`,
  cost_guardrail: `# Cost Guardrail Policy
# Blocks uploads when monthly bandwidth quota is exceeded

deny write sensor_data if environment.bandwidth_used >= 100`,
  multi_tenant_separation: `# Multi-Tenant Separation Policy
# Ensures tenants can only access their own resources

allow read sensor_data if subject.tenant_id == resource.owner_tenant and subject.clearance_level >= 2`,
};

export const SAMPLE_ABAC_INPUTS: Record<string, AbacInput> = {
  eu_allow: {
    subject: {
      tenant_id: "tenant-eu",
      user_id: "alice",
      device_location: "DE",
      roles: ["engineer"],
      clearance_level: 3,
    },
    action: "read",
    resource: {
      type: "sensor_data",
      id: "sensor-001",
      classification: "confidential",
      region: "EU",
      owner_tenant: "tenant-eu",
    },
    environment: {
      time: "2024-01-01T10:00:00Z",
      country: "DE",
      network: "corporate",
      risk_score: 0.2,
      bandwidth_used: 45,
    },
  },
  eu_deny: {
    subject: {
      tenant_id: "tenant-eu",
      user_id: "bob",
      device_location: "US",
      roles: ["analyst"],
      clearance_level: 2,
    },
    action: "read",
    resource: {
      type: "sensor_data",
      id: "sensor-042",
      classification: "confidential",
      region: "EU",
      owner_tenant: "tenant-eu",
    },
    environment: {
      time: "2024-01-01T12:00:00Z",
      country: "US",
      network: "remote",
      risk_score: 0.4,
      bandwidth_used: 20,
    },
  },
  quota_allow: {
    subject: {
      tenant_id: "tenant-cost",
      user_id: "carol",
      roles: ["operator"],
    },
    action: "write",
    resource: {
      type: "sensor_data",
      id: "stream-007",
      classification: "internal",
      region: "US",
      owner_tenant: "tenant-cost",
    },
    environment: {
      time: "2024-01-02T08:00:00Z",
      network: "edge",
      bandwidth_used: 50,
      message_count: 1200,
      risk_score: 0.3,
    },
  },
  quota_deny: {
    subject: {
      tenant_id: "tenant-cost",
      user_id: "dave",
      roles: ["operator"],
    },
    action: "write",
    resource: {
      type: "sensor_data",
      id: "stream-999",
      classification: "internal",
      region: "US",
      owner_tenant: "tenant-cost",
    },
    environment: {
      time: "2024-01-02T08:05:00Z",
      network: "edge",
      bandwidth_used: 150,
      message_count: 2000,
      risk_score: 0.5,
    },
  },
  cross_tenant_deny: {
    subject: {
      tenant_id: "tenant-a",
      user_id: "erin",
      clearance_level: 1,
      roles: ["viewer"],
    },
    action: "read",
    resource: {
      type: "sensor_data",
      id: "asset-55",
      classification: "restricted",
      region: "EU",
      owner_tenant: "tenant-b",
    },
    environment: {
      time: "2024-01-03T15:00:00Z",
      country: "DE",
      network: "vpn",
      risk_score: 0.1,
    },
  },
};

export function getPolicyDescription(policyKey: keyof typeof SAMPLE_POLICIES) {
  switch (policyKey) {
    case "data_residency":
      return "Restricts access to EU-region resources to devices operating from approved EU locations.";
    case "cost_guardrail":
      return "Denies write operations when bandwidth consumption exceeds the configured quota.";
    case "multi_tenant_separation":
      return "Allows access only when the subject and resource belong to the same tenant and clearance is sufficient.";
    default:
      return "Sample policy template.";
  }
}
