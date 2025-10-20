# Unit tests for templates/multi_tenant_separation.rego
# Tests multi-tenant separation template policy

package templates.multi_tenant_separation

import rego.v1

# Test allow for same tenant with sufficient clearance
test_allow_same_tenant_sufficient_clearance {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"clearance_level": 3,
			"roles": ["operator"],
		},
		"resource": {"owner_tenant": "tenant-a"},
		"action": "read",
	}
	allow with input as input_data
}

# Test deny for same tenant with insufficient clearance
test_deny_same_tenant_insufficient_clearance {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"clearance_level": 1,
			"roles": ["operator"],
		},
		"resource": {"owner_tenant": "tenant-a"},
		"action": "read",
	}
	not allow with input as input_data
}

# Test deny for cross-tenant access
test_deny_cross_tenant_access {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"clearance_level": 5,
			"roles": ["operator"],
		},
		"resource": {"owner_tenant": "tenant-b"},
		"action": "read",
	}
	not allow with input as input_data
}

# Test allow for admin override on clearance
test_allow_admin_override {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"clearance_level": 1,
			"roles": ["admin"],
		},
		"resource": {"owner_tenant": "tenant-a"},
		"action": "read",
	}
	allow with input as input_data
}

# Test deny for admin cross-tenant access
test_deny_admin_cross_tenant {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"clearance_level": 5,
			"roles": ["admin"],
		},
		"resource": {"owner_tenant": "tenant-b"},
		"action": "read",
	}
	not allow with input as input_data
}

# Test allow for write action
test_allow_write_action {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"clearance_level": 3,
			"roles": ["operator"],
		},
		"resource": {"owner_tenant": "tenant-a"},
		"action": "write",
	}
	allow with input as input_data
}

# Test deny for invalid action
test_deny_invalid_action {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"clearance_level": 3,
			"roles": ["operator"],
		},
		"resource": {"owner_tenant": "tenant-a"},
		"action": "delete",
	}
	not allow with input as input_data
}

# Test deny reason for cross-tenant access
test_deny_reason_cross_tenant {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"clearance_level": 3,
			"roles": ["operator"],
		},
		"resource": {"owner_tenant": "tenant-b"},
		"action": "read",
	}
	deny with input as input_data
	contains(deny_reason, "Cross-tenant access denied") with input as input_data
}

# Test clearance level boundary (exactly at level 2)
test_clearance_level_boundary {
	input_at_boundary := {
		"subject": {
			"tenant_id": "tenant-a",
			"clearance_level": 2,
			"roles": ["operator"],
		},
		"resource": {"owner_tenant": "tenant-a"},
		"action": "read",
	}
	allow with input as input_at_boundary

	input_below_boundary := {
		"subject": {
			"tenant_id": "tenant-a",
			"clearance_level": 1,
			"roles": ["operator"],
		},
		"resource": {"owner_tenant": "tenant-a"},
		"action": "read",
	}
	not allow with input as input_below_boundary
}
