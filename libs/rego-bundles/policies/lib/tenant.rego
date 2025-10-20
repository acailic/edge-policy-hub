# Multi-tenant isolation and validation
# Provides reusable functions for enforcing hard tenant boundaries and zero-trust isolation

package lib.tenant

import rego.v1

# Check if subject tenant ID matches resource owner tenant
# Parameters:
#   subject_tenant_id: Tenant ID from subject
#   resource_owner_tenant: Tenant ID that owns the resource
# Returns: true if both tenant IDs match exactly
# Usage: tenant.matches(input.subject.tenant_id, input.resource.owner_tenant)
matches(subject_tenant_id, resource_owner_tenant) {
	subject_tenant_id == resource_owner_tenant
}

# Check if subject is the owner of the resource (tenant and user level)
# Parameters:
#   subject: Subject object with tenant_id and user_id
#   resource: Resource object with owner_tenant and owner_user
# Returns: true if both tenant and user IDs match
# Usage: tenant.is_owner(input.subject, input.resource)
is_owner(subject, resource) {
	subject.tenant_id == resource.owner_tenant
	subject.user_id == resource.owner_user
}

# Check if subject has sufficient clearance level
# Parameters:
#   subject: Subject object with clearance_level field
#   required_level: Minimum required clearance level
# Returns: true if subject.clearance_level >= required_level
# Usage: tenant.has_clearance(input.subject, 2)
has_clearance(subject, required_level) {
	subject.clearance_level >= required_level
}

# Check if subject has a specific role
# Parameters:
#   subject: Subject object with roles array
#   role_name: Role name to check for
# Returns: true if role_name is in subject.roles
# Usage: tenant.has_role(input.subject, "admin")
has_role(subject, role_name) {
	role_name in subject.roles
}

# Check if subject has any role from a list
# Parameters:
#   subject: Subject object with roles array
#   role_list: Array of role names to check
# Returns: true if subject has at least one role from the list
# Usage: tenant.has_any_role(input.subject, ["admin", "operator"])
has_any_role(subject, role_list) {
	some role in role_list
	role in subject.roles
}

# Validate tenant boundary with strict checks
# This enforces the hard tenant boundary pattern - no cross-tenant access
# Parameters:
#   subject_tenant: Tenant ID from subject
#   resource_tenant: Tenant ID that owns the resource
# Returns: true only if both are non-empty and match exactly
# Usage: tenant.validate_tenant_boundary(input.subject.tenant_id, input.resource.owner_tenant)
validate_tenant_boundary(subject_tenant, resource_tenant) {
	subject_tenant != ""
	resource_tenant != ""
	subject_tenant == resource_tenant
}
