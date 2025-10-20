export interface SubjectAttributes {
  tenant_id: string;
  user_id?: string;
  device_id?: string;
  roles?: string[];
  clearance_level?: number;
  device_location?: string;
}

export interface ResourceAttributes {
  type: string;
  id?: string;
  classification?: string;
  region?: string;
  owner_tenant: string;
  owner_user?: string;
}

export interface EnvironmentAttributes {
  time: string;
  country?: string;
  network?: string;
  risk_score?: number;
  bandwidth_used?: number;
  message_count?: number;
}

export interface AbacInput {
  subject: SubjectAttributes;
  action: string;
  resource: ResourceAttributes;
  environment: EnvironmentAttributes;
}
