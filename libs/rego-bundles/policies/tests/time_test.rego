# Unit tests for lib/time.rego
# Tests time-based access control helpers

package lib.time

import rego.v1

# Test is_business_hours returns true within 09:00-17:00 UTC
test_is_business_hours_true {
	is_business_hours("2025-10-16T10:00:00Z") # 10 AM UTC
	is_business_hours("2025-10-16T16:59:00Z") # just before 5 PM
}

# Test is_business_hours returns false outside business hours
test_is_business_hours_false {
	not is_business_hours("2025-10-16T08:00:00Z") # 8 AM, before business hours
	not is_business_hours("2025-10-16T18:00:00Z") # 6 PM, after business hours
	not is_business_hours("2025-10-16T23:00:00Z") # 11 PM, night time
}

# Test is_weekday returns true for Monday-Friday
test_is_weekday_true {
	is_weekday("2025-10-13T12:00:00Z") # Monday
	is_weekday("2025-10-17T12:00:00Z") # Friday
}

# Test is_weekday returns false for Saturday-Sunday
test_is_weekday_false {
	not is_weekday("2025-10-18T12:00:00Z") # Saturday
	not is_weekday("2025-10-19T12:00:00Z") # Sunday
}

# Test is_within_window returns true when timestamp is in window
test_is_within_window_true {
	timestamp := "2025-10-16T12:00:00Z"
	start := "2025-10-16T10:00:00Z"
	end := "2025-10-16T14:00:00Z"
	is_within_window(timestamp, start, end)
}

# Test is_within_window returns false when timestamp is outside window
test_is_within_window_false {
	timestamp := "2025-10-16T15:00:00Z"
	start := "2025-10-16T10:00:00Z"
	end := "2025-10-16T14:00:00Z"
	not is_within_window(timestamp, start, end)
}

# Test is_expired returns true when timestamp is older than max_age_seconds
test_is_expired_true {
	old_timestamp := "2025-10-16T10:00:00Z"
	max_age := 3600 # 1 hour in seconds
	current_time := time.parse_rfc3339_ns("2025-10-16T12:00:00Z") # 2 hours later
	is_expired(old_timestamp, max_age) with time.now_ns as current_time
}

# Test is_expired returns false when timestamp is within max_age_seconds
test_is_expired_false {
	recent_timestamp := "2025-10-16T11:30:00Z"
	max_age := 3600 # 1 hour in seconds
	current_time := time.parse_rfc3339_ns("2025-10-16T12:00:00Z") # 30 minutes later
	not is_expired(recent_timestamp, max_age) with time.now_ns as current_time
}

# Test parse_iso8601 returns valid nanosecond value
test_parse_iso8601_valid {
	parsed := parse_iso8601("2025-10-16T14:30:00Z")
	parsed > 0 # Should return positive nanosecond value
}

# Test parse_iso8601 fails with invalid timestamp (missing Z)
test_parse_iso8601_invalid_missing_timezone {
	not parse_iso8601("2025-10-16T14:30:00")
}

# Test parse_iso8601 fails with malformed date
test_parse_iso8601_invalid_malformed_date {
	not parse_iso8601("2025-13-32T14:30:00Z") # invalid month and day
}

# Test parse_iso8601 fails with invalid format
test_parse_iso8601_invalid_format {
	not parse_iso8601("2025/10/16 14:30:00") # wrong format
	not parse_iso8601("invalid-timestamp")
}

# Test is_business_hours fails with invalid timestamp
test_is_business_hours_invalid_timestamp {
	not is_business_hours("invalid-timestamp")
	not is_business_hours("2025-10-16T14:30:00") # missing timezone
}

# Test is_weekday fails with invalid timestamp
test_is_weekday_invalid_timestamp {
	not is_weekday("invalid-timestamp")
	not is_weekday("2025-13-32T14:30:00Z") # invalid date
}

# Test is_within_window fails with invalid timestamps
test_is_within_window_invalid_timestamps {
	not is_within_window("invalid", "2025-10-16T10:00:00Z", "2025-10-16T14:00:00Z")
	not is_within_window("2025-10-16T12:00:00Z", "invalid-start", "2025-10-16T14:00:00Z")
	not is_within_window("2025-10-16T12:00:00Z", "2025-10-16T10:00:00Z", "invalid-end")
}
