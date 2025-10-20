# Multi-Tenant Separation Template Policy
# Purpose: Enforce zero-trust tenant isolation to prevent data leakage between tenants
#
# Input requirements:
#   - subject.tenant_id: Tenant identifier for the requesting subject
#   - subject.clearance_level: Numeric clearance level (minimum 2 required)
#   - subject.roles: Array of role names (e.g., ["admin", "operator"])
#   - resource.owner_tenant: Tenant that owns the resource
#   - action: Operation being performed (e.g., "read", "write")
#
# Expected behavior:
#   - Hard boundary: no cross-tenant access even if roles match
#   - Minimum clearance level 2 required for normal access
#   - Admin role can override clearance requirement but NOT tenant boundary
#   - This is the foundational policy that should be included in all tenant bundles
#
# Security note:
#   This enforces the core multi-tenant isolation pattern required by the system architecture.
#   Even users with identical roles in different tenants cannot access each other's resources.
#
# Usage example:
#   This prevents tenant A from accessing tenant B's resources:
#   Subject: {"tenant_id": "tenant-a", "clearance_level": 5, "roles": ["admin"]}
#   Resource: {"owner_tenant": "tenant-b"}
#   Result: DENIED (cross-tenant access)

package templates.multi_tenant_separation

import rego.v1

# Import helper module
import data.lib.tenant

# Default deny for security
default allow := false

# Allow same-tenant access with sufficient clearance
allow {
	# Strict tenant boundary validation
	tenant.validate_tenant_boundary(input.subject.tenant_id, input.resource.owner_tenant)

	# Require minimum clearance level 2
	tenant.has_clearance(input.subject, 2)

	# Only allow read or write actions
	input.action in ["read", "write"]
}

# Allow admin override for clearance requirement (but NOT tenant boundary)
allow {
	# Tenant must still match (hard boundary)
	tenant.matches(input.subject.tenant_id, input.resource.owner_tenant)

	# Admin role bypasses clearance level check
	tenant.has_role(input.subject, "admin")

	# Valid action
	input.action in ["read", "write"]
}

# Deny cross-tenant access attempts with reason
deny {
	input.subject.tenant_id != input.resource.owner_tenant
}

# Provide human-readable denial reason for cross-tenant access
deny_reason := reason {
	deny
	subject_tenant := input.subject.tenant_id
	resource_tenant := input.resource.owner_tenant
	reason := sprintf("Cross-tenant access denied: tenant '%v' cannot access resources owned by '%v'", [subject_tenant, resource_tenant])
}
