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

#[cfg(target_os = "linux")]
pub fn default_store() -> Box<dyn SecretStore> {
    Box::new(crate::secret_service::SecretServiceStore)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub fn default_store() -> Box<dyn SecretStore> {
    Box::new(NullStore)
}

/// Placeholder for platforms without a native keystore.
/// Reads nothing; drops writes.
#[cfg(not(any(target_os = "macos", target_os = "linux")))]
pub struct NullStore;

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
impl SecretStore for NullStore {
    fn read(&self, _id: &str) -> Option<Zeroizing<String>> {
        None
    }

    fn write(&self, _id: &str, _secret: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        Ok(())
    }
}
