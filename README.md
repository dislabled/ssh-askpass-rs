# ssh-askpass-rs

A macOS SSH askpass helper, inspired by [ksshaskpass](https://github.com/KDE/ksshaskpass).

Displays native macOS dialogs for SSH credential prompts and stores secrets in the macOS Keychain.

## Requirements

- macOS 14 (Sonoma) or later
- Apple Silicon or Intel Mac

## Installation

### Homebrew (recommended)

```sh
brew install dislabled/ssh-askpass-rs/ssh-askpass-rs
```

### Build from source

You should know how to do this, if not use brew.
```sh
git clone https://github.com/dislabled/ssh-askpass-rs.git
cd ssh-askpass-rs
cargo build --release
cp target/release/ssh-askpass-rs /usr/local/bin/
```

## Setup

Add the following to your shell profile (`~/.zshrc` or `~/.bashrc`):

```sh
export SSH_ASKPASS=$(which ssh-askpass-rs)
export SSH_ASKPASS_REQUIRE=force
```

For system-wide use (applies to GUI apps and not just terminal sessions), install the provided LaunchAgent:

```sh
cp contrib/com.github.dislabled.ssh-askpass-rs.plist ~/Library/LaunchAgents/
launchctl load ~/Library/LaunchAgents/com.github.dislabled.ssh-askpass-rs.plist
```

## Usage

`ssh-askpass-rs` is called automatically by OpenSSH when a passphrase or password is needed. You do not invoke it directly.

- **Password dialogs** show a "Remember in Keychain" checkbox. If checked, the credential is stored and returned silently on future requests.
- **Bad passphrase** prompts also offer the checkbox, letting you overwrite a stale Keychain entry.
- **Confirm dialogs** (`SSH_ASKPASS_PROMPT=confirm`) show Accept/Cancel.
- **Unknown host key** dialogs show the fingerprint and Yes/No buttons.

## License

[GPL-3.0](LICENSE) + If it blows up your laptop, I am not responsible
