import { invoke } from "@tauri-apps/api/core";
import type {
  CreateTenantRequest,
  Tenant,
  UpdateTenantRequest,
} from "../types/tenant";
import type {
  CompilePolicyResponse,
  DeployPolicyResponse,
  PolicyBundle,
  PolicyMetadata,
  TestPolicyResponse,
} from "../types/policy";
import type { AbacInput } from "../types/abac";
import type {
  AuditLogEntry,
  AuditLogFilter,
  QuotaMetrics,
  QuotaStatus,
} from "../types/monitoring";

function mapError(error: unknown): Error {
  if (error instanceof Error) {
    return error;
  }

  if (typeof error === "string") {
    return new Error(error);
  }

  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof (error as { message: unknown }).message === "string"
  ) {
    return new Error((error as { message: string }).message);
  }

  return new Error("Unexpected error while communicating with backend");
}

async function callCommand<T>(command: string, args?: Record<string, unknown>) {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw mapError(error);
  }
}

export async function listTenants(statusFilter?: string) {
  return callCommand<Tenant[]>("list_tenants", {
    status_filter: statusFilter,
  });
}

export async function getTenant(tenantId: string) {
  return callCommand<Tenant>("get_tenant", { tenant_id: tenantId });
}

export async function createTenant(request: CreateTenantRequest) {
  return callCommand<Tenant>("create_tenant", { request });
}

export async function updateTenant(
  tenantId: string,
  request: UpdateTenantRequest,
) {
  return callCommand<Tenant>("update_tenant", { tenant_id: tenantId, request });
}

export async function deleteTenant(tenantId: string) {
  await callCommand<void>("delete_tenant", { tenant_id: tenantId });
}

export async function setQuotaLimits(
  tenantId: string,
  messageLimit: number,
  bandwidthLimitGb: number,
) {
  await callCommand<void>("set_quota_limits", {
    tenant_id: tenantId,
    message_limit: messageLimit,
    bandwidth_limit_gb: bandwidthLimitGb,
  });
}

export async function compilePolicyDsl(
  source: string,
  tenantId: string,
  metadata?: PolicyMetadata,
) {
  return callCommand<CompilePolicyResponse>("compile_policy_dsl", {
    source,
    tenant_id: tenantId,
    metadata,
  });
}

export async function testPolicy(tenantId: string, input: AbacInput) {
  return callCommand<TestPolicyResponse>("test_policy", { tenant_id: tenantId, input });
}

export async function deployPolicy(
  tenantId: string,
  regoCode: string,
  metadata: PolicyMetadata,
  activate: boolean,
) {
  return callCommand<DeployPolicyResponse>("deploy_policy", {
    tenant_id: tenantId,
    rego_code: regoCode,
    metadata,
    activate,
  });
}

export async function listPolicyBundles(tenantId: string) {
  return callCommand<PolicyBundle[]>("list_policy_bundles", { tenant_id: tenantId });
}

export async function getPolicyBundle(bundleId: string) {
  return callCommand<PolicyBundle>("get_policy_bundle", { bundle_id: bundleId });
}

export async function activatePolicyBundle(bundleId: string) {
  await callCommand<void>("activate_policy_bundle", { bundle_id: bundleId });
}

export async function rollbackPolicy(tenantId: string, bundleId: string) {
  await callCommand<void>("rollback_policy", { tenant_id: tenantId, bundle_id: bundleId });
}

export async function queryAuditLogs(filter: AuditLogFilter) {
  return callCommand<AuditLogEntry[]>("query_audit_logs", {
    tenant_id: filter.tenant_id,
    start_time: filter.start_time,
    end_time: filter.end_time,
    decision: filter.decision,
    protocol: filter.protocol,
    limit: filter.limit,
  });
}

export async function getQuotaMetrics(tenantId: string) {
  return callCommand<QuotaMetrics>("get_quota_metrics", { tenant_id: tenantId });
}

export async function listAllQuotaMetrics() {
  return callCommand<QuotaMetrics[]>("list_all_quota_metrics");
}

export async function checkQuotaStatus(tenantId: string) {
  return callCommand<QuotaStatus>("check_quota_status", { tenant_id: tenantId });
}
