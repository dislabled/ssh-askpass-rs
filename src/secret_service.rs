use crate::store::SecretStore;
use dbus_secret_service::{EncryptionType, SecretService};
use std::cell::OnceCell;
use std::collections::HashMap;
use zeroize::Zeroizing;

const SERVICE: &str = "ssh-askpass-rs";

/// Freedesktop Secret Service backend (gnome-keyring, kwallet, ...).
#[derive(Default)]
pub struct SecretServiceStore {
    conn: OnceCell<SecretService>,
}

impl SecretServiceStore {
    fn attributes(id: &str) -> HashMap<&str, &str> {
        HashMap::from([("service", SERVICE), ("account", id)])
    }

    /// Connect lazily and reuse the session for every operation.
    fn service(&self) -> Result<&SecretService, Box<dyn std::error::Error>> {
        if let Some(ss) = self.conn.get() {
            return Ok(ss);
        }
        let ss = SecretService::connect(EncryptionType::Dh)?;
        Ok(self.conn.get_or_init(|| ss))
    }
}

impl SecretStore for SecretServiceStore {
    fn read(&self, id: &str) -> Result<Option<Zeroizing<String>>, Box<dyn std::error::Error>> {
        let ss = self.service()?;
        let found = ss.search_items(Self::attributes(id))?;

        // Prefer an already-unlocked match
        let item = match found
            .unlocked
            .into_iter()
            .next()
            .or_else(|| found.locked.into_iter().next())
        {
            Some(item) => item,
            None => return Ok(None),
        };
        item.ensure_unlocked()?;

        let bytes = item.get_secret()?;
        Ok(Some(Zeroizing::new(String::from_utf8(bytes)?)))
    }

    fn write(&self, id: &str, secret: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let ss = self.service()?;
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

    // Search only
    fn exists(&self, id: &str) -> bool {
        match self.service() {
            Ok(ss) => ss
                .search_items(Self::attributes(id))
                .is_ok_and(|found| !found.unlocked.is_empty() || !found.locked.is_empty()),
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Live round-trip against the running Secret Service.
    #[test]
    #[ignore]
    fn roundtrip_live() {
        let store = SecretServiceStore::default();
        let id = "ssh-askpass-rs-selftest@test";
        assert!(!store.exists(id));
        store.write(id, b"test1234").expect("write");
        assert!(store.exists(id));
        let got = store.read(id).expect("read").expect("read back");
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
