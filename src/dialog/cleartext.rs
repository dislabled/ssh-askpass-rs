use crate::dialog::{set_security_icon, DialogResult};
use crate::security::disable_core_dumps;
use objc2_app_kit::{
    NSAlert, NSAlertFirstButtonReturn, NSAlertStyle, NSApplication, NSApplicationActivationPolicy,
    NSControlStateValueOn, NSTextField,
};
use objc2_foundation::{MainThreadMarker, NSRect, NSString};
use zeroize::Zeroizing;

pub fn show(prompt: &str, identifier: Option<&str>) -> DialogResult {
    disable_core_dumps();

    let mtm = MainThreadMarker::new().unwrap();

    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    app.activate();

    let alert = NSAlert::new(mtm);
    set_security_icon(&alert);
    let title = NSString::from_str("SSH");
    alert.setMessageText(&title);

    let prompt_str = NSString::from_str(prompt);
    alert.setInformativeText(&prompt_str);

    let frame = NSRect {
        origin: objc2_foundation::NSPoint { x: 0.0, y: 0.0 },
        size: objc2_foundation::NSSize {
            width: 300.0,
            height: 24.0,
        },
    };
    let field = NSTextField::initWithFrame(mtm.alloc(), frame);

    let ok_label = NSString::from_str("OK");
    alert.addButtonWithTitle(&ok_label);
    let cancel_label = NSString::from_str("Cancel");
    alert.addButtonWithTitle(&cancel_label);

    alert.setAccessoryView(Some(&field));
    alert.setAlertStyle(NSAlertStyle::Informational);

    let show_keychain_checkbox = identifier.is_some();
    if show_keychain_checkbox {
        alert.setShowsSuppressionButton(true);
        if let Some(checkbox) = alert.suppressionButton() {
            let label = NSString::from_str("Remember in Keychain");
            checkbox.setTitle(&label);
        }
    }

    let window = alert.window();
    window.makeFirstResponder(Some(&field));

    let response = alert.runModal();

    if response == NSAlertFirstButtonReturn {
        // value is an NSString in AppKit-managed memory which i
        // dont know if can be wiped.
        // s is moved into Zeroizing below and will be zeroed on drop.
        let value = field.stringValue();
        let s = value.to_string();
        drop(field);

        let save_to_keychain = show_keychain_checkbox
            && alert
                .suppressionButton()
                .map(|b| b.state() == NSControlStateValueOn)
                .unwrap_or(false);

        DialogResult::Accepted {
            secret: Zeroizing::new(s),
            save_to_keychain,
        }
    } else {
        drop(field);
        DialogResult::Cancelled
    }
}
