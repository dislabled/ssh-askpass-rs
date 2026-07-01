mod dialog;
mod keychain;
mod prompt;
mod security;

use dialog::DialogResult;
use prompt::{parse_prompt, prompt_type_from_env};
use std::io::Write;

/// Autofill confirmation is on by default; setting SSH_ASKPASS_NO_CONFIRM to a
/// non-empty value disables it system-wide (restoring silent autofill).
fn autofill_confirm_disabled() -> bool {
    std::env::var_os("SSH_ASKPASS_NO_CONFIRM").is_some_and(|v| !v.is_empty())
}

fn main() {
    security::disable_core_dumps();

    let prompt_str = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Please enter passphrase".to_string());

    let prompt_type = prompt_type_from_env();
    let parsed = parse_prompt(&prompt_str, &prompt_type);

    // Attempt keychain lookup
    if !parsed.skip_keychain {
        if let Some(id) = &parsed.identifier {
            if let Some(password) = keychain::read(id) {
                // For reusable remote passwords, confirm before silently
                // releasing the secret (unless disabled system-wide via
                // SSH_ASKPASS_NO_CONFIRM). On cancel, fall through to the
                // manual entry dialog below.
                let approved = !parsed.confirm_autofill
                    || autofill_confirm_disabled()
                    || dialog::confirm_autofill(&prompt_str, id);
                if approved {
                    let _ = std::io::stdout().write_all(password.as_bytes());
                    if !password.ends_with('\n') {
                        let _ = std::io::stdout().write_all(b"\n");
                    }
                    let _ = std::io::stdout().flush();
                    drop(password);
                    std::process::exit(0);
                }
                drop(password);
            }
        }
    }

    // Show dialog
    let result = dialog::show(&parsed.display_type, &prompt_str, parsed.identifier.as_deref());

    match result {
        DialogResult::Accepted { secret, save_to_keychain } => {
            // Write credential to stdout without creating an intermediate String copy
            let _ = std::io::stdout().write_all(secret.as_bytes());
            if !secret.ends_with('\n') {
                let _ = std::io::stdout().write_all(b"\n");
            }
            let _ = std::io::stdout().flush();

            // Store in keychain only if the user checked the checkbox
            if save_to_keychain {
                if let Some(id) = &parsed.identifier {
                    let _ = keychain::write(id, secret.as_bytes());
                }
            }

            drop(secret);
            std::process::exit(0);
        }
        DialogResult::Cancelled => {
            security::sigint_parent();
            std::process::exit(1);
        }
    }
}
