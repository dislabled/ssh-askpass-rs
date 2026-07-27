use std::env;
use zeroize::Zeroizing;

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

/// Outcome of a prompt, from either frontend.
pub enum DialogResult {
    Accepted {
        secret: Zeroizing<String>,
        save_secret: bool,
    },
    Cancelled,
}

impl DialogResult {
    /// The "yes" answer ssh expects for confirm / host-key acceptances.
    pub fn yes() -> Self {
        DialogResult::Accepted {
            secret: Zeroizing::new("yes\n".to_string()),
            save_secret: false,
        }
    }
}

#[derive(Debug)]
pub struct ParsedPrompt {
    pub display_type: DisplayType,
    pub identifier: Option<String>,
    pub skip_keychain: bool,
    /// Whether autofilling a stored credential for this prompt should be
    /// confirmed by the user first. True only for reusable remote/account
    /// passwords (SSH login, network PAM, git credential, git-lfs); false for
    /// key passphrases and token PINs. Only consulted on the keychain
    /// fast-path (when skip_keychain is false and a credential is found).
    pub confirm_autofill: bool,
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
            confirm_autofill: false,
        };
    }

    if *prompt_type == PromptType::Confirm {
        return ParsedPrompt {
            display_type: DisplayType::Confirm,
            identifier: None,
            skip_keychain: true,
            confirm_autofill: false,
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
            confirm_autofill: false,
        };
    }

    // Remote password auth (openssh): *'s password:
    if let Some(id) = extract_before(prompt, "'s password: ") {
        if id.contains('@') && !id.contains('(') {
            return ParsedPrompt {
                display_type: DisplayType::Password,
                identifier: Some(id),
                skip_keychain: false,
                confirm_autofill: true,
            };
        }
    }

    // PAM variant: *'s Password:
    if let Some(id) = extract_before(prompt, "'s Password: ") {
        if id.contains('@') && !id.contains('(') {
            return ParsedPrompt {
                display_type: DisplayType::Password,
                identifier: Some(id),
                skip_keychain: false,
                confirm_autofill: true,
            };
        }
    }

    // PAM variant: * password:
    if let Some(id) = extract_before(prompt, " password: ") {
        if id.contains('@') && !id.contains('(') {
            return ParsedPrompt {
                display_type: DisplayType::Password,
                identifier: Some(id),
                skip_keychain: false,
                confirm_autofill: true,
            };
        }
    }

    // PAM variant: * Password:
    if let Some(id) = extract_before(prompt, " Password: ") {
        if id.contains('@') && !id.contains('(') {
            return ParsedPrompt {
                display_type: DisplayType::Password,
                identifier: Some(id),
                skip_keychain: false,
                confirm_autofill: true,
            };
        }
    }

    // Old/new password change prompts: "Enter|Retype <user>'s old|new password: "
    // skip keychain
    if (prompt.starts_with("Enter ") || prompt.starts_with("Retype "))
        && (prompt.contains("'s old password: ") || prompt.contains("'s new password: "))
    {
        return ParsedPrompt {
            display_type: DisplayType::Password,
            identifier: None,
            skip_keychain: true,
            confirm_autofill: false,
        };
    }

    // Enter passphrase for '<key>':  (single-quoted, openssh)
    // Enter passphrase for key '<key>':  (single-quoted, git ssh variant)
    if prompt.starts_with("Enter passphrase for ") && prompt.ends_with("': ") {
        let id = extract_single_quoted(prompt);
        return ParsedPrompt {
            display_type: DisplayType::Password,
            identifier: id,
            skip_keychain: false,
            confirm_autofill: false,
        };
    }

    // Enter passphrase for <key>:  (unquoted, no single quote in key)
    if prompt.starts_with("Enter passphrase for ")
        && prompt.ends_with(": ")
        && !prompt.contains('\'')
    {
        let after = &prompt["Enter passphrase for ".len()..];
        let key = after
            .trim_end_matches(": ")
            .trim_end_matches(" (will confirm each use)")
            .to_string();
        return ParsedPrompt {
            display_type: DisplayType::Password,
            identifier: Some(key),
            skip_keychain: false,
            confirm_autofill: false,
        };
    }

    // Bad passphrase: skip the keychain lookup (we already tried it and it was wrong),
    // but keep the identifier so the dialog can offer to overwrite the stale entry.
    if let Some(after) = prompt.strip_prefix("Bad passphrase, try again for ") {
        let key = after
            .trim_end_matches(": ")
            .trim_end_matches(" (will confirm each use)")
            .to_string();
        return ParsedPrompt {
            display_type: DisplayType::Password,
            identifier: Some(key),
            skip_keychain: true,
            confirm_autofill: false,
        };
    }

    // Enter PIN for '<token>':  (single-quoted)
    if prompt.starts_with("Enter PIN for '") && prompt.ends_with("': ") {
        let id = extract_single_quoted(prompt);
        return ParsedPrompt {
            display_type: DisplayType::Pin,
            identifier: id,
            skip_keychain: false,
            confirm_autofill: false,
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
            confirm_autofill: false,
        };
    }

    // Password for '<id>':  (single-quoted, git credential)
    if prompt.starts_with("Password for '") && prompt.ends_with("': ") {
        let id = extract_single_quoted(prompt);
        return ParsedPrompt {
            display_type: DisplayType::Password,
            identifier: id,
            skip_keychain: false,
            confirm_autofill: true,
        };
    }

    // Password for "<id>"  (double-quoted, git-lfs)
    if prompt.starts_with("Password for \"") {
        let id = extract_double_quoted(prompt);
        return ParsedPrompt {
            display_type: DisplayType::Password,
            identifier: id,
            skip_keychain: false,
            confirm_autofill: true,
        };
    }

    // Verification code (OTP)
    if prompt == "Verification code: " {
        return ParsedPrompt {
            display_type: DisplayType::ClearText,
            identifier: None,
            skip_keychain: true,
            confirm_autofill: false,
        };
    }

    // Username: (bare)
    if prompt == "Username: " {
        return ParsedPrompt {
            display_type: DisplayType::ClearText,
            identifier: None,
            skip_keychain: true,
            confirm_autofill: false,
        };
    }

    // Username for '<id>':  (single-quoted)
    if prompt.starts_with("Username for '") && prompt.ends_with("': ") {
        let id = extract_single_quoted(prompt);
        return ParsedPrompt {
            display_type: DisplayType::ClearText,
            identifier: id,
            skip_keychain: true,
            confirm_autofill: false,
        };
    }

    // Username for "<id>"  (double-quoted)
    if prompt.starts_with("Username for \"") {
        let id = extract_double_quoted(prompt);
        return ParsedPrompt {
            display_type: DisplayType::ClearText,
            identifier: id,
            skip_keychain: true,
            confirm_autofill: false,
        };
    }

    // Password: (bare, git)
    if prompt == "Password: " {
        return ParsedPrompt {
            display_type: DisplayType::Password,
            identifier: None,
            skip_keychain: true,
            confirm_autofill: false,
        };
    }

    // Network equipment PAM: starts with '(', contains user@host in parens
    if prompt.starts_with('(') {
        let close = prompt.find(')');
        if let Some(pos) = close {
            let inner = &prompt[1..pos];
            if inner.contains('@') {
                let rest = &prompt[pos + 1..];
                if rest.starts_with(" Password:")
                    || rest.starts_with(" password:")
                    || rest.starts_with("'s Password:")
                    || rest.starts_with("'s password:")
                {
                    return ParsedPrompt {
                        display_type: DisplayType::Password,
                        identifier: Some(inner.to_string()),
                        skip_keychain: false,
                        confirm_autofill: true,
                    };
                }
            }
        }
    }

    // Fallback
    let msg = format!("ssh-askpass-rs: unrecognized prompt: {:?}", prompt);
    if env::var_os("NO_COLOR").is_some() {
        eprintln!("{msg}");
    } else {
        eprintln!("\x1b[31m{msg}\x1b[0m");
    }
    ParsedPrompt {
        display_type: DisplayType::Password,
        identifier: None,
        skip_keychain: true,
        confirm_autofill: false,
    }
}

/// Returns everything before the first occurrence of `suffix`, if present.
fn extract_before(s: &str, suffix: &str) -> Option<String> {
    s.find(suffix).map(|pos| s[..pos].to_string())
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Parse as an Entry prompt and assert every field of the result.
    fn check(prompt: &str, dt: DisplayType, id: Option<&str>, skip: bool, confirm: bool) {
        let r = parse_prompt(prompt, &PromptType::Entry);
        assert_eq!(r.display_type, dt, "display_type for {prompt:?}");
        assert_eq!(r.identifier.as_deref(), id, "identifier for {prompt:?}");
        assert_eq!(r.skip_keychain, skip, "skip_keychain for {prompt:?}");
        assert_eq!(
            r.confirm_autofill, confirm,
            "confirm_autofill for {prompt:?}"
        );
    }

    // PromptType shortcuts (don't depend on the prompt string)

    #[test]
    fn prompt_type_none_is_confirm_cancel() {
        let r = parse_prompt("anything", &PromptType::None);
        assert_eq!(r.display_type, DisplayType::ConfirmCancel);
        assert_eq!(r.identifier, None);
        assert!(r.skip_keychain);
        assert!(!r.confirm_autofill);
    }

    #[test]
    fn prompt_type_confirm_is_confirm() {
        let r = parse_prompt("anything", &PromptType::Confirm);
        assert_eq!(r.display_type, DisplayType::Confirm);
        assert_eq!(r.identifier, None);
        assert!(r.skip_keychain);
        assert!(!r.confirm_autofill);
    }

    // Remote password auth (the four @ forms), keychain + confirm

    #[test]
    fn remote_password_openssh() {
        check(
            "alice@server's password: ",
            DisplayType::Password,
            Some("alice@server"),
            false,
            true,
        );
    }

    #[test]
    fn remote_password_pam_capital() {
        check(
            "bob@host's Password: ",
            DisplayType::Password,
            Some("bob@host"),
            false,
            true,
        );
    }

    #[test]
    fn remote_password_pam_space_lower() {
        check(
            "admin@box password: ",
            DisplayType::Password,
            Some("admin@box"),
            false,
            true,
        );
    }

    #[test]
    fn remote_password_pam_space_capital() {
        check(
            "admin@box Password: ",
            DisplayType::Password,
            Some("admin@box"),
            false,
            true,
        );
    }

    // Password change: no identifier, skip keychain

    #[test]
    fn old_password_change() {
        check(
            "Enter alice's old password: ",
            DisplayType::Password,
            None,
            true,
            false,
        );
    }

    #[test]
    fn new_password_change_retype() {
        check(
            "Retype alice's new password: ",
            DisplayType::Password,
            None,
            true,
            false,
        );
    }

    // Key passphrases: keychain, but never auto-confirm

    #[test]
    fn passphrase_single_quoted() {
        check(
            "Enter passphrase for '/home/u/.ssh/id_ed25519': ",
            DisplayType::Password,
            Some("/home/u/.ssh/id_ed25519"),
            false,
            false,
        );
    }

    #[test]
    fn passphrase_git_key_variant() {
        check(
            "Enter passphrase for key '/home/u/.ssh/id_rsa': ",
            DisplayType::Password,
            Some("/home/u/.ssh/id_rsa"),
            false,
            false,
        );
    }

    #[test]
    fn passphrase_legacy_rsa_key_variant() {
        // Legacy/typed OpenSSH phrasing ksshaskpass handles via "for( RSA)? key".
        check(
            "Enter passphrase for RSA key '/home/u/.ssh/id_rsa': ",
            DisplayType::Password,
            Some("/home/u/.ssh/id_rsa"),
            false,
            false,
        );
    }

    #[test]
    fn passphrase_unquoted() {
        check(
            "Enter passphrase for /home/u/.ssh/id_rsa: ",
            DisplayType::Password,
            Some("/home/u/.ssh/id_rsa"),
            false,
            false,
        );
    }

    #[test]
    fn passphrase_unquoted_strips_confirm_each_use() {
        check(
            "Enter passphrase for /home/u/.ssh/id_rsa (will confirm each use): ",
            DisplayType::Password,
            Some("/home/u/.ssh/id_rsa"),
            false,
            false,
        );
    }

    #[test]
    fn bad_passphrase_keeps_id_but_skips_keychain() {
        check(
            "Bad passphrase, try again for /home/u/.ssh/id_rsa: ",
            DisplayType::Password,
            Some("/home/u/.ssh/id_rsa"),
            true,
            false,
        );
    }

    // PINs

    #[test]
    fn pin_token_single_quoted() {
        check(
            "Enter PIN for 'PIV Card': ",
            DisplayType::Pin,
            Some("PIV Card"),
            false,
            false,
        );
    }

    #[test]
    fn pin_ssh_agent_key_hashes_identifier() {
        // The ssh-agent key PIN uses an opaque hashed identifier and skips keychain.
        let r = parse_prompt(
            "Enter PIN for key /home/u/.ssh/id_ecdsa: ",
            &PromptType::Entry,
        );
        assert_eq!(r.display_type, DisplayType::Pin);
        assert!(r.skip_keychain);
        assert!(!r.confirm_autofill);
        assert!(r.identifier.as_deref().unwrap().starts_with("PIN:"));
    }

    // git credential / git-lfs

    #[test]
    fn git_credential_password() {
        check(
            "Password for 'https://github.com': ",
            DisplayType::Password,
            Some("https://github.com"),
            false,
            true,
        );
    }

    #[test]
    fn git_lfs_password_double_quoted() {
        check(
            "Password for \"https://github.com\"",
            DisplayType::Password,
            Some("https://github.com"),
            false,
            true,
        );
    }

    #[test]
    fn git_credential_username() {
        check(
            "Username for 'https://github.com': ",
            DisplayType::ClearText,
            Some("https://github.com"),
            true,
            false,
        );
    }

    #[test]
    fn git_lfs_username_double_quoted() {
        check(
            "Username for \"https://github.com\"",
            DisplayType::ClearText,
            Some("https://github.com"),
            true,
            false,
        );
    }

    // Bare / clear-text prompts

    #[test]
    fn verification_code() {
        check(
            "Verification code: ",
            DisplayType::ClearText,
            None,
            true,
            false,
        );
    }

    #[test]
    fn bare_username() {
        check("Username: ", DisplayType::ClearText, None, true, false);
    }

    #[test]
    fn bare_password() {
        check("Password: ", DisplayType::Password, None, true, false);
    }

    // Network-equipment PAM: "(user@host) Password:"

    #[test]
    fn network_pam_with_trailing_space() {
        check(
            "(admin@switch) Password: ",
            DisplayType::Password,
            Some("admin@switch"),
            false,
            true,
        );
    }

    #[test]
    fn network_pam_without_trailing_space() {
        check(
            "(administrator@test.example.com) Password:",
            DisplayType::Password,
            Some("administrator@test.example.com"),
            false,
            true,
        );
    }

    #[test]
    fn network_pam_apostrophe_lowercase_variant() {
        check(
            "(admin@switch)'s password: ",
            DisplayType::Password,
            Some("admin@switch"),
            false,
            true,
        );
    }

    // Unknown host key

    #[test]
    fn unknown_host_key() {
        let prompt = "The authenticity of host 'example.com (1.2.3.4)' can't be established.\n\
                      ED25519 key fingerprint is SHA256:abc123.\n\
                      Are you sure you want to continue connecting (yes/no/[fingerprint])? ";
        check(prompt, DisplayType::UnknownSshHost, None, true, false);
    }

    // Fallback

    #[test]
    fn unrecognized_falls_back_to_password_no_keychain() {
        check(
            "Some prompt we do not recognize",
            DisplayType::Password,
            None,
            true,
            false,
        );
    }
}
