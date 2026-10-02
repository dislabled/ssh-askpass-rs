use super::set_security_icon;
use crate::prompt::DialogResult;
use crate::security::disable_core_dumps;
use objc2::rc::Retained;
use objc2_app_kit::{
    NSAlert, NSAlertFirstButtonReturn, NSAlertStyle, NSApplication, NSApplicationActivationPolicy,
    NSControlStateValueOn, NSSecureTextField, NSTextField,
};
use objc2_foundation::{MainThreadMarker, NSPoint, NSRect, NSSize, NSString};
use zeroize::Zeroizing;

/// Prompt for a single line of text. `secure` picks a masked NSSecureTextField
/// (passwords, PINs) over a plain NSTextField (usernames, OTP codes), and with
/// it the window title and alert style.
pub fn show(
    mtm: MainThreadMarker,
    prompt: &str,
    secure: bool,
    identifier: Option<&str>,
    store: &dyn crate::store::SecretStore,
) -> DialogResult {
    disable_core_dumps();

    let app = NSApplication::sharedApplication(mtm);
    app.setActivationPolicy(NSApplicationActivationPolicy::Accessory);
    // A background (accessory) app must force itself frontmost or the alert
    // opens without keyboard focus. Plain activate() doesn't pull focus from
    // another app (notably under focus-follows-mouse window managers); the
    // ignoringOtherApps variant does.
    #[allow(deprecated)]
    app.activateIgnoringOtherApps(true);

    let alert = NSAlert::new(mtm);
    set_security_icon(&alert);

    let title = NSString::from_str(if secure {
        "Enter SSH Credentials"
    } else {
        "SSH"
    });
    alert.setMessageText(&title);

    let prompt_str = NSString::from_str(prompt);
    alert.setInformativeText(&prompt_str);

    let frame = NSRect {
        origin: NSPoint { x: 0.0, y: 0.0 },
        size: NSSize {
            width: 300.0,
            height: 24.0,
        },
    };
    // NSSecureTextField is an NSTextField subclass, so both share the rest of
    // the setup once upcast.
    let field: Retained<NSTextField> = if secure {
        Retained::into_super(NSSecureTextField::initWithFrame(mtm.alloc(), frame))
    } else {
        NSTextField::initWithFrame(mtm.alloc(), frame)
    };

    let ok_label = NSString::from_str("OK");
    alert.addButtonWithTitle(&ok_label);
    let cancel_label = NSString::from_str("Cancel");
    alert.addButtonWithTitle(&cancel_label);

    alert.setAccessoryView(Some(&field));
    alert.setAlertStyle(if secure {
        NSAlertStyle::Warning
    } else {
        NSAlertStyle::Informational
    });

    // Show the checkbox whenever we have an identifier — even on retry prompts
    // (skip_keychain=true) so the user can overwrite a stale keychain entry.
    let show_keychain_checkbox = identifier.is_some();
    if show_keychain_checkbox {
        // Word the checkbox like the terminal's save prompt: "Overwrite" when an
        // entry already exists, else "Remember".
        let overwrite = identifier.is_some_and(|id| store.exists(id).unwrap_or(false));
        alert.setShowsSuppressionButton(true);
        if let Some(checkbox) = alert.suppressionButton() {
            let label = NSString::from_str(if overwrite {
                "Overwrite Keychain entry"
            } else {
                "Remember in Keychain"
            });
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

        let save_secret = show_keychain_checkbox
            && alert
                .suppressionButton()
                .map(|b| b.state() == NSControlStateValueOn)
                .unwrap_or(false);

        DialogResult::Accepted {
            secret: Zeroizing::new(s),
            save_secret,
        }
    } else {
        drop(field);
        DialogResult::Cancelled
    }
}
