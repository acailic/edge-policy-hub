export interface DecisionEvent {
  event_id: string;
  tenant_id: string;
  timestamp: string;
  decision: {
    allow: boolean;
    redact?: string[];
    reason?: string;
  };
  input: Record<string, unknown>;
  metrics: {
    eval_duration_micros: number;
    tenant_id: string;
  };
}

export interface AuditLogEntry {
  log_id: string;
  tenant_id: string;
  timestamp: string;
  decision: string;
  protocol: string;
  subject: Record<string, unknown>;
  action: string;
  resource: Record<string, unknown>;
  environment: Record<string, unknown>;
  policy_version?: number;
  reason?: string;
  signature: string;
  uploaded: boolean;
}

export interface QuotaMetrics {
  tenant_id: string;
  message_count: number;
  bytes_sent: number;
  message_limit: number;
  bandwidth_limit_bytes: number;
  last_reset: string;
  period: string;
}

export interface QuotaStatus {
  exceeded: boolean;
  quota_type?: string;
  limit?: number;
  current?: number;
  warning_threshold_reached: boolean;
}

export interface AuditLogFilter {
  tenant_id: string;
  start_time?: string;
  end_time?: string;
  decision?: string;
  protocol?: string;
  limit?: number;
}
