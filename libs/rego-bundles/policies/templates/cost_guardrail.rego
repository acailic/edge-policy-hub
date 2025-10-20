# Cost Guardrail Template Policy
# Purpose: Prevent cost overruns by blocking uploads when monthly bandwidth exceeds 100 GB
#
# Input requirements:
#   - subject.tenant_id: Tenant identifier for isolation
#   - resource.owner_tenant: Tenant that owns the resource
#   - action: Operation being performed (e.g., "read", "write", "upload", "publish")
#   - environment.bandwidth_used: Current bandwidth usage in GB
#
# Expected behavior:
#   - Reads always allowed (don't count against quota)
#   - Writes blocked if quota exceeded (>= 100 GB)
#   - Quota data should be injected by enforcer from quota tracker
#
# Usage example:
#   To customize the bandwidth limit:
#   1. Change 100 to your desired limit in GB
#   2. Add additional quota checks (e.g., message count, storage)
#   3. Customize actions that count against quota

package templates.cost_guardrail

import rego.v1

# Import helper modules
import data.lib.quota
import data.lib.tenant

# Default deny for security
default allow := false

# Allow read operations (don't count against quota)
allow {
	# Enforce tenant isolation
	tenant.matches(input.subject.tenant_id, input.resource.owner_tenant)

	# Read operations are always allowed
	input.action == "read"
}

# Allow write operations if quota not exceeded
allow {
	# Enforce tenant isolation
	tenant.matches(input.subject.tenant_id, input.resource.owner_tenant)

	# Check action is a write-type operation
	input.action in ["write", "upload", "publish"]

	# Check bandwidth quota not exceeded (100 GB limit)
	quota.bandwidth_within_limit(input.environment.bandwidth_used, 100)
}

# Deny with reason when quota exceeded
deny {
	input.action in ["write", "upload", "publish"]
	quota.bandwidth_exceeded(input.environment.bandwidth_used, 100)
}

# Provide human-readable denial reason
deny_reason := reason {
	deny
	used := input.environment.bandwidth_used
	reason := sprintf("Bandwidth quota exceeded: %v GB / 100 GB", [used])
}
