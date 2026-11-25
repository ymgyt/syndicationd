use std::sync::Arc;

use rustls::crypto::{CryptoProvider, ring};

/// Installs ring as the process-wide rustls crypto provider.
///
/// Returns the provider that could not be installed if a default already exists.
pub fn install_tls_crypto_provider() -> Result<(), Arc<CryptoProvider>> {
    ring::default_provider().install_default()
}

#[cfg(test)]
mod tests {
    use super::install_tls_crypto_provider;

    #[test]
    fn rejects_a_second_installation() {
        install_tls_crypto_provider().unwrap();
        assert!(install_tls_crypto_provider().is_err());
    }
}
