# Homebrew formula for code-buddy
#
# Usage:
#   brew tap simpletoolsindia/tap
#   brew install code-buddy
#
# To update after a new release, run:
#   brew upgrade code-buddy

class CodeBuddy < Formula
  desc "AI coding assistant for local and open-source LLMs — Claude Code-style CLI"
  homepage "https://github.com/simpletoolsindia/code-buddy"
  version "0.1.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/simpletoolsindia/code-buddy/releases/download/v#{version}/code-buddy-v#{version}-aarch64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256_AARCH64_DARWIN"
    end
    on_intel do
      url "https://github.com/simpletoolsindia/code-buddy/releases/download/v#{version}/code-buddy-v#{version}-x86_64-apple-darwin.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256_X86_64_DARWIN"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/simpletoolsindia/code-buddy/releases/download/v#{version}/code-buddy-v#{version}-aarch64-unknown-linux-musl.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256_AARCH64_LINUX"
    end
    on_intel do
      url "https://github.com/simpletoolsindia/code-buddy/releases/download/v#{version}/code-buddy-v#{version}-x86_64-unknown-linux-musl.tar.gz"
      sha256 "REPLACE_WITH_ACTUAL_SHA256_X86_64_LINUX"
    end
  end

  def install
    bin.install "code-buddy"
  end

  def caveats
    <<~EOS
      Run `code-buddy` to start the interactive session.
      On first launch, the setup wizard will help you configure your provider and model.

      Set a web search API key to enable the web_search tool:
        code-buddy config set brave_api_key YOUR_KEY

      Configuration is stored at:
        #{Dir.home}/.config/code-buddy/config.toml
    EOS
  end

  test do
    assert_match "code-buddy", shell_output("#{bin}/code-buddy --version")
  end
end
