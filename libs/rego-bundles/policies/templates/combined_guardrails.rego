# Combined Guardrails Template Policy
# Purpose: Production-ready policy combining all guardrails for comprehensive edge enforcement
#
# Input requirements (all fields from subject, resource, environment, action):
#   - subject.tenant_id: Tenant identifier
#   - subject.clearance_level: Numeric clearance level
#   - subject.roles: Array of role names
#   - subject.device_location: Device country code
#   - resource.owner_tenant: Resource owner tenant
#   - resource.region: Resource region classification
#   - environment.country: Request origin country
#   - environment.bandwidth_used: Current bandwidth usage in GB
#   - environment.time: Request timestamp (ISO 8601)
#   - action: Operation being performed
#
# Expected behavior:
#   All checks must pass for allow to be true:
#   1. Tenant isolation (hard boundary)
#   2. Clearance level >= 2
#   3. Data residency (EU resources require EU location)
#   4. Cost guardrail (writes require quota available)
#   5. Time window (business hours or admin override)
#
# Customization:
#   - Adjust bandwidth limit (default 100 GB)
#   - Add/remove time restrictions
#   - Modify clearance level requirements
#   - Add additional guardrail checks
#
# Performance note:
#   Rego short-circuits on first failing check, so order matters.
#   Cheapest checks (tenant isolation) are evaluated first.
#
# Example input document:
# {
#   "subject": {
#     "tenant_id": "tenant-a",
#     "clearance_level": 3,
#     "roles": ["operator"],
#     "device_location": "DE"
#   },
#   "resource": {
#     "owner_tenant": "tenant-a",
#     "region": "EU"
#   },
#   "environment": {
#     "country": "DE",
#     "bandwidth_used": 50,
#     "time": "2025-10-16T10:00:00Z"
#   },
#   "action": "read"
# }

package templates.combined_guardrails

import rego.v1

# Import all helper modules
import data.lib.geo
import data.lib.quota
import data.lib.tenant
import data.lib.time

# Default deny for security
default allow := false

# Main allow rule with all checks combined
allow {
	# 1. Tenant isolation (cheapest check first)
	tenant.validate_tenant_boundary(input.subject.tenant_id, input.resource.owner_tenant)

	# 2. Clearance level check
	tenant.has_clearance(input.subject, 2)

	# 3. Data residency check (EU resources require EU location)
	data_residency_check

	# 4. Cost guardrail check (writes require quota)
	quota_check

	# 5. Time window check (business hours or admin)
	time_check
}

# Helper rule: Data residency validation
# Returns true if resource is non-EU OR subject is in EU
data_residency_check {
	# Non-EU resources have no geographic restrictions
	input.resource.region != "EU"
}

data_residency_check {
	# EU resources require EU location
	input.resource.region == "EU"
	geo.is_eu_country(input.environment.country)
}

# Helper rule: Quota validation
# Returns true if action is read OR quota not exceeded
quota_check {
	# Reads don't count against quota
	input.action == "read"
}

quota_check {
	# Writes require available quota (100 GB limit)
	input.action in ["write", "upload", "publish"]
	quota.bandwidth_within_limit(input.environment.bandwidth_used, 100)
}

# Helper rule: Time window validation
# Returns true if business hours OR admin role
time_check {
	# During business hours on weekdays
	time.is_business_hours(input.environment.time)
	time.is_weekday(input.environment.time)
}

time_check {
	# Admin override for time restrictions
	tenant.has_role(input.subject, "admin")
}

# Collect all violation reasons for debugging and audit logs
deny_reasons[reason] {
	not tenant.validate_tenant_boundary(input.subject.tenant_id, input.resource.owner_tenant)
	reason := "Cross-tenant access"
}

deny_reasons[reason] {
	not data_residency_check
	reason := "Data residency violation"
}

deny_reasons[reason] {
	not quota_check
	reason := "Quota exceeded"
}

deny_reasons[reason] {
	not time_check
	reason := "Outside business hours"
}
