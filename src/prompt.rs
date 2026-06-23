use std::env;

#[derive(Debug, PartialEq)]
pub enum PromptType {
    Entry,
    Confirm,
    None,
}

#[derive(Debug, PartialEq, Clone)]
pub enum DisplayType {
    Password,
    Pin,
    ClearText,
    Confirm,
    ConfirmCancel,
    UnknownSshHost,
}

#[derive(Debug)]
pub struct ParsedPrompt {
    pub display_type: DisplayType,
    pub identifier: Option<String>,
    pub skip_keychain: bool,
}

pub fn prompt_type_from_env() -> PromptType {
    match env::var("SSH_ASKPASS_PROMPT").as_deref() {
        Ok("confirm") => PromptType::Confirm,
        Ok("none") => PromptType::None,
        _ => PromptType::Entry,
    }
}

pub fn parse_prompt(prompt: &str, prompt_type: &PromptType) -> ParsedPrompt {
    if *prompt_type == PromptType::None {
        return ParsedPrompt {
            display_type: DisplayType::ConfirmCancel,
            identifier: None,
            skip_keychain: true,
        };
    }

    if *prompt_type == PromptType::Confirm {
        return ParsedPrompt {
            display_type: DisplayType::Confirm,
            identifier: None,
            skip_keychain: true,
        };
    }

    // Unknown SSH host key
    if prompt.starts_with("The authenticity of host '")
        && prompt.contains("can't be established")
        && prompt.contains("key fingerprint is")
        && prompt.contains("Are you sure you want to continue connecting")
    {
        return ParsedPrompt {
            display_type: DisplayType::UnknownSshHost,
            identifier: None,
            skip_keychain: true,
        };
    }

    // Remote password auth (openssh): *'s password:
    if let Some(id) = extract_after_prefix_before_suffix(prompt, "'s password: ", "") {
        if id.contains('@') && !id.contains('(') {
            return ParsedPrompt {
                display_type: DisplayType::Password,
                identifier: Some(id),
                skip_keychain: false,
            };
        }
    }

    // PAM variant: *'s Password:
    if let Some(id) = extract_after_prefix_before_suffix(prompt, "'s Password: ", "") {
        if id.contains('@') && !id.contains('(') {
            return ParsedPrompt {
                display_type: DisplayType::Password,
                identifier: Some(id),
                skip_keychain: false,
            };
        }
    }

    // PAM variant: * password:
    if let Some(id) = extract_after_prefix_before_suffix(prompt, " password: ", "") {
        if id.contains('@') && !id.contains('(') {
            return ParsedPrompt {
                display_type: DisplayType::Password,
                identifier: Some(id),
                skip_keychain: false,
            };
        }
    }

    // PAM variant: * Password:
    if let Some(id) = extract_after_prefix_before_suffix(prompt, " Password: ", "") {
        if id.contains('@') && !id.contains('(') {
            return ParsedPrompt {
                display_type: DisplayType::Password,
                identifier: Some(id),
                skip_keychain: false,
            };
        }
    }

    // Old password prompts (skip keychain)
    for prefix in &[
        "Enter 's old password: ",
        "Retype 's old password: ",
        "Enter 's new password: ",
        "Retype 's new password: ",
    ] {
        if prompt.starts_with(prefix) || prompt.contains(prefix) {
            return ParsedPrompt {
                display_type: DisplayType::Password,
                identifier: None,
                skip_keychain: true,
            };
        }
    }
    // Also check without the embedded apostrophe pattern
    if (prompt.starts_with("Enter ") || prompt.starts_with("Retype "))
        && (prompt.contains("'s old password: ") || prompt.contains("'s new password: "))
    {
        return ParsedPrompt {
            display_type: DisplayType::Password,
            identifier: None,
            skip_keychain: true,
        };
    }

    // Enter passphrase for '<key>':  (single-quoted)
    if prompt.starts_with("Enter passphrase for '") && prompt.ends_with("': ") {
        let id = extract_single_quoted(prompt);
        return ParsedPrompt {
            display_type: DisplayType::Password,
            identifier: id,
            skip_keychain: false,
        };
    }

    // Enter passphrase for <key>:  (unquoted, no single quote in key)
    if prompt.starts_with("Enter passphrase for ") && prompt.ends_with(": ") && !prompt.contains('\'') {
        let after = &prompt["Enter passphrase for ".len()..];
        let key = after
            .trim_end_matches(": ")
            .trim_end_matches(" (will confirm each use)")
            .to_string();
        return ParsedPrompt {
            display_type: DisplayType::Password,
            identifier: Some(key),
            skip_keychain: false,
        };
    }

    // Bad passphrase: skip the keychain lookup (we already tried it and it was wrong),
    // but keep the identifier so the dialog can offer to overwrite the stale entry.
    if prompt.starts_with("Bad passphrase, try again for ") {
        let after = &prompt["Bad passphrase, try again for ".len()..];
        let key = after
            .trim_end_matches(": ")
            .trim_end_matches(" (will confirm each use)")
            .to_string();
        return ParsedPrompt {
            display_type: DisplayType::Password,
            identifier: Some(key),
            skip_keychain: true,
        };
    }

    // Enter PIN for '<token>':  (single-quoted)
    if prompt.starts_with("Enter PIN for '") && prompt.ends_with("': ") {
        let id = extract_single_quoted(prompt);
        return ParsedPrompt {
            display_type: DisplayType::Pin,
            identifier: id,
            skip_keychain: false,
        };
    }

    // PIN for ssh-agent key (contains " key ", ends with ": ")
    if prompt.starts_with("Enter PIN") && prompt.contains(" key ") && prompt.ends_with(": ") {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        prompt.hash(&mut h);
        let id = format!("PIN:{:x}", h.finish());
        return ParsedPrompt {
            display_type: DisplayType::Pin,
            identifier: Some(id),
            skip_keychain: true,
        };
    }

    // Password for '<id>':  (single-quoted, git credential)
    if prompt.starts_with("Password for '") && prompt.ends_with("': ") {
        let id = extract_single_quoted(prompt);
        return ParsedPrompt {
            display_type: DisplayType::Password,
            identifier: id,
            skip_keychain: false,
        };
    }

    // Password for "<id>"  (double-quoted, git-lfs)
    if prompt.starts_with("Password for \"") {
        let id = extract_double_quoted(prompt);
        return ParsedPrompt {
            display_type: DisplayType::Password,
            identifier: id,
            skip_keychain: false,
        };
    }

    // Verification code (OTP)
    if prompt == "Verification code: " {
        return ParsedPrompt {
            display_type: DisplayType::ClearText,
            identifier: None,
            skip_keychain: true,
        };
    }

    // Username: (bare)
    if prompt == "Username: " {
        return ParsedPrompt {
            display_type: DisplayType::ClearText,
            identifier: None,
            skip_keychain: true,
        };
    }

    // Username for '<id>':  (single-quoted)
    if prompt.starts_with("Username for '") && prompt.ends_with("': ") {
        let id = extract_single_quoted(prompt);
        return ParsedPrompt {
            display_type: DisplayType::ClearText,
            identifier: id,
            skip_keychain: true,
        };
    }

    // Username for "<id>"  (double-quoted)
    if prompt.starts_with("Username for \"") {
        let id = extract_double_quoted(prompt);
        return ParsedPrompt {
            display_type: DisplayType::ClearText,
            identifier: id,
            skip_keychain: true,
        };
    }

    // Password: (bare, git)
    if prompt == "Password: " {
        return ParsedPrompt {
            display_type: DisplayType::Password,
            identifier: None,
            skip_keychain: true,
        };
    }

    // Network equipment PAM: starts with '(', contains user@host in parens
    if prompt.starts_with('(') {
        let close = prompt.find(')');
        if let Some(pos) = close {
            let inner = &prompt[1..pos];
            if inner.contains('@') {
                let rest = &prompt[pos + 1..];
                if rest.starts_with(" Password: ")
                    || rest.starts_with(" password: ")
                    || rest.starts_with("'s Password: ")
                    || rest.starts_with("'s password: ")
                {
                    return ParsedPrompt {
                        display_type: DisplayType::Password,
                        identifier: Some(inner.to_string()),
                        skip_keychain: false,
                    };
                }
            }
        }
    }

    // Fallback
    eprintln!("ssh-askpass-rs: unrecognized prompt: {:?}", prompt);
    ParsedPrompt {
        display_type: DisplayType::Password,
        identifier: None,
        skip_keychain: true,
    }
}

fn extract_after_prefix_before_suffix<'a>(s: &'a str, suffix: &str, _prefix: &str) -> Option<String> {
    if let Some(pos) = s.find(suffix) {
        Some(s[..pos].to_string())
    } else {
        None
    }
}

fn extract_single_quoted(s: &str) -> Option<String> {
    let first = s.find('\'')?;
    let last = s.rfind('\'')?;
    if first < last {
        Some(s[first + 1..last].to_string())
    } else {
        None
    }
}

fn extract_double_quoted(s: &str) -> Option<String> {
    let first = s.find('"')?;
    let last = s.rfind('"')?;
    if first < last {
        Some(s[first + 1..last].to_string())
    } else {
        None
    }
}
