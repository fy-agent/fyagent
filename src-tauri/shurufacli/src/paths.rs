use std::env;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

pub struct AppPaths {
    pub dir: PathBuf,
    pub config: PathBuf,
    pub db: PathBuf,
    pub config_search: Vec<PathBuf>,
}

impl AppPaths {
    pub fn from_dir(dir: PathBuf) -> Self {
        let config = dir.join("config.toml");
        Self {
            db: dir.join("context.db"),
            config: config.clone(),
            config_search: vec![config],
            dir,
        }
    }

    pub fn resolve() -> Result<Self> {
        let exe = env::current_exe().context("无法解析当前二进制路径")?;
        let exe_dir = exe
            .parent()
            .map(Path::to_path_buf)
            .context("二进制路径没有父目录")?;
        let cwd = env::current_dir().context("无法解析当前工作目录")?;

        let mut config_search = Vec::new();
        push_unique(&mut config_search, exe_dir.join("config.toml"));
        push_unique(&mut config_search, cwd.join("config.toml"));
        if let Some(home) = env::var_os("HOME").or_else(|| env::var_os("USERPROFILE")) {
            push_unique(
                &mut config_search,
                PathBuf::from(home)
                    .join(".fyagent")
                    .join("shurufacli")
                    .join("config.toml"),
            );
        }

        let config = config_search
            .iter()
            .find(|path| path.is_file())
            .cloned()
            .unwrap_or_else(|| exe_dir.join("config.toml"));
        let dir = config.parent().map(Path::to_path_buf).unwrap_or(exe_dir);

        Ok(Self {
            db: dir.join("context.db"),
            config,
            dir,
            config_search,
        })
    }

    pub fn missing_config_message(&self) -> String {
        let listed = self
            .config_search
            .iter()
            .map(|path| format!("  {}", path.display()))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "找不到配置文件。已查找：\n{listed}\n请复制 config.toml.example 为 config.toml，填写 url、model、api_key。"
        )
    }
}

fn push_unique(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_dir_keeps_config_and_db_together() {
        let paths = AppPaths::from_dir(PathBuf::from("/tmp/agent"));
        assert_eq!(paths.config, PathBuf::from("/tmp/agent/config.toml"));
        assert_eq!(paths.db, PathBuf::from("/tmp/agent/context.db"));
        assert_eq!(paths.dir, PathBuf::from("/tmp/agent"));
    }

    #[test]
    fn db_sits_beside_config() {
        let paths = AppPaths {
            dir: PathBuf::from("/tmp/agent"),
            config: PathBuf::from("/tmp/agent/config.toml"),
            db: PathBuf::from("/tmp/agent/context.db"),
            config_search: vec![PathBuf::from("/tmp/agent/config.toml")],
        };
        assert_eq!(paths.dir.join("context.db"), paths.db);
        assert_eq!(paths.dir.join("config.toml"), paths.config);
    }

    #[test]
    fn missing_config_lists_search_paths() {
        let paths = AppPaths {
            dir: PathBuf::from("/tmp/agent"),
            config: PathBuf::from("/tmp/agent/config.toml"),
            db: PathBuf::from("/tmp/agent/context.db"),
            config_search: vec![
                PathBuf::from("/tmp/agent/config.toml"),
                PathBuf::from("/tmp/cwd/config.toml"),
            ],
        };
        let msg = paths.missing_config_message();
        assert!(msg.contains("/tmp/agent/config.toml"));
        assert!(msg.contains("/tmp/cwd/config.toml"));
    }
}
