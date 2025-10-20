use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Deserialize)]
pub struct Usage {
    pub bandwidth_bytes: u64,
}

pub struct QuotaClient {
    http_client: Client,
    base_url: String,
    token: String,
}

impl QuotaClient {
    pub fn new(base_url: String, token: String) -> Result<Self> {
        let http_client = Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .context("Failed to build quota tracker client")?;

        Ok(Self {
            http_client,
            base_url: base_url.trim_end_matches('/').to_string(),
            token,
        })
    }

    pub async fn get_usage(&self, tenant_id: &str) -> Result<Usage> {
        let url = format!("{}/tenants/{}/usage", self.base_url, tenant_id);
        let response = self
            .http_client
            .get(&url)
            .bearer_auth(&self.token)
            .send()
            .await
            .with_context(|| format!("Failed to fetch quota usage from {}", url))?;

        if response.status().is_success() {
            response
                .json::<Usage>()
                .await
                .context("Failed to parse quota usage response")
        } else {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "unable to read error body".to_string());
            anyhow::bail!(
                "Quota service responded with {} for GET {}: {}",
                status,
                url,
                body
            );
        }
    }

    pub async fn increment(&self, tenant_id: &str, bytes: u64, request_id: &str) -> Result<()> {
        let url = format!("{}/tenants/{}/usage", self.base_url, tenant_id);
        let payload = IncrementRequest { bytes };

        let mut request_builder = self
            .http_client
            .post(&url)
            .bearer_auth(&self.token)
            .json(&payload);

        if !request_id.is_empty() {
            request_builder = request_builder.header("X-Request-Id", request_id);
        }

        let response = request_builder
            .send()
            .await
            .with_context(|| format!("Failed to update quota usage at {}", url))?;

        if response.status().is_success() {
            Ok(())
        } else {
            let status = response.status();
            let body = response
                .text()
                .await
                .unwrap_or_else(|_| "unable to read error body".to_string());
            anyhow::bail!(
                "Quota service responded with {} for POST {}: {}",
                status,
                url,
                body
            );
        }
    }
}

#[derive(Debug, Serialize)]
struct IncrementRequest {
    pub bytes: u64,
}
