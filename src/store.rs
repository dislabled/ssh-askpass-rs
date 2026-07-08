//! Credential storage backend.

use zeroize::Zeroizing;

/// A backend that stores ssh credentials keyed by an identifier string.
pub trait SecretStore {
    /// Return the stored secret for `id`, if any.
    fn read(&self, id: &str) -> Option<Zeroizing<String>>;

    /// Store `secret` under `id`.
    fn write(&self, id: &str, secret: &[u8]) -> Result<(), Box<dyn std::error::Error>>;
}

/// The active store.
pub fn default_store() -> Box<dyn SecretStore> {
    Box::new(crate::keychain::KeychainStore)
}
