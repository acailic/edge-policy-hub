import type { AbacInput } from './abac';

export type PolicyStatus = 'draft' | 'active' | 'inactive' | 'archived';

export interface PolicyMetadata {
  version: string;
  author?: string;
  description?: string;
  created_at: string;
}

export interface PolicyBundle {
  bundle_id: string;
  tenant_id: string;
  version: number;
  rego_code: string;
  metadata?: PolicyMetadata;
  status: PolicyStatus;
  created_at: string;
  activated_at?: string;
}

export interface CompilationError {
  message: string;
  line?: number;
  column?: number;
  attribute?: string;
}

export interface CompilePolicyRequest {
  source: string;
  tenant_id: string;
  metadata?: PolicyMetadata;
}

export interface CompilePolicyResponse {
  success: boolean;
  rego?: string;
  errors?: CompilationError[];
}

export interface TestPolicyRequest {
  tenant_id: string;
  input: AbacInput;
}

export interface TestPolicyResponse {
  allow: boolean;
  redact?: string[];
  reason?: string;
  eval_duration_micros?: number;
}

export interface DeployPolicyRequest {
  tenant_id: string;
  rego_code: string;
  metadata: PolicyMetadata;
  activate: boolean;
}

export interface DeployPolicyResponse {
  bundle_id: string;
  version: number;
}
