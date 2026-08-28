use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Mutex, OnceLock};

use crate::shurufacli::config::Config;
use crate::shurufacli::db::Store;
use crate::shurufacli::llm::complete_turn;
use crate::shurufacli::paths::AppPaths;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Runtime};

const EVENT_NAME: &str = "shurufa://event";
const BUSY_ERROR: &str = "正在生成中，请稍后再试";
const EMPTY_TEXT_ERROR: &str = "请先在输入法页面填写文本";

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ShurufaEvent {
    Started,
    Delta { text: String },
    Finished { output: String },
    Error { message: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShurufaConfigSnapshot {
    pub url: String,
    pub model: String,
    pub api_key: String,
    pub max_summaries: usize,
    pub timeout_secs: u64,
    pub configured: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShurufaSnapshot {
    pub prompt: String,
    pub config: ShurufaConfigSnapshot,
    pub running: bool,
    pub last_output: String,
    pub last_error: Option<String>,
    pub shortcut_label: String,
    pub data_dir: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ShurufaConfigInput {
    pub url: String,
    pub model: String,
    pub api_key: String,
    pub max_summaries: usize,
    pub timeout_secs: u64,
}

#[derive(Serialize)]
struct ConfigFile {
    url: String,
    model: String,
    api_key: String,
    max_summaries: usize,
    timeout_secs: u64,
}

#[derive(Deserialize)]
struct RawConfigFile {
    #[serde(default)]
    url: String,
    #[serde(default)]
    model: String,
    #[serde(default)]
    api_key: String,
    #[serde(default)]
    max_summaries: Option<usize>,
    #[serde(default)]
    timeout_secs: Option<u64>,
}

struct ShurufaRuntime {
    prompt: Mutex<String>,
    last_output: Mutex<String>,
    last_error: Mutex<Option<String>>,
    running: AtomicBool,
}

impl Default for ShurufaRuntime {
    fn default() -> Self {
        Self {
            prompt: Mutex::new(String::new()),
            last_output: Mutex::new(String::new()),
            last_error: Mutex::new(None),
            running: AtomicBool::new(false),
        }
    }
}

fn runtime() -> &'static ShurufaRuntime {
    static RUNTIME: OnceLock<ShurufaRuntime> = OnceLock::new();
    RUNTIME.get_or_init(ShurufaRuntime::default)
}

fn data_dir() -> PathBuf {
    crate::config::get_app_config_dir().join("shurufacli")
}

fn paths() -> AppPaths {
    AppPaths::from_dir(data_dir())
}

fn shortcut_label() -> String {
    if cfg!(target_os = "macos") {
        "⌘M".to_string()
    } else {
        "Ctrl+M".to_string()
    }
}

fn load_prompt_file() -> String {
    fs::read_to_string(data_dir().join("prompt.txt")).unwrap_or_default()
}

fn persist_prompt(prompt: &str) -> Result<(), String> {
    let dir = data_dir();
    fs::create_dir_all(&dir).map_err(|error| format!("无法创建输入法目录: {error}"))?;
    fs::write(dir.join("prompt.txt"), prompt).map_err(|error| format!("无法保存输入文本: {error}"))
}

fn current_prompt() -> String {
    let stored = runtime()
        .prompt
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .clone();
    if stored.trim().is_empty() {
        load_prompt_file()
    } else {
        stored
    }
}

fn empty_config() -> ShurufaConfigSnapshot {
    ShurufaConfigSnapshot {
        url: "https://api.openai.com/v1".into(),
        model: "gpt-4o-mini".into(),
        api_key: String::new(),
        max_summaries: 8,
        timeout_secs: 60,
        configured: false,
    }
}

fn load_config_snapshot() -> ShurufaConfigSnapshot {
    let path = paths().config;
    let Ok(raw) = fs::read_to_string(&path) else {
        return empty_config();
    };
    let Ok(parsed) = toml::from_str::<RawConfigFile>(&raw) else {
        return empty_config();
    };
    ShurufaConfigSnapshot {
        url: if parsed.url.trim().is_empty() {
            "https://api.openai.com/v1".into()
        } else {
            parsed.url
        },
        model: if parsed.model.trim().is_empty() {
            "gpt-4o-mini".into()
        } else {
            parsed.model
        },
        api_key: parsed.api_key,
        max_summaries: parsed.max_summaries.unwrap_or(8).clamp(1, 32),
        timeout_secs: parsed.timeout_secs.filter(|value| *value > 0).unwrap_or(60),
        configured: Config::load(&path).is_ok(),
    }
}

fn emit_event<R: Runtime>(app: &AppHandle<R>, event: ShurufaEvent) {
    if let Err(error) = app.emit(EVENT_NAME, event) {
        log::warn!("failed to emit shurufa event: {error}");
    }
}

fn acquire_running() -> Result<(), String> {
    runtime()
        .running
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .map(|_| ())
        .map_err(|_| BUSY_ERROR.into())
}

fn start_typer() -> Option<mpsc::Sender<String>> {
    #[cfg(any(target_os = "macos", target_os = "windows"))]
    {
        use enigo::{Enigo, Keyboard, Settings};

        let (tx, rx) = mpsc::channel::<String>();
        std::thread::Builder::new()
            .name("shurufa-type".into())
            .spawn(move || {
                let mut enigo = match Enigo::new(&Settings::default()) {
                    Ok(enigo) => enigo,
                    Err(error) => {
                        log::warn!("shurufa enigo init failed: {error}");
                        while rx.recv().is_ok() {}
                        return;
                    }
                };
                while let Ok(delta) = rx.recv() {
                    if let Err(error) = enigo.text(&delta) {
                        log::warn!("shurufa type failed: {error}");
                    }
                }
            })
            .ok()?;
        Some(tx)
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        None
    }
}

pub async fn run_ingest<R: Runtime>(
    app: AppHandle<R>,
    type_into_focus: bool,
) -> Result<String, String> {
    run_ingest_text(app, current_prompt(), type_into_focus).await
}

pub async fn run_ingest_text<R: Runtime>(
    app: AppHandle<R>,
    text: String,
    type_into_focus: bool,
) -> Result<String, String> {
    let runtime = runtime();
    if let Err(message) = acquire_running() {
        return Err(message);
    }

    let prompt = text.trim().to_string();
    if prompt.is_empty() {
        runtime.running.store(false, Ordering::SeqCst);
        let message = EMPTY_TEXT_ERROR.to_string();
        *runtime
            .last_error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(message.clone());
        emit_event(
            &app,
            ShurufaEvent::Error {
                message: message.clone(),
            },
        );
        return Err(message);
    }

    *runtime
        .last_error
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    *runtime
        .last_output
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = String::new();
    emit_event(&app, ShurufaEvent::Started);

    let app_for_stream = app.clone();
    let typer = if type_into_focus { start_typer() } else { None };

    let result = async {
        let paths = paths();
        if !paths.config.is_file() {
            return Err(format!(
                "还没有可用的模型配置。请先在输入法页面填写 url / model / api_key 并保存。目录：{}",
                paths.dir.display()
            ));
        }
        let cfg = Config::load(&paths.config).map_err(|error| error.to_string())?;
        let history = {
            let store = Store::open(&paths.db).map_err(|error| error.to_string())?;
            store
                .recent_summaries(cfg.max_summaries)
                .map_err(|error| error.to_string())?
        };
        let turn = complete_turn(&cfg, &history, &prompt, |delta| {
            emit_event(
                &app_for_stream,
                ShurufaEvent::Delta {
                    text: delta.to_string(),
                },
            );
            if let Some(tx) = &typer {
                let _ = tx.send(delta.to_string());
            }
            Ok(())
        })
        .await
        .map_err(|error| format!("{error:#}"))?;
        {
            let store = Store::open(&paths.db).map_err(|error| error.to_string())?;
            store
                .append_turn(&prompt, &turn.summary)
                .map_err(|error| error.to_string())?;
        }
        Ok(turn)
    }
    .await;

    drop(typer);

    match result {
        Ok(turn) => {
            *runtime
                .last_output
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = turn.optimized_prompt.clone();
            runtime.running.store(false, Ordering::SeqCst);
            emit_event(
                &app,
                ShurufaEvent::Finished {
                    output: turn.optimized_prompt.clone(),
                },
            );
            Ok(turn.optimized_prompt)
        }
        Err(message) => {
            *runtime
                .last_error
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(message.clone());
            runtime.running.store(false, Ordering::SeqCst);
            emit_event(
                &app,
                ShurufaEvent::Error {
                    message: message.clone(),
                },
            );
            Err(message)
        }
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn ingest_shortcut() -> tauri_plugin_global_shortcut::Shortcut {
    use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

    if cfg!(target_os = "macos") {
        Shortcut::new(Some(Modifiers::META), Code::KeyM)
    } else {
        Shortcut::new(Some(Modifiers::CONTROL), Code::KeyM)
    }
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn global_shortcut_plugin<R: Runtime>() -> tauri::plugin::TauriPlugin<R> {
    use tauri_plugin_global_shortcut::ShortcutState;

    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            if *shortcut != ingest_shortcut() {
                return;
            }
            let handle = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(message) = run_ingest(handle, true).await {
                    log::warn!("shurufa hotkey ingest failed: {message}");
                }
            });
        })
        .build()
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
pub fn register_global_shortcut<R: Runtime>(app: &AppHandle<R>) {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    if let Err(error) = app.global_shortcut().register(ingest_shortcut()) {
        log::warn!("failed to register shurufa global shortcut: {error}");
    } else {
        log::info!("shurufa global shortcut registered: {}", shortcut_label());
    }
}

#[tauri::command]
pub fn shurufa_get_snapshot() -> ShurufaSnapshot {
    let runtime = runtime();
    let prompt = current_prompt();
    *runtime
        .prompt
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = prompt.clone();
    ShurufaSnapshot {
        prompt,
        config: load_config_snapshot(),
        running: runtime.running.load(Ordering::SeqCst),
        last_output: runtime
            .last_output
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone(),
        last_error: runtime
            .last_error
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone(),
        shortcut_label: shortcut_label(),
        data_dir: data_dir().display().to_string(),
    }
}

#[tauri::command]
pub fn shurufa_set_prompt(text: String) -> Result<(), String> {
    *runtime()
        .prompt
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = text.clone();
    persist_prompt(&text)
}

#[tauri::command]
pub fn shurufa_save_config(input: ShurufaConfigInput) -> Result<ShurufaConfigSnapshot, String> {
    let dir = data_dir();
    fs::create_dir_all(&dir).map_err(|error| format!("无法创建输入法目录: {error}"))?;
    let encoded = toml::to_string_pretty(&ConfigFile {
        url: input.url.trim().to_string(),
        model: input.model.trim().to_string(),
        api_key: input.api_key.trim().to_string(),
        max_summaries: input.max_summaries.clamp(1, 32),
        timeout_secs: if input.timeout_secs == 0 {
            60
        } else {
            input.timeout_secs
        },
    })
    .map_err(|error| format!("无法序列化配置: {error}"))?;
    let path = paths().config;
    fs::write(&path, encoded).map_err(|error| format!("无法写入配置: {error}"))?;
    Ok(load_config_snapshot())
}

#[tauri::command]
pub fn shurufa_clear_session() -> Result<usize, String> {
    let store = Store::open(&paths().db).map_err(|error| error.to_string())?;
    crate::shurufacli::clear_session(&store).map_err(|error| error.to_string())
}

#[tauri::command]
pub async fn shurufa_run(app: AppHandle) -> Result<String, String> {
    run_ingest(app, false).await
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reset_running() {
        runtime().running.store(false, Ordering::SeqCst);
    }

    #[test]
    fn first_admission_succeeds_second_returns_exact_busy_string() {
        reset_running();
        assert_eq!(acquire_running(), Ok(()));
        let second = std::thread::scope(|scope| scope.spawn(acquire_running).join().unwrap());
        assert_eq!(second, Err("正在生成中，请稍后再试".into()));
        reset_running();
        assert_eq!(acquire_running(), Ok(()));
        reset_running();
    }
}
