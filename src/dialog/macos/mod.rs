mod autofill;
mod cleartext;
mod confirm;
mod host_key;
mod password;

use crate::prompt::{DialogResult, DisplayType};
use objc2_app_kit::{NSAlert, NSImage};
use objc2_foundation::{MainThreadMarker, NSString};

pub(super) fn set_security_icon(alert: &NSAlert) {
    let name = NSString::from_str("NSSecurity");
    if let Some(icon) = NSImage::imageNamed(&name) {
        unsafe { alert.setIcon(Some(&icon)) };
    }
}

/// Verifies we're on the main thread; AppKit calls below require it.
fn main_thread() -> MainThreadMarker {
    match MainThreadMarker::new() {
        Some(mtm) => mtm,
        None => {
            eprintln!("ssh-askpass-rs: cannot show a dialog off the main thread");
            crate::security::terminate_ssh();
            std::process::exit(1);
        }
    }
}

pub fn show(
    display_type: &DisplayType,
    prompt: &str,
    identifier: Option<&str>,
    store: &dyn crate::store::SecretStore,
) -> DialogResult {
    let mtm = main_thread();
    match display_type {
        DisplayType::Password | DisplayType::Pin => {
            password::show(mtm, prompt, display_type, identifier, store)
        }
        DisplayType::ClearText => cleartext::show(mtm, prompt, identifier, store),
        DisplayType::Confirm => confirm::show(mtm, prompt, false),
        DisplayType::ConfirmCancel => confirm::show(mtm, prompt, true),
        DisplayType::UnknownSshHost => host_key::show(mtm, prompt),
    }
}

pub fn confirm_autofill(prompt: &str, identifier: &str) -> bool {
    autofill::confirm_autofill(main_thread(), prompt, identifier)
}
