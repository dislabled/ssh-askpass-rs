mod dialog;
mod keychain;
mod prompt;
mod security;
mod terminal;

use dialog::DialogResult;
use prompt::{parse_prompt, prompt_type_from_env};
use std::io::Write;

/// Autofill confirmation is on by default; setting SSH_ASKPASS_NO_CONFIRM to a
/// non-empty value disables it system-wide
fn autofill_confirm_disabled() -> bool {
    std::env::var_os("SSH_ASKPASS_NO_CONFIRM").is_some_and(|v| !v.is_empty())
}

/// Frontend to present, selected via `SSH_ASKPASS_MODE`.
enum Frontend {
    /// Terminal when a tty exists, else GUI (default).
    Auto,
    /// Always the GUI dialogs.
    Gui,
    /// Require the terminal; error out when there's no tty.
    Terminal,
}

fn frontend_from_env() -> Frontend {
    match std::env::var("SSH_ASKPASS_MODE")
        .map(|v| v.trim().to_ascii_lowercase())
        .as_deref()
    {
        Ok("gui") => Frontend::Gui,
        Ok("terminal") | Ok("inline") | Ok("tty") => Frontend::Terminal,
        _ => Frontend::Auto,
    }
}

fn main() {
    security::disable_core_dumps();

    let prompt_str = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Please enter passphrase".to_string());

    let prompt_type = prompt_type_from_env();
    let parsed = parse_prompt(&prompt_str, &prompt_type);

    // Terminal when a tty exists, else GUI. SSH_ASKPASS_MODE overrides this.
    let frontend = frontend_from_env();
    let mut tty = match frontend {
        Frontend::Gui => None,
        Frontend::Auto | Frontend::Terminal => terminal::open(),
    };

    // Strict terminal mode: error out rather than silently pop the GUI.
    if matches!(frontend, Frontend::Terminal) && tty.is_none() {
        eprintln!(
            "ssh-askpass-rs: SSH_ASKPASS_MODE=terminal, but no controlling terminal \
             (/dev/tty) is available"
        );
        security::terminate_ssh();
        std::process::exit(1);
    }

    // Attempt keychain lookup
    if !parsed.skip_keychain {
        if let Some(id) = &parsed.identifier {
            if let Some(password) = keychain::read(id) {
                use terminal::AutofillChoice;

                // Gate reusable remote passwords behind a confirmation (unless
                // disabled system-wide via SSH_ASKPASS_NO_CONFIRM)
                let choice = if !parsed.confirm_autofill || autofill_confirm_disabled() {
                    AutofillChoice::Send
                } else {
                    match tty.as_mut() {
                        Some(t) => terminal::confirm_autofill(t, id),
                        None => {
                            if dialog::confirm_autofill(&prompt_str, id) {
                                AutofillChoice::Send
                            } else {
                                AutofillChoice::Manual
                            }
                        }
                    }
                };

                match choice {
                    AutofillChoice::Send => {
                        let _ = std::io::stdout().write_all(password.as_bytes());
                        if !password.ends_with('\n') {
                            let _ = std::io::stdout().write_all(b"\n");
                        }
                        let _ = std::io::stdout().flush();
                        drop(password);
                        std::process::exit(0);
                    }
                    // Fall through to the manual entry prompt below.
                    AutofillChoice::Manual => drop(password),
                    // Abort so a wrong stored password can't burn ssh's retries.
                    AutofillChoice::Cancel => {
                        drop(password);
                        security::terminate_ssh();
                        std::process::exit(1);
                    }
                }
            }
        }
    }

    // Prompt: use the terminal when we have one, else fall back to the GUI.
    let result = tty
        .as_mut()
        .map(|t| {
            terminal::show(
                t,
                &parsed.display_type,
                &prompt_str,
                parsed.identifier.as_deref(),
            )
        })
        .unwrap_or_else(|| {
            dialog::show(
                &parsed.display_type,
                &prompt_str,
                parsed.identifier.as_deref(),
            )
        });

    match result {
        DialogResult::Accepted {
            secret,
            save_to_keychain,
        } => {
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
            security::terminate_ssh();
            std::process::exit(1);
        }
    }
}
