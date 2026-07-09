use crate::store::SecretStore;
use dbus_secret_service::{EncryptionType, SecretService};
use std::collections::HashMap;
use zeroize::Zeroizing;

const SERVICE: &str = "ssh-askpass-rs";

/// Freedesktop Secret Service backend (gnome-keyring, kwallet, ...).
pub struct SecretServiceStore;

impl SecretServiceStore {
    fn attributes(id: &str) -> HashMap<&str, &str> {
        HashMap::from([("service", SERVICE), ("account", id)])
    }
}

impl SecretStore for SecretServiceStore {
    fn read(&self, id: &str) -> Option<Zeroizing<String>> {
        // Dh keeps the secret encrypted in transit over the session bus.
        let ss = SecretService::connect(EncryptionType::Dh).ok()?;
        let found = ss.search_items(Self::attributes(id)).ok()?;

        // Prefer an already-unlocked match
        let item = found
            .unlocked
            .into_iter()
            .next()
            .or_else(|| found.locked.into_iter().next())?;
        item.ensure_unlocked().ok()?;

        let bytes = item.get_secret().ok()?;
        String::from_utf8(bytes).ok().map(Zeroizing::new)
    }

    fn write(&self, id: &str, secret: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let ss = SecretService::connect(EncryptionType::Dh)?;
        let collection = ss.get_default_collection()?;
        // Unlock first so the write can't silently fail.
        collection.ensure_unlocked()?;

        collection.create_item(
            &format!("{SERVICE}: {id}"),
            Self::attributes(id),
            secret,
            true,
            "text/plain",
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Live round-trip against the running Secret Service.
    #[test]
    #[ignore]
    fn roundtrip_live() {
        let store = SecretServiceStore;
        let id = "ssh-askpass-rs-selftest@test";
        store.write(id, b"test1234").expect("write");
        let got = store.read(id).expect("read back");
        assert_eq!(got.as_str(), "test1234");

        // Clean up
        let ss = SecretService::connect(EncryptionType::Dh).unwrap();
        for item in ss
            .search_items(SecretServiceStore::attributes(id))
            .unwrap()
            .unlocked
        {
            let _ = item.delete();
        }
    }
}
