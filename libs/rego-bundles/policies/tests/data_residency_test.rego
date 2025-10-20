# Unit tests for templates/data_residency.rego
# Tests data residency template policy

package templates.data_residency

import rego.v1

# Test allow for EU resource with EU location
test_allow_eu_resource_eu_location {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"device_location": "DE",
		},
		"resource": {
			"region": "EU",
			"owner_tenant": "tenant-a",
		},
		"environment": {"country": "DE"},
		"action": "read",
	}
	allow with input as input_data
}

# Test deny for EU resource with non-EU location
test_deny_eu_resource_non_eu_location {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"device_location": "US",
		},
		"resource": {
			"region": "EU",
			"owner_tenant": "tenant-a",
		},
		"environment": {"country": "US"},
		"action": "read",
	}
	not allow with input as input_data
}

# Test allow for non-EU resource from any location
test_allow_non_eu_resource_any_location {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"device_location": "US",
		},
		"resource": {
			"region": "US",
			"owner_tenant": "tenant-a",
		},
		"environment": {"country": "US"},
		"action": "read",
	}
	allow with input as input_data
}

# Test deny for cross-tenant access
test_deny_cross_tenant_access {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"device_location": "DE",
		},
		"resource": {
			"region": "EU",
			"owner_tenant": "tenant-b",
		},
		"environment": {"country": "DE"},
		"action": "read",
	}
	not allow with input as input_data
}

# Test allow with multiple EU countries
test_allow_multiple_eu_countries {
	input_fr := {
		"subject": {"tenant_id": "tenant-a", "device_location": "FR"},
		"resource": {"region": "EU", "owner_tenant": "tenant-a"},
		"environment": {"country": "FR"},
		"action": "read",
	}
	allow with input as input_fr

	input_nl := {
		"subject": {"tenant_id": "tenant-a", "device_location": "NL"},
		"resource": {"region": "EU", "owner_tenant": "tenant-a"},
		"environment": {"country": "NL"},
		"action": "read",
	}
	allow with input as input_nl

	input_it := {
		"subject": {"tenant_id": "tenant-a", "device_location": "IT"},
		"resource": {"region": "EU", "owner_tenant": "tenant-a"},
		"environment": {"country": "IT"},
		"action": "read",
	}
	allow with input as input_it
}

# Test allow for write action
test_allow_write_action {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"device_location": "DE",
		},
		"resource": {
			"region": "EU",
			"owner_tenant": "tenant-a",
		},
		"environment": {"country": "DE"},
		"action": "write",
	}
	allow with input as input_data
}

# Test deny for invalid action
test_deny_invalid_action {
	input_data := {
		"subject": {
			"tenant_id": "tenant-a",
			"device_location": "DE",
		},
		"resource": {
			"region": "EU",
			"owner_tenant": "tenant-a",
		},
		"environment": {"country": "DE"},
		"action": "delete",
	}
	not allow with input as input_data
}
