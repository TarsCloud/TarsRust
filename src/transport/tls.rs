//! TLS utilities for secure connections
//!
//! This module provides helper functions for creating TLS configurations
//! and loading certificates for both client and server.

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

use rustls_pemfile::{certs, private_key};
use tokio_rustls::rustls::{
    self,
    pki_types::{CertificateDer, PrivateKeyDer, ServerName},
    ClientConfig, RootCertStore, ServerConfig,
};
use tokio_rustls::{TlsAcceptor, TlsConnector};

use crate::{Result, TarsError};

/// Load certificates from a PEM file
pub fn load_certs(path: impl AsRef<Path>) -> Result<Vec<CertificateDer<'static>>> {
    let file = File::open(path.as_ref()).map_err(|e| {
        TarsError::Config(format!("Failed to open cert file: {}", e))
    })?;
    let mut reader = BufReader::new(file);

    let certs_result: std::result::Result<Vec<_>, _> = certs(&mut reader).collect();
    certs_result.map_err(|e| {
        TarsError::Config(format!("Failed to parse certificates: {}", e))
    })
}

/// Load private key from a PEM file
pub fn load_private_key(path: impl AsRef<Path>) -> Result<PrivateKeyDer<'static>> {
    let file = File::open(path.as_ref()).map_err(|e| {
        TarsError::Config(format!("Failed to open key file: {}", e))
    })?;
    let mut reader = BufReader::new(file);

    private_key(&mut reader)
        .map_err(|e| TarsError::Config(format!("Failed to parse private key: {}", e)))?
        .ok_or_else(|| TarsError::Config("No private key found in file".into()))
}

/// Create a client TLS configuration with custom CA certificate
///
/// # Arguments
/// * `ca_cert_path` - Path to the CA certificate PEM file
///
/// # Example
/// ```ignore
/// let tls_config = create_client_config("ca.pem")?;
/// let client_config = TarsClientConfig::ssl(Arc::new(tls_config));
/// ```
pub fn create_client_config(ca_cert_path: impl AsRef<Path>) -> Result<ClientConfig> {
    let ca_certs = load_certs(ca_cert_path)?;

    let mut root_store = RootCertStore::empty();
    for cert in ca_certs {
        root_store.add(cert).map_err(|e| {
            TarsError::Config(format!("Failed to add CA cert to store: {}", e))
        })?;
    }

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    Ok(config)
}

/// Create a client TLS configuration with system root certificates
///
/// This uses the default system root certificate store.
pub fn create_client_config_with_native_roots() -> Result<ClientConfig> {
    let root_store = RootCertStore {
        roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
    };

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    Ok(config)
}

/// Create a client TLS configuration that skips certificate verification
///
/// **WARNING**: This is insecure and should only be used for testing!
pub fn create_insecure_client_config() -> Result<ClientConfig> {
    let config = ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoVerifier))
        .with_no_client_auth();

    Ok(config)
}

/// Create a client TLS configuration with client certificate (mTLS)
///
/// # Arguments
/// * `ca_cert_path` - Path to the CA certificate PEM file
/// * `client_cert_path` - Path to the client certificate PEM file
/// * `client_key_path` - Path to the client private key PEM file
pub fn create_mtls_client_config(
    ca_cert_path: impl AsRef<Path>,
    client_cert_path: impl AsRef<Path>,
    client_key_path: impl AsRef<Path>,
) -> Result<ClientConfig> {
    let ca_certs = load_certs(ca_cert_path)?;
    let client_certs = load_certs(client_cert_path)?;
    let client_key = load_private_key(client_key_path)?;

    let mut root_store = RootCertStore::empty();
    for cert in ca_certs {
        root_store.add(cert).map_err(|e| {
            TarsError::Config(format!("Failed to add CA cert to store: {}", e))
        })?;
    }

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_client_auth_cert(client_certs, client_key)
        .map_err(|e| TarsError::Config(format!("Failed to set client auth: {}", e)))?;

    Ok(config)
}

/// Create a server TLS configuration
///
/// # Arguments
/// * `cert_path` - Path to the server certificate PEM file
/// * `key_path` - Path to the server private key PEM file
///
/// # Example
/// ```ignore
/// let tls_config = create_server_config("server.pem", "server.key")?;
/// let server_config = TarsServerConfig::ssl("0.0.0.0:10000", Arc::new(tls_config));
/// ```
pub fn create_server_config(
    cert_path: impl AsRef<Path>,
    key_path: impl AsRef<Path>,
) -> Result<ServerConfig> {
    let certs = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| TarsError::Config(format!("Failed to create server config: {}", e)))?;

    Ok(config)
}

/// Create a server TLS configuration with client certificate verification (mTLS)
///
/// # Arguments
/// * `cert_path` - Path to the server certificate PEM file
/// * `key_path` - Path to the server private key PEM file
/// * `ca_cert_path` - Path to the CA certificate PEM file for client verification
pub fn create_mtls_server_config(
    cert_path: impl AsRef<Path>,
    key_path: impl AsRef<Path>,
    ca_cert_path: impl AsRef<Path>,
) -> Result<ServerConfig> {
    let certs = load_certs(cert_path)?;
    let key = load_private_key(key_path)?;
    let ca_certs = load_certs(ca_cert_path)?;

    let mut root_store = RootCertStore::empty();
    for cert in ca_certs {
        root_store.add(cert).map_err(|e| {
            TarsError::Config(format!("Failed to add CA cert to store: {}", e))
        })?;
    }

    let client_verifier = rustls::server::WebPkiClientVerifier::builder(Arc::new(root_store))
        .build()
        .map_err(|e| TarsError::Config(format!("Failed to create client verifier: {}", e)))?;

    let config = ServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(certs, key)
        .map_err(|e| TarsError::Config(format!("Failed to create server config: {}", e)))?;

    Ok(config)
}

/// Create a TLS connector from client config
pub fn create_tls_connector(config: Arc<ClientConfig>) -> TlsConnector {
    TlsConnector::from(config)
}

/// Create a TLS acceptor from server config
pub fn create_tls_acceptor(config: Arc<ServerConfig>) -> TlsAcceptor {
    TlsAcceptor::from(config)
}

/// Parse server name from address string
pub fn parse_server_name(addr: &str) -> Result<ServerName<'static>> {
    // Extract host from "host:port" format
    let host = addr.split(':').next().unwrap_or(addr);

    ServerName::try_from(host.to_string())
        .map_err(|e| TarsError::Config(format!("Invalid server name '{}': {}", host, e)))
}

/// Certificate verifier that accepts any certificate (for testing only)
#[derive(Debug)]
struct NoVerifier;

impl rustls::client::danger::ServerCertVerifier for NoVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> std::result::Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> std::result::Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ECDSA_NISTP521_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_server_name() {
        let name = parse_server_name("example.com:443").unwrap();
        assert!(matches!(name, ServerName::DnsName(_)));

        let name = parse_server_name("localhost").unwrap();
        assert!(matches!(name, ServerName::DnsName(_)));
    }

    #[test]
    fn test_create_insecure_client_config() {
        let config = create_insecure_client_config();
        assert!(config.is_ok());
    }
}
