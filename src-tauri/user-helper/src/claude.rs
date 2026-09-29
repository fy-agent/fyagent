//! Closed Claude CLI distribution facts shared by the host and ordinary-user helper.

pub const CLAUDE_NPM_PACKAGE: &str = "@anthropic-ai/claude-code";
pub const CLAUDE_MIN_NODE_MAJOR: u32 = 22;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClaudeOwner {
    Npm,
    Native,
    External,
}

pub fn installation_owner(path: &str, real: &str, source: &str) -> ClaudeOwner {
    let real = real.replace('\\', "/").to_ascii_lowercase();
    let path = path.replace('\\', "/").to_ascii_lowercase();
    if source == "homebrew" {
        // Tooling labels /opt/homebrew/bin by location, even when npm owns
        // the resolved executable. Require the same global npm prefix;
        // actual Homebrew Cellar/Caskroom installations stay external.
        let npm_in_brew_prefix = path.strip_suffix("/bin/claude").is_some_and(|prefix| {
            !prefix.is_empty()
                && !real.contains("/cellar/")
                && !real.contains("/caskroom/")
                && real.strip_prefix(prefix).is_some_and(|tail| {
                    tail.starts_with("/lib/node_modules/@anthropic-ai/claude-code/")
                })
        });
        return if npm_in_brew_prefix {
            ClaudeOwner::Npm
        } else {
            ClaudeOwner::External
        };
    }
    if matches!(source, "bun" | "pnpm" | "volta" | "scoop") {
        return ClaudeOwner::External;
    }
    if real.contains("/.local/share/claude/versions/")
        || real.contains("/claude/versions/")
        || real.ends_with("/.local/bin/claude.exe")
    {
        return ClaudeOwner::Native;
    }
    if real.contains("/node_modules/@anthropic-ai/claude-code/")
        || path.contains("/node_modules/@anthropic-ai/claude-code/")
    {
        ClaudeOwner::Npm
    } else {
        ClaudeOwner::External
    }
}

pub const fn windows_executable_names() -> &'static [&'static str] {
    &["claude.cmd", "claude.exe", "claude"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn homebrew_bin_can_resolve_to_same_prefix_official_npm_package() {
        for entry in ["bin/claude.exe", "cli.js"] {
            assert_eq!(
                installation_owner(
                    "/opt/homebrew/bin/claude",
                    &format!("/opt/homebrew/lib/node_modules/@anthropic-ai/claude-code/{entry}"),
                    "homebrew"
                ),
                ClaudeOwner::Npm
            );
        }
    }

    #[test]
    fn homebrew_label_does_not_admit_other_owners_or_prefixes() {
        for real in [
            "/opt/homebrew/Cellar/claude-code/2.1/bin/claude",
            "/opt/homebrew/Caskroom/claude-code/2.1/claude",
            "/opt/homebrew/Cellar/claude-code/2.1/lib/node_modules/@anthropic-ai/claude-code/cli.js",
            "/elsewhere/lib/node_modules/@anthropic-ai/claude-code/bin/claude.exe",
            "/opt/homebrew-other/lib/node_modules/@anthropic-ai/claude-code/cli.js",
            "/opt/homebrew/lib/node_modules/@anthropic-ai/claude-code-other/cli.js",
            "/Users/a/.local/share/claude/versions/2.1.280",
        ] {
            assert_eq!(
                installation_owner("/opt/homebrew/bin/claude", real, "homebrew"),
                ClaudeOwner::External,
                "{real}"
            );
        }
        for source in ["bun", "pnpm", "volta", "scoop"] {
            assert_eq!(
                installation_owner(
                    "/opt/homebrew/bin/claude",
                    "/opt/homebrew/lib/node_modules/@anthropic-ai/claude-code/bin/claude.exe",
                    source
                ),
                ClaudeOwner::External,
                "{source}"
            );
        }
    }

    #[test]
    fn windows_native_npm_and_other_managers_remain_distinct() {
        assert_eq!(
            installation_owner(
                r"C:\Users\a\.local\bin\claude.exe",
                r"C:\Users\a\.local\bin\claude.exe",
                "system"
            ),
            ClaudeOwner::Native
        );
        assert_eq!(
            installation_owner(
                r"C:\Users\a\npm\claude.cmd",
                r"C:\Users\a\npm\node_modules\@anthropic-ai\claude-code\bin\claude.exe",
                "system"
            ),
            ClaudeOwner::Npm
        );
        assert_eq!(
            installation_owner(
                "/user/volta/bin/claude",
                "/user/volta/node_modules/@anthropic-ai/claude-code/bin/claude",
                "volta"
            ),
            ClaudeOwner::External
        );
        assert_eq!(
            installation_owner("/tmp/claude", "/tmp/claude", "system"),
            ClaudeOwner::External
        );
    }
}
