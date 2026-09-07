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
    if matches!(source, "homebrew" | "bun" | "pnpm" | "volta" | "scoop") {
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
