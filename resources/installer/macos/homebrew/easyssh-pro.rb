class EasysshPro < Formula
  desc "Enterprise SSH client with team collaboration"
  homepage "https://github.com/anixops/easyssh"
  url "https://github.com/anixops/easyssh/releases/download/v0.3.0/easyssh-pro-0.3.0.tar.gz"
  sha256 "PLACEHOLDER_SHA256"
  license "MIT"

  depends_on :macos
  depends_on "rust" => :build
  depends_on "pkg-config" => :build

  # Runtime dependencies
  depends_on "sqlite"
  depends_on "openssl@3"

  def install
    system "cargo", "build", "--release", "--profile", "release-pro"
    bin.install "target/release-pro/easyssh-pro"

    # Install resources
    (share/"easyssh-pro").install "resources" if File.directory?("resources")

    # Install Pro server (optional local mode)
    (share/"easyssh-pro").install "target/release-pro/server" if File.directory?("target/release-pro/server")

    # Install .app bundle if available
    if File.directory?("target/release-pro/EasySSH Pro.app")
      prefix.install "target/release-pro/EasySSH Pro.app"
    end

    # Bash completion
    generate_completions_from_executable(bin/"easyssh-pro", "completions")
  end

  def caveats
    <<~EOS
      EasySSH Pro has been installed!

      To launch from terminal:
        easyssh-pro

      Enterprise Features:
        - Team management and RBAC
        - SSO integration (SAML/OIDC)
        - Audit logging
        - Shared snippets

      Pro Server (local mode):
        #{share}/easyssh-pro/server/easyssh-pro-server

      For documentation:
        https://docs.anixops.com/easyssh-pro
    EOS
  end

  service do
    run [opt_share/"easyssh-pro/server/easyssh-pro-server", "--config", etc/"easyssh-pro/server-config.yaml"]
    keep_alive true
    log_path var/"log/easyssh-pro-server.log"
    error_log_path var/"log/easyssh-pro-server-error.log"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/easyssh-pro --version")
  end
end
