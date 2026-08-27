use std::io::{self, IsTerminal, Read, Write};
use std::time::Instant;

use anyhow::{Result, bail};
use clap::{Parser, Subcommand};

use shurufacli::config::Config;
use shurufacli::db::Store;
use shurufacli::paths::AppPaths;
use shurufacli::{clear_session, ingest};

#[derive(Parser)]
#[command(
    name = "shurufacli",
    version,
    about = "Vibe coding 语音 Agent：输入本轮识别文本，流式输出可填进文本框的优化提示词",
    after_help = "典型用法：\n  shurufacli ingest \"把登录按钮改成主色，点了要有 loading\"\n  shurufacli clear\n\nstdout 只包含优化提示词，并随模型增量 flush，便于上位机写入文本框。\n配置与数据库默认放在二进制同目录：config.toml、context.db"
)]
#[command(arg_required_else_help = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 写入本轮识别文本，把优化提示词流式打印到 stdout
    Ingest {
        /// 本轮语音识别文本。省略时从 stdin 读取
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        text: Vec<String>,
        /// 等流结束后，stdout 输出 JSON（含 summary 与 optimized_prompt）
        #[arg(long, short)]
        json: bool,
        /// 向 stderr 打印 SQLite / 模型耗时
        #[arg(long, short)]
        verbose: bool,
    },
    /// 清空当前会话的摘要上下文
    Clear,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("{err:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    let paths = AppPaths::resolve()?;

    match cli.command {
        Commands::Ingest {
            text,
            json,
            verbose,
        } => {
            let user_text = resolve_text(text)?;
            if !paths.config.is_file() {
                bail!("{}", paths.missing_config_message());
            }
            let cfg = Config::load(&paths.config)?;

            let t0 = Instant::now();
            let store = Store::open(&paths.db)?;
            let sqlite_ms = t0.elapsed().as_secs_f64() * 1000.0;

            let t1 = Instant::now();
            let mut stdout = io::stdout();
            let result = ingest(&user_text, &cfg, &store, |delta| {
                if !json {
                    stdout.write_all(delta.as_bytes())?;
                    stdout.flush()?;
                }
                Ok(())
            })
            .await?;
            let llm_ms = t1.elapsed().as_secs_f64() * 1000.0;

            if verbose {
                eprintln!(
                    "sqlite {:.2}ms  llm {:.2}ms  db {}",
                    sqlite_ms,
                    llm_ms,
                    paths.db.display()
                );
            }

            if json {
                println!("{}", serde_json::to_string(&result)?);
            }
        }
        Commands::Clear => {
            let store = Store::open(&paths.db)?;
            let deleted = clear_session(&store)?;
            eprintln!("已清空 {deleted} 条会话摘要");
        }
    }
    Ok(())
}

fn resolve_text(parts: Vec<String>) -> Result<String> {
    let joined = parts.join(" ");
    let joined = joined.trim();
    if !joined.is_empty() {
        return Ok(joined.to_string());
    }

    if io::stdin().is_terminal() {
        bail!("请提供本轮识别文本，例如：shurufacli ingest \"把登录按钮改成主色\"");
    }

    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf)?;
    let text = buf.trim();
    if text.is_empty() {
        bail!("stdin 为空，请提供本轮识别文本");
    }
    Ok(text.to_string())
}
