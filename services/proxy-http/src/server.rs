use crate::config::ProxyConfig;
use crate::proxy::handler::ProxyHandler;
use anyhow::{Context, Result};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio_rustls::TlsAcceptor;
use tracing::{error, info};

pub struct ProxyServer {
    config: Arc<ProxyConfig>,
    handler: Arc<ProxyHandler>,
    tls_acceptor: Option<TlsAcceptor>,
}

impl ProxyServer {
    /// Create a new proxy server
    pub fn new(config: ProxyConfig) -> Result<Self> {
        let config = Arc::new(config);

        // Create TLS acceptor if mTLS is enabled
        let tls_acceptor = if config.enable_mtls {
            Some(Self::create_tls_acceptor(&config)?)
        } else {
            None
        };

        // Create proxy handler
        let handler = Arc::new(ProxyHandler::new(config.clone())?);

        Ok(Self {
            config,
            handler,
            tls_acceptor,
        })
    }

    /// Create TLS acceptor with optional client authentication
    fn create_tls_acceptor(config: &ProxyConfig) -> Result<TlsAcceptor> {
        use rustls::pki_types::CertificateDer;
        use rustls::server::WebPkiClientVerifier;
        use rustls::RootCertStore;
        use std::fs::File;
        use std::io::BufReader;

        // Load server certificate
        let cert_path = config
            .tls_cert_path
            .as_ref()
            .context("TLS_CERT_PATH not set")?;
        let cert_file = File::open(cert_path)
            .context(format!("Failed to open certificate file: {:?}", cert_path))?;
        let mut cert_reader = BufReader::new(cert_file);
        let certs: Vec<CertificateDer> = rustls_pemfile::certs(&mut cert_reader)
            .collect::<Result<Vec<_>, _>>()
            .context("Failed to parse certificate")?;

        // Load server private key
        let key_path = config
            .tls_key_path
            .as_ref()
            .context("TLS_KEY_PATH not set")?;
        let key_file =
            File::open(key_path).context(format!("Failed to open key file: {:?}", key_path))?;
        let mut key_reader = BufReader::new(key_file);

        let private_key = rustls_pemfile::private_key(&mut key_reader)
            .context("Failed to parse private key")?
            .context("No private key found in file")?;

        // Load client CA for client certificate verification
        let ca_path = config
            .tls_client_ca_path
            .as_ref()
            .context("TLS_CLIENT_CA_PATH not set")?;
        let ca_file =
            File::open(ca_path).context(format!("Failed to open CA file: {:?}", ca_path))?;
        let mut ca_reader = BufReader::new(ca_file);

        let mut root_store = RootCertStore::empty();
        for cert in rustls_pemfile::certs(&mut ca_reader) {
            let cert = cert.context("Failed to parse CA certificate")?;
            root_store
                .add(cert)
                .context("Failed to add CA certificate to root store")?;
        }

        // Build TLS config with client authentication
        let client_verifier = WebPkiClientVerifier::builder(Arc::new(root_store))
            .build()
            .context("Failed to build client verifier")?;

        let tls_config = rustls::ServerConfig::builder()
            .with_client_cert_verifier(client_verifier)
            .with_single_cert(certs, private_key)
            .context("Failed to build TLS config")?;

        Ok(TlsAcceptor::from(Arc::new(tls_config)))
    }

    /// Run the proxy server
    pub async fn run(self) -> Result<()> {
        let addr: SocketAddr = self
            .config
            .listen_addr()
            .parse()
            .context("Invalid listen address")?;

        let listener = TcpListener::bind(&addr)
            .await
            .context(format!("Failed to bind to {}", addr))?;

        info!(
            "Proxy server listening on {} (mTLS: {})",
            addr, self.config.enable_mtls
        );

        let server = Arc::new(self);

        loop {
            let (stream, peer_addr) = match listener.accept().await {
                Ok(conn) => conn,
                Err(e) => {
                    error!("Failed to accept connection: {}", e);
                    continue;
                }
            };

            let server = Arc::clone(&server);

            tokio::spawn(async move {
                if let Err(e) = server.handle_connection(stream, peer_addr).await {
                    error!("Connection error from {}: {}", peer_addr, e);
                }
            });
        }
    }

    /// Handle a single connection
    async fn handle_connection(
        &self,
        stream: tokio::net::TcpStream,
        peer_addr: SocketAddr,
    ) -> Result<()> {
        if let Some(ref tls_acceptor) = self.tls_acceptor {
            // mTLS connection
            let tls_stream = tls_acceptor
                .accept(stream)
                .await
                .context("TLS handshake failed")?;

            // Extract peer certificate information
            let (io, tls_conn) = tls_stream.into_inner();
            let peer_certs = tls_conn
                .peer_certificates()
                .map(|certs| certs.to_vec())
                .unwrap_or_default();

            info!(
                "mTLS connection from {} with {} client certificate(s)",
                peer_addr,
                peer_certs.len()
            );

            // Store peer info for handler
            let peer_info = Arc::new(PeerInfo {
                addr: peer_addr,
                certificates: peer_certs,
            });

            // Serve HTTP over TLS
            let io = TokioIo::new(io);
            let handler: Arc<ProxyHandler> = Arc::clone(&self.handler);
            let peer_info_clone = Arc::clone(&peer_info);

            let service = service_fn(move |req| {
                let handler = Arc::clone(&handler);
                let peer_info = Arc::clone(&peer_info_clone);
                async move { handler.handle_request(req, Some(peer_info)).await }
            });

            http1::Builder::new()
                .serve_connection(io, service)
                .await
                .context("Failed to serve connection")?;
        } else {
            // Plain HTTP connection
            let io = TokioIo::new(stream);
            let handler: Arc<ProxyHandler> = Arc::clone(&self.handler);

            let peer_info = Arc::new(PeerInfo {
                addr: peer_addr,
                certificates: vec![],
            });

            let service = service_fn(move |req| {
                let handler = Arc::clone(&handler);
                let peer_info = Arc::clone(&peer_info);
                async move { handler.handle_request(req, Some(peer_info)).await }
            });

            http1::Builder::new()
                .serve_connection(io, service)
                .await
                .context("Failed to serve connection")?;
        }

        Ok(())
    }

    /// Graceful shutdown
    pub async fn shutdown(self) {
        info!("Proxy server shutting down gracefully");
        // Additional cleanup if needed
    }
}

/// Peer connection information
#[derive(Debug, Clone)]
pub struct PeerInfo {
    pub addr: SocketAddr,
    pub certificates: Vec<rustls::pki_types::CertificateDer<'static>>,
}

impl PeerInfo {
    /// Get client IP address
    pub fn client_ip(&self) -> std::net::IpAddr {
        self.addr.ip()
    }

    /// Get the first client certificate if available
    pub fn client_cert(&self) -> Option<&rustls::pki_types::CertificateDer<'static>> {
        self.certificates.first()
    }

    /// Check if connection has client certificates
    pub fn has_client_cert(&self) -> bool {
        !self.certificates.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    #[test]
    fn test_peer_info() {
        let peer_info = PeerInfo {
            addr: "127.0.0.1:1234".parse().unwrap(),
            certificates: vec![],
        };

        assert_eq!(
            peer_info.client_ip(),
            IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))
        );
        assert!(!peer_info.has_client_cert());
        assert!(peer_info.client_cert().is_none());
    }
}
