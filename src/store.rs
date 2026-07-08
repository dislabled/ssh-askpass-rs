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
#[cfg(target_os = "macos")]
pub fn default_store() -> Box<dyn SecretStore> {
    Box::new(crate::keychain::KeychainStore)
}

#[cfg(not(target_os = "macos"))]
pub fn default_store() -> Box<dyn SecretStore> {
    Box::new(NullStore)
}

/// Placeholder for platforms without a native keystore.
/// Reads nothing; drops writes.
#[cfg(not(target_os = "macos"))]
pub struct NullStore;

#[cfg(not(target_os = "macos"))]
impl SecretStore for NullStore {
    fn read(&self, _id: &str) -> Option<Zeroizing<String>> {
        None
    }

    fn write(&self, _id: &str, _secret: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
