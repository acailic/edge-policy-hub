# Unit tests for lib/quota.rego
# Tests quota and cost guardrail helpers

package lib.quota

import rego.v1

# Test bandwidth_exceeded returns true at and over limit
test_bandwidth_exceeded_true {
	bandwidth_exceeded(100, 100) # at limit
	bandwidth_exceeded(101, 100) # over limit
	bandwidth_exceeded(150, 100) # well over limit
}

# Test bandwidth_exceeded returns false under limit
test_bandwidth_exceeded_false {
	not bandwidth_exceeded(50, 100) # under limit
	not bandwidth_exceeded(99, 100) # just under limit
	not bandwidth_exceeded(0, 100) # no usage
}

# Test bandwidth_within_limit
test_bandwidth_within_limit {
	bandwidth_within_limit(50, 100)
	bandwidth_within_limit(99, 100)
	not bandwidth_within_limit(100, 100) # at limit is not within
	not bandwidth_within_limit(101, 100) # over limit
}

# Test message_count_exceeded
test_message_count_exceeded {
	message_count_exceeded(50000, 50000) # at limit
	message_count_exceeded(50001, 50000) # over limit
	not message_count_exceeded(49999, 50000) # under limit
}

# Test quota_percentage calculation
test_quota_percentage {
	quota_percentage(50, 100) == 50
	quota_percentage(75, 100) == 75
	quota_percentage(100, 100) == 100
	quota_percentage(0, 100) == 0
}

# Test quota_remaining calculation
test_quota_remaining {
	quota_remaining(50, 100) == 50
	quota_remaining(99, 100) == 1
	quota_remaining(100, 100) == 0
}

# Test is_approaching_limit
test_is_approaching_limit {
	is_approaching_limit(85, 100, 80) # 85% > 80% threshold
	is_approaching_limit(90, 100, 80)
	not is_approaching_limit(75, 100, 80) # 75% < 80% threshold
	not is_approaching_limit(50, 100, 80)
}

# Test edge cases with zero and negative limits
test_edge_cases {
	not bandwidth_exceeded(0, 0) # zero limit edge case
}

# Test quota_percentage with zero limit (should return 0 to avoid division by zero)
test_quota_percentage_zero_limit {
	quota_percentage(50, 0) == 0
	quota_percentage(100, 0) == 0
}

# Test quota_percentage with negative limit (should return 0)
test_quota_percentage_negative_limit {
	quota_percentage(50, -10) == 0
}

# Test is_approaching_limit with zero limit (should be false)
test_is_approaching_limit_zero_limit {
	not is_approaching_limit(50, 0, 80)
	not is_approaching_limit(100, 0, 80)
}

# Test is_approaching_limit with negative limit (should be false)
test_is_approaching_limit_negative_limit {
	not is_approaching_limit(50, -10, 80)
}
