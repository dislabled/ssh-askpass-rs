class SshAskpassRs < Formula
  desc "macOS SSH askpass helper with native dialogs and Keychain integration"
  homepage "https://github.com/dislabled/ssh-askpass-rs"
  url "https://github.com/dislabled/ssh-askpass-rs/releases/download/v0.1.6/ssh-askpass-rs-macos.tar.gz"
  sha256 "be6ba973ffedd51c942f66e34ca9c2c1a7646ae97448453feda93f47e59b25ef"
  license "GPL-3.0-only"
  version "0.1.6"

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
