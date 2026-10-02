mod autofill;
mod confirm;
mod host_key;
mod text_input;

pub use autofill::confirm_autofill;
pub use confirm::ask_retry;

use crate::prompt::{DialogResult, DisplayType};

pub fn show(
    display_type: &DisplayType,
    prompt: &str,
    identifier: Option<&str>,
    store: &dyn crate::store::SecretStore,
) -> DialogResult {
    match display_type {
        DisplayType::Password | DisplayType::Pin => {
            text_input::show(prompt, true, identifier, store)
        }
        DisplayType::ClearText => text_input::show(prompt, false, identifier, store),
        DisplayType::Confirm => confirm::show(prompt, false),
        DisplayType::ConfirmCancel => confirm::show(prompt, true),
        DisplayType::UnknownSshHost => host_key::show(prompt),
    }
}
