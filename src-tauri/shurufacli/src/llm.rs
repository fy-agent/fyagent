use std::time::Duration;

use anyhow::{Context, Result, bail};
use async_openai::Client;
use async_openai::config::OpenAIConfig;
use async_openai::types::responses::{
    CreateResponseArgs, ResponseStreamEvent, ResponseTextDoneEvent,
};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::config::Config;

/// 模型只输出优化后的提示词正文。stdout 按 Responses SSE 的 output_text.delta 原样转发到文本框。
const SYSTEM_PROMPT: &str = r#"你是输入法上的 Vibe Coding 提示词优化器，不是编码 Agent。用户说完这句话后，原文会发给 Cursor 等编码 Agent，并由那边记录完整会话。你输出的内容会流式填进输入框，随后原样发出。

因此：你不是在给编码 Agent 补全历史，而是把「这一句口语」编译成一句可执行的当前指令。编码 Agent 自己有上下文；你只负责强调本轮真正相关的那一点。

输入材料：
- 历史输入：供你内部消歧。默认全部忽略。
- 本轮识别文本：语音转写，可能有口语、语气词、同音错字、缺少标点。这是唯一要优化的对象。

何时才允许用到历史（相关才写进输出）：
- 本轮有指代或省略，如「这个 / 那个 / 它 / 刚才那个 / 再 / 也 / 同样」，必须点明所指对象（控件、文件、功能），否则编码 Agent 会对错东西。
- 本轮明显延续上一件事（同一按钮、同一页、同一文件），只需带上那个对象名或一条硬约束。
- 历史里和本轮无关的轮次：当作不存在，禁止出现在输出里。

输出里禁止：
- 复述、罗列、摘要多轮历史（不要「此前已经…然后…现在…」）。
- 把会话纪要、背景故事、未提及的任务塞进提示词。
- 给编码 Agent 上课式的上下文补全；它不需要你替它回忆全文。
- JSON、Markdown 代码围栏、标签、前言、解释。

输出契约：
- 只输出本轮优化后的提示词正文。
- 以本轮意图为主体，短、具体、可执行。
- 相关对象用最少的词钉死（例如「登录按钮」），不要把历史整段搬进来。
- 改什么；改哪里（仅当本轮或相关指代已经明确）；不要动什么（仅当本轮说到）；怎样算完成（仅当本轮隐含）。
- 去掉「那个、然后、嗯、就是、你帮我」等语气词；纠正明显的语音识别错误。
- 不要猜测用户没说的技术栈、文件路径、架构或依赖。
- 不要写代码，不要展开到函数级实现步骤。
- 简单请求就一两句；不要为了显得完整而加无关背景。
- 用语跟本轮一致：用户说中文就输出中文。
"#;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TurnResult {
    pub summary: String,
    pub optimized_prompt: String,
}

pub async fn complete_turn<F>(
    cfg: &Config,
    history: &[String],
    user_text: &str,
    mut on_prompt_delta: F,
) -> Result<TurnResult>
where
    F: FnMut(&str) -> Result<()>,
{
    let openai = OpenAIConfig::new()
        .with_api_base(&cfg.url)
        .with_api_key(&cfg.api_key);
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(cfg.timeout_secs))
        .build()
        .context("创建 HTTP 客户端失败")?;
    let client = Client::with_config(openai).with_http_client(http);

    let request = CreateResponseArgs::default()
        .model(cfg.model.clone())
        .instructions(SYSTEM_PROMPT)
        .input(build_user_payload(history, user_text))
        .build()
        .context("构造 Responses 请求失败")?;

    let mut stream = client
        .responses()
        .create_stream(request)
        .await
        .context("调用 OpenAI Responses 流式接口失败")?;

    let mut prompt = String::new();
    let mut done_text = None;

    while let Some(event) = stream.next().await {
        match event.context("读取 Responses 流式事件失败")? {
            ResponseStreamEvent::ResponseOutputTextDelta(delta) => {
                if !delta.delta.is_empty() {
                    on_prompt_delta(&delta.delta)?;
                    prompt.push_str(&delta.delta);
                }
            }
            ResponseStreamEvent::ResponseOutputTextDone(ResponseTextDoneEvent { text, .. }) => {
                if !text.is_empty() {
                    done_text = Some(text);
                }
            }
            ResponseStreamEvent::ResponseFailed(failed) => {
                bail!("Responses API 失败：{}", format_response_error(&failed.response));
            }
            ResponseStreamEvent::ResponseError(err) => {
                bail!("Responses API 错误：{}", err.message);
            }
            ResponseStreamEvent::ResponseIncomplete(incomplete) => {
                bail!(
                    "Responses API 未完成：{}",
                    format_response_error(&incomplete.response)
                );
            }
            _ => {}
        }
    }

    let optimized_prompt = done_text
        .filter(|text| !text.trim().is_empty())
        .unwrap_or(prompt)
        .trim()
        .to_string();
    if optimized_prompt.is_empty() {
        bail!("模型没有输出优化提示词");
    }

    Ok(TurnResult {
        summary: user_text.trim().to_string(),
        optimized_prompt,
    })
}

pub fn build_user_payload(history: &[String], user_text: &str) -> String {
    let mut out = String::from(
        "只优化「本轮识别文本」。历史仅供消歧：无关轮次忽略；相关时只把所指对象写入输出，不要复述历史全文。只输出提示词正文。\n\n## 历史输入（内部消歧，默认忽略）\n",
    );
    if history.is_empty() {
        out.push_str("（无）\n");
    } else {
        for (i, item) in history.iter().enumerate() {
            out.push_str(&(i + 1).to_string());
            out.push_str(". ");
            out.push_str(item);
            out.push('\n');
        }
    }
    out.push_str("\n## 本轮识别文本\n");
    out.push_str(user_text.trim());
    out
}

fn format_response_error(response: &async_openai::types::responses::Response) -> String {
    response
        .error
        .as_ref()
        .map(|err| err.message.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "未知错误".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_marks_empty_history() {
        let text = build_user_payload(&[], "把登录按钮改成主色");
        assert!(text.contains("（无）"));
        assert!(text.contains("把登录按钮改成主色"));
    }

    #[test]
    fn payload_lists_history_in_order() {
        let text = build_user_payload(
            &["登录页改主色按钮".into(), "点击要有 loading".into()],
            "再把文案改成开始",
        );
        assert!(text.contains("1. 登录页改主色按钮"));
        assert!(text.contains("2. 点击要有 loading"));
        assert!(text.contains("再把文案改成开始"));
    }
}
