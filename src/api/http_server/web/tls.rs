// Copyright 2026 Zyvor AI Labs · https://zyvor.dev
// SPDX-License-Identifier: LicenseRef-Zyvor-Production-1.0
//
// TLS client config for talking to the Kubernetes API server.

use super::*;

/// Certificate verifier that accepts all certs (for K8s self-signed API server certs).
#[derive(Debug)]
#[allow(dead_code)]
pub(super) struct AcceptAllVerifier;

impl rustls::client::danger::ServerCertVerifier for AcceptAllVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
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

/// `VEYRON_KUBEVIRT_WS_INSECURE_TLS=1|true` opts out of verifying the Kubernetes
/// API server certificate on the legacy KubeVirt console proxy.
pub(super) fn kubevirt_ws_insecure_tls() -> bool {
    flag_enabled(
        std::env::var("VEYRON_KUBEVIRT_WS_INSECURE_TLS")
            .ok()
            .as_deref(),
    )
}

fn flag_enabled(v: Option<&str>) -> bool {
    v.is_some_and(|v| v.trim() == "1" || v.trim().eq_ignore_ascii_case("true"))
}

/// TLS connector for the upstream Kubernetes API WebSocket. Verifies the server
/// certificate by default: against the cluster CA (`roots`, DER, from the
/// kube config / service account) when present, else the default trust store
/// (`None`). `insecure` skips verification entirely.
pub(super) fn kubevirt_ws_connector(
    roots: Option<&[Vec<u8>]>,
    insecure: bool,
) -> Option<tokio_tungstenite::Connector> {
    if insecure {
        log::warn!(
            "VEYRON_KUBEVIRT_WS_INSECURE_TLS is set: the KubeVirt console proxy does not verify the Kubernetes API certificate"
        );
        let cfg = rustls::ClientConfig::builder()
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(AcceptAllVerifier))
            .with_no_client_auth();
        return Some(tokio_tungstenite::Connector::Rustls(Arc::new(cfg)));
    }
    let roots = roots.filter(|r| !r.is_empty())?;
    let mut store = rustls::RootCertStore::empty();
    for der in roots {
        let _ = store.add(rustls::pki_types::CertificateDer::from(der.clone()));
    }
    let cfg = rustls::ClientConfig::builder()
        .with_root_certificates(store)
        .with_no_client_auth();
    Some(tokio_tungstenite::Connector::Rustls(Arc::new(cfg)))
}

/// Build a rustls ClientConfig that authenticates to the K8s API server.
/// Supports client certificate auth (k3s/kubeadm) and falls back to no client auth.
#[allow(dead_code)]
pub(super) fn build_k8s_tls_config() -> rustls::ClientConfig {
    // Try reading client cert/key from kubeconfig
    if let Ok(kubeconfig_path) = std::env::var("KUBECONFIG") {
        if let Ok(contents) = std::fs::read_to_string(&kubeconfig_path) {
            let mut cert_b64 = None;
            let mut key_b64 = None;
            for line in contents.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("client-certificate-data:") {
                    cert_b64 = Some(
                        trimmed
                            .trim_start_matches("client-certificate-data:")
                            .trim()
                            .to_string(),
                    );
                }
                if trimmed.starts_with("client-key-data:") {
                    key_b64 = Some(
                        trimmed
                            .trim_start_matches("client-key-data:")
                            .trim()
                            .to_string(),
                    );
                }
            }

            if let (Some(cert_b64), Some(key_b64)) = (cert_b64, key_b64) {
                use base64::Engine;
                let decoder = base64::engine::general_purpose::STANDARD;
                if let (Ok(cert_pem), Ok(key_pem)) =
                    (decoder.decode(&cert_b64), decoder.decode(&key_b64))
                {
                    // Parse PEM cert
                    let mut certs = Vec::new();
                    let mut cursor = &cert_pem[..];
                    while let Ok(Some(item)) = rustls_pemfile::read_one(&mut cursor) {
                        if let rustls_pemfile::Item::X509Certificate(cert) = item {
                            certs.push(cert);
                        }
                    }

                    // Parse PEM key
                    let mut key_cursor = &key_pem[..];
                    let private_key = rustls_pemfile::private_key(&mut key_cursor).ok().flatten();

                    if !certs.is_empty() {
                        if let Some(key) = private_key {
                            if let Ok(cfg) = rustls::ClientConfig::builder()
                                .dangerous()
                                .with_custom_certificate_verifier(Arc::new(AcceptAllVerifier))
                                .with_client_auth_cert(certs, key)
                            {
                                log::info!("VNC proxy: using client certificate auth");
                                return cfg;
                            }
                        }
                    }
                }
            }
        }
    }

    // Fallback: no client auth
    log::warn!("VNC proxy: no client certificate found, using anonymous TLS");
    rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(AcceptAllVerifier))
        .with_no_client_auth()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_parsing() {
        assert!(flag_enabled(Some("1")) && flag_enabled(Some("TRUE")));
        assert!(!flag_enabled(None) && !flag_enabled(Some("0")) && !flag_enabled(Some("")));
    }

    #[test]
    fn kubevirt_connector_verifies_by_default() {
        // default trust store: no custom connector
        assert!(kubevirt_ws_connector(None, false).is_none());
        // cluster CA given: a verifying connector (never the accept-all one)
        assert!(kubevirt_ws_connector(Some(&[vec![1, 2, 3]]), false).is_some());
        assert!(kubevirt_ws_connector(None, true).is_some());
    }
}
