# ssh-askpass-rs

An SSH askpass helper, inspired by [ksshaskpass](https://invent.kde.org/plasma/ksshaskpass).

On macOS it shows native AppKit dialogs and stores secrets in the Keychain.
On Linux it shows GUI dialogs (built with [Slint](https://slint.dev/)) or runs inline in the terminal, and stores
secrets in your keyring over the freedesktop
[Secret Service](https://specifications.freedesktop.org/secret-service-spec/) API (gnome-keyring, KDE's ksecretd/kwallet, KeePassXC — whatever provides it).

## Requirements

**macOS**
- macOS 14 (Sonoma) or later
- Apple Silicon or Intel Mac

**Linux**
- A running Secret Service provider (gnome-keyring, ksecretd/kwalletd, KeePassXC, …)
- `libdbus-1`
- A Wayland or X11 session (for GUI dialogs)

## Installation

### Homebrew (macOS)

```sh
brew tap dislabled/ssh-askpass-rs https://github.com/dislabled/ssh-askpass-rs
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

For system-wide use (applies to GUI apps and not just terminal sessions), install the provided LaunchAgent: (macOS)

```sh
cp contrib/com.github.dislabled.ssh-askpass-rs.plist ~/Library/LaunchAgents/
launchctl load ~/Library/LaunchAgents/com.github.dislabled.ssh-askpass-rs.plist
```

## Usage

`ssh-askpass-rs` is called automatically by OpenSSH when a passphrase or password is needed. You do not invoke it directly.


I have so fixed on porting this from [ksshaskpass](https://invent.kde.org/plasma/ksshaskpass), and forgot that i could tune it to my liking.
Started on the gui, but personally I dont see the popping a GUI from the terminal and back.
There is now a inline dialog in the terminal instead. Since the GUI is already there, there is the option to choose.
    - `SSH_ASKPASS_MODE=auto` inline terminal when a /dev/tty exists, else GUI (default)
    - `SSH_ASKPASS_MODE=gui` Force GUI dialogs (AppKit on macOS, Slint on Linux)
    - `SSH_ASKPASS_MODE=terminal` strict inline, will error out and sigint shh when there is no TTY

I put in some color to distinguish prompts from ssh vs ssh-askpass-rs. Should honor the standard `NO_COLOR` env var.

- **Password dialogs** show a "Remember in Keychain/keyring" checkbox. If checked, the credential is stored and returned silently on future requests.
- **Bad passphrase** prompts also offer the checkbox, letting you overwrite a stale Keychain/keyring entry.
- **Confirm dialogs** (`SSH_ASKPASS_PROMPT=confirm`) show Accept/Cancel.
- **Unknown host key** dialogs show the fingerprint and Yes/No buttons.

> [!WARNING]
> As the function of this program is inherently unsafe (It can release stored credentials if you connect to a rogue server),
> I have tried to find a solution to verify connected server vs request. I just cannot find a proper way to do this, without breaking functionality.
> As a workaround, a confirmation window is added before releasing credentials, showing the connected server and what credentials are sought after.
> On by default, and can be disabled by setting `SSH_ASKPASS_NO_CONFIRM=<nonempty>`

## License

[GPL-3.0](LICENSE) + If it blows up your laptop, I am not responsible
