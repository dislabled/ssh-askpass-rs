use crate::dialog::set_security_icon;
use crate::security::disable_core_dumps;
use objc2_app_kit::{
    NSAlert, NSAlertSecondButtonReturn, NSAlertStyle, NSApplication, NSApplicationActivationPolicy,
};
use objc2_foundation::{MainThreadMarker, NSString};

/// Ask the user to approve sending a stored credential before it is written to
/// stdout. Shows the raw prompt ("what is asking") alongside the resolved
/// identifier ("what is about to be sent") so a mismatch is visible.
///
/// "Don't Send" is the first/default button (highlighted, Return), so the safe
/// choice is the default; releasing the secret requires explicitly choosing
/// "Send". Returns true only on "Send".
///
/// The default button is deliberately NOT titled "Cancel": AppKit special-cases
/// that title (binds it to Escape and never makes it the highlighted Return
/// default), which leaves the dialog without keyboard focus.
pub fn confirm_autofill(prompt: &str, identifier: &str) -> bool {
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
    alert.setMessageText(&NSString::from_str("Send stored credential?"));

    let body = format!(
        "A request is asking for:\n  {}\n\nStored credential to send:\n  {}",
        prompt.trim_end(),
        identifier
    );
    alert.setInformativeText(&NSString::from_str(&body));
    alert.setAlertStyle(NSAlertStyle::Warning);

    // First button is the default. "Don't Send" (safe) is the default; "Send"
    // releases the secret. Avoid the title "Cancel" — AppKit would special-case
    // it and the dialog would lose its keyboard default.
    alert.addButtonWithTitle(&NSString::from_str("Don't Send"));
    alert.addButtonWithTitle(&NSString::from_str("Send"));

    alert.runModal() == NSAlertSecondButtonReturn
}
