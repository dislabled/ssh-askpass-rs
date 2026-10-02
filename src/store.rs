//! Credential storage backend.

use zeroize::Zeroizing;

/// Keying namespace every stored secret lives under, shared by all backends.
pub const SERVICE: &str = "ssh-askpass-rs";

/// A backend that stores ssh credentials keyed by an identifier string.
pub trait SecretStore {
    /// Return the stored secret for `id`. `Ok(None)` means nothing is
    /// stored; `Err` means the backend itself failed
    fn read(&self, id: &str) -> Result<Option<Zeroizing<String>>, Box<dyn std::error::Error>>;

    /// Store `secret` under `id`.
    fn write(&self, id: &str, secret: &[u8]) -> Result<(), Box<dyn std::error::Error>>;

    /// `Err` means the backend itself failed (e.g. the store is locked and
    /// couldn't be queried), as opposed to a confirmed absence.
    fn exists(&self, id: &str) -> Result<bool, Box<dyn std::error::Error>> {
        Ok(self.read(id)?.is_some())
    }
}

/// The active store.
#[cfg(target_os = "macos")]
pub fn default_store() -> Box<dyn SecretStore> {
    Box::new(crate::keychain::KeychainStore)
}

#[cfg(target_os = "linux")]
pub fn default_store() -> Box<dyn SecretStore> {
    Box::new(crate::secret_service::SecretServiceStore::default())
}
