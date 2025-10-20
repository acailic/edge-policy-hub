# Edge Policy HTTP Proxy Service

Reverse HTTP(S) proxy with OPA policy enforcement using **Envoy or Nginx + OPA sidecar** architecture for multi-tenant edge deployments.

## Features

- **Reverse Proxy**: Forward HTTP/HTTPS requests to upstream services via Envoy or Nginx
- **Policy Enforcement**: OPA sidecar evaluates policies for every request with ABAC attributes
- **Multi-Tenant Authentication**: Extract tenant ID from mTLS certificates or JWT tokens
- **Standards-Based**: Uses Envoy ext_authz or Nginx auth_request for policy integration
- **Cloud-Native**: Container-based deployment with sidecar pattern
- **Quota Integration**: Track bandwidth usage for cost guardrails (integrated with quota service)
- **Audit Logging**: Structured logging of all requests, policy decisions, and responses

## Architecture

This service uses one of two proxy configurations:

### Option 1: Envoy + OPA (Recommended)
Modern cloud-native proxy with native gRPC support for ext_authz filter.

Request flow:
1. Client connects to Envoy (HTTP:8080 or HTTPS:8443 with mTLS)
2. Envoy JWT filter validates token (if present)
3. Envoy ext_authz filter calls OPA sidecar via gRPC
4. OPA evaluates policy with request context (tenant, user, resource, action, environment)
5. If allowed, Envoy forwards request to upstream backend
6. Response returns to client
7. Decision logs sent to audit store

### Option 2: Nginx + OPA
Traditional reverse proxy for simpler deployments.

Request flow:
1. Client connects to Nginx (HTTPS:8080 or HTTPS:8443 with mTLS)
2. Nginx auth_request subrequest to OPA HTTP API
3. OPA evaluates policy and returns 200 (allow) or 403 (deny)
4. If allowed, Nginx forwards request to upstream
5. Response returns to client

## Quick Start

See [DEPLOYMENT.md](./DEPLOYMENT.md) for detailed deployment instructions.

```bash
# Start Envoy + OPA
docker-compose -f docker-compose.envoy.yml up -d

# OR start Nginx + OPA
docker-compose -f docker-compose.nginx.yml up -d
```

## Components

- **envoy.yaml**: Envoy proxy configuration with ext_authz filter
- **nginx.conf**: Nginx configuration with auth_request
- **opa-config.yaml**: OPA sidecar configuration
- **policies/authz.rego**: Authorization policy in Rego
- **docker-compose.envoy.yml**: Envoy deployment
- **docker-compose.nginx.yml**: Nginx deployment

## Legacy Rust Implementation

A previous Rust-based proxy implementation exists in `src/` but is **not actively used** in deployments. The Envoy/Nginx + OPA sidecar approach is the recommended architecture for production use, providing better ecosystem integration and maintainability.

## Configuration (Envoy/Nginx)

See [DEPLOYMENT.md](./DEPLOYMENT.md) for Envoy and Nginx configuration details.

For OPA policy configuration, see `policies/authz.rego`.

---

## Legacy Rust Implementation Configuration

**Note:** The following configuration applies to the legacy Rust-based proxy in `src/`. For production deployments, use Envoy or Nginx as described above.

Environment variables:

**Server Settings:**
- `PROXY_HOST` - Listen host (default: 0.0.0.0)
- `PROXY_PORT` - Listen port (default: 8080)
- `UPSTREAM_URL` - Backend service URL (default: http://localhost:8000)
- `REQUEST_TIMEOUT_SECS` - Request timeout (default: 30)
- `MAX_BODY_SIZE_BYTES` - Max body size for buffering (default: 10485760 = 10MB)

**Enforcer Integration:**
- `ENFORCER_URL` - OPA enforcer service URL (default: http://127.0.0.1:8181)

**Upstream Behavior:**
- `FORWARD_AUTH_HEADER` - Forward the inbound `Authorization` header to the upstream service (default: false)

**TLS Settings:**
- `ENABLE_MTLS` - Enable mTLS client authentication (default: false)
- `TLS_CERT_PATH` - Server certificate path (required if HTTPS)
- `TLS_KEY_PATH` - Server private key path (required if HTTPS)
- `TLS_CLIENT_CA_PATH` - Client CA certificate path (required if mTLS)

**JWT Settings:**
- `ENABLE_JWT` - Enable JWT authentication (default: false)
- `JWT_SECRET` - Shared secret for HS256 (optional)
- `JWT_PUBLIC_KEY_PATH` - Public key path for RS256 (optional)
- `JWT_ISSUER` - Expected issuer claim (optional)
- `JWT_AUDIENCE` - Expected audience claim (optional)

**Logging:**
- `LOG_LEVEL` - Logging level (default: info)

**Quota Tracker (optional):**
- `QUOTA_TRACKER_URL` - Base URL of the quota tracking service
- `QUOTA_TRACKER_TOKEN` - Bearer token used when calling the quota service

## Tenant ID Extraction

### mTLS Certificate

Tenant ID extracted from client certificate in order of preference:
1. Subject Alternative Name (SAN) with URI format: `tenant:{tenant_id}`
2. Common Name (CN) in subject

Example certificate generation:
```bash
# Create client certificate with tenant ID in SAN
openssl req -new -x509 -days 365 -key client.key -out client.crt \
  -subj "/CN=tenant-a" \
  -addext "subjectAltName=URI:tenant:tenant-a"
```

### JWT Token

Tenant ID extracted from JWT claims in order of preference:
1. `tenant_id` claim
2. `tid` claim (Azure AD format)
3. `organization_id` claim

Example JWT payload:
```json
{
  "sub": "user_123",
  "tenant_id": "tenant-a",
  "roles": ["operator"],
  "iss": "https://auth.example.com",
  "aud": "edge-policy-hub",
  "exp": 1234567890
}
```

### Fallback (Testing Only)

If neither mTLS nor JWT is enabled, tenant ID can be provided via `X-Tenant-ID` header (not recommended for production).

## ABAC Attribute Mapping

The proxy collects attributes from requests and maps them to the ABAC input structure:

**Subject Attributes:**
- `tenant_id`: From certificate or JWT
- `user_id`: From JWT `sub` claim or certificate CN
- `device_id`: From JWT custom claim or certificate field
- `roles`: From JWT `roles` or `scope` claim
- `clearance_level`: Default 1 (can be customized)

**Action:**
- `GET` → `read`
- `POST`, `PUT`, `PATCH` → `write`
- `DELETE` → `delete`

**Resource Attributes:**
- `type`: Extracted from request path (e.g., `/api/sensors` → `sensors`)
- `id`: Extracted from path parameters
- `classification`: From `X-Classification` header or `class` query parameter
- `region`: From `X-Region` header or `region` query parameter
- `owner_tenant`: Same as subject tenant_id

**Environment Attributes:**
- `time`: Current timestamp (ISO 8601)
- `country`: From GeoIP headers (e.g., `X-Geo-Country`) when present
- `network`: Client IP address
- `bandwidth_used`: Current bandwidth usage provided by quota tracker (bytes), when available

## Field-Level Redaction

If the enforcer policy returns a `redact` array, the proxy removes specified fields from JSON responses.

**Path Matching Behavior:**
- Paths can be fully qualified from root (e.g., `"user.pii.email"`) or relative (e.g., `"pii.email"`)
- The engine first tries exact match from root level
- If not found, searches for the path at any depth in the JSON structure
- For relative paths like `"pii.email"`, matches any occurrence in nested structures
- Array elements are searched recursively

**Examples:**
```json
// Input JSON
{
  "user": {
    "name": "Alice",
    "pii": {
      "email": "alice@example.com",
      "phone": "+1234567890"
    }
  }
}

// Redact paths: ["pii.email"]
// Result: email field removed from user.pii

// Redact paths: ["user.pii.email"]
// Result: same - email field removed from user.pii

// Redact paths: ["email"]
// Result: any "email" field at any depth is removed
```

## Development

```bash
# Build
cargo build --package edge-policy-proxy-http

# Run (requires enforcer service running)
cargo run --package edge-policy-proxy-http

# Run with custom config
UPSTREAM_URL=http://localhost:9000 \
ENFORCER_URL=http://localhost:8181 \
cargo run --package edge-policy-proxy-http

# Test
cargo test --package edge-policy-proxy-http
```

## Testing

### Test with curl (no auth)

```bash
# Start proxy
ENABLE_MTLS=false ENABLE_JWT=false cargo run

# Make request with tenant header
curl -H "X-Tenant-ID: tenant-a" http://localhost:8080/api/data
```

See the README in the plan for complete documentation including examples, testing instructions, and integration details.
