mod dialog;
mod keychain;
mod prompt;
mod security;

use dialog::DialogResult;
use prompt::{parse_prompt, prompt_type_from_env};
use std::io::Write;

fn main() {
    let prompt_str = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Please enter passphrase".to_string());

    let prompt_type = prompt_type_from_env();
    let parsed = parse_prompt(&prompt_str, &prompt_type);

    // Attempt keychain lookup
    if !parsed.skip_keychain {
        if let Some(id) = &parsed.identifier {
            if let Some(password) = keychain::read(id) {
                print!("{}\n", password.as_str());
                let _ = std::io::stdout().flush();
                std::process::exit(0);
            }
        }
    }

    // Show dialog
    let result = dialog::show(&parsed.display_type, &prompt_str, parsed.identifier.as_deref());

    match result {
        DialogResult::Accepted { secret, save_to_keychain } => {
            // Write credential to stdout (OpenSSH reads from here)
            let output = if secret.ends_with('\n') {
                secret.as_str().to_string()
            } else {
                format!("{}\n", secret.as_str())
            };
            print!("{}", output);
            let _ = std::io::stdout().flush();

            // Store in keychain only if the user checked the checkbox
            if save_to_keychain {
                if let Some(id) = &parsed.identifier {
                    let _ = keychain::write(id, secret.as_bytes());
                }
            }

            std::process::exit(0);
        }
        DialogResult::Cancelled => {
            security::sigint_parent();
            std::process::exit(1);
        }
    }
}
