export type TenantStatus = "active" | "suspended" | "deleted";

export interface TenantConfig {
  quotas?: {
    message_limit: number;
    bandwidth_limit_gb: number;
  };
  features?: {
    data_residency?: string[];
    pii_redaction?: boolean;
  };
}

export interface Tenant {
  tenant_id: string;
  name: string;
  status: TenantStatus;
  created_at: string;
  updated_at: string;
  config?: TenantConfig;
}

export interface CreateTenantRequest {
  tenant_id: string;
  name: string;
  config?: TenantConfig;
}

export interface UpdateTenantRequest {
  name?: string;
  status?: TenantStatus;
  config?: TenantConfig;
}
