# Multi-Tenant Separation Policy
# Ensures tenants can only access their own resources

allow read sensor_data if subject.tenant_id == resource.owner_tenant and subject.clearance_level >= 2
