//! These cases need the crate-private switch lock and SQLite connection:
//! integration tests cannot pause between accepted imports and projection.

use super::*;
use crate::database::Database;
use rusqlite::hooks::{AuthAction, AuthContext, Authorization};
use serde_json::json;
use std::ffi::OsString;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

struct HomeGuard {
    previous_home: Option<OsString>,
    previous_settings: crate::settings::AppSettings,
}

impl HomeGuard {
    fn new(home: &Path) -> Self {
        let guard = Self {
            previous_home: std::env::var_os("FYAGENT_TEST_HOME"),
            previous_settings: crate::settings::get_settings(),
        };
        std::env::set_var("FYAGENT_TEST_HOME", home);
        // Cached Codex directory overrides from another unit test must not
        // redirect this fixture outside its temporary home.
        crate::settings::replace_settings_in_memory_for_test(Default::default());
        guard
    }
}

impl Drop for HomeGuard {
    fn drop(&mut self) {
        crate::settings::replace_settings_in_memory_for_test(self.previous_settings.clone());
        match self.previous_home.take() {
            Some(value) => std::env::set_var("FYAGENT_TEST_HOME", value),
            None => std::env::remove_var("FYAGENT_TEST_HOME"),
        }
    }
}

fn seed_sources(home: &Path) -> Vec<u8> {
    let qoder = home.join(".qoderworkcn/mcp.json");
    fs::create_dir_all(qoder.parent().unwrap()).unwrap();
    let original = serde_json::to_vec(&json!({"mcpServers": {
        "first": {"command":"echo"},
        "second": {"command":"echo", "env":{"TOKEN":"private-sentinel"}}
    }}))
    .unwrap();
    fs::write(qoder, &original).unwrap();
    let codex = crate::codex_config::get_codex_config_path();
    fs::create_dir_all(codex.parent().unwrap()).unwrap();
    // Import accepts this legacy section; projection migrates it. Checking
    // the canonical section later proves the next target actually ran.
    fs::write(codex, "[mcp.servers.codex_good]\ncommand = \"echo\"\n").unwrap();
    original
}

fn wait_for_committed_server(state: &AppState, id: &str) {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        if state.db.get_all_mcp_servers().unwrap().contains_key(id) {
            return;
        }
        assert!(Instant::now() < deadline, "source did not commit: {id}");
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn import_with_paused_projection(
    state: &AppState,
    before_projection: impl FnOnce(),
) -> McpImportReport {
    std::thread::scope(|scope| {
        // Block the second import until the first has committed and we own
        // its projection lock. The guards live inside the scope so a failed
        // wait releases them before scope waits for the worker to terminate.
        let codex_guard =
            futures::executor::block_on(state.proxy_service.lock_switch_for_app("codex"));
        let worker = scope.spawn(|| {
            McpService::import_from_sources(state, vec![McpTargetId::QoderWork, McpTargetId::Codex])
        });
        wait_for_committed_server(state, "second");
        let qoder_guard =
            futures::executor::block_on(state.proxy_service.lock_switch_for_app("qoderwork"));
        drop(codex_guard);
        wait_for_committed_server(state, "codex_good");
        // Both source transactions are committed; neither target has projected.
        // No fixed sleep determines when the failure is injected.
        before_projection();
        drop(qoder_guard);
        worker
            .join()
            .unwrap()
            .expect("accepted imports return a report")
    })
}

fn assert_accepted_sources_and_codex_projection(
    state: &AppState,
    home: &Path,
    report: &McpImportReport,
) {
    assert_eq!(report.sources.len(), 2);
    for (source, target, added) in [
        (&report.sources[0], McpTargetId::QoderWork, 2),
        (&report.sources[1], McpTargetId::Codex, 1),
    ] {
        assert_eq!(source.source, target);
        assert!(source.failure_code.is_none());
        assert_eq!(
            source.counts,
            McpImportCounts {
                added,
                ..Default::default()
            }
        );
    }
    let durable = state.db.get_all_mcp_servers().unwrap();
    assert_eq!(durable.len(), 3);
    assert!(durable["first"].apps.qoderwork);
    assert!(durable["second"].apps.qoderwork);
    assert!(durable["codex_good"].apps.codex);
    assert_eq!(durable["second"].server["env"]["TOKEN"], "private-sentinel");
    let views = McpService::get_server_views(state).unwrap();
    assert_eq!(views["first"].sources, vec![McpTargetId::QoderWork]);
    assert_eq!(views["second"].sources, vec![McpTargetId::QoderWork]);
    assert_eq!(views["codex_good"].sources, vec![McpTargetId::Codex]);
    let live: toml::Value =
        toml::from_str(&fs::read_to_string(crate::codex_config::get_codex_config_path()).unwrap())
            .unwrap();
    assert_eq!(
        live["mcp_servers"]["codex_good"]["command"].as_str(),
        Some("echo")
    );
    assert!(live["mcp_servers"].get("first").is_none());
    assert!(live["mcp_servers"].get("second").is_none());
    assert!(live.get("mcp").and_then(|mcp| mcp.get("servers")).is_none());
    let wire = serde_json::to_value(report).unwrap();
    assert_eq!(wire["projectionFailed"], report.projection_failures.len());
    assert_eq!(
        wire["projectionFailures"].as_array().unwrap().len(),
        report.projection_failed
    );
    let public = wire.to_string();
    for private in [
        "private-sentinel",
        "broken-target-secret",
        home.to_str().unwrap(),
    ] {
        assert!(!public.contains(private));
    }
    for path in [
        home.to_path_buf(),
        home.join(".qoderworkcn/mcp.json"),
        crate::codex_config::get_codex_config_path(),
    ] {
        // Compare JSON-escaped paths as well as raw paths on Windows.
        let escaped = serde_json::to_string(&path.to_string_lossy()).unwrap();
        assert!(!public.contains(&escaped[1..escaped.len() - 1]));
    }
}

#[test]
#[serial_test::serial]
fn import_reports_projection_failed_after_catalogue_read_error_and_continues() {
    let home = tempfile::tempdir().unwrap();
    let _guard = HomeGuard::new(home.path());
    let state = AppState::new(Arc::new(Database::memory().unwrap()));
    let original = seed_sources(home.path());
    let denied = Arc::new(AtomicBool::new(false));
    let hook_denied = Arc::clone(&denied);
    let report = import_with_paused_projection(&state, || {
        // The current per-entry adapters have no stable external fixture for
        // the default reason: validation/parse/I/O failures map to the other
        // two reasons. Exercise a real Database error from the post-import
        // catalogue read instead; this correctly reports a null server ID,
        // not a fabricated per-entry failure. Deny only the first read so the
        // next target must still load the catalogue and project successfully.
        state.db.conn.lock().unwrap().authorizer(Some(
            move |context: AuthContext<'_>| match context.action {
                AuthAction::Read {
                    table_name: "mcp_servers",
                    ..
                } if !hook_denied.swap(true, Ordering::SeqCst) => Authorization::Deny,
                _ => Authorization::Allow,
            },
        ));
    });
    state
        .db
        .conn
        .lock()
        .unwrap()
        .authorizer(None::<fn(AuthContext<'_>) -> Authorization>);
    assert!(denied.load(Ordering::SeqCst));
    assert_eq!(report.projection_failed, 1);
    assert_eq!(report.projection_failures.len(), 1);
    let failure = &report.projection_failures[0];
    assert_eq!(failure.target, McpTargetId::QoderWork);
    assert_eq!(failure.reason, "projection_failed");
    assert!(failure.server_id.is_none());
    let wire = serde_json::to_value(&report).unwrap();
    assert_eq!(wire["projectionFailures"][0]["target"], "qoderwork");
    assert_eq!(wire["projectionFailures"][0]["reason"], "projection_failed");
    assert!(wire["projectionFailures"][0]["serverId"].is_null());
    assert_eq!(
        fs::read(home.path().join(".qoderworkcn/mcp.json")).unwrap(),
        original
    );
    assert_accepted_sources_and_codex_projection(&state, home.path(), &report);
}

#[test]
#[serial_test::serial]
fn import_reports_target_read_failure_after_commit_without_rollback_and_continues() {
    let home = tempfile::tempdir().unwrap();
    let _guard = HomeGuard::new(home.path());
    let state = AppState::new(Arc::new(Database::memory().unwrap()));
    seed_sources(home.path());
    let path = home.path().join(".qoderworkcn/mcp.json");
    let broken = b"{broken-target-secret";
    let report = import_with_paused_projection(&state, || {
        // Corrupt only after both imports commit. A pre-import corrupt file
        // would exercise source_failed instead. Malformed JSON yields
        // AppError::Json -> invalid_config on Windows and macOS,
        // without permission assumptions or scheduler-dependent file races.
        fs::write(&path, broken).unwrap();
    });
    assert_eq!(report.projection_failed, 3);
    assert_eq!(report.projection_failures.len(), 3);
    let mut ids = report
        .projection_failures
        .iter()
        .map(|failure| {
            assert_eq!(failure.target, McpTargetId::QoderWork);
            assert_eq!(failure.reason, "invalid_config");
            failure.server_id.as_deref().unwrap()
        })
        .collect::<Vec<_>>();
    ids.sort_unstable();
    // Even the disabled entry's removal is attempted after earlier failures.
    assert_eq!(ids, vec!["codex_good", "first", "second"]);
    let wire = serde_json::to_value(&report).unwrap();
    for failure in wire["projectionFailures"].as_array().unwrap() {
        assert_eq!(failure["target"], "qoderwork");
        assert_eq!(failure["reason"], "invalid_config");
    }
    assert_eq!(fs::read(&path).unwrap(), broken);
    assert_accepted_sources_and_codex_projection(&state, home.path(), &report);
}
