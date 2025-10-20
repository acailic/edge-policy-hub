# Time-based access control
# Provides reusable functions for enforcing time windows and session expiry

package lib.time

import rego.v1

# Check if timestamp falls within business hours (09:00-17:00 UTC)
# Parameters:
#   timestamp: ISO 8601 timestamp string (e.g., "2025-10-16T14:30:00Z")
# Returns: true if hour is between 9 and 16 (inclusive)
# Usage: time.is_business_hours(input.environment.time)
is_business_hours(timestamp) {
	parsed := parse_iso8601(timestamp)
	ns_per_second := 1000000000
	ns_per_hour := ns_per_second * 3600
	hours_since_epoch := parsed / ns_per_hour
	hour_of_day := hours_since_epoch % 24
	hour_of_day >= 9
	hour_of_day < 17
}

# Check if timestamp is a weekday (Monday-Friday)
# Parameters:
#   timestamp: ISO 8601 timestamp string
# Returns: true if day is Monday through Friday
# Usage: time.is_weekday(input.environment.time)
is_weekday(timestamp) {
	parsed := parse_iso8601(timestamp)
	ns_per_second := 1000000000
	ns_per_day := ns_per_second * 86400
	days_since_epoch := parsed / ns_per_day
	# Unix epoch (1970-01-01) was a Thursday (day 4)
	day_of_week := (days_since_epoch + 4) % 7
	# Monday = 1, Friday = 5
	day_of_week >= 1
	day_of_week <= 5
}

# Check if timestamp is within a custom time window
# Parameters:
#   timestamp: ISO 8601 timestamp to check
#   start_time: Window start time (ISO 8601)
#   end_time: Window end time (ISO 8601)
# Returns: true if timestamp is between start and end
# Usage: time.is_within_window(input.environment.time, "2025-10-16T10:00:00Z", "2025-10-16T14:00:00Z")
is_within_window(timestamp, start_time, end_time) {
	parsed := parse_iso8601(timestamp)
	start := parse_iso8601(start_time)
	end := parse_iso8601(end_time)
	parsed >= start
	parsed <= end
}

# Check if timestamp is older than max_age_seconds from current time
# Parameters:
#   timestamp: ISO 8601 timestamp to check
#   max_age_seconds: Maximum age in seconds
# Returns: true if timestamp is older than max_age_seconds
# Usage: time.is_expired(input.session.created_at, 3600)
is_expired(timestamp, max_age_seconds) {
	parsed := parse_iso8601(timestamp)
	now := time.now_ns()
	age_ns := now - parsed
	ns_per_second := 1000000000
	age_seconds := age_ns / ns_per_second
	age_seconds > max_age_seconds
}

# Parse ISO 8601 timestamp string to nanoseconds since epoch
# Parameters:
#   timestamp_string: ISO 8601 format timestamp (e.g., "2025-10-16T14:30:00Z")
# Returns: Nanoseconds since Unix epoch
# Usage: parsed_ns := time.parse_iso8601("2025-10-16T14:30:00Z")
# Note: Timestamps should be in ISO 8601 format with timezone (Z for UTC or +HH:MM offset)
parse_iso8601(timestamp_string) := result {
	result := time.parse_rfc3339_ns(timestamp_string)
}
