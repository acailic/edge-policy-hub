# APAC Logistics Real-Time Tracking Policy
# Allows high-volume GPS telemetry within APAC region with data sovereignty enforcement

allow publish gps_telemetry if
  # Tenant isolation is the first guardrail
  subject.tenant_id == "tenant-apac-logistics" and
  # Only GPS tracking devices can publish telemetry
  subject.device_id in ["gps_tracker"] and
  # Vehicles must report from approved countries
  environment.country in ["SG", "JP", "AU", "IN"] and
  # High message quota accommodates fleet tracking
  environment.message_count < 200000

allow subscribe gps_telemetry if
  subject.tenant_id == "tenant-apac-logistics" and
  # Dispatch, managers, and admins can view vehicle streams
  subject.roles in ["dispatcher", "manager", "admin"] and
  environment.country in ["SG", "JP", "AU", "IN"]

deny write gps_telemetry if
  # Prevent any cross-border data movement
  environment.country not in ["SG", "JP", "AU", "IN"]
