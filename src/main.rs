mod dialog;
#[cfg(target_os = "macos")]
mod keychain;
mod prompt;
#[cfg(target_os = "linux")]
mod secret_service;
mod security;
mod store;
mod terminal;

use prompt::{parse_prompt, prompt_type_from_env, DialogResult, DisplayType};
use std::io::Write;
use terminal::AutofillChoice;

/// Autofill confirmation is on by default; setting SSH_ASKPASS_NO_CONFIRM to a
/// non-empty value disables it system-wide
fn autofill_confirm_disabled() -> bool {
    std::env::var_os("SSH_ASKPASS_NO_CONFIRM").is_some_and(|v| !v.is_empty())
}

/// Write the credential newline-terminated, without an intermediate String copy.
/// Errors are the caller's to handle: ssh reads the answer from a pipe, and a
/// failed write means it never arrives.
fn write_secret<W: Write>(out: &mut W, secret: &str) -> std::io::Result<()> {
    out.write_all(secret.as_bytes())?;
    if !secret.ends_with('\n') {
        out.write_all(b"\n")?;
    }
    out.flush()
}

/// Deliver the credential to ssh on stdout.
fn deliver(secret: &str) -> std::io::Result<()> {
    let stdout = std::io::stdout();
    write_secret(&mut stdout.lock(), secret)
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

// GUI fallback, when no controlling terminal.
#[cfg(any(target_os = "macos", target_os = "linux"))]
fn gui_show(
    display_type: &DisplayType,
    prompt: &str,
    identifier: Option<&str>,
    store: &dyn store::SecretStore,
) -> DialogResult {
    dialog::show(display_type, prompt, identifier, store)
}

#[cfg(any(target_os = "macos", target_os = "linux"))]
fn gui_confirm_autofill(prompt: &str, id: &str) -> AutofillChoice {
    if dialog::confirm_autofill(prompt, id) {
        AutofillChoice::Send
    } else {
        AutofillChoice::Manual
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn gui_show(
    _display_type: &DisplayType,
    _prompt: &str,
    _identifier: Option<&str>,
    _store: &dyn store::SecretStore,
) -> DialogResult {
    no_gui()
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn gui_confirm_autofill(_prompt: &str, _id: &str) -> AutofillChoice {
    no_gui()
}

/// Linux: Ask whether to retry a failed secret-store operation
#[cfg(target_os = "linux")]
fn confirm_retry(tty: Option<&mut std::fs::File>, message: &str) -> bool {
    match tty {
        Some(t) => terminal::ask_retry(t, message),
        None => dialog::ask_retry(message),
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn no_gui() -> ! {
    eprintln!(
        "ssh-askpass-rs: no controlling terminal (/dev/tty) available and no GUI \
         backend on this platform"
    );
    security::terminate_ssh();
    std::process::exit(1);
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

    let store = store::default_store();

    // Attempt keychain lookup
    if !parsed.skip_keychain {
        if let Some(id) = &parsed.identifier {
            let lookup = loop {
                match store.read(id) {
                    Ok(v) => break v,
                    Err(e) => {
                        eprintln!("ssh-askpass-rs: warning: secret store read failed: {e}");
                        #[cfg(target_os = "linux")]
                        {
                            let msg = format!("Secret store is unavailable: {e}\nTry again?");
                            if confirm_retry(tty.as_mut(), &msg) {
                                continue;
                            }
                        }
                        // Fall through to manual entry.
                        break None;
                    }
                }
            };
            if let Some(password) = lookup {
                // Gate reusable remote passwords behind a confirmation (unless
                // disabled system-wide via SSH_ASKPASS_NO_CONFIRM)
                let choice = if !parsed.confirm_autofill || autofill_confirm_disabled() {
                    AutofillChoice::Send
                } else {
                    match tty.as_mut() {
                        Some(t) => terminal::confirm_autofill(t, id),
                        None => gui_confirm_autofill(&prompt_str, id),
                    }
                };

                match choice {
                    AutofillChoice::Send => {
                        let delivered = deliver(&password);
                        drop(password);
                        if let Err(e) = delivered {
                            eprintln!("ssh-askpass-rs: failed to deliver credential: {e}");
                            std::process::exit(1);
                        }
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
                store.as_ref(),
                &parsed.display_type,
                &prompt_str,
                parsed.identifier.as_deref(),
            )
        })
        .unwrap_or_else(|| {
            gui_show(
                &parsed.display_type,
                &prompt_str,
                parsed.identifier.as_deref(),
                store.as_ref(),
            )
        });

    match result {
        DialogResult::Accepted {
            secret,
            save_secret,
        } => {
            if let Err(e) = deliver(&secret) {
                eprintln!("ssh-askpass-rs: failed to deliver credential: {e}");
                drop(secret);
                std::process::exit(1);
            }

            // Store in keychain only if the user checked the checkbox
            if save_secret {
                if let Some(id) = &parsed.identifier {
                    loop {
                        match store.write(id, secret.as_bytes()) {
                            Ok(()) => break,
                            Err(e) => {
                                eprintln!(
                                    "ssh-askpass-rs: warning: failed to save password: {e}"
                                );
                                #[cfg(target_os = "linux")]
                                {
                                    let msg = format!(
                                        "Secret store is unavailable: {e}\nTry again?"
                                    );
                                    if confirm_retry(tty.as_mut(), &msg) {
                                        continue;
                                    }
                                }
                                break;
                            }
                        }
                    }
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

#[cfg(test)]
mod tests {
    use super::write_secret;
    use std::io::{Error, ErrorKind, Write};

    /// Fails every write, like a stdout pipe whose reader has gone away.
    struct BrokenPipe;

    impl Write for BrokenPipe {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Err(Error::new(ErrorKind::BrokenPipe, "broken pipe"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn appends_newline() {
        let mut out = Vec::new();
        write_secret(&mut out, "hunter2").unwrap();
        assert_eq!(out, b"hunter2\n");
    }

    #[test]
    fn does_not_double_terminate() {
        let mut out = Vec::new();
        write_secret(&mut out, "hunter2\n").unwrap();
        assert_eq!(out, b"hunter2\n");
    }

    #[test]
    fn empty_secret_still_terminated() {
        let mut out = Vec::new();
        write_secret(&mut out, "").unwrap();
        assert_eq!(out, b"\n");
    }

    #[test]
    fn write_failure_is_reported() {
        let err = write_secret(&mut BrokenPipe, "hunter2").unwrap_err();
        assert_eq!(err.kind(), ErrorKind::BrokenPipe);
    }
}
