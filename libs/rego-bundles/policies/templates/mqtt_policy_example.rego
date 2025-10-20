# MQTT Policy Template
# Purpose: Demonstrate MQTT-specific policy enforcement with topic validation and quota checks
#
# Input requirements:
#   - subject.tenant_id: Tenant identifier
#   - action: "publish" or "subscribe"
#   - resource.topic: MQTT topic or topic filter
#   - resource.qos: MQTT QoS level (0, 1, or 2)
#   - environment.message_count: Current message count for quota
#
# Expected behavior:
#   - Enforce topic namespace isolation
#   - Allow publish if quota not exceeded
#   - Allow subscribe to tenant's own topics
#   - Deny cross-tenant access

package templates.mqtt_policy_example

import rego.v1

# Default deny
default allow := false

# Allow publish if all checks pass
allow if {
    input.action == "publish"

    # Validate topic starts with tenant ID
    topic_namespace_valid

    # Check message quota not exceeded (50,000 messages/day)
    message_count_ok

    # Check QoS is valid
    input.resource.qos in [0, 1, 2]
}

# Allow subscribe if topic filter is within tenant namespace
allow if {
    input.action == "subscribe"

    # Validate topic filter starts with tenant ID
    topic_namespace_valid

    # Check QoS is valid
    input.resource.qos in [0, 1, 2]
}

# Helper: Validate topic namespace
topic_namespace_valid if {
    # Extract tenant ID from topic (format: tenant_id/...)
    topic_parts := split(input.resource.topic, "/")
    count(topic_parts) > 0
    topic_tenant := topic_parts[0]

    # Verify it matches subject tenant
    topic_tenant == input.subject.tenant_id
}

# Helper: Check message quota
message_count_ok if {
    input.environment.message_count < 50000
}

# Deny with reason for quota exceeded
deny contains reason if {
    input.action == "publish"
    not message_count_ok
    count := input.environment.message_count
    reason := sprintf("Message quota exceeded: %v / 50000 messages", [count])
}

# Deny with reason for cross-tenant access
deny contains "Cross-tenant topic access denied" if {
    not topic_namespace_valid
}

# Optional: Redact sensitive fields for specific topics
redact contains "location.gps" if {
    input.action == "publish"
    startswith(input.resource.topic, input.subject.tenant_id)
    contains(input.resource.topic, "sensors/gps")
}

redact contains "device.serial_number" if {
    input.action == "publish"
    startswith(input.resource.topic, input.subject.tenant_id)
}
