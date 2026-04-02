class EasysshStandard < Formula
  desc "Full-featured SSH client with embedded terminal"
  homepage "https://github.com/anixops/easyssh"
  url "https://github.com/anixops/easyssh/releases/download/v0.3.0/easyssh-standard-0.3.0.tar.gz"
  sha256 "PLACEHOLDER_SHA256"
  license "MIT"

  depends_on :macos
  depends_on "rust" => :build
  depends_on "pkg-config" => :build

  # Runtime dependencies
  depends_on "sqlite"

  def install
    system "cargo", "build", "--release", "--profile", "release-standard"
    bin.install "target/release-standard/easyssh-standard"

    # Install resources
    (share/"easyssh-standard").install "resources" if File.directory?("resources")

    # Install .app bundle if available
    if File.directory?("target/release-standard/EasySSH Standard.app")
      prefix.install "target/release-standard/EasySSH Standard.app"
    end

    # Bash completion
    generate_completions_from_executable(bin/"easyssh-standard", "completions")
  end

  def caveats
    <<~EOS
      EasySSH Standard has been installed!

      To launch from terminal:
        easyssh-standard

      Features:
        - Embedded WebGL-accelerated terminal
        - Multi-tab support
        - Split-screen layout
        - SFTP file manager
    EOS
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/easyssh-standard --version")
  end
end
