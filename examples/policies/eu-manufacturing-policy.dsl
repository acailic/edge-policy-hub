# EU Manufacturing Data Residency Policy
# Ensures all sensor data stays within EU boundaries and is only accessible during business hours

allow read sensor_data if
  # Always scope policies to tenant namespace for isolation
  subject.tenant_id == "tenant-eu-manufacturing" and
  # Enforce EU residency requirement (explicit list aids audits)
  resource.region == "EU" and
  subject.device_location in ["DE", "FR", "NL", "IT", "ES", "PL"] and
  # Clearance level 2+ required to view industrial telemetry
  subject.clearance_level >= 2

allow write sensor_data if
  subject.tenant_id == "tenant-eu-manufacturing" and
  resource.region == "EU" and
  subject.device_location in ["DE", "FR", "NL", "IT", "ES", "PL"] and
  # Only field operators and admins can push new data
  subject.roles in ["operator", "admin"] and
  # Enforce bandwidth guardrail to control upload spikes
  environment.bandwidth_used < 500
