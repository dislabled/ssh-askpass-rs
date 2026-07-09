class SshAskpassRs < Formula
  desc "macOS SSH askpass helper with native dialogs and Keychain integration"
  homepage "https://github.com/dislabled/ssh-askpass-rs"
  url "https://github.com/dislabled/ssh-askpass-rs/releases/download/v0.2.0/ssh-askpass-rs-macos.tar.gz"
  sha256 "bae82e9f9361fc7cb0e091c7d4ba07a15f16c6c53ea9b887eec2dbbe280f587a"
  license "GPL-3.0-only"
  version "0.2.0"

  depends_on :macos => :sonoma

  def install
    bin.install "ssh-askpass-rs"
    pkgshare.install "com.github.dislabled.ssh-askpass-rs.plist"
  end

  def caveats
    <<~EOS
      To enable ssh-askpass-rs for your shell, add to ~/.zshrc:
        export SSH_ASKPASS="#{bin}/ssh-askpass-rs"
        export SSH_ASKPASS_REQUIRE=force

      For system-wide use (including GUI apps), install the LaunchAgent:
        cp #{pkgshare}/com.github.dislabled.ssh-askpass-rs.plist ~/Library/LaunchAgents/
        launchctl load ~/Library/LaunchAgents/com.github.dislabled.ssh-askpass-rs.plist
    EOS
  end

  test do
    assert_predicate bin/"ssh-askpass-rs", :executable?
  end
end
