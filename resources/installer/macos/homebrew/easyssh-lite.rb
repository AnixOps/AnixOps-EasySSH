class EasysshLite < Formula
  desc "Minimal SSH configuration vault with native terminal launcher"
  homepage "https://github.com/anixops/easyssh"
  url "https://github.com/anixops/easyssh/releases/download/v0.3.0/easyssh-lite-0.3.0.tar.gz"
  sha256 "PLACEHOLDER_SHA256"
  license "MIT"

  # macOS only (Linux handled by .deb/.rpm)
  depends_on :macos
  depends_on "rust" => :build
  depends_on "pkg-config" => :build

  # GTK4 is not readily available on macOS, bundle or require
  # For now, build from source

  def install
    system "cargo", "build", "--release", "--profile", "release-lite"
    bin.install "target/release-lite/easyssh-lite"

    # Install resources
    (share/"easyssh-lite").install "resources" if File.directory?("resources")

    # Install .app bundle if available
    if File.directory?("target/release-lite/EasySSH Lite.app")
      prefix.install "target/release-lite/EasySSH Lite.app"
    end

    # Bash completion
    generate_completions_from_executable(bin/"easyssh-lite", "completions")
  end

  def caveats
    <<~EOS
      EasySSH Lite has been installed!

      To launch from terminal:
        easyssh-lite

      Or find it in your Applications folder.

      First run setup:
        1. Launch EasySSH Lite
        2. Set up your master password
        3. Add your SSH servers
    EOS
  end

  test do
    # Test that binary runs and shows version
    assert_match version.to_s, shell_output("#{bin}/easyssh-lite --version")
  end
end
