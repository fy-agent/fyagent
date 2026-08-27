pub mod config;
pub mod db;
pub mod llm;
pub mod paths;

use anyhow::Result;

use self::config::Config;
use self::db::Store;
use self::llm::{TurnResult, complete_turn};

pub async fn ingest<F>(
    text: &str,
    cfg: &Config,
    store: &Store,
    on_prompt_delta: F,
) -> Result<TurnResult>
where
    F: FnMut(&str) -> Result<()>,
{
    let history = store.recent_summaries(cfg.max_summaries)?;
    let result = complete_turn(cfg, &history, text, on_prompt_delta).await?;
    store.append_turn(text, &result.summary)?;
    Ok(result)
}

pub fn clear_session(store: &Store) -> Result<usize> {
    store.clear()
}
