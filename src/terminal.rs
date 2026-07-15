//! Terminal (inline) frontend.

use crate::prompt::{DialogResult, DisplayType};
use crate::store::SecretStore;
use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::io::{AsRawFd, RawFd};
use zeroize::{Zeroize, Zeroizing};

/// Open the controlling terminal, if any. Otherwise fall back to the GUI dialogs.
pub fn open() -> Option<File> {
    OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
        .ok()
}

/// What to do with a stored credential once the user has been asked.
pub enum AutofillChoice {
    /// Send the stored credential to ssh.
    Send,
    /// Don't send it; fall through to a manual entry prompt.
    Manual,
    /// Abort entirely (SIGINT ssh, exit).
    Cancel,
}

/// Confirm inline before releasing a stored credential.
pub fn confirm_autofill<T: Read + Write + RawInput>(
    tty: &mut T,
    identifier: &str,
) -> AutofillChoice {
    let _ = write!(
        tty,
        "{}Send stored credential for '{identifier}'? [y]es / [n]o, type it / [c]ancel: ",
        prefix()
    );
    let _ = tty.flush();

    let _cbreak = tty.cbreak();
    loop {
        match read_byte(tty) {
            // EOF (Ctrl-D): treat as cancel rather than silently proceeding.
            None => return AutofillChoice::Cancel,
            Some(b'y') | Some(b'Y') => return echo(tty, "y", AutofillChoice::Send),
            Some(b'c') | Some(b'C') => return echo(tty, "c", AutofillChoice::Cancel),
            Some(b'n') | Some(b'N') => return echo(tty, "n", AutofillChoice::Manual),
            Some(b'\r') | Some(b'\n') => return echo(tty, "", AutofillChoice::Manual),
            _ => continue,
        }
    }
}

/// Echo the resolved answer (cbreak suppressed the keystroke) and return `value`.
fn echo<T: Write, V>(tty: &mut T, shown: &str, value: V) -> V {
    let _ = writeln!(tty, "{shown}");
    value
}

/// Prompt on the terminal. Handles every `DisplayType` inline.
pub fn show(
    tty: &mut File,
    store: &dyn SecretStore,
    display_type: &DisplayType,
    prompt: &str,
    identifier: Option<&str>,
) -> DialogResult {
    match display_type {
        DisplayType::Password | DisplayType::Pin => {
            read_input(tty, store, prompt, true, identifier)
        }
        DisplayType::ClearText => read_input(tty, store, prompt, false, identifier),
        DisplayType::Confirm => confirm_prompt(tty, prompt),
        DisplayType::ConfirmCancel => confirm_cancel_prompt(tty, prompt),
        DisplayType::UnknownSshHost => host_key_prompt(tty, prompt),
    }
}

/// Accept/Cancel confirm dialog.
fn confirm_prompt<T: Read + Write + RawInput>(tty: &mut T, prompt: &str) -> DialogResult {
    if prompt_yes_no(tty, prompt, false) {
        DialogResult::yes()
    } else {
        DialogResult::Cancelled
    }
}

/// Cancel-only notice.
fn confirm_cancel_prompt<T: Read + Write>(tty: &mut T, prompt: &str) -> DialogResult {
    let _ = write!(tty, "{}{prompt} [Enter to dismiss] ", prefix());
    let _ = tty.flush();
    let _ = read_line(tty);
    let _ = writeln!(tty);
    DialogResult::Cancelled
}

/// Unknown-host-key warning.
fn host_key_prompt<T: Read + Write + RawInput>(tty: &mut T, prompt: &str) -> DialogResult {
    let cleaned = prompt
        .replace("(yes/no/[fingerprint])", "")
        .replace("(yes/no)", "")
        .replace("Are you sure", "\nAre you sure")
        .trim()
        .to_string();

    if prompt_yes_no(tty, &cleaned, false) {
        DialogResult::yes()
    } else {
        DialogResult::Cancelled
    }
}

/// Prefix that distinguishes application prompts from ssh's output.
fn prefix() -> &'static str {
    if std::env::var_os("NO_COLOR").is_some() {
        "ssh-askpass-rs >> "
    } else {
        "\x1b[1;36mssh-askpass-rs >>\x1b[0m "
    }
}

fn read_input(
    tty: &mut File,
    store: &dyn SecretStore,
    prompt: &str,
    secret_input: bool,
    identifier: Option<&str>,
) -> DialogResult {
    let value = {
        // Disable echo before printing the prompt!
        let _echo = if secret_input {
            EchoGuard::disable(tty.as_raw_fd())
        } else {
            None
        };

        let _ = write!(tty, "{}{}", prefix(), prompt);
        let _ = tty.flush();

        let line = read_line(tty);
        if secret_input {
            let _ = writeln!(tty);
        }
        line
    };

    let secret = match value {
        Some(s) => s,
        // EOF (Ctrl-D) with no input: treat as cancel.
        None => return DialogResult::Cancelled,
    };

    let save_secret = match identifier {
        Some(id) => {
            // Change wording based on if there is a stored secret.
            let question = if store.exists(id) {
                format!("Overwrite stored password for '{id}'?")
            } else {
                format!("Save password for '{id}'?")
            };
            prompt_yes_no(tty, &question, false)
        }
        None => false,
    };

    DialogResult::Accepted {
        secret,
        save_secret,
    }
}

/// Ask a `[y/N]` / `[Y/n]` question.
fn prompt_yes_no<T: Read + Write + RawInput>(
    tty: &mut T,
    question: &str,
    default_yes: bool,
) -> bool {
    let hint = if default_yes { "[Y/n]" } else { "[y/N]" };
    let _ = write!(tty, "{}{question} {hint} ", prefix());
    let _ = tty.flush();

    let _cbreak = tty.cbreak();
    loop {
        match read_byte(tty) {
            Some(b'y') | Some(b'Y') => return echo(tty, "y", true),
            Some(b'n') | Some(b'N') => return echo(tty, "n", false),
            Some(b'\r') | Some(b'\n') => return echo(tty, "", default_yes),
            None => return default_yes,
            _ => continue,
        }
    }
}

fn read_line<R: Read>(tty: &mut R) -> Option<Zeroizing<String>> {
    let mut bytes: Zeroizing<Vec<u8>> = Zeroizing::new(Vec::new());
    let mut b = [0u8; 1];
    loop {
        match tty.read(&mut b) {
            Ok(0) => {
                if bytes.is_empty() {
                    return None;
                }
                break;
            }
            Ok(_) => match b[0] {
                b'\n' => break,
                b'\r' => continue,
                _ => bytes.push(b[0]),
            },
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        }
    }
    b.zeroize();

    let s = match String::from_utf8(std::mem::take(&mut *bytes)) {
        Ok(s) => s,
        Err(e) => {
            let mut raw = Zeroizing::new(e.into_bytes());
            let lossy = String::from_utf8_lossy(&raw).into_owned();
            raw.zeroize();
            lossy
        }
    };
    Some(Zeroizing::new(s))
}

/// Read a single byte. None on EOF/error.
fn read_byte<R: Read>(tty: &mut R) -> Option<u8> {
    let mut b = [0u8; 1];
    loop {
        match tty.read(&mut b) {
            Ok(0) => return None,
            Ok(_) => return Some(b[0]),
            Err(ref e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(_) => return None,
        }
    }
}

/// A terminal that can enter single-key (cbreak) mode.
pub(crate) trait RawInput {
    fn cbreak(&self) -> Option<CbreakGuard> {
        None
    }
}

impl RawInput for File {
    fn cbreak(&self) -> Option<CbreakGuard> {
        CbreakGuard::enable(self.as_raw_fd())
    }
}

/// Puts the terminal in single-key mode (ICANON + ECHO off)
pub(crate) struct CbreakGuard {
    fd: RawFd,
    orig: libc::termios,
}

impl CbreakGuard {
    fn enable(fd: RawFd) -> Option<Self> {
        // SAFETY: tcgetattr fills a zeroed termios; tcsetattr applies a copy.
        unsafe {
            let mut term: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(fd, &mut term) != 0 {
                return None;
            }
            let orig = term;
            term.c_lflag &= !(libc::ICANON | libc::ECHO);
            term.c_cc[libc::VMIN] = 1;
            term.c_cc[libc::VTIME] = 0;
            // TCSAFLUSH drops any type-ahead so a buffered key can't auto-answer.
            if libc::tcsetattr(fd, libc::TCSAFLUSH, &term) != 0 {
                return None;
            }
            Some(CbreakGuard { fd, orig })
        }
    }
}

impl Drop for CbreakGuard {
    fn drop(&mut self) {
        // SAFETY: restoring the exact termios we captured in enable().
        unsafe {
            libc::tcsetattr(self.fd, libc::TCSAFLUSH, &self.orig);
        }
    }
}

/// Make sure a password isnt printed when typed.
struct EchoGuard {
    fd: RawFd,
    orig: libc::termios,
}

impl EchoGuard {
    fn disable(fd: RawFd) -> Option<Self> {
        // SAFETY: tcgetattr fills a zeroed termios; tcsetattr applies a copy.
        unsafe {
            let mut term: libc::termios = std::mem::zeroed();
            if libc::tcgetattr(fd, &mut term) != 0 {
                return None;
            }
            let orig = term;
            term.c_lflag &= !libc::ECHO;
            if libc::tcsetattr(fd, libc::TCSAFLUSH, &term) != 0 {
                return None;
            }
            Some(EchoGuard { fd, orig })
        }
    }
}

impl Drop for EchoGuard {
    fn drop(&mut self) {
        // SAFETY: restoring the exact termios we captured in disable().
        unsafe {
            libc::tcsetattr(self.fd, libc::TCSAFLUSH, &self.orig);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// In-memory stand-in for the tty
    struct FakeTty {
        input: Cursor<Vec<u8>>,
        output: Vec<u8>,
    }

    impl FakeTty {
        fn new(input: &[u8]) -> Self {
            FakeTty {
                input: Cursor::new(input.to_vec()),
                output: Vec::new(),
            }
        }
        fn printed(&self) -> String {
            String::from_utf8_lossy(&self.output).into_owned()
        }
    }

    impl Read for FakeTty {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            self.input.read(buf)
        }
    }

    impl Write for FakeTty {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.output.extend_from_slice(buf);
            Ok(buf.len())
        }
        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    // No real fd; cbreak() falls back to the no-op default.
    impl RawInput for FakeTty {}

    fn secret_of(r: &DialogResult) -> Option<&str> {
        match r {
            DialogResult::Accepted { secret, .. } => Some(secret),
            DialogResult::Cancelled => None,
        }
    }

    #[test]
    fn confirm_yes_accepts_with_yes_newline() {
        let mut tty = FakeTty::new(b"y\n");
        let r = confirm_prompt(&mut tty, "Allow this action?");
        assert_eq!(secret_of(&r), Some("yes\n"));
        assert!(tty.printed().contains("[y/N]"));
    }

    #[test]
    fn confirm_empty_defaults_to_cancel() {
        let mut tty = FakeTty::new(b"\n");
        assert!(matches!(
            confirm_prompt(&mut tty, "Allow this action?"),
            DialogResult::Cancelled
        ));
    }

    #[test]
    fn confirm_no_cancels() {
        let mut tty = FakeTty::new(b"n\n");
        assert!(matches!(
            confirm_prompt(&mut tty, "Allow this action?"),
            DialogResult::Cancelled
        ));
    }

    #[test]
    fn confirm_eof_cancels() {
        let mut tty = FakeTty::new(b"");
        assert!(matches!(
            confirm_prompt(&mut tty, "Allow this action?"),
            DialogResult::Cancelled
        ));
    }

    #[test]
    fn host_key_yes_accepts_and_strips_hint() {
        let prompt = "The authenticity of host 'x (1.2.3.4)' can't be established.\n\
                      Are you sure you want to continue connecting (yes/no/[fingerprint])? ";
        let mut tty = FakeTty::new(b"y\n");
        let r = host_key_prompt(&mut tty, prompt);
        assert_eq!(secret_of(&r), Some("yes\n"));
        let shown = tty.printed();
        assert!(!shown.contains("(yes/no/[fingerprint])"));
        assert!(shown.contains("[y/N]"));
    }

    #[test]
    fn host_key_empty_defaults_to_cancel() {
        let prompt = "The authenticity of host 'x' can't be established.\n\
                      Are you sure you want to continue connecting (yes/no/[fingerprint])? ";
        let mut tty = FakeTty::new(b"\n");
        assert!(matches!(
            host_key_prompt(&mut tty, prompt),
            DialogResult::Cancelled
        ));
    }

    #[test]
    fn confirm_cancel_always_cancels() {
        let mut tty = FakeTty::new(b"\n");
        assert!(matches!(
            confirm_cancel_prompt(&mut tty, "Just a notice."),
            DialogResult::Cancelled
        ));
    }

    #[test]
    fn autofill_choice_from_input() {
        let mut yes = FakeTty::new(b"y\n");
        assert!(matches!(
            confirm_autofill(&mut yes, "user@host"),
            AutofillChoice::Send
        ));
        let mut cancel = FakeTty::new(b"c\n");
        assert!(matches!(
            confirm_autofill(&mut cancel, "user@host"),
            AutofillChoice::Cancel
        ));
        let mut manual = FakeTty::new(b"\n");
        assert!(matches!(
            confirm_autofill(&mut manual, "user@host"),
            AutofillChoice::Manual
        ));
        let mut eof = FakeTty::new(b"");
        assert!(matches!(
            confirm_autofill(&mut eof, "user@host"),
            AutofillChoice::Cancel
        ));
    }
}
