# Example: Tenant-specific policy using helper modules
# This demonstrates how to import and use the shared helpers
#
# To adapt this for a specific tenant:
# 1. Change package name to: package tenants.{your_tenant_id}
# 2. Customize limits (bandwidth quota, clearance level, etc.)
# 3. Add/remove checks based on tenant requirements
# 4. Deploy to config/tenants.d/{tenant_id}/ along with helper modules

package tenants.example_tenant

import rego.v1

# Import helper modules
import data.lib.geo
import data.lib.quota
import data.lib.tenant
import data.lib.time

# Default deny for security
default allow := false

# Main allow rule combining multiple helpers
# This demonstrates composing checks using helper rules for clarity
allow {
	# Tenant isolation (required for all policies)
	# This ensures the subject and resource belong to the same tenant
	tenant.validate_tenant_boundary(input.subject.tenant_id, input.resource.owner_tenant)

	# Clearance check - minimum level 2 required
	tenant.has_clearance(input.subject, 2)

	# Data residency check (if EU resource)
	data_residency_ok

	# Quota check (if write action)
	quota_ok

	# Time check (business hours or admin)
	time_ok
}

# Helper rule: Data residency check
# Non-EU resources are always OK
data_residency_ok {
	input.resource.region != "EU"
}

# EU resources require EU location
data_residency_ok {
	input.resource.region == "EU"
	geo.is_eu_country(input.environment.country)
}

# Helper rule: Quota check
# Reads don't count against quota
quota_ok {
	input.action == "read"
}

# Writes must be under quota (100 GB limit)
quota_ok {
	input.action in ["write", "upload"]
	quota.bandwidth_within_limit(input.environment.bandwidth_used, 100)
}

# Helper rule: Time check
# Business hours on weekdays
time_ok {
	time.is_business_hours(input.environment.time)
	time.is_weekday(input.environment.time)
}

# Admin override for time restrictions
time_ok {
	tenant.has_role(input.subject, "admin")
}

# Collect deny reasons for debugging and audit logging
# These help operators understand why a request was denied
deny_reasons[reason] {
	not tenant.validate_tenant_boundary(input.subject.tenant_id, input.resource.owner_tenant)
	reason := "Cross-tenant access denied"
}

deny_reasons[reason] {
	not data_residency_ok
	reason := "Data residency violation"
}

deny_reasons[reason] {
	not quota_ok
	reason := "Quota exceeded"
}

deny_reasons[reason] {
	not time_ok
	reason := "Outside business hours"
}

# Example input document for testing:
# {
#   "subject": {
#     "tenant_id": "example_tenant",
#     "clearance_level": 3,
#     "roles": ["operator"],
#     "user_id": "user-123"
#   },
#   "resource": {
#     "owner_tenant": "example_tenant",
#     "region": "EU",
#     "id": "resource-456"
#   },
#   "environment": {
#     "country": "DE",
#     "bandwidth_used": 50,
#     "time": "2025-10-16T10:00:00Z"
#   },
#   "action": "read"
# }
#
# How helper imports work:
# - import data.lib.geo makes all functions from lib/geo.rego available
# - Call functions using: geo.is_eu_country(...), quota.bandwidth_within_limit(...), etc.
# - Helpers must be present in the bundle directory structure
#
# How to compose checks:
# - Break down complex logic into helper rules (data_residency_ok, quota_ok, time_ok)
# - Use helper rules in main allow rule for readability
# - Each helper rule can have multiple definitions (alternative paths to satisfy)
#
# How this pattern scales:
# - Add new checks by creating new helper rules
# - Import additional library modules as needed
# - Deny reasons help with debugging and compliance auditing
# - Performance: Rego short-circuits on first failing condition
