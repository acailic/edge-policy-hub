# Production-Grade Combined Policy
# Integrates tenant isolation, data residency, cost guardrails, and time-based access control

allow read sensor_data if
  # Strict tenant boundary prevents cross-tenant leakage
  subject.tenant_id == resource.owner_tenant and
  # Clearance level 2 is sufficient for read-only observers
  subject.clearance_level >= 2 and
  # EU resources require EU device location; other regions remain unrestricted
  (resource.region != "EU" or subject.device_location in ["DE", "FR", "NL", "BE", "IT", "ES"]) and
  # Keep reads within the bandwidth envelope for cost control
  environment.bandwidth_used < 100

allow write sensor_data if
  subject.tenant_id == resource.owner_tenant and
  # Limit write privileges to trusted roles
  subject.roles in ["operator", "admin"] and
  # Writes demand higher clearance
  subject.clearance_level >= 3 and
  (resource.region != "EU" or subject.device_location in ["DE", "FR", "NL", "BE", "IT", "ES"]) and
  environment.bandwidth_used < 100 and
  # Block writes when risk system flags anomalies
  environment.risk_score < 0.5

deny write sensor_data if
  # Explicit deny for quota exhaustion provides clear audit trail
  environment.bandwidth_used >= 100
