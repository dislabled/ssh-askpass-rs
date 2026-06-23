use crate::dialog::DialogResult;
use crate::security::disable_core_dumps;
use objc2_app_kit::{
    NSAlert, NSAlertFirstButtonReturn, NSAlertStyle, NSApplication, NSApplicationActivationPolicy,
};
use objc2_foundation::{MainThreadMarker, NSString};
use zeroize::Zeroizing;

pub fn show(prompt: &str, cancel_only: bool) -> DialogResult {
    disable_core_dumps();

    let mtm = MainThreadMarker::new().unwrap();

    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    app.activate();

    let alert = NSAlert::new(mtm);
    let title = NSString::from_str("SSH");
    alert.setMessageText(&title);

    let prompt_str = NSString::from_str(prompt);
    alert.setInformativeText(&prompt_str);
    alert.setAlertStyle(NSAlertStyle::Warning);

    if cancel_only {
        let cancel_label = NSString::from_str("Cancel");
        alert.addButtonWithTitle(&cancel_label);
    } else {
        let accept_label = NSString::from_str("Accept");
        alert.addButtonWithTitle(&accept_label);
        let cancel_label = NSString::from_str("Cancel");
        alert.addButtonWithTitle(&cancel_label);
    }

    let response = alert.runModal();

    if !cancel_only && response == NSAlertFirstButtonReturn {
        DialogResult::Accepted {
            secret: Zeroizing::new("yes\n".to_string()),
            save_to_keychain: false,
        }
    } else {
        DialogResult::Cancelled
    }
}
