# Code Buddy Homebrew Formula
#
# Add the tap:
#   brew tap simpletoolsindia/tap
#   brew install code-buddy
#
# Or install directly:
#   brew install simpletoolsindia/tap/code-buddy

class CodeBuddy < Formula
  desc "AI coding assistant for your terminal — Claude Code-style TUI with Ollama, LM Studio, OpenRouter and more"
  homepage "https://github.com/simpletoolsindia/code-buddy"
  url "https://github.com/simpletoolsindia/code-buddy/archive/553f97e29034b0f23ae369bcee07a080b7b52115.tar.gz"
  sha256 "22774b03b40e734c911c67fb1dedef7afaf914ae790ab1fd94425edda5deb09b"
  license "MIT"
  head "https://github.com/simpletoolsindia/code-buddy.git", branch: "main"

  bottle :unneeded

  depends_on "rust" => :build

  def install
    system "cargo", "install",
      "--path", "crates/cli",
      "--root", prefix,
      "--locked"
  end

  def caveats
    <<~EOS
      Run `code-buddy` to start. The setup wizard will guide you on first launch.

      Enable web search (optional):
        code-buddy config set brave_api_key YOUR_KEY

      Config file: ~/.config/code-buddy/config.toml
    EOS
  end

  test do
    assert_match "code-buddy", shell_output("#{bin}/code-buddy --version")
  end
end
