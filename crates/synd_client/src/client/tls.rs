use std::sync::Arc;

use rustls::{
    ClientConfig, DigitallySignedStruct, RootCertStore, SignatureScheme,
    client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    pki_types::{CertificateDer, ServerName, UnixTime, pem::PemObject as _},
};
use tokio_tungstenite::Connector;

use crate::SyndApiError;

#[derive(Clone, Debug, Default)]
pub(super) struct ClientTls {
    root_certificates: Vec<CertificateDer<'static>>,
    accept_invalid_certificates: bool,
}

impl ClientTls {
    pub(super) fn add_root_certificate_pem(&mut self, pem: &[u8]) -> Result<(), SyndApiError> {
        let certificates = CertificateDer::pem_slice_iter(pem)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| SyndApiError::InvalidRootCertificate {
                message: error.to_string(),
            })?;
        if certificates.is_empty() {
            return Err(SyndApiError::InvalidRootCertificate {
                message: "PEM input does not contain a certificate".to_owned(),
            });
        }
        self.root_certificates.extend(certificates);
        Ok(())
    }

    pub(super) fn accept_invalid_certificates(&mut self, accept: bool) {
        self.accept_invalid_certificates = accept;
    }

    pub(super) fn configure_http(
        &self,
        mut builder: reqwest::ClientBuilder,
    ) -> Result<reqwest::ClientBuilder, SyndApiError> {
        for certificate in &self.root_certificates {
            let certificate = reqwest::Certificate::from_der(certificate.as_ref())
                .map_err(SyndApiError::BuildRequest)?;
            builder = builder.add_root_certificate(certificate);
        }
        Ok(builder.danger_accept_invalid_certs(self.accept_invalid_certificates))
    }

    pub(super) fn websocket_connector(&self) -> Result<Option<Connector>, SyndApiError> {
        if self.root_certificates.is_empty() && !self.accept_invalid_certificates {
            return Ok(None);
        }

        let provider = rustls::crypto::ring::default_provider();
        let supported_schemes = provider
            .signature_verification_algorithms
            .supported_schemes();
        let mut roots = RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        for certificate in &self.root_certificates {
            roots.add(certificate.clone()).map_err(|error| {
                SyndApiError::InvalidRootCertificate {
                    message: error.to_string(),
                }
            })?;
        }

        let mut config = ClientConfig::builder_with_provider(Arc::new(provider))
            .with_safe_default_protocol_versions()
            .map_err(|error| SyndApiError::TlsConfiguration {
                message: error.to_string(),
            })?
            .with_root_certificates(roots)
            .with_no_client_auth();
        if self.accept_invalid_certificates {
            config
                .dangerous()
                .set_certificate_verifier(Arc::new(NoCertificateVerification {
                    supported_schemes,
                }));
        }

        Ok(Some(Connector::Rustls(Arc::new(config))))
    }
}

#[derive(Debug)]
struct NoCertificateVerification {
    supported_schemes: Vec<SignatureScheme>,
}

impl ServerCertVerifier for NoCertificateVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _certificate: &CertificateDer<'_>,
        _signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _certificate: &CertificateDer<'_>,
        _signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        self.supported_schemes.clone()
    }
}
