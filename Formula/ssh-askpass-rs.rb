class SshAskpassRs < Formula
  desc "macOS SSH askpass helper with native dialogs and Keychain integration"
  homepage "https://github.com/dislabled/ssh-askpass-rs"
  url "https://github.com/dislabled/ssh-askpass-rs/releases/download/v0.3.1/ssh-askpass-rs-macos.tar.gz"
  sha256 "f77746db9bed811eff25885a6c006181c1884ff53c531ba240967d1cb94a66ce"
  license "GPL-3.0-only"
  version "0.3.1"

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
