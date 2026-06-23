use security_framework::passwords::{
    delete_generic_password, get_generic_password, set_generic_password,
};
use zeroize::Zeroizing;

const SERVICE: &str = "ssh-askpass-rs";

pub fn read(identifier: &str) -> Option<Zeroizing<String>> {
    if let Ok(bytes) = get_generic_password(SERVICE, identifier) {
        if let Ok(s) = String::from_utf8(bytes) {
            return Some(Zeroizing::new(s));
        }
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
                return Some(password);
            }
        }
    }

    None
}

pub fn write(identifier: &str, password: &[u8]) -> Result<(), security_framework::base::Error> {
    set_generic_password(SERVICE, identifier, password)
}
