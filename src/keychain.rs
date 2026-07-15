use crate::store::{SecretStore, SERVICE};
use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};
use zeroize::Zeroizing;

/// `errSecItemNotFound`: absence of an item, as opposed to a real failure.
const ERR_SEC_ITEM_NOT_FOUND: i32 = -25300;

/// macOS Keychain backend.
pub struct KeychainStore;

impl SecretStore for KeychainStore {
    fn read(
        &self,
        identifier: &str,
    ) -> Result<Option<Zeroizing<String>>, Box<dyn std::error::Error>> {
        match get_generic_password(SERVICE, identifier) {
            Ok(bytes) => return Ok(Some(Zeroizing::new(String::from_utf8(bytes)?))),
            Err(e) if e.code() == ERR_SEC_ITEM_NOT_FOUND => {}
            Err(e) => return Err(e.into()),
        }

        // Legacy key migrations
        let legacy_candidates = [
            format!("'{}'", identifier),
            format!("{} ", identifier),
            format!("'{}' ", identifier),
        ];

        for legacy_key in &legacy_candidates {
            if let Ok(bytes) = get_generic_password(SERVICE, legacy_key.as_str()) {
                if let Ok(s) = String::from_utf8(bytes) {
                    let password = Zeroizing::new(s);
                    // Migrate: write under correct key, delete legacy
                    let _ = set_generic_password(SERVICE, identifier, password.as_bytes());
                    let _ = delete_generic_password(SERVICE, legacy_key.as_str());
                    return Ok(Some(password));
                }
            }
        }

        Ok(None)
    }

    fn write(&self, identifier: &str, password: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        set_generic_password(SERVICE, identifier, password)?;
        Ok(())
    }
}
