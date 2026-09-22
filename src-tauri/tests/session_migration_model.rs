#![cfg(feature = "test-hooks")]

mod support;

use fyagent_lib::migration_test_hooks::{extract, identity, model, package};

use model::{
    ExtractionReport, MessageKind, MigratableMessage, MigratableSession, OmittedCounts,
    OriginIdentity, PathFamily, RestoreRequest, RestoreStage, SessionPackage, PACKAGE_SCHEMA,
};
use serde_json::{json, Value};

fn fixture_path(name: &str) -> String {
    format!(
        "{}/../tests/session-migration/fixtures/{name}",
        env!("CARGO_MANIFEST_DIR")
    )
}

fn extract_fixture(
    name: &str,
    detected_version: Option<&str>,
) -> Result<MigratableSession, model::MigrationError> {
    // Source identity checks resolve the configured Codex store. Keep those
    // settings and paths inside the shared test-home fixture on every platform.
    let _guard = support::test_mutex().lock().expect("test mutex poisoned");
    support::reset_test_fs();
    let state = tempfile::tempdir().expect("create isolated migration identity state");
    identity::with_local_state_dir(state.path(), || {
        extract::extract_session("codex", &fixture_path(name), detected_version)
    })
}

fn valid_package() -> Value {
    json!({
        "schema": PACKAGE_SCHEMA,
        "exportedAt": 1_795_478_400_000_i64,
        "exporter": {
            "app": "fyagent",
            "appVersion": "0.4.6",
            "platform": "macos"
        },
        "sessions": [{
            "snapshotId": format!("fys1:{}", "b".repeat(64)),
            "contentDigest": format!("fyc1:{}", "a".repeat(64)),
            "origin": {
                "originId": "11111111-1111-4111-8111-111111111111",
                "providerId": "codex",
                "sessionId": "synthetic-session",
                "cliVersion": "0.154.0",
                "storeFingerprint": "synthetic-store"
            },
            "workspaceLabel": "demo",
            "messages": [{
                "seq": 0,
                "kind": "userText",
                "text": "🪁\r\n/Users/alice/demo\r\nD:\\work\\demo"
            }, {
                "seq": 1,
                "kind": "assistantFinal",
                "text": "```ts\r\nconst answer = \"FINAL-T2-KEEP\";\r\n```"
            }],
            "extraction": {
                "ruleId": "codex.rollout.phase-v1",
                "ruleVerifiedVersions": ["=0.154.0"],
                "openUserMessages": [],
                "omitted": {
                    "toolEvents": 2,
                    "reasoningBlocks": 1,
                    "commentaryMessages": 1,
                    "attachments": 1,
                    "runtimeInjections": 2,
                    "unknownBlocks": 0
                },
                "sourcePathFamily": "posix"
            }
        }]
    })
}

fn valid_semantic_package() -> SessionPackage {
    let messages = vec![
        MigratableMessage {
            seq: 0,
            kind: MessageKind::UserText,
            text: "问题正文".to_owned(),
            ts: None,
        },
        MigratableMessage {
            seq: 1,
            kind: MessageKind::AssistantFinal,
            text: "最终答复".to_owned(),
            ts: None,
        },
    ];
    let origin = OriginIdentity {
        origin_id: "fyo1:synthetic-origin".to_owned(),
        provider_id: "codex".to_owned(),
        session_id: Some("synthetic-session".to_owned()),
        cli_version: Some("0.154.0".to_owned()),
        store_fingerprint: None,
    };
    let content_digest = identity::content_digest(&messages);
    let snapshot_id = identity::snapshot_id(&origin.origin_id, &content_digest);
    package::build_package(
        vec![MigratableSession {
            snapshot_id,
            content_digest,
            origin,
            title: None,
            created_at: None,
            last_active_at: None,
            workspace_label: Some("demo".to_owned()),
            messages,
            extraction: ExtractionReport {
                rule_id: "codex.rollout.phase-v1".to_owned(),
                rule_verified_versions: vec!["=0.154.0".to_owned()],
                open_user_messages: vec![],
                omitted: OmittedCounts::default(),
                source_path_family: PathFamily::Posix,
            },
        }],
        1_795_478_400_000,
    )
}

fn object_at_mut<'a>(
    value: &'a mut Value,
    pointer: &str,
) -> &'a mut serde_json::Map<String, Value> {
    value
        .pointer_mut(pointer)
        .and_then(Value::as_object_mut)
        .unwrap_or_else(|| panic!("fixture object missing at {pointer}"))
}

#[test]
fn session_migration_package_schema_rejects_unknown_fields_at_every_closed_level() {
    for pointer in [
        "",
        "/exporter",
        "/sessions/0",
        "/sessions/0/origin",
        "/sessions/0/messages/0",
        "/sessions/0/extraction",
        "/sessions/0/extraction/omitted",
    ] {
        let mut candidate = valid_package();
        object_at_mut(&mut candidate, pointer)
            .insert("reasoning".to_owned(), json!("LEAK_REASONING_94"));

        let error = serde_json::from_value::<SessionPackage>(candidate)
            .expect_err("unknown fields must fail closed");
        assert!(
            error.to_string().contains("reasoning"),
            "unknown field at {pointer} produced {error}"
        );
    }
}

#[test]
fn session_migration_package_schema_rejects_unknown_message_kinds() {
    let mut candidate = valid_package();
    candidate["sessions"][0]["messages"][1]["kind"] = json!("assistantCommentary");

    serde_json::from_value::<SessionPackage>(candidate)
        .expect_err("commentary must not be representable in a package");
}

#[test]
fn session_migration_message_roundtrip_preserves_crlf_paths_code_and_empty_final() {
    let messages = vec![
        MigratableMessage {
            seq: 0,
            kind: MessageKind::UserText,
            text: "🪁\r\n/Users/alice/demo\r\nD:\\work\\demo".to_owned(),
            ts: None,
        },
        MigratableMessage {
            seq: 1,
            kind: MessageKind::AssistantFinal,
            text: "```ts\r\nconst answer = \"FINAL-T2-KEEP\";\r\n```".to_owned(),
            ts: None,
        },
        MigratableMessage {
            seq: 2,
            kind: MessageKind::AssistantFinal,
            text: String::new(),
            ts: None,
        },
    ];

    let bytes = serde_json::to_vec(&messages).expect("serialize messages");
    let decoded: Vec<MigratableMessage> =
        serde_json::from_slice(&bytes).expect("deserialize messages");
    assert_eq!(decoded, messages);
    assert_eq!(decoded[2].text, "");
}

#[test]
fn session_migration_restore_request_has_no_overwrite_variant() {
    let base = json!({
        "packagePath": "/tmp/synthetic.fy-session.json",
        "requestId": "33333333-3333-4333-8333-333333333333",
        "snapshotIds": [],
        "targetProviderId": "codex",
        "targetWorkspace": "/tmp/fyagent-session-migration"
    });

    for supported in ["defaultImport", "saveAsNewCopy"] {
        let mut candidate = base.clone();
        candidate["requestKind"] = json!(supported);
        serde_json::from_value::<RestoreRequest>(candidate)
            .unwrap_or_else(|error| panic!("{supported} should parse: {error}"));
    }

    let mut overwrite = base;
    overwrite["requestKind"] = json!("overwrite");
    serde_json::from_value::<RestoreRequest>(overwrite)
        .expect_err("overwrite must never become a restore mode");
}

#[test]
fn session_migration_uncertain_outcomes_block_replay_until_positive_evidence() {
    for stage in [
        RestoreStage::NativeWritePending,
        RestoreStage::NeedsReconciliation,
        RestoreStage::Ambiguous,
    ] {
        assert!(stage.blocks_native_write(), "{stage:?} must block replay");
    }
    assert!(
        !RestoreStage::Failed.blocks_native_write(),
        "failed means the backend obtained positive no-effect evidence"
    );
}

#[test]
fn session_migration_parser_rejects_duplicate_keys_and_trailing_documents() {
    let raw = serde_json::to_string(&valid_semantic_package()).expect("serialize package");
    let duplicate = raw.replacen(
        "\"schema\":\"fyagent.session.v1\"",
        "\"schema\":\"fyagent.session.v1\",\"schema\":\"fyagent.session.v1\"",
        1,
    );
    let duplicate_error =
        package::parse_package(&duplicate).expect_err("duplicate keys must fail closed");
    assert_eq!(duplicate_error.code(), "packageMalformed");

    let trailing = format!("{raw}\n{{\"unexpected\":true}}");
    let trailing_error =
        package::parse_package(&trailing).expect_err("trailing document must fail closed");
    assert_eq!(trailing_error.code(), "packageMalformed");
}

#[test]
fn session_migration_parser_recomputes_digest_before_accepting_body() {
    let mut value = serde_json::to_value(valid_semantic_package()).expect("encode fixture");
    value["sessions"][0]["messages"][1]["text"] = json!("篡改后的答复");

    let error =
        package::parse_package(&value.to_string()).expect_err("digest mismatch must be rejected");
    assert_eq!(error.code(), "packageMalformed");
    assert!(error.to_string().contains("contentDigest"));
}

#[test]
fn session_migration_codex_extractor_keeps_only_verbatim_user_and_final_text() {
    let session = extract_fixture("codex-golden.jsonl", Some("0.154.0"))
        .expect("verified synthetic Codex fixture should extract");

    assert_eq!(session.messages.len(), 6);
    assert_eq!(
        session.messages[0].text,
        "合成档案：纸鹤的编号是 KITE-728。\n保持大小写。"
    );
    assert_eq!(
        session.messages[3].text,
        "路径 `/Users/alice/demo` 与 `D:\\work\\demo` 保持原文。\r\n```ts\r\nconst answer = \"FINAL-T2-KEEP\";\r\n```"
    );
    let exported_text = session
        .messages
        .iter()
        .map(|message| message.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for forbidden in [
        "LEAK_TOOL_ARG_91",
        "LEAK_TOOL_OUT_92",
        "LEAK_PROGRESS_93",
        "LEAK_REASONING_94",
        "LEAK_ATTACHMENT_95",
        "LEAK_FILE_96",
    ] {
        assert!(
            !exported_text.contains(forbidden),
            "{forbidden} escaped the final-only extractor"
        );
    }
    assert_eq!(session.extraction.omitted.tool_events, 2);
    assert_eq!(session.extraction.omitted.reasoning_blocks, 1);
    assert_eq!(session.extraction.omitted.commentary_messages, 1);
    assert_eq!(session.extraction.omitted.runtime_injections, 2);

    let expected_messages = session.messages.clone();
    let raw_package =
        serde_json::to_string(&package::build_package(vec![session], 1_795_478_400_000))
            .expect("serialize extracted package");
    let parsed = package::parse_package(&raw_package).expect("parse extracted package");
    assert_eq!(parsed.sessions[0].messages, expected_messages);
}

#[test]
fn session_migration_codex_extractor_fails_closed_for_unknown_final_or_version() {
    let unknown_final = extract_fixture("codex-final-indeterminate.jsonl", Some("0.154.0"))
        .expect_err("assistant text without phase must fail closed");
    assert_eq!(unknown_final.code(), "finalAnswerIndeterminate");

    // Source headers, not the currently installed CLI, certify the format.
    let source_version =
        extract_fixture("codex-golden.jsonl", None).expect("source header version is sufficient");
    assert_eq!(
        source_version.origin.cli_version.as_deref(),
        Some("0.154.0")
    );
    let fixture = std::fs::read_to_string(fixture_path("codex-golden.jsonl")).unwrap();
    let mut records: Vec<Value> = fixture
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    records[0]["payload"]
        .as_object_mut()
        .unwrap()
        .remove("cli_version");
    let state = tempfile::tempdir().unwrap();
    let source_path = state.path().join("unknown-source-version.jsonl");
    let text = records
        .iter()
        .map(Value::to_string)
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&source_path, text).unwrap();
    identity::with_local_state_dir(state.path(), || {
        let unknown_version =
            extract::extract_session("codex", &source_path.to_string_lossy(), Some("0.154.0"))
                .expect_err("an installed CLI cannot certify an unknown source header");
        assert_eq!(unknown_version.code(), "extractionRuleVersionMismatch");
    });
}

#[test]
fn session_migration_codex_extractor_preserves_consecutive_and_open_users() {
    let session = extract_fixture("codex-ordered-users.jsonl", Some("0.154.0"))
        .expect("ordered synthetic Codex fixture should extract");

    assert_eq!(
        session
            .messages
            .iter()
            .map(|message| (message.kind, message.text.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (MessageKind::UserText, "A：这一问被用户中断，没有答复。"),
            (MessageKind::UserText, "B：重新提问，只回答 B。"),
            (MessageKind::AssistantFinal, "B 的最终答复。"),
            (MessageKind::UserText, "C：尾部未完成输入。"),
        ]
    );
    assert_eq!(session.extraction.open_user_messages, vec![0, 3]);
}

#[test]
fn session_migration_same_body_from_different_origins_never_shares_snapshot() {
    let messages = valid_semantic_package().sessions.remove(0).messages;
    let digest = identity::content_digest(&messages);
    let source_a = identity::snapshot_id("fyo1:source-a", &digest);
    let source_b = identity::snapshot_id("fyo1:source-b", &digest);

    assert_ne!(source_a, source_b);
    assert_eq!(digest, identity::content_digest(&messages));
}
