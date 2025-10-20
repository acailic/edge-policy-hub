# Edge Policy Hub - Authorization Policy for Envoy/Nginx + OPA
package edge_policy.authz

import future.keywords.if
import future.keywords.in

# Default deny
default allow := false

# Allow decision with optional redaction
allow := {
    "allowed": decision,
    "headers": response_headers,
    "body": response_body,
    "http_status": status_code
} if {
    decision := allow_request
    response_headers := build_response_headers
    response_body := build_response_body
    status_code := build_status_code
}

# Main authorization logic
allow_request if {
    # Extract tenant and context
    tenant_id := get_tenant_id
    tenant_id != ""

    # Verify tenant exists
    tenant := data.tenants[tenant_id]

    # Check basic tenant status
    tenant.enabled == true

    # Validate authentication
    auth_valid

    # Check ABAC rules
    abac_allow

    # Check quota limits
    quota_ok
}

# Tenant ID extraction
get_tenant_id := tenant_id if {
    tenant_id := input.attributes.request.http.headers["x-tenant-id"]
} else := tenant_id if {
    # Extract from JWT claims
    jwt := jwt_decode(input.attributes.request.http.headers.authorization)
    tenant_id := jwt.payload.tenant_id
} else := tenant_id if {
    # Extract from mTLS certificate
    cert_subject := input.attributes.source.certificate
    tenant_id := extract_tenant_from_cert(cert_subject)
} else := ""

# JWT validation
auth_valid if {
    # JWT bearer token
    bearer := input.attributes.request.http.headers.authorization
    startswith(bearer, "Bearer ")
    jwt := jwt_decode(bearer)
    jwt.valid
} else if {
    # mTLS authentication
    cert := input.attributes.source.certificate
    cert != ""
    cert_valid(cert)
}

# ABAC evaluation
abac_allow if {
    tenant_id := get_tenant_id
    tenant := data.tenants[tenant_id]

    # Build ABAC context
    user := get_user_attributes
    resource := get_resource_attributes
    action := get_action_attributes
    environment := get_environment_attributes

    # Evaluate tenant policy
    policy := data.policies[tenant.policy_id]
    evaluate_policy(policy, user, resource, action, environment)
}

# Quota validation
quota_ok if {
    tenant_id := get_tenant_id
    tenant := data.tenants[tenant_id]

    # Check bandwidth quota
    current_bandwidth := get_current_bandwidth(tenant_id)
    current_bandwidth < tenant.quota_limits.bandwidth_bytes

    # Check request quota
    current_requests := get_current_requests(tenant_id)
    current_requests < tenant.quota_limits.requests_per_hour
}

# Helper: Extract user attributes
get_user_attributes := user if {
    jwt := jwt_decode(input.attributes.request.http.headers.authorization)
    user := {
        "id": jwt.payload.sub,
        "roles": jwt.payload.roles,
        "groups": jwt.payload.groups,
        "attributes": jwt.payload
    }
} else := user if {
    cert := input.attributes.source.certificate
    user := {
        "id": extract_user_from_cert(cert),
        "roles": extract_roles_from_cert(cert),
        "attributes": {}
    }
}

# Helper: Extract resource attributes
get_resource_attributes := resource if {
    resource := {
        "type": determine_resource_type(input.attributes.request.http.path),
        "id": extract_resource_id(input.attributes.request.http.path),
        "path": input.attributes.request.http.path,
        "classification": input.attributes.request.http.headers["x-classification"]
    }
}

# Helper: Extract action attributes
get_action_attributes := action if {
    action := {
        "type": map_http_method_to_action(input.attributes.request.http.method),
        "method": input.attributes.request.http.method
    }
}

# Helper: Extract environment attributes
get_environment_attributes := environment if {
    environment := {
        "time": time.now_ns(),
        "client_ip": input.attributes.source.address.Address.SocketAddress.address,
        "region": input.attributes.request.http.headers["x-region"],
        "network": determine_network_zone(input.attributes.source.address.Address.SocketAddress.address)
    }
}

# Helper: Determine resource type from path
determine_resource_type(path) := resource_type if {
    parts := split(path, "/")
    count(parts) > 1
    resource_type := parts[1]
} else := "unknown"

# Helper: Extract resource ID from path
extract_resource_id(path) := resource_id if {
    parts := split(path, "/")
    count(parts) > 2
    resource_id := parts[2]
} else := ""

# Helper: Map HTTP method to action type
map_http_method_to_action(method) := action if {
    method_map := {
        "GET": "read",
        "POST": "create",
        "PUT": "update",
        "PATCH": "update",
        "DELETE": "delete"
    }
    action := method_map[method]
} else := "unknown"

# Helper: Determine network zone
determine_network_zone(ip) := zone if {
    # Check if private IP
    is_private_ip(ip)
    zone := "internal"
} else := "external"

is_private_ip(ip) if {
    startswith(ip, "10.")
} else if {
    startswith(ip, "192.168.")
} else if {
    startswith(ip, "172.")
}

# Helper: Extract tenant from certificate
extract_tenant_from_cert(cert) := tenant_id if {
    # Parse cert subject for O=TenantID
    regex.match(`O=([^,]+)`, cert)
    matches := regex.find_n(`O=([^,]+)`, cert, 1)
    count(matches) > 0
    tenant_id := trim_prefix(matches[0], "O=")
} else := ""

# Helper: Extract user from certificate
extract_user_from_cert(cert) := user_id if {
    regex.match(`CN=([^,]+)`, cert)
    matches := regex.find_n(`CN=([^,]+)`, cert, 1)
    count(matches) > 0
    user_id := trim_prefix(matches[0], "CN=")
} else := ""

# Helper: Extract roles from certificate
extract_roles_from_cert(cert) := roles if {
    regex.match(`OU=([^,]+)`, cert)
    matches := regex.find_all(`OU=([^,]+)`, cert)
    roles := [trim_prefix(match, "OU=") | match := matches[_]]
} else := []

# Helper: Certificate validation
cert_valid(cert) if {
    cert != ""
    # Add additional cert validation logic
}

# Helper: JWT decode (simplified, use real JWT validation)
jwt_decode(bearer) := jwt if {
    token := trim_prefix(bearer, "Bearer ")
    # In real implementation, validate signature and expiry
    jwt := {
        "valid": true,
        "payload": {}  # Parse actual JWT
    }
}

# Helper: Policy evaluation
evaluate_policy(policy, user, resource, action, environment) if {
    # Evaluate policy rules
    rule := policy.rules[_]
    rule.action == action.type
    rule.resource == resource.type
    check_conditions(rule.conditions, user, resource, action, environment)
}

check_conditions(conditions, user, resource, action, environment) if {
    # Check all conditions
    condition := conditions[_]
    evaluate_condition(condition, user, resource, action, environment)
}

evaluate_condition(condition, user, resource, action, environment) if {
    # Simple condition evaluation
    condition.type == "role"
    condition.value in user.roles
} else if {
    condition.type == "region"
    condition.value == environment.region
}

# Response headers
build_response_headers := headers if {
    tenant_id := get_tenant_id
    headers := {
        "x-tenant-id": tenant_id,
        "x-opa-decision": "allow"
    }
}

# Response body
build_response_body := "" if {
    allow_request
} else := body if {
    body := "Access denied by policy"
}

# HTTP status code
build_status_code := 200 if {
    allow_request
} else := 403

# Helper: Get current bandwidth (integrate with quota tracker)
get_current_bandwidth(tenant_id) := 0  # TODO: Call quota tracker API

# Helper: Get current requests (integrate with quota tracker)
get_current_requests(tenant_id) := 0  # TODO: Call quota tracker API
