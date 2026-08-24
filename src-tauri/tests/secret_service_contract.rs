#[path = "../src/services/secret/mod.rs"]
mod secret;

use std::collections::HashSet;

use secret::{
    MaterialMatches, MemoryFailureMode, MemorySecretBackend, NativeSecretBackend,
    SecretAvailability, SecretBackend, SecretErrorCode, SecretMaterial, SecretPresence,
    SecretPurpose, SecretRef, SecretService,
};
use serde_json::Value;
use uuid::Uuid;

fn material(value: &str) -> SecretMaterial {
    SecretMaterial::from_native_input(value.as_bytes().to_vec(), SecretPurpose::CodexApiKey)
        .expect("valid test material")
}

fn normalized_key(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn collect_keys(value: &Value, keys: &mut Vec<String>) {
    match value {
        Value::Object(object) => {
            for (key, value) in object {
                keys.push(normalized_key(key));
                collect_keys(value, keys);
            }
        }
        Value::Array(values) => {
            for value in values {
                collect_keys(value, keys);
            }
        }
        _ => {}
    }
}

#[test]
fn generated_refs_are_unique_strict_uuid_v4_identities() {
    let mut refs = HashSet::new();
    for _ in 0..10_000 {
        let secret_ref = SecretRef::generate();
        let value = secret_ref.as_str();
        assert_eq!(value.len(), 36);
        assert!(value.starts_with("sec_"));
        assert_eq!(&value[16..17], "4");
        assert!(matches!(&value[20..21], "8" | "9" | "a" | "b"));
        assert!(refs.insert(value.to_owned()));
        assert_eq!(
            SecretRef::parse(value).expect("parse generated"),
            secret_ref
        );
    }

    for invalid in [
        "sec_",
        "sec_00000000000000000000000000000000",
        "sec_00000000000040007000000000000000",
        "sec_0000000000004000800000000000000G",
        "SEC_00000000000040008000000000000000",
        "provider_codex_primary",
    ] {
        assert!(SecretRef::parse(invalid).is_err(), "accepted {invalid}");
    }
}

#[test]
fn memory_backend_covers_crud_readback_and_version_rotation() {
    let backend = MemorySecretBackend::new();
    let service = SecretService::new(backend.clone());
    let canary_v1 = format!("fyagent-contract-canary-v1-{}", Uuid::new_v4());
    let canary_v2 = format!("fyagent-contract-canary-v2-{}", Uuid::new_v4());

    let created = service
        .create(material(&canary_v1), SecretPurpose::CodexApiKey)
        .expect("create");
    let first = created.handle();
    assert!(backend.contains(first.secret_ref()));
    assert_eq!(created.presence(), SecretPresence::Present);
    assert_eq!(created.availability(), SecretAvailability::Ready);
    assert!(service
        .with_material(&first, MaterialMatches::new(canary_v1.as_bytes()))
        .expect("read"));

    let replaced = service
        .replace(&first, material(&canary_v2), SecretPurpose::CodexApiKey)
        .expect("replace");
    let second = replaced.handle();
    assert_eq!(first.secret_ref(), second.secret_ref());
    assert_ne!(first.version().as_str(), second.version().as_str());
    assert!(service
        .with_material(&second, MaterialMatches::new(canary_v2.as_bytes()))
        .expect("read replacement"));

    let probed = service
        .probe(&second, SecretPurpose::CodexApiKey)
        .expect("probe");
    assert_eq!(probed.presence(), SecretPresence::Present);
    assert_eq!(probed.availability(), SecretAvailability::Ready);

    service.delete(&second).expect("delete");
    assert!(!backend.contains(second.secret_ref()));
    let missing = service
        .probe(&second, SecretPurpose::CodexApiKey)
        .expect("missing summary");
    assert_eq!(missing.presence(), SecretPresence::Missing);
    assert_eq!(missing.availability(), SecretAvailability::Missing);
    assert_eq!(
        service
            .with_material(&second, MaterialMatches::new(canary_v2.as_bytes()))
            .expect_err("missing read")
            .code(),
        SecretErrorCode::Missing
    );
}

#[test]
fn create_is_not_upsert_and_replace_requires_an_existing_record() {
    let backend = MemorySecretBackend::new();
    let secret_ref = SecretRef::generate();
    backend
        .create_new(&secret_ref, &material("first-runtime-canary"))
        .expect("first create");
    assert_eq!(
        backend
            .create_new(&secret_ref, &material("second-runtime-canary"))
            .expect_err("duplicate create")
            .code(),
        SecretErrorCode::AlreadyExists
    );

    let missing_ref = SecretRef::generate();
    assert_eq!(
        backend
            .replace(&missing_ref, &material("replacement-runtime-canary"))
            .expect_err("missing replace")
            .code(),
        SecretErrorCode::Missing
    );
}

#[test]
fn locked_denied_and_unavailable_are_source_free_and_never_fallback() {
    let backend = MemorySecretBackend::new();
    let service = SecretService::new(backend.clone());
    let created = service
        .create(
            material("failure-mode-runtime-canary"),
            SecretPurpose::CodexApiKey,
        )
        .expect("create");
    let handle = created.handle();

    for (mode, availability, code) in [
        (
            MemoryFailureMode::Locked,
            SecretAvailability::Locked,
            SecretErrorCode::Locked,
        ),
        (
            MemoryFailureMode::Denied,
            SecretAvailability::Denied,
            SecretErrorCode::PermissionDenied,
        ),
        (
            MemoryFailureMode::Unavailable,
            SecretAvailability::Unavailable,
            SecretErrorCode::BackendUnavailable,
        ),
    ] {
        backend.set_mode(mode);
        let before = backend.operation_count();
        let summary = service
            .probe(&handle, SecretPurpose::CodexApiKey)
            .expect("stable failure summary");
        assert_eq!(summary.presence(), SecretPresence::Unknown);
        assert_eq!(summary.availability(), availability);
        assert_eq!(backend.operation_count(), before + 1);

        let before = backend.operation_count();
        let error = service
            .with_material(
                &handle,
                MaterialMatches::new(b"failure-mode-runtime-canary"),
            )
            .expect_err("fail closed read");
        assert_eq!(error.code(), code);
        assert_eq!(backend.operation_count(), before + 1);
    }
}

#[test]
fn public_dtos_errors_and_debug_output_are_material_free() {
    let backend = MemorySecretBackend::new();
    let service = SecretService::new(backend);
    let canary = format!("fyagent-dto-canary-{}", Uuid::new_v4());
    let material = material(&canary);
    assert!(!format!("{material:?}").contains(&canary));

    let summary = service
        .create(material, SecretPurpose::CodexApiKey)
        .expect("create");
    let serialized = serde_json::to_value(&summary).expect("serialize summary");
    let serialized_text = serialized.to_string();
    assert!(!serialized_text.contains(&canary));

    let mut keys = Vec::new();
    collect_keys(&serialized, &mut keys);
    let forbidden = [
        "secretvalue",
        "value",
        "apikey",
        "authorization",
        "credentialblob",
        "backendlocator",
        "rawerror",
        "rawmessage",
        "materialdigest",
        "providerSettings",
        "liveSettings",
    ]
    .into_iter()
    .map(normalized_key)
    .collect::<HashSet<_>>();
    assert!(keys.iter().all(|key| !forbidden.contains(key)), "{keys:?}");

    let error = secret::SecretServiceError::permission_denied();
    let error_text = serde_json::to_string(&error).expect("serialize error");
    assert_eq!(
        error_text,
        r#"{"code":"SECRET_PERMISSION_DENIED","retryable":true,"action":"reviewSystemPermissions"}"#
    );
    assert!(!error_text.contains(&canary));
    assert!(!format!("{error:?} {error}").contains(&canary));
}

#[cfg(any(target_os = "macos", target_os = "windows"))]
#[test]
#[ignore = "requires explicit matching-host OS credential-store HIL"]
fn native_os_backend_crud_readback() {
    assert_eq!(
        std::env::var("FYAGENT_NATIVE_SECRET_TEST").as_deref(),
        Ok("1"),
        "set FYAGENT_NATIVE_SECRET_TEST=1 explicitly"
    );

    let service = SecretService::new(NativeSecretBackend::new());
    let first_canary = format!("fyagent-native-v1-{}", Uuid::new_v4());
    let second_canary = format!("fyagent-native-v2-{}", Uuid::new_v4());
    let created = service
        .create(material(&first_canary), SecretPurpose::CodexApiKey)
        .expect("native create/readback");
    let first = created.handle();

    let result = (|| {
        if !service.with_material(&first, MaterialMatches::new(first_canary.as_bytes()))? {
            return Err(secret::SecretServiceError::verify_failed());
        }
        let replaced =
            service.replace(&first, material(&second_canary), SecretPurpose::CodexApiKey)?;
        let second = replaced.handle();
        if !service.with_material(&second, MaterialMatches::new(second_canary.as_bytes()))? {
            return Err(secret::SecretServiceError::verify_failed());
        }
        Ok::<_, secret::SecretServiceError>(second)
    })();

    let cleanup = service.delete(&first);
    assert!(result.is_ok(), "native CRUD failed: {result:?}");
    cleanup.expect("native cleanup");
    let missing = service
        .probe(&first, SecretPurpose::CodexApiKey)
        .expect("native missing readback");
    assert_eq!(missing.availability(), SecretAvailability::Missing);
}
