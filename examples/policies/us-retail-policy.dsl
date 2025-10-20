# US Retail PCI-DSS Compliance Policy
# Restricts payment data access to business hours and authorized personnel only

allow read payment_data if
  # All access must remain within tenant boundary
  subject.tenant_id == "tenant-us-retail" and
  resource.owner_tenant == "tenant-us-retail" and
  # Cashiers, managers, and admins can read during store hours
  subject.roles in ["cashier", "manager", "admin"] and
  environment.time >= "09:00" and
  environment.time <= "17:00" and
  # PCI-DSS requires elevated clearance for payment data
  subject.clearance_level >= 3

allow write payment_data if
  subject.tenant_id == "tenant-us-retail" and
  resource.owner_tenant == "tenant-us-retail" and
  # Only admins can perform write operations
  subject.roles in ["admin"] and
  # Block writes when fraud detection flags high risk
  environment.risk_score < 0.3
