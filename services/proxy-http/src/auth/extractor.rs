use super::{AuthError, AuthMethod, TenantContext, AUTHORIZATION_HEADER, TENANT_ID_HEADER};
use crate::config::{JwtAlgorithm, ProxyConfig};
use anyhow::Context;
use http::HeaderMap;
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::fs;
use tracing::{debug, info, warn};
use x509_parser::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct JwtClaims {
    sub: Option<String>,
    tenant_id: Option<String>,
    tid: Option<String>,
    organization_id: Option<String>,
    roles: Option<Vec<String>>,
    scope: Option<String>,
    device_id: Option<String>,
    iss: Option<String>,
    aud: Option<String>,
    exp: Option<usize>,
}

pub struct TenantExtractor {
    enable_mtls: bool,
    enable_jwt: bool,
    jwt_decoding_key: Option<DecodingKey>,
    jwt_validation: Option<Validation>,
}

impl TenantExtractor {
    pub fn new(config: &ProxyConfig) -> anyhow::Result<Self> {
        let (jwt_decoding_key, jwt_validation) = if config.enable_jwt {
            let algorithm = match config.jwt_algorithm {
                JwtAlgorithm::HS256 => Algorithm::HS256,
                JwtAlgorithm::HS384 => Algorithm::HS384,
                JwtAlgorithm::HS512 => Algorithm::HS512,
                JwtAlgorithm::RS256 => Algorithm::RS256,
                JwtAlgorithm::RS384 => Algorithm::RS384,
                JwtAlgorithm::RS512 => Algorithm::RS512,
                JwtAlgorithm::ES256 => Algorithm::ES256,
                JwtAlgorithm::ES384 => Algorithm::ES384,
            };

            let decoding_key = match config.jwt_algorithm {
                JwtAlgorithm::HS256 | JwtAlgorithm::HS384 | JwtAlgorithm::HS512 => {
                    let secret = config
                        .jwt_secret
                        .as_ref()
                        .context("JWT secret missing for HMAC algorithm")?;
                    DecodingKey::from_secret(secret.as_bytes())
                }
                JwtAlgorithm::RS256 | JwtAlgorithm::RS384 | JwtAlgorithm::RS512 => {
                    let key_path = config
                        .jwt_public_key_path
                        .as_ref()
                        .context("JWT public key path missing for RSA algorithm")?;
                    let key_data = fs::read(key_path)?;
                    DecodingKey::from_rsa_pem(&key_data)?
                }
                JwtAlgorithm::ES256 | JwtAlgorithm::ES384 => {
                    let key_path = config
                        .jwt_public_key_path
                        .as_ref()
                        .context("JWT public key path missing for ECDSA algorithm")?;
                    let key_data = fs::read(key_path)?;
                    DecodingKey::from_ec_pem(&key_data)?
                }
            };

            let mut validation = Validation::new(algorithm);
            validation.algorithms = vec![algorithm];

            if let Some(issuer) = &config.jwt_issuer {
                validation.set_issuer(&[issuer]);
            }

            if let Some(audience) = &config.jwt_audience {
                validation.set_audience(&[audience]);
            }

            (Some(decoding_key), Some(validation))
        } else {
            (None, None)
        };

        Ok(Self {
            enable_mtls: config.enable_mtls,
            enable_jwt: config.enable_jwt,
            jwt_decoding_key,
            jwt_validation,
        })
    }

    pub fn extract_from_certificate(&self, cert_der: &[u8]) -> Result<TenantContext, AuthError> {
        let (_, cert) = parse_x509_certificate(cert_der)?;

        debug!("Parsing X.509 certificate for tenant ID");

        // Try extracting from SAN first
        if let Ok(Some(san_ext)) = cert.subject_alternative_name() {
            for name in &san_ext.value.general_names {
                if let GeneralName::URI(uri) = name {
                    if uri.starts_with("tenant:") {
                        let tenant_id = uri.trim_start_matches("tenant:").to_string();
                        info!(tenant_id = %tenant_id, "Extracted tenant ID from certificate SAN");
                        return Ok(TenantContext::new(tenant_id, AuthMethod::MTls));
                    }
                }
            }
        }

        // Fallback to CN in subject
        if let Some(cn) = cert
            .subject()
            .iter_common_name()
            .next()
            .and_then(|cn| cn.as_str().ok())
        {
            let tenant_id = cn.to_string();
            info!(tenant_id = %tenant_id, "Extracted tenant ID from certificate CN");
            return Ok(TenantContext::new(tenant_id, AuthMethod::MTls));
        }

        warn!("No tenant ID found in certificate");
        Err(AuthError::TenantIdNotFound)
    }

    pub fn extract_from_jwt(&self, token: &str) -> Result<TenantContext, AuthError> {
        let decoding_key = self
            .jwt_decoding_key
            .as_ref()
            .ok_or(AuthError::UnsupportedAuthMethod)?;

        let validation = self
            .jwt_validation
            .as_ref()
            .ok_or(AuthError::UnsupportedAuthMethod)?;

        debug!("Decoding JWT token");

        let token_data = decode::<JwtClaims>(token, decoding_key, validation)?;
        let claims = token_data.claims;

        // Extract tenant ID from claims (try multiple fields)
        let tenant_id = claims
            .tenant_id
            .or(claims.tid)
            .or(claims.organization_id)
            .ok_or(AuthError::TenantIdNotFound)?;

        info!(tenant_id = %tenant_id, "Extracted tenant ID from JWT");

        let mut context = TenantContext::new(tenant_id, AuthMethod::Jwt);

        if let Some(user_id) = claims.sub {
            context = context.with_user_id(user_id);
        }

        if let Some(device_id) = claims.device_id {
            context = context.with_device_id(device_id);
        }

        // Extract roles from either 'roles' array or 'scope' string
        let roles = if let Some(roles) = claims.roles {
            roles
        } else if let Some(scope) = claims.scope {
            scope.split_whitespace().map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        };

        if !roles.is_empty() {
            context = context.with_roles(roles);
        }

        Ok(context)
    }

    pub fn extract_from_request(
        &self,
        headers: &HeaderMap,
        peer_certs: Option<&[Vec<u8>]>,
    ) -> Result<TenantContext, AuthError> {
        let mut mtls_context: Option<TenantContext> = None;
        let mut jwt_context: Option<TenantContext> = None;

        // Try mTLS extraction
        if self.enable_mtls {
            if let Some(certs) = peer_certs {
                if let Some(cert_der) = certs.first() {
                    match self.extract_from_certificate(cert_der) {
                        Ok(ctx) => {
                            debug!("Successfully extracted context from mTLS certificate");
                            mtls_context = Some(ctx);
                        }
                        Err(e) => {
                            warn!(error = %e, "Failed to extract tenant from certificate");
                        }
                    }
                }
            }
        }

        // Try JWT extraction
        if self.enable_jwt {
            if let Some(auth_header) = headers.get(AUTHORIZATION_HEADER) {
                if let Ok(auth_str) = auth_header.to_str() {
                    if let Some(token) = auth_str.strip_prefix("Bearer ") {
                        match self.extract_from_jwt(token) {
                            Ok(ctx) => {
                                debug!("Successfully extracted context from JWT");
                                jwt_context = Some(ctx);
                            }
                            Err(e) => {
                                warn!(error = %e, "Failed to extract tenant from JWT");
                            }
                        }
                    }
                }
            }
        }

        // Merge contexts or validate consistency
        match (mtls_context, jwt_context) {
            (Some(mtls_ctx), Some(jwt_ctx)) => {
                // Both methods succeeded, verify tenant IDs match
                if mtls_ctx.tenant_id != jwt_ctx.tenant_id {
                    return Err(AuthError::TenantIdMismatch {
                        cert_tenant: mtls_ctx.tenant_id,
                        jwt_tenant: jwt_ctx.tenant_id,
                    });
                }
                // Merge contexts, preferring JWT for user info
                let mut merged = mtls_ctx;
                if jwt_ctx.user_id.is_some() {
                    merged.user_id = jwt_ctx.user_id;
                }
                if jwt_ctx.device_id.is_some() {
                    merged.device_id = jwt_ctx.device_id;
                }
                if !jwt_ctx.roles.is_empty() {
                    merged.roles = jwt_ctx.roles;
                }
                Ok(merged)
            }
            (Some(ctx), None) | (None, Some(ctx)) => {
                // One method succeeded
                Ok(ctx)
            }
            (None, None) => {
                // Neither method succeeded, try fallback header for testing
                if !self.enable_mtls && !self.enable_jwt {
                    if let Some(tenant_header) = headers.get(TENANT_ID_HEADER) {
                        if let Ok(tenant_id) = tenant_header.to_str() {
                            info!(tenant_id = %tenant_id, "Using X-Tenant-ID header (testing mode)");
                            return Ok(TenantContext::new(
                                tenant_id.to_string(),
                                AuthMethod::Header,
                            ));
                        }
                    }
                }
                Err(AuthError::TenantIdNotFound)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProxyConfig;
    use chrono::{Duration, Utc};
    use jsonwebtoken::{encode, EncodingKey, Header};
    use std::io::Write;
    use tempfile::NamedTempFile;

    const RSA_PRIVATE_KEY: &str = include_str!("fixtures/test-rsa-private.pem");
    const RSA_PUBLIC_KEY: &str = include_str!("fixtures/test-rsa-public.pem");

    fn base_config() -> ProxyConfig {
        ProxyConfig {
            host: "127.0.0.1".to_string(),
            port: 0,
            upstream_url: "http://localhost:9000".to_string(),
            request_timeout_secs: 5,
            max_body_size_bytes: 1024,
            enforcer_url: "http://localhost:8181".to_string(),
            enable_mtls: false,
            tls_cert_path: None,
            tls_key_path: None,
            tls_client_ca_path: None,
            enable_jwt: false,
            jwt_secret: None,
            jwt_public_key_path: None,
            jwt_issuer: None,
            jwt_audience: None,
            jwt_algorithm: JwtAlgorithm::RS256,
            forward_auth_header: false,
            log_level: "warn".to_string(),
            quota_tracker_url: None,
            quota_tracker_token: None,
            default_region: None,
        }
    }

    #[test]
    fn hs256_tokens_validate_with_matching_algorithm() {
        let mut config = base_config();
        config.enable_jwt = true;
        config.jwt_algorithm = JwtAlgorithm::HS256;
        config.jwt_secret = Some("super-secret".to_string());

        let extractor = TenantExtractor::new(&config).expect("extractor should initialize");

        let exp = (Utc::now() + Duration::hours(1)).timestamp() as usize;
        let claims = JwtClaims {
            sub: Some("user-1".to_string()),
            tenant_id: Some("tenant-hs".to_string()),
            tid: None,
            organization_id: None,
            roles: None,
            scope: None,
            device_id: None,
            iss: None,
            aud: None,
            exp: Some(exp),
        };

        let token = encode(
            &Header::new(Algorithm::HS256),
            &claims,
            &EncodingKey::from_secret(config.jwt_secret.as_ref().unwrap().as_bytes()),
        )
        .expect("token should encode");

        let context = extractor
            .extract_from_jwt(&token)
            .expect("token should validate");
        assert_eq!(context.tenant_id, "tenant-hs");

        let invalid_token = encode(
            &Header::new(Algorithm::HS384),
            &claims,
            &EncodingKey::from_secret(config.jwt_secret.unwrap().as_bytes()),
        )
        .expect("token should encode");

        let result = extractor.extract_from_jwt(&invalid_token);
        assert!(matches!(result, Err(AuthError::InvalidJwt(_))));
    }

    #[test]
    fn rs256_tokens_require_configured_algorithm() {
        let mut public_key_file = NamedTempFile::new().expect("public key file");
        public_key_file
            .write_all(RSA_PUBLIC_KEY.as_bytes())
            .expect("write public key");

        let mut config = base_config();
        config.enable_jwt = true;
        config.jwt_algorithm = JwtAlgorithm::RS256;
        config.jwt_public_key_path = Some(public_key_file.path().to_path_buf());

        let extractor = TenantExtractor::new(&config).expect("extractor should initialize");
        let exp = (Utc::now() + Duration::hours(1)).timestamp() as usize;
        let claims = JwtClaims {
            sub: Some("user-1".to_string()),
            tenant_id: Some("tenant-rs".to_string()),
            tid: None,
            organization_id: None,
            roles: None,
            scope: None,
            device_id: None,
            iss: None,
            aud: None,
            exp: Some(exp),
        };

        let token = encode(
            &Header::new(Algorithm::RS256),
            &claims,
            &EncodingKey::from_rsa_pem(RSA_PRIVATE_KEY.as_bytes()).expect("load private key"),
        )
        .expect("token should encode");

        let context = extractor
            .extract_from_jwt(&token)
            .expect("token should validate");
        assert_eq!(context.tenant_id, "tenant-rs");

        let mismatched = encode(
            &Header::new(Algorithm::RS512),
            &claims,
            &EncodingKey::from_rsa_pem(RSA_PRIVATE_KEY.as_bytes()).expect("load private key"),
        )
        .expect("token should encode");

        let result = extractor.extract_from_jwt(&mismatched);
        assert!(matches!(result, Err(AuthError::InvalidJwt(_))));
    }
}
