mod cleartext;
mod confirm;
mod host_key;
mod password;

use crate::prompt::DisplayType;
use objc2_app_kit::{NSAlert, NSImage};
use objc2_foundation::NSString;
use zeroize::Zeroizing;

pub(super) fn set_security_icon(alert: &NSAlert) {
    let name = NSString::from_str("NSSecurity");
    if let Some(icon) = NSImage::imageNamed(&name) {
        unsafe { alert.setIcon(Some(&icon)) };
    }
}

pub enum DialogResult {
    Accepted {
        secret: Zeroizing<String>,
        save_to_keychain: bool,
    },
    Cancelled,
}

pub fn show(display_type: &DisplayType, prompt: &str, identifier: Option<&str>) -> DialogResult {
    match display_type {
        DisplayType::Password | DisplayType::Pin => {
            password::show(prompt, display_type, identifier)
        }
        DisplayType::ClearText => cleartext::show(prompt, identifier),
        DisplayType::Confirm => confirm::show(prompt, false),
        DisplayType::ConfirmCancel => confirm::show(prompt, true),
        DisplayType::UnknownSshHost => host_key::show(prompt),
    }
}
