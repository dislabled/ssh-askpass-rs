use crate::dialog::{set_security_icon, DialogResult};
use crate::security::disable_core_dumps;
use objc2_app_kit::{
    NSAlert, NSAlertFirstButtonReturn, NSAlertStyle, NSApplication, NSApplicationActivationPolicy,
};
use objc2_foundation::{MainThreadMarker, NSString};
use zeroize::Zeroizing;

pub fn show(prompt: &str) -> DialogResult {
    disable_core_dumps();

    let mtm = MainThreadMarker::new().unwrap();

    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    app.activate();

    let alert = NSAlert::new(mtm);
    set_security_icon(&alert);
    let title = NSString::from_str("Unknown SSH Host Key");
    alert.setMessageText(&title);

    let cleaned = prompt
        .replace("(yes/no/[fingerprint])", "")
        .replace("(yes/no)", "")
        .replace("Are you sure", "\nAre you sure")
        .trim()
        .to_string();

    let prompt_str = NSString::from_str(&cleaned);
    alert.setInformativeText(&prompt_str);
    alert.setAlertStyle(NSAlertStyle::Warning);

    let yes_label = NSString::from_str("Yes");
    alert.addButtonWithTitle(&yes_label);
    let no_label = NSString::from_str("No");
    alert.addButtonWithTitle(&no_label);

    let response = alert.runModal();

    if response == NSAlertFirstButtonReturn {
        DialogResult::Accepted {
            secret: Zeroizing::new("yes\n".to_string()),
            save_to_keychain: false,
        }
    } else {
        DialogResult::Cancelled
    }
}
