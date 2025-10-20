# EU Data Residency Policy
# Ensures that EU-classified resources can only be accessed from EU locations

allow read sensor_data if subject.tenant_id == "tenant-eu" and resource.region == "EU" and subject.device_location in ["DE", "FR", "NL", "BE", "IT", "ES"]
