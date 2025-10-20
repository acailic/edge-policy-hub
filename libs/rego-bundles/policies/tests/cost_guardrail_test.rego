# Unit tests for templates/cost_guardrail.rego
# Tests cost guardrail template policy

package templates.cost_guardrail

import rego.v1

# Test allow for read action regardless of quota
test_allow_read_always {
	input_data := {
		"subject": {"tenant_id": "tenant-a"},
		"resource": {"owner_tenant": "tenant-a"},
		"action": "read",
		"environment": {"bandwidth_used": 150}, # over quota
	}
	allow with input as input_data
}

# Test allow for write action under quota
test_allow_write_under_quota {
	input_data := {
		"subject": {"tenant_id": "tenant-a"},
		"resource": {"owner_tenant": "tenant-a"},
		"action": "write",
		"environment": {"bandwidth_used": 50}, # under 100 GB limit
	}
	allow with input as input_data
}

# Test deny for write action at quota limit
test_deny_write_at_quota {
	input_data := {
		"subject": {"tenant_id": "tenant-a"},
		"resource": {"owner_tenant": "tenant-a"},
		"action": "write",
		"environment": {"bandwidth_used": 100}, # at limit
	}
	not allow with input as input_data
}

# Test deny for write action over quota
test_deny_write_over_quota {
	input_data := {
		"subject": {"tenant_id": "tenant-a"},
		"resource": {"owner_tenant": "tenant-a"},
		"action": "write",
		"environment": {"bandwidth_used": 150}, # over limit
	}
	not allow with input as input_data
}

# Test allow for upload action under quota
test_allow_upload_under_quota {
	input_data := {
		"subject": {"tenant_id": "tenant-a"},
		"resource": {"owner_tenant": "tenant-a"},
		"action": "upload",
		"environment": {"bandwidth_used": 75},
	}
	allow with input as input_data
}

# Test deny for upload action over quota
test_deny_upload_over_quota {
	input_data := {
		"subject": {"tenant_id": "tenant-a"},
		"resource": {"owner_tenant": "tenant-a"},
		"action": "upload",
		"environment": {"bandwidth_used": 101},
	}
	not allow with input as input_data
}

# Test allow for publish action under quota
test_allow_publish_under_quota {
	input_data := {
		"subject": {"tenant_id": "tenant-a"},
		"resource": {"owner_tenant": "tenant-a"},
		"action": "publish",
		"environment": {"bandwidth_used": 90},
	}
	allow with input as input_data
}

# Test deny for cross-tenant write even under quota
test_deny_cross_tenant_write {
	input_data := {
		"subject": {"tenant_id": "tenant-a"},
		"resource": {"owner_tenant": "tenant-b"},
		"action": "write",
		"environment": {"bandwidth_used": 50},
	}
	not allow with input as input_data
}

# Test deny reason for quota exceeded
test_deny_reason_quota_exceeded {
	input_data := {
		"subject": {"tenant_id": "tenant-a"},
		"resource": {"owner_tenant": "tenant-a"},
		"action": "write",
		"environment": {"bandwidth_used": 150},
	}
	deny with input as input_data
	contains(deny_reason, "Bandwidth quota exceeded") with input as input_data
}
