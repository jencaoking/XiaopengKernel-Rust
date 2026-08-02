//! TLS configuration shared by HTTP/1.1, HTTP/2, and HTTP/3 transports.

use rustls::{ClientConfig, RootCertStore};
use std::sync::Arc;

/// Build a `rustls::ClientConfig` that:
/// 1. Tries to load the system's native root certificates.
/// 2. Falls back to the Mozilla root CA bundle (`webpki-roots`) if none are found.
pub fn build_tls_config() -> Arc<ClientConfig> {
    let mut root_store = RootCertStore::empty();

    // Try system certificates first.
    let native_count = load_native_certs(&mut root_store);
    if native_count == 0 {
        // Fallback: embed Mozilla root CAs.
        root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    }

    let config = ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();

    Arc::new(config)
}

/// Loads native OS root certificates into `store`.
/// Returns the number of certs successfully loaded.
fn load_native_certs(store: &mut RootCertStore) -> usize {
    match rustls_native_certs::load_native_certs() {
        Ok(certs) => {
            let mut count = 0usize;
            for cert in certs {
                if store.add(cert).is_ok() {
                    count += 1;
                }
            }
            count
        }
        Err(_) => 0,
    }
}
