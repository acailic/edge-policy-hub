#[allow(dead_code)]

use anyhow::{anyhow, Context, Result};
use rand::Rng;
use reqwest::Client;
use serde::Serialize;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use tokio::fs;
use tokio::process::{Child, Command};
use tokio::time::sleep;
use tracing::{debug, error, info};

// When used as a standalone test module
#[cfg(not(feature = "bench-include"))]
use {edge_policy_dsl, md5, serde_json, uuid};

// When used as a module included in bench_support
#[cfg(feature = "bench-include")]
use super::{edge_policy_dsl, md5, serde_json, uuid};

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub name: &'static str,
    pub package: &'static str,
    pub env: HashMap<String, String>,
    pub args: Vec<String>,
    pub health_url: String,
}

#[derive(Debug)]
pub struct ServiceProcess {
    pub config: ServiceConfig,
    pub child: Child,
}

#[derive(Debug, Clone)]
pub struct PortConfig {
    pub enforcer: u16,
    pub proxy_http: u16,
    pub proxy_http_upstream: u16,
    pub mqtt_bridge: u16,
    pub mqtt_bridge_ws: u16,
    pub audit_store: u16,
    pub quota_tracker: u16,
    pub ui_port: u16,
}

impl PortConfig {
    pub fn allocate() -> Result<Self> {
        Ok(Self {
            enforcer: find_free_port()?,
            proxy_http: find_free_port()?,
            proxy_http_upstream: find_free_port()?,
            mqtt_bridge: find_free_port()?,
            mqtt_bridge_ws: find_free_port()?,
            audit_store: find_free_port()?,
            quota_tracker: find_free_port()?,
            ui_port: find_free_port()?,
        })
    }
}

#[derive(Debug)]
pub struct HarnessTempDirs {
    pub root: TempDir,
    pub enforcer: PathBuf,
    pub proxy_http: PathBuf,
    pub mqtt_bridge: PathBuf,
    pub audit_store: PathBuf,
    pub quota_tracker: PathBuf,
}

impl HarnessTempDirs {
    pub fn new() -> Result<Self> {
        let root = TempDir::new().context("creating harness root tempdir")?;
        let base = root.path();
        let enforcer = base.join("enforcer");
        let proxy_http = base.join("proxy-http");
        let mqtt_bridge = base.join("bridge-mqtt");
        let audit_store = base.join("audit-store");
        let quota_tracker = base.join("quota-tracker");
        std::fs::create_dir_all(&enforcer)?;
        std::fs::create_dir_all(&proxy_http)?;
        std::fs::create_dir_all(&mqtt_bridge)?;
        std::fs::create_dir_all(&audit_store)?;
        std::fs::create_dir_all(&quota_tracker)?;
        Ok(Self {
            root,
            enforcer,
            proxy_http,
            mqtt_bridge,
            audit_store,
            quota_tracker,
        })
    }
}

pub struct TestHarness {
    workspace_dir: PathBuf,
    ports: PortConfig,
    pub temp_dirs: HarnessTempDirs,
    service_blueprints: Vec<ServiceConfig>,
    pub services: HashMap<&'static str, ServiceProcess>,
    http_client: Client,
}

impl TestHarness {
    pub async fn new() -> Result<Self> {
        let workspace_dir =
            PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()));
        let temp_dirs = HarnessTempDirs::new()?;
        let ports = PortConfig::allocate()?;
        let http_client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .context("building reqwest client")?;
        Ok(Self {
            workspace_dir,
            ports,
            temp_dirs,
            service_blueprints: Vec::new(),
            services: HashMap::new(),
            http_client,
        })
    }

    pub fn ports(&self) -> &PortConfig {
        &self.ports
    }

    pub fn http_client(&self) -> &Client {
        &self.http_client
    }

    pub fn enforcer_bundle_dir(&self) -> &Path {
        &self.temp_dirs.enforcer
    }

    pub async fn start_all_services(&mut self) -> Result<()> {
        #[cfg(not(feature = "bench-include"))]
        tracing_subscriber::fmt::try_init().ok();
        #[cfg(feature = "bench-include")]
        super::tracing_subscriber::fmt::try_init().ok();
        if self.service_blueprints.is_empty() {
            self.service_blueprints = self.service_configs()?;
        }
        for config in self.service_blueprints.clone() {
            let child = self.spawn_service(&config).await?;
            self.wait_for_service_health(&config.health_url, Duration::from_secs(30))
                .await
                .with_context(|| format!("waiting for {} health", config.name))?;
            self.services
                .insert(config.name, ServiceProcess { config, child });
        }
        Ok(())
    }

    fn service_configs(&self) -> Result<Vec<ServiceConfig>> {
        let mut configs = Vec::new();
        let mut common_env = HashMap::new();
        common_env.insert(
            "RUST_LOG".into(),
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        );
        common_env.insert(
            "EDGE_POLICY_TEST_DATA_DIR".into(),
            self.temp_dirs.root.path().display().to_string(),
        );

        configs.push(ServiceConfig {
            name: "enforcer",
            package: "edge-policy-enforcer",
            env: {
                let mut env = common_env.clone();
                env.insert("ENFORCER_PORT".into(), self.ports.enforcer.to_string());
                env.insert(
                    "ENFORCER_BUNDLE_DIR".into(),
                    self.temp_dirs.enforcer.display().to_string(),
                );
                env
            },
            args: vec![
                "--port".into(),
                self.ports.enforcer.to_string(),
                "--bundle-dir".into(),
                self.temp_dirs.enforcer.display().to_string(),
                "--audit-store-url".into(),
                format!("http://127.0.0.1:{}", self.ports.audit_store),
                "--quota-tracker-url".into(),
                format!("http://127.0.0.1:{}", self.ports.quota_tracker),
            ],
            health_url: format!("http://127.0.0.1:{}/health", self.ports.enforcer),
        });

        configs.push(ServiceConfig {
            name: "audit-store",
            package: "edge-policy-audit-store",
            env: {
                let mut env = common_env.clone();
                env.insert(
                    "AUDIT_STORE_PORT".into(),
                    self.ports.audit_store.to_string(),
                );
                env.insert(
                    "AUDIT_STORE_DATA_DIR".into(),
                    self.temp_dirs.audit_store.display().to_string(),
                );
                env
            },
            args: vec![
                "--port".into(),
                self.ports.audit_store.to_string(),
                "--data-dir".into(),
                self.temp_dirs.audit_store.display().to_string(),
            ],
            health_url: format!("http://127.0.0.1:{}/health", self.ports.audit_store),
        });

        configs.push(ServiceConfig {
            name: "quota-tracker",
            package: "edge-policy-quota-tracker",
            env: {
                let mut env = common_env.clone();
                env.insert(
                    "QUOTA_TRACKER_PORT".into(),
                    self.ports.quota_tracker.to_string(),
                );
                env.insert(
                    "QUOTA_TRACKER_DATA_DIR".into(),
                    self.temp_dirs.quota_tracker.display().to_string(),
                );
                env
            },
            args: vec![
                "--port".into(),
                self.ports.quota_tracker.to_string(),
                "--data-dir".into(),
                self.temp_dirs.quota_tracker.display().to_string(),
            ],
            health_url: format!("http://127.0.0.1:{}/health", self.ports.quota_tracker),
        });

        configs.push(ServiceConfig {
            name: "proxy-http",
            package: "edge-policy-proxy-http",
            env: {
                let mut env = common_env.clone();
                env.insert("PROXY_HTTP_PORT".into(), self.ports.proxy_http.to_string());
                env.insert(
                    "PROXY_HTTP_UPSTREAM_PORT".into(),
                    self.ports.proxy_http_upstream.to_string(),
                );
                env.insert(
                    "PROXY_HTTP_DATA_DIR".into(),
                    self.temp_dirs.proxy_http.display().to_string(),
                );
                env
            },
            args: vec![
                "--port".into(),
                self.ports.proxy_http.to_string(),
                "--enforcer-url".into(),
                format!("http://127.0.0.1:{}", self.ports.enforcer),
                "--audit-store-url".into(),
                format!("http://127.0.0.1:{}", self.ports.audit_store),
                "--quota-tracker-url".into(),
                format!("http://127.0.0.1:{}", self.ports.quota_tracker),
            ],
            health_url: format!("http://127.0.0.1:{}/health", self.ports.proxy_http),
        });

        configs.push(ServiceConfig {
            name: "bridge-mqtt",
            package: "edge-policy-bridge-mqtt",
            env: {
                let mut env = common_env.clone();
                env.insert("MQTT_BRIDGE_PORT".into(), self.ports.mqtt_bridge.to_string());
                env.insert(
                    "MQTT_BRIDGE_WS_PORT".into(),
                    self.ports.mqtt_bridge_ws.to_string(),
                );
                env.insert(
                    "MQTT_BRIDGE_DATA_DIR".into(),
                    self.temp_dirs.mqtt_bridge.display().to_string(),
                );
                env
            },
            args: vec![
                "--port".into(),
                self.ports.mqtt_bridge.to_string(),
                "--ws-port".into(),
                self.ports.mqtt_bridge_ws.to_string(),
                "--enforcer-url".into(),
                format!("http://127.0.0.1:{}", self.ports.enforcer),
            ],
            health_url: format!("http://127.0.0.1:{}/health", self.ports.mqtt_bridge),
        });

        Ok(configs)
    }

    async fn spawn_service(&self, config: &ServiceConfig) -> Result<Child> {
        info!("Starting {} service", config.name);
        let mut command = Command::new("cargo");
        command
            .current_dir(&self.workspace_dir)
            .arg("run")
            .arg("--quiet")
            .arg("--package")
            .arg(config.package)
            .arg("--")
            .args(&config.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        for (key, value) in &config.env {
            command.env(key, value);
        }
        let child = command
            .spawn()
            .with_context(|| format!("spawning {} service", config.name))?;
        Ok(child)
    }

    pub async fn stop_all_services(&mut self) -> Result<()> {
        info!("Stopping services");
        for service in self.services.values_mut() {
            if let Err(err) = service.child.start_kill() {
                error!("failed to send kill to {}: {err:#}", service.config.name);
            }
            if let Err(err) = service.child.wait().await {
                error!("failed to await {} shutdown: {err:#}", service.config.name);
            }
        }
        self.services.clear();
        Ok(())
    }

    pub async fn cleanup(&mut self) -> Result<()> {
        self.stop_all_services().await?;
        fs::remove_dir_all(self.temp_dirs.root.path())
            .await
            .context("removing temp directories")
            .ok();
        Ok(())
    }

    pub async fn wait_for_service_health(
        &self,
        url: &str,
        timeout: Duration,
    ) -> Result<()> {
        let start = Instant::now();
        while start.elapsed() < timeout {
            match self.http_client.get(url).send().await {
                Ok(response) if response.status().is_success() => return Ok(()),
                Ok(response) => {
                    debug!("health check for {url} returned {}", response.status());
                }
                Err(err) => {
                    debug!("health check for {url} failed: {err}");
                }
            };
            sleep(Duration::from_millis(250)).await;
        }
        Err(anyhow!("timeout waiting for service health at {url}"))
    }

    pub async fn create_test_tenant<T>(&self, tenant_id: &str, config: &T) -> Result<()>
    where
        T: Serialize,
    {
        let payload = serde_json::json!({
            "tenant_id": tenant_id,
            "name": tenant_id,
            "config": config
        });
        let url = format!("http://127.0.0.1:{}/api/tenants", self.ports.audit_store);
        let response = self
            .http_client
            .post(url)
            .json(&payload)
            .send()
            .await
            .context("creating tenant via audit-store")?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "failed to create tenant {}: {} {body}",
                tenant_id,
                status
            ));
        }
        Ok(())
    }

    pub async fn deploy_test_policy(
        &self,
        tenant_id: &str,
        dsl_source: &str,
    ) -> Result<()> {
        let bundle_dir = self.temp_dirs.enforcer.join(tenant_id);
        tokio::fs::create_dir_all(&bundle_dir)
            .await
            .context("creating tenant bundle directory")?;

        // Write DSL source for traceability
        let dsl_path = bundle_dir.join("policy.dsl");
        tokio::fs::write(&dsl_path, dsl_source.as_bytes())
            .await
            .context("writing DSL policy")?;

        // Compile DSL to Rego
        let compiled = edge_policy_dsl::compile_policy(
            dsl_source,
            &tenant_id.replace('-', "_"),
            None,
        )
        .map_err(|e| anyhow!("Failed to compile DSL policy: {}", e))?;

        // Generate unique bundle_id and checksum before moving compiled.rego
        let bundle_id = format!("bundle-{}", uuid::Uuid::new_v4());
        let rego_checksum = format!("{:x}", md5::compute(compiled.rego.as_bytes()));

        // Write generated Rego to policy.rego
        let rego_path = bundle_dir.join("policy.rego");
        tokio::fs::write(&rego_path, compiled.rego)
            .await
            .context("writing compiled rego")?;

        // Write metadata.json
        let metadata_path = bundle_dir.join("metadata.json");
        let metadata = serde_json::json!({
            "version": compiled.metadata.version,
            "author": compiled.metadata.author,
            "description": compiled.metadata.description,
            "created_at": compiled.metadata.created_at,
        });
        tokio::fs::write(&metadata_path, serde_json::to_string_pretty(&metadata)?)
            .await
            .context("writing metadata.json")?;

        // Write data.json with minimal content
        let data_path = bundle_dir.join("data.json");
        let data = serde_json::json!({});
        tokio::fs::write(&data_path, serde_json::to_string_pretty(&data)?)
            .await
            .context("writing data.json")?;

        // Register bundle according to OpenAPI spec
        let bundle_payload = serde_json::json!({
            "bundle_id": bundle_id,
            "tenant_id": tenant_id,
            "version": "test",
            "metadata": {
                "dsl_source": dsl_source,
                "rego_path": rego_path.display().to_string(),
                "checksum": rego_checksum
            }
        });

        let url = format!("http://127.0.0.1:{}/api/bundles", self.ports.audit_store);
        let response = self
            .http_client
            .post(&url)
            .json(&bundle_payload)
            .send()
            .await
            .context("registering bundle in audit-store")?;
        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "failed to register bundle for {}: {} - {}",
                tenant_id,
                status,
                body
            ));
        }

        // Parse response to get the actual bundle_id (server may override)
        let response_body: serde_json::Value = response.json().await.unwrap_or_default();
        let final_bundle_id = response_body["bundle_id"]
            .as_str()
            .unwrap_or(&bundle_id)
            .to_string();

        // Activate bundle using the correct path format
        let activate_url = format!(
            "http://127.0.0.1:{}/api/bundles/{}/activate",
            self.ports.audit_store, final_bundle_id
        );
        let activate_response = self
            .http_client
            .post(&activate_url)
            .send()
            .await
            .context("activating bundle in audit-store")?;
        if !activate_response.status().is_success() {
            let status = activate_response.status();
            let body = activate_response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "failed to activate bundle {}: {} - {}",
                final_bundle_id,
                status,
                body
            ));
        }

        let reload_url = format!(
            "http://127.0.0.1:{}/v1/tenants/{tenant_id}/reload",
            self.ports.enforcer
        );
        self.http_client
            .post(reload_url)
            .send()
            .await
            .context("triggering enforcer reload")?
            .error_for_status()
            .context("enforcer reload failed")?;

        Ok(())
    }

    pub async fn stop_service(&mut self, name: &str) -> Result<()> {
        if let Some(mut service) = self.services.remove(name) {
            service
                .child
                .start_kill()
                .with_context(|| format!("stopping {name}"))?;
            let _ = service.child.wait().await;
            Ok(())
        } else {
            Err(anyhow!("service {name} not running"))
        }
    }

    pub async fn restart_service(&mut self, name: &str) -> Result<()> {
        let config = self
            .service_blueprints
            .iter()
            .find(|cfg| cfg.name == name)
            .cloned()
            .ok_or_else(|| anyhow!("unknown service {name}"))?;
        let child = self.spawn_service(&config).await?;
        self.wait_for_service_health(&config.health_url, Duration::from_secs(30))
            .await?;
        self.services
            .insert(config.name, ServiceProcess { config, child });
        Ok(())
    }
}

pub fn default_tenant_config() -> serde_json::Value {
    serde_json::json!({
        "quotas": {
            "message_limit": 10_000,
            "bandwidth_limit_gb": 250
        },
        "features": {
            "data_residency": ["EU"],
            "pii_redaction": true
        }
    })
}

pub fn random_tenant_id(prefix: &str) -> String {
    let mut rng = rand::thread_rng();
    format!("{}-{}", prefix, rng.gen::<u32>())
}

pub fn find_free_port() -> Result<u16> {
    let port = std::net::TcpListener::bind("127.0.0.1:0")
        .context("binding to ephemeral port")?
        .local_addr()
        .context("reading socket address")?
        .port();
    Ok(port)
}
