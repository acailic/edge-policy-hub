# Quota and cost guardrail checks
# Provides reusable functions for enforcing bandwidth limits and message rate limiting

package lib.quota

import rego.v1

# Check if bandwidth quota has been exceeded
# Parameters:
#   used_gb: Current bandwidth usage in GB
#   limit_gb: Maximum allowed bandwidth in GB
# Returns: true if used >= limit
# Usage: quota.bandwidth_exceeded(input.environment.bandwidth_used, 100)
bandwidth_exceeded(used_gb, limit_gb) {
	used_gb >= limit_gb
}

# Check if bandwidth is within quota limit
# Parameters:
#   used_gb: Current bandwidth usage in GB
#   limit_gb: Maximum allowed bandwidth in GB
# Returns: true if used < limit
# Usage: quota.bandwidth_within_limit(input.environment.bandwidth_used, 100)
bandwidth_within_limit(used_gb, limit_gb) {
	used_gb < limit_gb
}

# Check if message count has exceeded the limit
# Parameters:
#   count: Current message count
#   limit: Maximum allowed message count
# Returns: true if count >= limit
# Usage: quota.message_count_exceeded(input.environment.messages_sent, 50000)
message_count_exceeded(count, limit) {
	count >= limit
}

# Calculate quota usage percentage
# Parameters:
#   used: Current usage amount
#   limit: Maximum allowed amount
# Returns: Percentage value (0-100+), or 0 if limit is 0
# Note: If limit is 0 or negative, returns 0 to avoid division by zero
# Usage: percentage := quota.quota_percentage(input.environment.bandwidth_used, 100)
quota_percentage(used, limit) := 0 {
	limit <= 0
}

quota_percentage(used, limit) := result {
	limit > 0
	result := (used / limit) * 100
}

# Calculate remaining quota
# Parameters:
#   used: Current usage amount
#   limit: Maximum allowed amount
# Returns: Remaining quota amount
# Usage: remaining := quota.quota_remaining(input.environment.bandwidth_used, 100)
quota_remaining(used, limit) := result {
	result := limit - used
}

# Check if quota usage is approaching the limit
# Parameters:
#   used: Current usage amount
#   limit: Maximum allowed amount
#   threshold_percent: Threshold percentage (e.g., 80 for 80%)
# Returns: true if usage percentage exceeds threshold, false if limit is 0 or negative
# Usage: quota.is_approaching_limit(input.environment.bandwidth_used, 100, 80)
# Note: Quota data should be injected via data.tenants.{tenant_id}.quotas or passed as parameters
is_approaching_limit(used, limit, threshold_percent) {
	limit > 0
	percentage := quota_percentage(used, limit)
	percentage > threshold_percent
}
