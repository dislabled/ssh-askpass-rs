mod autofill;
mod cleartext;
mod confirm;
mod host_key;
mod password;

pub use autofill::confirm_autofill;

use crate::prompt::{DialogResult, DisplayType};

pub fn show(
    display_type: &DisplayType,
    prompt: &str,
    identifier: Option<&str>,
    store: &dyn crate::store::SecretStore,
) -> DialogResult {
    match display_type {
        DisplayType::Password | DisplayType::Pin => {
            password::show(prompt, display_type, identifier, store)
        }
        DisplayType::ClearText => cleartext::show(prompt, identifier, store),
        DisplayType::Confirm => confirm::show(prompt, false),
        DisplayType::ConfirmCancel => confirm::show(prompt, true),
        DisplayType::UnknownSshHost => host_key::show(prompt),
    }
}
