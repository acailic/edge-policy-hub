use serde_json::Value;
use tracing::debug;

use super::TenantValidationError;

pub fn validate_tenant_match(
    url_tenant_id: &str,
    input: &Value,
) -> Result<(), TenantValidationError> {
    validate_tenant_id_format(url_tenant_id)?;

    let input_tenant = input
        .get("subject")
        .and_then(|subject| subject.get("tenant_id"))
        .and_then(|tenant| tenant.as_str())
        .ok_or(TenantValidationError::MissingInputTenant)?;

    validate_tenant_id_format(input_tenant)?;

    debug!(
        url_tenant = url_tenant_id,
        input_tenant, "validating tenant match"
    );

    if input_tenant != url_tenant_id {
        return Err(TenantValidationError::Mismatch {
            url_tenant: url_tenant_id.to_string(),
            input_tenant: input_tenant.to_string(),
        });
    }

    Ok(())
}

pub fn validate_tenant_id_format(tenant_id: &str) -> Result<(), TenantValidationError> {
    if tenant_id.is_empty() || tenant_id.len() > 64 {
        return Err(TenantValidationError::InvalidTenantId(
            tenant_id.to_string(),
        ));
    }

    if !tenant_id
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(TenantValidationError::InvalidTenantId(
            tenant_id.to_string(),
        ));
    }

    Ok(())
}
