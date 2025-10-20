# Edge Policy HTTP Proxy - Deployment Guide

This service provides HTTP/HTTPS reverse proxy with policy enforcement using **Envoy or Nginx + OPA sidecar** architecture.

## Architecture

The proxy uses one of two configurations:

1. **Envoy + OPA (Recommended)**: Modern, cloud-native proxy with native gRPC support
2. **Nginx + OPA**: Traditional reverse proxy for simpler deployments

Both configurations integrate with OPA (Open Policy Agent) as a sidecar for policy evaluation.

## Components

### Envoy Configuration
- **envoy.yaml**: Main Envoy configuration with ext_authz filter
- **Port 8080**: HTTP listener with optional mTLS
- **Port 8443**: HTTPS listener with mandatory mTLS
- **Port 9901**: Admin interface

### Nginx Configuration
- **nginx.conf**: Nginx configuration with auth_request to OPA
- **Port 8080**: HTTPS listener with optional client certificate
- **Port 8443**: HTTPS listener with mandatory mTLS

### OPA Sidecar
- **Port 8181**: HTTP API for policy queries
- **Port 9191**: gRPC for Envoy ext_authz (Envoy only)
- **opa-config.yaml**: OPA configuration
- **policies/authz.rego**: Authorization policy

## Prerequisites

- Docker and Docker Compose
- TLS certificates (server, client, CA)
- OPA policy bundles
- Access to upstream backend services

## Quick Start

### Option 1: Envoy + OPA

```bash
# Generate certificates (if needed)
cd services/proxy-http
mkdir -p certs
# ... generate server-cert.pem, server-key.pem, client-cert.pem, client-key.pem, ca-cert.pem

# Configure environment
cp .env.example .env
# Edit .env with your settings

# Start services
docker-compose -f docker-compose.envoy.yml up -d

# Check status
curl http://localhost:9901/stats  # Envoy admin
curl http://localhost:8181/health  # OPA health
```

### Option 2: Nginx + OPA

```bash
# Generate certificates (if needed)
cd services/proxy-http
mkdir -p certs

# Configure environment
cp .env.example .env

# Start services
docker-compose -f docker-compose.nginx.yml up -d

# Check status
curl -k https://localhost:8080/health  # Nginx health
curl http://localhost:8181/health      # OPA health
```

## Configuration

### Environment Variables

Create `.env` file with:

```bash
# Upstream backend
UPSTREAM_HOST=backend.example.com
UPSTREAM_PORT=443

# JWT configuration (Envoy only)
JWT_ISSUER=https://auth.example.com
JWT_AUDIENCE=edge-policy-hub
JWKS_URI=https://auth.example.com/.well-known/jwks.json
JWKS_HOST=auth.example.com
JWKS_PORT=443

# OPA bundle service
BUNDLE_SERVICE_URL=http://audit-store:8080
BUNDLE_SERVICE_TOKEN=your-token-here
```

### TLS Certificates

Place certificates in `certs/` directory:

- `server-cert.pem`: Server certificate
- `server-key.pem`: Server private key
- `client-cert.pem`: Client certificate for upstream connections
- `client-key.pem`: Client private key
- `ca-cert.pem`: CA certificate for client verification

### OPA Policies

Place Rego policies in `policies/` directory. The main authorization policy is in `policies/authz.rego`.

Policy should evaluate `edge_policy.authz.allow` and return:

```json
{
  "allowed": true,
  "headers": {"x-tenant-id": "tenant1"},
  "http_status": 200
}
```

## Authentication Methods

The proxy supports multiple authentication methods:

1. **JWT Bearer Token**: `Authorization: Bearer <token>`
2. **mTLS**: Client certificate verification
3. **API Key**: `X-API-Key` header (configure in OPA policy)

## Policy Enforcement Flow

### Envoy Flow
1. Client request arrives at Envoy
2. JWT filter validates token (if present)
3. ext_authz filter calls OPA via gRPC
4. OPA evaluates policy with request context
5. If allowed, request forwards to upstream
6. Response returns to client

### Nginx Flow
1. Client request arrives at Nginx
2. auth_request subrequest to OPA
3. OPA evaluates policy
4. If allowed (200), request forwards to upstream
5. If denied (403), error returns to client

## Testing

### Test with JWT (Envoy)

```bash
# Get JWT token
TOKEN=$(curl -s -X POST https://auth.example.com/token \
  -d 'grant_type=client_credentials' \
  -d 'client_id=test' \
  -d 'client_secret=secret' | jq -r .access_token)

# Make request
curl -H "Authorization: Bearer $TOKEN" \
     -H "X-Tenant-ID: tenant1" \
     https://localhost:8080/api/resource
```

### Test with mTLS

```bash
curl --cert certs/client-cert.pem \
     --key certs/client-key.pem \
     --cacert certs/ca-cert.pem \
     -H "X-Tenant-ID: tenant1" \
     https://localhost:8443/api/resource
```

### Test OPA directly

```bash
# Query policy
curl -X POST http://localhost:8181/v1/data/edge_policy/authz/allow \
  -H "Content-Type: application/json" \
  -d '{
    "input": {
      "attributes": {
        "request": {
          "http": {
            "method": "GET",
            "path": "/api/resource",
            "headers": {"x-tenant-id": "tenant1"}
          }
        },
        "source": {
          "address": {"Address": {"SocketAddress": {"address": "10.0.0.1"}}}
        }
      }
    }
  }'
```

## Monitoring

### Envoy Metrics

```bash
# View all stats
curl http://localhost:9901/stats

# View ext_authz stats
curl http://localhost:9901/stats | grep ext_authz

# View cluster health
curl http://localhost:9901/clusters
```

### OPA Metrics

```bash
# Decision logs
docker logs edge-policy-opa

# API status
curl http://localhost:8181/health
curl http://localhost:8181/metrics
```

## Production Deployment

### Kubernetes Deployment

Example deployment manifest:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: edge-policy-proxy
spec:
  replicas: 3
  template:
    spec:
      containers:
      - name: envoy
        image: envoyproxy/envoy:v1.28-latest
        ports:
        - containerPort: 8080
        - containerPort: 8443
        volumeMounts:
        - name: config
          mountPath: /etc/envoy
        - name: certs
          mountPath: /etc/envoy/certs
      - name: opa
        image: openpolicyagent/opa:latest-envoy
        ports:
        - containerPort: 8181
        - containerPort: 9191
        volumeMounts:
        - name: opa-config
          mountPath: /config
        - name: policies
          mountPath: /policies
      volumes:
      - name: config
        configMap:
          name: envoy-config
      - name: opa-config
        configMap:
          name: opa-config
      - name: certs
        secret:
          secretName: proxy-certs
      - name: policies
        configMap:
          name: opa-policies
```

### High Availability

- Run multiple proxy replicas
- Use load balancer in front
- Configure OPA bundle replication
- Monitor health endpoints

### Security Hardening

1. Enable mTLS on all listeners
2. Restrict OPA admin API access
3. Use short-lived JWT tokens
4. Rotate certificates regularly
5. Enable audit logging
6. Implement rate limiting

## Troubleshooting

### Common Issues

**Envoy fails to start**
- Check certificate paths in envoy.yaml
- Verify OPA is running: `docker logs edge-policy-opa`
- Check Envoy logs: `docker logs edge-policy-envoy`

**OPA denies all requests**
- Verify policy is loaded: `curl http://localhost:8181/v1/data`
- Check decision logs: `docker logs edge-policy-opa`
- Test policy directly (see Testing section)

**Upstream connection fails**
- Verify UPSTREAM_HOST and UPSTREAM_PORT
- Check upstream TLS configuration
- Verify client certificates

**JWT validation fails (Envoy)**
- Verify JWKS endpoint is reachable
- Check JWT issuer and audience
- Ensure clock sync between services

## Migration from Rust Proxy

The previous Rust-based proxy implementation has been replaced with Envoy/Nginx + OPA sidecar for better maintainability and ecosystem integration. The Rust code remains in the repository for reference but is not actively used in deployments.

### Key Differences

- **Policy Engine**: Now OPA (Rego) instead of embedded Rust
- **Authentication**: Handled by Envoy/Nginx filters
- **Configuration**: YAML-based instead of Rust config structs
- **Deployment**: Container-based sidecar pattern

## Support

For issues or questions:
- Check Envoy documentation: https://www.envoyproxy.io/docs
- OPA documentation: https://www.openpolicyagent.org/docs
- Project issues: https://github.com/your-org/edge-policy-hub/issues
