#![allow(non_snake_case)]

use tauri::ipc::{Channel, Response};
use tauri_plugin_dialog::DialogExt;

use crate::session_manager;
use crate::session_manager::cache::chunk_ranges;
use crate::session_manager::content::{self, BlockContent};
use crate::session_manager::model::{ContentRef, ImageRef, TranscriptChunk};

#[tauri::command]
pub async fn list_sessions() -> Result<Vec<session_manager::SessionMeta>, String> {
    let sessions = tauri::async_runtime::spawn_blocking(session_manager::scan_sessions)
        .await
        .map_err(|e| format!("Failed to scan sessions: {e}"))?;
    Ok(sessions)
}

/// 一次性返回全部消息（兼容旧前端；「复制整段」等一次性场景也用它）。走解析缓存。
#[tauri::command]
pub async fn get_session_messages(
    providerId: String,
    sourcePath: String,
) -> Result<Vec<session_manager::SessionMessage>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        session_manager::load_transcript(&providerId, &sourcePath)
            .map(|loaded| loaded.transcript.messages.clone())
    })
    .await
    .map_err(|e| format!("Failed to load session messages: {e}"))?
}

/// 分块流式读取会话（§5.1，决策 D1）：依次发送 `Header` → 若干 `Messages` → `Done`。
///
/// 读取失败时先发一个 `Error` 包，再以同样的文案返回 `Err`，前端任选一处处理。
#[tauri::command]
pub async fn stream_session_messages(
    providerId: String,
    sourcePath: String,
    onChunk: Channel<TranscriptChunk>,
) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let loaded = match session_manager::load_transcript(&providerId, &sourcePath) {
            Ok(loaded) => loaded,
            Err(message) => {
                let _ = onChunk.send(TranscriptChunk::Error {
                    message: message.clone(),
                });
                return Err(message);
            }
        };
        let transcript = loaded.transcript;
        let send = |chunk: TranscriptChunk| {
            onChunk
                .send(chunk)
                .map_err(|e| format!("Failed to send session chunk: {e}"))
        };

        send(TranscriptChunk::Header {
            total: transcript.messages.len(),
            turns: transcript.turns.clone(),
            cached: loaded.cached,
            parse_ms: loaded.parse_ms,
        })?;
        for (start, end) in chunk_ranges(&transcript.message_bytes) {
            send(TranscriptChunk::Messages {
                start,
                messages: transcript.messages[start..end].to_vec(),
            })?;
        }
        send(TranscriptChunk::Done {
            payload_bytes: transcript.approx_bytes as u64,
        })
    })
    .await
    .map_err(|e| format!("Failed to stream session messages: {e}"))?
}

/// 按 [`ContentRef`] 取工具输出 / 参数 / 思考 / diff 全文，按字符分页（默认且最多 512K 字符一页）。
#[tauri::command]
pub async fn get_session_block_content(
    providerId: String,
    sourcePath: String,
    contentRef: ContentRef,
    offset: Option<u32>,
    limit: Option<u32>,
) -> Result<BlockContent, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let source = content::validate_source(&providerId, &sourcePath)?;
        let full = content::resolve_content_ref(&source, &contentRef)?;
        Ok(content::paginate(&full, offset, limit))
    })
    .await
    .map_err(|e| format!("Failed to load block content: {e}"))?
}

/// 读取会话图片的原始字节（决策 D2）；前端按 `ImageRef.mediaType` 生成 Blob URL。
#[tauri::command]
pub async fn get_session_image(
    providerId: String,
    sourcePath: String,
    image: ImageRef,
) -> Result<Response, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let source = content::validate_source(&providerId, &sourcePath)?;
        content::load_image(&source, &image).map(Response::new)
    })
    .await
    .map_err(|e| format!("Failed to load session image: {e}"))?
}

/// 在文件管理器中显示某个已存在的文件或目录（决策 D3）；不提供「用默认程序打开」。
#[tauri::command]
pub async fn reveal_session_path(path: String) -> Result<bool, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let path = content::resolve_reveal_path(&path)?;
        tauri_plugin_opener::reveal_item_in_dir(&path)
            .map_err(|_| "无法在文件管理器中显示该路径".to_string())?;
        Ok(true)
    })
    .await
    .map_err(|e| format!("Failed to reveal path: {e}"))?
}

/// 在用户选定的终端里恢复会话；终端模块当前仅支持 macOS。
///
/// `command` 按来自可信渲染代码的命令文本交给 shell，本命令不校验其命令语义。
/// `cwd` 由终端模块转义或作为独立参数传递；`custom_config` 是自定义启动模板，
/// 其引用上下文不能仅靠 `cwd` 转义保证安全。上述处理不使外部输入自动可信。
/// 不可信输入触达命令执行的报告范围见仓库根目录 `SECURITY.md`。
#[tauri::command]
pub async fn launch_session_terminal(
    command: String,
    cwd: Option<String>,
    custom_config: Option<String>,
) -> Result<bool, String> {
    let command = command.clone();
    let cwd = cwd.clone();
    let custom_config = custom_config.clone();

    let target = crate::settings::get_preferred_terminal()
        .ok_or_else(|| "Terminal resume is not supported on this platform".to_string())?;

    tauri::async_runtime::spawn_blocking(move || {
        session_manager::terminal::launch_terminal(
            &target,
            &command,
            cwd.as_deref(),
            custom_config.as_deref(),
        )
    })
    .await
    .map_err(|e| format!("Failed to launch terminal: {e}"))??;

    Ok(true)
}

#[tauri::command]
pub async fn delete_session(
    providerId: String,
    sessionId: String,
    sourcePath: String,
) -> Result<bool, String> {
    let provider_id = providerId.clone();
    let session_id = sessionId.clone();
    let source_path = sourcePath.clone();

    tauri::async_runtime::spawn_blocking(move || {
        session_manager::delete_session(&provider_id, &session_id, &source_path)
    })
    .await
    .map_err(|e| format!("Failed to delete session: {e}"))?
}

#[tauri::command]
pub async fn delete_sessions(
    items: Vec<session_manager::DeleteSessionRequest>,
) -> Result<Vec<session_manager::DeleteSessionOutcome>, String> {
    tauri::async_runtime::spawn_blocking(move || session_manager::delete_sessions(&items))
        .await
        .map_err(|e| format!("Failed to delete sessions: {e}"))
}

/// 把会话导出为 Markdown 文件：由后端弹出保存对话框并写入。写入路径只能是用户在
/// 对话框里选的位置，前端不能指定任意路径。用户取消时返回 `None`，否则返回保存路径。
#[tauri::command]
pub async fn export_session_markdown<R: tauri::Runtime>(
    app: tauri::AppHandle<R>,
    #[allow(non_snake_case)] defaultName: String,
    content: String,
) -> Result<Option<String>, String> {
    let picked = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("Markdown", &["md"])
            .set_file_name(&defaultName)
            .blocking_save_file()
    })
    .await
    .map_err(|e| format!("打开保存对话框失败: {e}"))?;

    let Some(picked) = picked else {
        return Ok(None);
    };
    let path = picked
        .into_path()
        .map_err(|e| format!("无效的保存路径: {e}"))?;
    crate::config::write_text_file(&path, &content).map_err(|e| e.to_string())?;
    Ok(Some(path.to_string_lossy().into_owned()))
}
