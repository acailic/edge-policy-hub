# Data Residency Template Policy
# Purpose: Enforce GDPR-compliant data residency by blocking EU data access from non-EU locations
#
# Input requirements:
#   - subject.tenant_id: Tenant identifier for isolation
#   - subject.device_location: Device country code (e.g., "DE", "US") - optional
#   - resource.region: Resource classification (e.g., "EU", "US")
#   - resource.owner_tenant: Tenant that owns the resource
#   - environment.country: Request origin country code - optional
#   - action: Operation being performed (e.g., "read", "write")
#
# Expected behavior:
#   - Allow only if all conditions pass:
#     * Tenant isolation: subject and resource belong to same tenant
#     * EU resource: resource.region == "EU"
#     * EU location: subject location is within EU (checks both environment.country and subject.device_location)
#     * Valid action: action is "read" or "write"
#
# Usage example:
#   To adapt this for a specific tenant:
#   1. Change package to: package tenants.{tenant_id}
#   2. Add explicit tenant_id check if needed
#   3. Customize geo restrictions based on tenant requirements

package templates.data_residency

import rego.v1

# Import helper modules
import data.lib.geo
import data.lib.tenant

# Default deny for security
default allow := false

# Allow access to EU resources from EU locations only
allow {
	# Enforce tenant isolation - no cross-tenant access
	tenant.matches(input.subject.tenant_id, input.resource.owner_tenant)

	# Check resource is classified as EU
	input.resource.region == "EU"

	# Check subject location is in EU
	# Accept location from either environment.country or subject.device_location
	eu_location_check

	# Only allow read or write actions
	input.action in ["read", "write"]
}

# Helper rule: Check if request originates from EU
# Accepts location from either environment.country or subject.device_location
eu_location_check {
	geo.is_eu_country(input.environment.country)
}

eu_location_check {
	geo.is_eu_country(input.subject.device_location)
}

# Allow access to non-EU resources from any location (no geo restriction)
allow {
	# Enforce tenant isolation
	tenant.matches(input.subject.tenant_id, input.resource.owner_tenant)

	# Non-EU resources have no geographic restrictions
	input.resource.region != "EU"

	# Valid action
	input.action in ["read", "write"]
}
