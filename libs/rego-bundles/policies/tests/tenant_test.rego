# Unit tests for lib/tenant.rego
# Tests multi-tenant isolation and validation helpers

package lib.tenant

import rego.v1

# Test matches returns true for same tenant
test_matches_true {
	matches("tenant-a", "tenant-a")
	matches("tenant-123", "tenant-123")
}

# Test matches returns false for different tenants
test_matches_false {
	not matches("tenant-a", "tenant-b")
	not matches("tenant-a", "tenant-A") # case-sensitive
	not matches("", "tenant-a") # empty tenant ID
}

# Test is_owner returns true when both tenant and user match
test_is_owner_true {
	subject := {"tenant_id": "tenant-a", "user_id": "user-1"}
	resource := {"owner_tenant": "tenant-a", "owner_user": "user-1"}
	is_owner(subject, resource)
}

# Test is_owner returns false for different tenant
test_is_owner_false_different_tenant {
	subject := {"tenant_id": "tenant-a", "user_id": "user-1"}
	resource := {"owner_tenant": "tenant-b", "owner_user": "user-1"}
	not is_owner(subject, resource)
}

# Test is_owner returns false for different user
test_is_owner_false_different_user {
	subject := {"tenant_id": "tenant-a", "user_id": "user-1"}
	resource := {"owner_tenant": "tenant-a", "owner_user": "user-2"}
	not is_owner(subject, resource)
}

# Test has_clearance returns true when level is sufficient
test_has_clearance_true {
	subject := {"clearance_level": 3}
	has_clearance(subject, 2) # 3 >= 2
	has_clearance(subject, 3) # 3 >= 3
}

# Test has_clearance returns false when level is insufficient
test_has_clearance_false {
	subject := {"clearance_level": 1}
	not has_clearance(subject, 2) # 1 < 2
}

# Test has_role returns true when role exists
test_has_role_true {
	subject := {"roles": ["admin", "operator"]}
	has_role(subject, "admin")
	has_role(subject, "operator")
}

# Test has_role returns false when role doesn't exist
test_has_role_false {
	subject := {"roles": ["viewer"]}
	not has_role(subject, "admin")
	not has_role(subject, "operator")
}

# Test has_any_role returns true when at least one role matches
test_has_any_role_true {
	subject := {"roles": ["operator"]}
	has_any_role(subject, ["admin", "operator"])
}

# Test has_any_role returns false when no roles match
test_has_any_role_false {
	subject := {"roles": ["viewer"]}
	not has_any_role(subject, ["admin", "operator"])
}

# Test validate_tenant_boundary returns true for valid same-tenant access
test_validate_tenant_boundary_valid {
	validate_tenant_boundary("tenant-a", "tenant-a")
}

# Test validate_tenant_boundary returns false for different tenants
test_validate_tenant_boundary_invalid {
	not validate_tenant_boundary("tenant-a", "tenant-b")
	not validate_tenant_boundary("", "tenant-a") # missing tenant ID
	not validate_tenant_boundary("tenant-a", "") # missing resource tenant
}
