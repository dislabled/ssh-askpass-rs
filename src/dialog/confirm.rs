use crate::dialog::{set_security_icon, DialogResult};
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
    // A background (accessory) app must force itself frontmost or the alert
    // opens without keyboard focus / a highlighted default button. Plain
    // activate() doesn't pull focus from another app; the ignoringOtherApps
    // variant does. runModal still centers the window, so don't reposition it.
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);

    let alert = NSAlert::new(mtm);
    set_security_icon(&alert);
    let title = NSString::from_str("SSH");
    alert.setMessageText(&title);

    let prompt_str = NSString::from_str(prompt);
    alert.setInformativeText(&prompt_str);
    alert.setAlertStyle(NSAlertStyle::Warning);

    if cancel_only {
        // Use "OK", not "Cancel": a lone button titled "Cancel" is special-cased
        // by AppKit (bound to Escape, never the highlighted Return default), so
        // it wouldn't respond to the keyboard. Dismissing still cancels (the
        // function returns Cancelled below regardless of the label).
        let ok_label = NSString::from_str("OK");
        alert.addButtonWithTitle(&ok_label);
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
