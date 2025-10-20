# Unit tests for templates/combined_guardrails.rego
# Tests combined guardrails template policy

package templates.combined_guardrails

import rego.v1

# Test allow when all checks pass
test_allow_all_checks_pass {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"clearance_level": 3,
			"roles": ["operator"],
			"device_location": "DE",
		},
		"resource": {
			"owner_tenant": "tenant-a",
			"region": "EU",
		},
		"environment": {
			"country": "DE",
			"bandwidth_used": 50,
			"time": "2025-10-16T10:00:00Z",
		},
		"action": "read",
	}
	allow with input as input_data
}

# Test deny for cross-tenant access
test_deny_cross_tenant {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"clearance_level": 3,
			"roles": ["operator"],
			"device_location": "DE",
		},
		"resource": {
			"owner_tenant": "tenant-b",
			"region": "EU",
		},
		"environment": {
			"country": "DE",
			"bandwidth_used": 50,
			"time": "2025-10-16T10:00:00Z",
		},
		"action": "read",
	}
	not allow with input as input_data
	"Cross-tenant access" in deny_reasons with input as input_data
}

# Test deny for data residency violation
test_deny_data_residency_violation {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"clearance_level": 3,
			"roles": ["operator"],
			"device_location": "US",
		},
		"resource": {
			"owner_tenant": "tenant-a",
			"region": "EU",
		},
		"environment": {
			"country": "US",
			"bandwidth_used": 50,
			"time": "2025-10-16T10:00:00Z",
		},
		"action": "read",
	}
	not allow with input as input_data
	"Data residency violation" in deny_reasons with input as input_data
}

# Test deny for quota exceeded
test_deny_quota_exceeded {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"clearance_level": 3,
			"roles": ["operator"],
			"device_location": "DE",
		},
		"resource": {
			"owner_tenant": "tenant-a",
			"region": "EU",
		},
		"environment": {
			"country": "DE",
			"bandwidth_used": 150,
			"time": "2025-10-16T10:00:00Z",
		},
		"action": "write",
	}
	not allow with input as input_data
	"Quota exceeded" in deny_reasons with input as input_data
}

# Test deny for outside business hours
test_deny_outside_business_hours {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"clearance_level": 3,
			"roles": ["operator"],
			"device_location": "DE",
		},
		"resource": {
			"owner_tenant": "tenant-a",
			"region": "EU",
		},
		"environment": {
			"country": "DE",
			"bandwidth_used": 50,
			"time": "2025-10-16T20:00:00Z", # 8 PM
		},
		"action": "read",
	}
	not allow with input as input_data
	"Outside business hours" in deny_reasons with input as input_data
}

# Test allow for admin outside business hours
test_allow_admin_outside_business_hours {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"clearance_level": 3,
			"roles": ["admin"],
			"device_location": "DE",
		},
		"resource": {
			"owner_tenant": "tenant-a",
			"region": "EU",
		},
		"environment": {
			"country": "DE",
			"bandwidth_used": 50,
			"time": "2025-10-16T20:00:00Z",
		},
		"action": "read",
	}
	allow with input as input_data
}

# Test allow for non-EU resource from any location
test_allow_non_eu_resource_any_location {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"clearance_level": 3,
			"roles": ["operator"],
			"device_location": "US",
		},
		"resource": {
			"owner_tenant": "tenant-a",
			"region": "US",
		},
		"environment": {
			"country": "US",
			"bandwidth_used": 50,
			"time": "2025-10-16T10:00:00Z",
		},
		"action": "read",
	}
	allow with input as input_data
}

# Test allow for read action over quota
test_allow_read_over_quota {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"clearance_level": 3,
			"roles": ["operator"],
			"device_location": "DE",
		},
		"resource": {
			"owner_tenant": "tenant-a",
			"region": "EU",
		},
		"environment": {
			"country": "DE",
			"bandwidth_used": 150,
			"time": "2025-10-16T10:00:00Z",
		},
		"action": "read",
	}
	allow with input as input_data
}

# Test multiple violations collected in deny_reasons
test_multiple_violations {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"clearance_level": 3,
			"roles": ["operator"],
			"device_location": "US",
		},
		"resource": {
			"owner_tenant": "tenant-b",
			"region": "EU",
		},
		"environment": {
			"country": "US",
			"bandwidth_used": 150,
			"time": "2025-10-16T20:00:00Z",
		},
		"action": "write",
	}
	not allow with input as input_data
	count(deny_reasons) >= 2 with input as input_data
}

# Test helper rule: data_residency_check
test_helper_data_residency_check {
	# Non-EU resource should pass
	input_non_eu := {
		"resource": {"region": "US"},
		"environment": {"country": "US"},
	}
	data_residency_check with input as input_non_eu

	# EU resource from EU location should pass
	input_eu_valid := {
		"resource": {"region": "EU"},
		"environment": {"country": "DE"},
	}
	data_residency_check with input as input_eu_valid
}

# Test helper rule: quota_check
test_helper_quota_check {
	# Read should pass
	input_read := {
		"action": "read",
		"environment": {"bandwidth_used": 150},
	}
	quota_check with input as input_read

	# Write under quota should pass
	input_write_ok := {
		"action": "write",
		"environment": {"bandwidth_used": 50},
	}
	quota_check with input as input_write_ok
}

# Test helper rule: time_check
test_helper_time_check {
	# Business hours should pass
	input_business := {
		"environment": {"time": "2025-10-16T10:00:00Z"},
		"subject": {"roles": ["operator"]},
	}
	time_check with input as input_business

	# Admin should pass
	input_admin := {
		"environment": {"time": "2025-10-16T20:00:00Z"},
		"subject": {"roles": ["admin"]},
	}
	time_check with input as input_admin
}
