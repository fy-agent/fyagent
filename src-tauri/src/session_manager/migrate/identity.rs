//! The three separate identities: source, content, and target mapping.
//!
//! They are computed independently on purpose. An earlier design folded them
//! into a single key, which made two contradictory promises at once: the key
//! contained the source session ID yet was supposed to stay equal after the
//! target minted a new ID, and reconciliation recomputed it from that new
//! target ID, so it could never match.
//!
//! Every concatenation is length-prefixed (u64 little-endian over UTF-8 byte
//! lengths) inside a fixed domain string, so no field boundary is ambiguous.

use std::collections::BTreeMap;
#[cfg(test)]
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[path = "identity_platform.rs"]
mod platform;

use sha2::{Digest, Sha256};

use super::model::{limits, MigratableMessage, MigrationError, MigrationResult};

const CONTENT_DOMAIN: &str = "fyagent.session.content.v1";
const SNAPSHOT_DOMAIN: &str = "fyagent.snapshot.v1";
const SLOT_DOMAIN: &str = "fyagent.slot.v1";
const ORIGIN_DOMAIN: &str = "fyagent.origin.v1";
const DEVICE_DOMAIN: &str = "fyagent.device.v1";
const INSTALL_DOMAIN: &str = "fyagent.install.v2";
const STORE_DOMAIN: &str = "fyagent.store-instance.v1";
const NAMESPACE_DOMAIN: &str = "fyagent.source-namespace.v2";

/// Local, never-exported state for migration identity.
#[cfg(not(any(test, feature = "test-hooks")))]
const LOCAL_STATE_DIR: &str = "session-migration";
const INSTALL_IDENTITY_FILE: &str = "install-identity.json";
const SOURCE_NAMESPACE_FILE: &str = "source-namespaces.json";
const IDENTITY_LOCK_FILE: &str = "identity.lock";
const MAX_STATE_BYTES: u64 = 8 * 1024 * 1024;

pub fn hex_lower(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    out
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex_lower(&hasher.finalize())
}

/// Domain-separated, length-prefixed hash over a field list.
fn domain_hash(domain: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    hasher.update([0u8]);
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part.as_bytes());
    }
    hex_lower(&hasher.finalize())
}

/// Digest of the transcript alone.
///
/// Provider, session ID, paths, timestamps, title, CLI version, platform,
/// export time and omission counts are all excluded: re-exporting the same
/// conversation must land on the same digest.
///
/// Each message contributes `kind byte + u64 length + bytes`, so an empty
/// `assistantFinal` (9 bytes) and a missing one (0 bytes) cannot collide, and
/// `["ab"]` cannot collide with `["a", "b"]`.
pub fn content_digest(messages: &[MigratableMessage]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(CONTENT_DOMAIN.as_bytes());
    hasher.update([0u8]);
    for message in messages {
        hasher.update([message.kind.digest_byte()]);
        hasher.update((message.text.len() as u64).to_le_bytes());
        hasher.update(message.text.as_bytes());
    }
    format!("fyc1:{}", hex_lower(&hasher.finalize()))
}

/// Semantic snapshot: this source conversation at this content.
pub fn snapshot_id(origin_id: &str, content_digest: &str) -> String {
    format!(
        "fys1:{}",
        domain_hash(SNAPSHOT_DOMAIN, &[origin_id, content_digest])
    )
}

/// Default-import slot: one snapshot lands in one target store on one device
/// exactly once.
pub fn idempotency_slot(
    snapshot_id: &str,
    target_provider_id: &str,
    target_store_id: &str,
    device_binding: &str,
) -> String {
    format!(
        "fyslot1:{}",
        domain_hash(
            SLOT_DOMAIN,
            &[
                snapshot_id,
                target_provider_id,
                target_store_id,
                device_binding,
            ],
        )
    )
}

/// Source identity for a session with a trustworthy native ID.
///
/// `store_namespace` is a random per-store UUID kept on the source machine, so
/// the packaged identity leaks neither an absolute path nor a machine secret.
pub fn origin_id_from_native(provider_id: &str, store_namespace: &str, session_id: &str) -> String {
    format!(
        "fyo1:{}",
        domain_hash(ORIGIN_DOMAIN, &[provider_id, store_namespace, session_id])
    )
}

/// Stable only for the current source file instance. The legacy extractor
/// passes `store_key|source_path`; direct paths are also accepted. Unknown or
/// ambiguous locations fail closed instead of permanently binding a raw path.
pub fn stable_random_origin_id(provider_id: &str, key: &str) -> MigrationResult<String> {
    let direct = Path::new(key);
    let candidates = if direct.is_absolute() && direct.exists() {
        vec![direct]
    } else {
        key.match_indices('|')
            .filter_map(|(offset, _)| {
                let path = Path::new(&key[offset + 1..]);
                (path.is_absolute() && path.exists()).then_some(path)
            })
            .collect::<Vec<_>>()
    };
    let [path] = candidates.as_slice() else {
        return Err(unidentified(provider_id));
    };
    let instance = store_instance_id(provider_id, path)?;
    let namespace = source_store_namespace(provider_id, &format!("origin-instance:{instance}"))?;
    Ok(format!("fyo1r:{namespace}"))
}

/// A provider store instance, not merely its current pathname. Normal writes
/// preserve this identity; replacing the file/directory at the path does not.
/// Provider-owned installation UUIDs can be combined with this value by the
/// native writer without trusting any imported identity.
pub fn store_instance_id(provider_id: &str, path: &Path) -> MigrationResult<String> {
    validate_identity_field("providerId", provider_id)?;
    let canonical = path.canonicalize().map_err(|_| unidentified(provider_id))?;
    let instance = platform::file_instance(&canonical).map_err(|_| unidentified(provider_id))?;
    let path_digest = sha256_hex(canonical.as_os_str().as_encoded_bytes());
    Ok(format!(
        "fystore1:{}",
        domain_hash(STORE_DOMAIN, &[provider_id, &path_digest, &instance])
    ))
}

fn unidentified(provider_id: &str) -> MigrationError {
    MigrationError::TargetStoreUnidentified {
        provider_id: provider_id.to_string(),
    }
}

/// Pre-allocated reconciliation anchor, written before any native call.
pub fn new_nonce() -> String {
    format!("fynonce1-{}", uuid::Uuid::new_v4().simple())
}

pub fn new_attempt_id() -> String {
    format!("fyatt1-{}", uuid::Uuid::new_v4().simple())
}

/// Reject an identity field instead of sanitizing it.
///
/// Sanitizing first and hashing afterwards can make two genuinely different
/// identities compare equal, which manufactures false duplicates.
pub fn validate_identity_field(field: &str, value: &str) -> MigrationResult<()> {
    if value.is_empty() {
        return Err(MigrationError::IdentityFieldInvalid {
            field: field.to_string(),
            reason: "empty".to_string(),
        });
    }
    if value.chars().count() > limits::IDENTITY_FIELD_CHARS {
        return Err(MigrationError::IdentityFieldInvalid {
            field: field.to_string(),
            reason: format!("longer than {} characters", limits::IDENTITY_FIELD_CHARS),
        });
    }
    for ch in value.chars() {
        if (ch as u32) < 0x20 || ch as u32 == 0x7f {
            return Err(MigrationError::IdentityFieldInvalid {
                field: field.to_string(),
                reason: "contains a control character".to_string(),
            });
        }
        if matches!(ch, '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}' | '\u{200e}' | '\u{200f}')
        {
            return Err(MigrationError::IdentityFieldInvalid {
                field: field.to_string(),
                reason: "contains a bidirectional override".to_string(),
            });
        }
    }
    Ok(())
}

/// `workspaceLabel` is display metadata, so an invalid value degrades to absent
/// rather than failing the export. It must stay a bare basename.
pub fn accept_workspace_label(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.chars().count() > limits::IDENTITY_FIELD_CHARS {
        return None;
    }
    if trimmed.contains(['/', '\\', ':']) {
        return None;
    }
    if trimmed.chars().any(|ch| (ch as u32) < 0x20) {
        return None;
    }
    Some(trimmed.to_string())
}

#[cfg(any(test, feature = "test-hooks"))]
thread_local! {
    static LOCAL_STATE_DIR_OVERRIDE: std::cell::RefCell<Option<PathBuf>> =
        const { std::cell::RefCell::new(None) };
    static MACHINE_FINGERPRINT_OVERRIDE: std::cell::RefCell<Option<String>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(any(test, feature = "test-hooks"))]
struct TestIdentityGuard {
    directory: Option<PathBuf>,
    fingerprint: Option<String>,
}

#[cfg(any(test, feature = "test-hooks"))]
impl Drop for TestIdentityGuard {
    fn drop(&mut self) {
        LOCAL_STATE_DIR_OVERRIDE.with(|cell| *cell.borrow_mut() = self.directory.take());
        MACHINE_FINGERPRINT_OVERRIDE.with(|cell| *cell.borrow_mut() = self.fingerprint.take());
    }
}

/// Redirect all identity state and inject a synthetic machine fingerprint.
/// The guard restores nested overrides even if the body panics. No unit test
/// falls back to the developer's HOME or probes their real OS identity.
#[cfg(any(test, feature = "test-hooks"))]
pub fn with_local_state_dir<T>(dir: &Path, body: impl FnOnce() -> T) -> T {
    let directory =
        LOCAL_STATE_DIR_OVERRIDE.with(|cell| cell.borrow_mut().replace(dir.to_path_buf()));
    let fingerprint = MACHINE_FINGERPRINT_OVERRIDE.with(|cell| {
        let previous = cell.borrow().clone();
        if previous.is_none() {
            *cell.borrow_mut() = Some("synthetic-machine-user".into());
        }
        previous
    });
    let _guard = TestIdentityGuard {
        directory,
        fingerprint,
    };
    body()
}

#[cfg(test)]
fn with_machine_fingerprint<T>(fingerprint: &str, body: impl FnOnce() -> T) -> T {
    let directory = LOCAL_STATE_DIR_OVERRIDE.with(|cell| cell.borrow().clone());
    let fingerprint = MACHINE_FINGERPRINT_OVERRIDE
        .with(|cell| cell.borrow_mut().replace(fingerprint.to_string()));
    let _guard = TestIdentityGuard {
        directory,
        fingerprint,
    };
    body()
}

fn state_error(reason: &str) -> MigrationError {
    MigrationError::ReceiptStoreFailed {
        reason: reason.to_string(),
    }
}

fn local_state_dir() -> MigrationResult<PathBuf> {
    #[cfg(any(test, feature = "test-hooks"))]
    {
        LOCAL_STATE_DIR_OVERRIDE
            .with(|cell| cell.borrow().clone())
            .ok_or_else(|| state_error("test identity state directory was not injected"))
    }
    #[cfg(not(any(test, feature = "test-hooks")))]
    {
        Ok(crate::config::get_app_config_dir().join(LOCAL_STATE_DIR))
    }
}

fn machine_fingerprint() -> MigrationResult<String> {
    #[cfg(any(test, feature = "test-hooks"))]
    {
        MACHINE_FINGERPRINT_OVERRIDE
            .with(|cell| cell.borrow().clone())
            .filter(|value| !value.is_empty())
            .ok_or_else(|| state_error("test machine fingerprint was not injected"))
    }
    #[cfg(not(any(test, feature = "test-hooks")))]
    {
        platform::machine_user_fingerprint()
            .map_err(|_| state_error("OS machine/user identity could not be established"))
    }
}

/// This file is never replaced or deleted. Each call opens its own handle, so
/// the lock protects the entire read-modify-write across threads and processes.
/// Locking a JSON file that is atomically replaced would not provide this property.
fn with_locked_state<T>(
    body: impl FnOnce(&platform::IdentityDirectory) -> MigrationResult<T>,
) -> MigrationResult<T> {
    let path = local_state_dir()?;
    std::fs::create_dir_all(&path)
        .map_err(|_| state_error("migration identity directory could not be created"))?;
    let directory = platform::IdentityDirectory::open(&path)
        .map_err(|_| state_error("migration identity directory could not be pinned"))?;
    with_locked_directory(directory, body)
}

fn with_locked_directory<T>(
    directory: platform::IdentityDirectory,
    body: impl FnOnce(&platform::IdentityDirectory) -> MigrationResult<T>,
) -> MigrationResult<T> {
    let file = directory.open_lock(IDENTITY_LOCK_FILE).map_err(|error| {
        state_error(&format!(
            "migration identity lock could not be opened ({:?}, OS error {:?})",
            error.kind(),
            error.raw_os_error()
        ))
    })?;
    let lock = platform::IdentityLock::acquire(file)
        .map_err(|_| state_error("migration identity lock could not be acquired"))?;
    directory
        .check_locked(IDENTITY_LOCK_FILE, &lock)
        .map_err(|_| {
            state_error("migration identity directory or lock changed while acquiring authority")
        })?;
    let result = body(&directory)?;
    directory
        .check_locked(IDENTITY_LOCK_FILE, &lock)
        .map_err(|_| {
            state_error("migration identity directory or lock changed during the transaction")
        })?;
    Ok(result)
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct InstallState {
    version: u8,
    install_id: String,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct NamespaceState {
    version: u8,
    #[serde(deserialize_with = "deserialize_unique_namespaces")]
    namespaces: BTreeMap<String, String>,
}

fn deserialize_unique_namespaces<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<BTreeMap<String, String>, D::Error> {
    struct Unique;
    impl<'de> serde::de::Visitor<'de> for Unique {
        type Value = BTreeMap<String, String>;
        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a namespace object without duplicate keys")
        }
        fn visit_map<A: serde::de::MapAccess<'de>>(
            self,
            mut access: A,
        ) -> Result<Self::Value, A::Error> {
            let mut map = BTreeMap::new();
            while let Some((key, value)) = access.next_entry::<String, String>()? {
                if map.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("duplicate namespace key"));
                }
            }
            Ok(map)
        }
    }
    deserializer.deserialize_map(Unique)
}

fn read_state<T: serde::de::DeserializeOwned>(
    directory: &platform::IdentityDirectory,
    name: &str,
) -> MigrationResult<Option<T>> {
    let Some(bytes) = directory.read_child(name, MAX_STATE_BYTES).map_err(|_| {
        state_error("migration identity state could not be read from its locked directory")
    })?
    else {
        return Ok(None);
    };
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| state_error("migration identity state is invalid; refusing to reset it"))
}

fn write_state<T: Serialize>(
    directory: &platform::IdentityDirectory,
    name: &str,
    state: &T,
) -> MigrationResult<()> {
    let bytes = serde_json::to_vec_pretty(state)
        .map_err(|_| state_error("migration identity state could not be encoded"))?;
    if bytes.len() as u64 > MAX_STATE_BYTES {
        return Err(state_error(
            "migration identity state exceeds its size limit",
        ));
    }
    directory.write_child(name, &bytes).map_err(|_| {
        state_error("migration identity state could not be committed to its locked directory")
    })
}

fn valid_uuid(value: &str) -> bool {
    uuid::Uuid::parse_str(value).is_ok_and(|id| id.get_version_num() == 4 && !id.is_nil())
}

fn install_identity_locked(directory: &platform::IdentityDirectory) -> MigrationResult<String> {
    let fingerprint = machine_fingerprint()?;
    let location = directory.instance();
    let stored: InstallState = match read_state(directory, INSTALL_IDENTITY_FILE)? {
        Some(stored) => stored,
        None => {
            // An orphan namespace file must not be silently assigned a new seed.
            if directory
                .read_child(SOURCE_NAMESPACE_FILE, MAX_STATE_BYTES)
                .map_err(|_| state_error("migration namespace state could not be inspected"))?
                .is_some()
            {
                return Err(state_error(
                    "migration installation identity is missing for existing namespace state",
                ));
            }
            let fresh = InstallState {
                version: 2,
                install_id: uuid::Uuid::new_v4().simple().to_string(),
            };
            write_state(directory, INSTALL_IDENTITY_FILE, &fresh)?;
            fresh
        }
    };
    if stored.version != 2 || !valid_uuid(&stored.install_id) {
        return Err(state_error(
            "migration installation identity is invalid; refusing to reset it",
        ));
    }
    Ok(format!(
        "fyinstall2:{}",
        domain_hash(
            INSTALL_DOMAIN,
            &[&stored.install_id, &fingerprint, location]
        )
    ))
}

/// Random installation seed bound to the actual OS machine/user and current
/// installation directory. A copied JSON seed alone cannot inherit receipts.
pub fn install_identity() -> MigrationResult<String> {
    with_locked_state(install_identity_locked)
}

/// Random namespace for one source store, serialized under the same OS lock
/// as installation identity. Keys include the machine-bound installation ID.
pub fn source_store_namespace(provider_id: &str, store_key: &str) -> MigrationResult<String> {
    validate_identity_field("providerId", provider_id)?;
    if store_key.is_empty() {
        return Err(state_error("source store identity is empty"));
    }
    with_locked_state(|directory| {
        let install = install_identity_locked(directory)?;
        let mut stored: NamespaceState = read_state(directory, SOURCE_NAMESPACE_FILE)?
            .unwrap_or_else(|| NamespaceState {
                version: 2,
                namespaces: BTreeMap::new(),
            });
        if stored.version != 2
            || stored.namespaces.iter().any(|(key, value)| {
                key.len() != 64
                    || !key
                        .bytes()
                        .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                    || !valid_uuid(value)
            })
        {
            return Err(state_error(
                "migration source namespaces are invalid; refusing to reset them",
            ));
        }
        let key = domain_hash(NAMESPACE_DOMAIN, &[&install, provider_id, store_key]);
        if let Some(existing) = stored.namespaces.get(&key) {
            return Ok(existing.clone());
        }
        let namespace = uuid::Uuid::new_v4().simple().to_string();
        stored.namespaces.insert(key, namespace.clone());
        write_state(directory, SOURCE_NAMESPACE_FILE, &stored)?;
        Ok(namespace)
    })
}

/// Binds a receipt row to this machine and this target store.
///
/// Rows whose binding does not match the current machine are dead on read:
/// they take part in neither idempotency nor reconciliation.
pub fn device_binding(target_store_id: &str) -> MigrationResult<String> {
    let install_id = install_identity()?;
    Ok(format!(
        "fydev1:{}",
        domain_hash(DEVICE_DOMAIN, &[&install_id, target_store_id])
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session_manager::migrate::model::MessageKind;

    fn message(kind: MessageKind, text: &str) -> MigratableMessage {
        MigratableMessage {
            seq: 0,
            kind,
            text: text.to_string(),
            ts: None,
        }
    }

    #[test]
    fn content_digest_ignores_everything_except_kind_and_text() {
        let mut a = message(MessageKind::UserText, "hello");
        let mut b = a.clone();
        a.seq = 0;
        a.ts = Some(1);
        b.seq = 9;
        b.ts = Some(999_999);
        assert_eq!(content_digest(&[a]), content_digest(&[b]));
    }

    #[test]
    fn empty_final_answer_does_not_collide_with_a_missing_one() {
        let user = message(MessageKind::UserText, "q");
        let empty_final = message(MessageKind::AssistantFinal, "");
        let with_empty = content_digest(&[user.clone(), empty_final]);
        let without = content_digest(&[user]);
        assert_ne!(with_empty, without);
    }

    #[test]
    fn length_prefix_separates_message_boundaries() {
        let joined = content_digest(&[message(MessageKind::UserText, "ab")]);
        let split = content_digest(&[
            message(MessageKind::UserText, "a"),
            message(MessageKind::UserText, "b"),
        ]);
        assert_ne!(joined, split);
    }

    #[test]
    fn kind_participates_in_the_digest() {
        let as_user = content_digest(&[message(MessageKind::UserText, "same")]);
        let as_assistant = content_digest(&[message(MessageKind::AssistantFinal, "same")]);
        assert_ne!(as_user, as_assistant);
    }

    #[test]
    fn snapshot_separates_same_content_from_different_sources() {
        let digest = content_digest(&[message(MessageKind::UserText, "same body")]);
        assert_ne!(
            snapshot_id("fyo1:aaa", &digest),
            snapshot_id("fyo1:bbb", &digest)
        );
    }

    #[test]
    fn slot_changes_with_every_target_dimension() {
        let base = idempotency_slot("snap", "codex", "store", "dev");
        assert_ne!(base, idempotency_slot("snap2", "codex", "store", "dev"));
        assert_ne!(base, idempotency_slot("snap", "opencode", "store", "dev"));
        assert_ne!(base, idempotency_slot("snap", "codex", "store2", "dev"));
        assert_ne!(base, idempotency_slot("snap", "codex", "store", "dev2"));
        assert_eq!(base, idempotency_slot("snap", "codex", "store", "dev"));
    }

    #[test]
    fn domain_hash_is_not_vulnerable_to_field_shifting() {
        // Without length prefixes these two field lists would hash the same.
        assert_ne!(
            domain_hash(SLOT_DOMAIN, &["ab", "c"]),
            domain_hash(SLOT_DOMAIN, &["a", "bc"])
        );
    }

    #[test]
    fn identity_validation_rejects_instead_of_sanitizing() {
        assert!(validate_identity_field("sessionId", "019cc369-bd7c").is_ok());
        assert!(validate_identity_field("sessionId", "").is_err());
        assert!(validate_identity_field("sessionId", "bad\u{0}id").is_err());
        assert!(validate_identity_field("sessionId", "bad\u{202e}id").is_err());
        assert!(validate_identity_field("sessionId", &"a".repeat(201)).is_err());
        assert!(validate_identity_field("sessionId", &"a".repeat(200)).is_ok());
    }

    #[test]
    fn workspace_label_degrades_to_absent_for_path_like_values() {
        assert_eq!(accept_workspace_label(" repo "), Some("repo".to_string()));
        assert_eq!(accept_workspace_label("a/b"), None);
        assert_eq!(accept_workspace_label("C:\\x"), None);
        assert_eq!(accept_workspace_label("a:b"), None);
        assert_eq!(accept_workspace_label(""), None);
    }

    #[test]
    fn installation_is_stable_but_copied_identity_cannot_bind_another_machine_or_user() {
        let state = tempfile::tempdir().unwrap();
        with_local_state_dir(state.path(), || {
            let a = with_machine_fingerprint("machine-a/user-a", || install_identity().unwrap());
            let seed = std::fs::read(state.path().join(INSTALL_IDENTITY_FILE)).unwrap();
            assert_eq!(
                a,
                with_machine_fingerprint("machine-a/user-a", || install_identity().unwrap())
            );
            let b = with_machine_fingerprint("machine-b/user-a", || install_identity().unwrap());
            let c = with_machine_fingerprint("machine-a/user-b", || install_identity().unwrap());
            assert_ne!(a, b);
            assert_ne!(a, c);
            assert_eq!(
                seed,
                std::fs::read(state.path().join(INSTALL_IDENTITY_FILE)).unwrap()
            );
        });
    }

    #[test]
    fn copied_configuration_directory_does_not_inherit_install_or_source_namespaces() {
        let original = tempfile::tempdir().unwrap();
        let copied = tempfile::tempdir().unwrap();
        let (install, namespace) = with_local_state_dir(original.path(), || {
            (
                install_identity().unwrap(),
                source_store_namespace("codex", "source").unwrap(),
            )
        });
        for name in [INSTALL_IDENTITY_FILE, SOURCE_NAMESPACE_FILE] {
            std::fs::copy(original.path().join(name), copied.path().join(name)).unwrap();
        }
        with_local_state_dir(copied.path(), || {
            assert_ne!(install, install_identity().unwrap());
            assert_ne!(
                namespace,
                source_store_namespace("codex", "source").unwrap()
            );
        });
    }

    #[test]
    fn corruption_is_not_replaced_with_a_fresh_identity() {
        let state = tempfile::tempdir().unwrap();
        with_local_state_dir(state.path(), || {
            install_identity().unwrap();
            let path = state.path().join(INSTALL_IDENTITY_FILE);
            for bytes in [
                b"{".as_slice(),
                br#"{"version":2,"installId":""}"#,
                br#"{"version":9,"installId":"c6b2c599-9e36-4b16-986f-7690096d09cf"}"#,
            ] {
                std::fs::write(&path, bytes).unwrap();
                assert!(install_identity().is_err());
                assert_eq!(std::fs::read(&path).unwrap(), bytes);
            }
        });
    }

    #[test]
    fn damaged_or_duplicate_namespace_data_fails_closed() {
        let state = tempfile::tempdir().unwrap();
        with_local_state_dir(state.path(), || {
            source_store_namespace("codex", "source").unwrap();
            let path = state.path().join(SOURCE_NAMESPACE_FILE);
            let key = "a".repeat(64);
            let duplicate = format!(
                r#"{{"version":2,"namespaces":{{"{key}":"c6b2c599-9e36-4b16-986f-7690096d09cf","{key}":"bb41df7f-cf0d-4a66-a49b-f8c50d5d24fb"}}}}"#
            );
            for bytes in [b"[]".to_vec(), b"{".to_vec(), duplicate.into_bytes()] {
                std::fs::write(&path, &bytes).unwrap();
                assert!(source_store_namespace("codex", "source").is_err());
                assert_eq!(std::fs::read(&path).unwrap(), bytes);
            }
        });
    }

    #[test]
    fn file_replacement_changes_store_and_unknown_origin_but_appends_do_not() {
        use std::io::Write;
        let state = tempfile::tempdir().unwrap();
        let source = state.path().join("history-one.jsonl");
        std::fs::write(&source, "first").unwrap();
        let key = format!("store|{}", source.display());
        with_local_state_dir(&state.path().join("identity"), || {
            let store = store_instance_id("codex", &source).unwrap();
            let origin = stable_random_origin_id("codex", &key).unwrap();
            OpenOptions::new()
                .append(true)
                .open(&source)
                .unwrap()
                .write_all(b"more")
                .unwrap();
            assert_eq!(store, store_instance_id("codex", &source).unwrap());
            assert_eq!(origin, stable_random_origin_id("codex", &key).unwrap());
            // Keep the old inode live so the test never depends on allocation timing.
            std::fs::rename(&source, state.path().join("old.jsonl")).unwrap();
            std::fs::write(&source, "firstmore").unwrap();
            assert_ne!(store, store_instance_id("codex", &source).unwrap());
            assert_ne!(origin, stable_random_origin_id("codex", &key).unwrap());
            assert!(stable_random_origin_id("codex", "not-a-file").is_err());
        });
    }

    #[test]
    fn directory_replacement_invalidates_install_binding_even_with_copied_json() {
        let base = tempfile::tempdir().unwrap();
        let state = base.path().join("identity");
        let original = with_local_state_dir(&state, || install_identity().unwrap());
        let saved = std::fs::read(state.join(INSTALL_IDENTITY_FILE)).unwrap();
        std::fs::rename(&state, base.path().join("old-identity")).unwrap();
        std::fs::create_dir(&state).unwrap();
        std::fs::write(state.join(INSTALL_IDENTITY_FILE), saved).unwrap();
        assert_ne!(
            original,
            with_local_state_dir(&state, || install_identity().unwrap())
        );
    }

    #[test]
    fn concurrent_threads_keep_every_namespace_and_one_installation() {
        let state = tempfile::tempdir().unwrap();
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(12));
        let workers = (0..12)
            .map(|index| {
                let directory = state.path().to_path_buf();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    with_local_state_dir(&directory, || {
                        barrier.wait();
                        (
                            install_identity().unwrap(),
                            source_store_namespace("codex", "shared").unwrap(),
                            source_store_namespace("codex", &format!("unique-{index}")).unwrap(),
                        )
                    })
                })
            })
            .collect::<Vec<_>>();
        let results = workers
            .into_iter()
            .map(|worker| worker.join().unwrap())
            .collect::<Vec<_>>();
        with_local_state_dir(state.path(), || {
            for (index, (install, shared, unique)) in results.iter().enumerate() {
                assert_eq!(install, &results[0].0);
                assert_eq!(shared, &results[0].1);
                assert_eq!(
                    unique,
                    &source_store_namespace("codex", &format!("unique-{index}")).unwrap()
                );
            }
            let stored: NamespaceState =
                with_locked_state(|directory| read_state(directory, SOURCE_NAMESPACE_FILE))
                    .unwrap()
                    .unwrap();
            assert_eq!(stored.namespaces.len(), 13);
        });
    }

    // Invoked by the next test in fresh OS processes, never with a real HOME.
    #[test]
    fn identity_process_worker() {
        let Some(directory) = std::env::var_os("FYAGENT_IDENTITY_TEST_CHILD_DIR") else {
            return;
        };
        let index = std::env::var("FYAGENT_IDENTITY_TEST_CHILD_INDEX").unwrap();
        let directory = PathBuf::from(directory);
        with_local_state_dir(&directory, || {
            let shared = source_store_namespace("codex", "shared-process").unwrap();
            let unique = source_store_namespace("codex", &format!("process-{index}")).unwrap();
            std::fs::write(
                directory.join(format!("result-{index}")),
                format!("{shared}\n{unique}"),
            )
            .unwrap();
        });
    }

    #[test]
    fn independent_processes_share_lock_without_lost_namespaces() {
        let state = tempfile::tempdir().unwrap();
        let test_module = module_path!().split_once("::").unwrap().1;
        let test_name = format!("{test_module}::identity_process_worker");
        let workers = (0..6)
            .map(|index| {
                std::process::Command::new(std::env::current_exe().unwrap())
                    .args(["--exact", &test_name, "--test-threads=1"])
                    .env("FYAGENT_IDENTITY_TEST_CHILD_DIR", state.path())
                    .env("FYAGENT_IDENTITY_TEST_CHILD_INDEX", index.to_string())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .unwrap()
            })
            .collect::<Vec<_>>();
        for mut worker in workers {
            assert!(worker.wait().unwrap().success());
        }
        with_local_state_dir(state.path(), || {
            let shared = source_store_namespace("codex", "shared-process").unwrap();
            for index in 0..6 {
                let unique = source_store_namespace("codex", &format!("process-{index}")).unwrap();
                assert_eq!(
                    std::fs::read_to_string(state.path().join(format!("result-{index}"))).unwrap(),
                    format!("{shared}\n{unique}")
                );
            }
            let stored: NamespaceState =
                with_locked_state(|directory| read_state(directory, SOURCE_NAMESPACE_FILE))
                    .unwrap()
                    .unwrap();
            assert_eq!(stored.namespaces.len(), 7);
        });
    }

    #[test]
    fn override_guards_restore_context_after_nested_panic() {
        let outer = tempfile::tempdir().unwrap();
        let inner = tempfile::tempdir().unwrap();
        with_local_state_dir(outer.path(), || {
            let original = install_identity().unwrap();
            let result = std::panic::catch_unwind(|| {
                with_local_state_dir(inner.path(), || {
                    with_machine_fingerprint("different-machine", || panic!("synthetic panic"));
                })
            });
            assert!(result.is_err());
            assert_eq!(original, install_identity().unwrap());
        });
        assert!(local_state_dir().is_err());
        assert!(machine_fingerprint().is_err());
    }

    #[test]
    fn same_directory_rename_preserves_install_binding_and_source_namespace() {
        let base = tempfile::tempdir().unwrap();
        let old = base.path().join("old-name");
        let new = base.path().join("new-name");
        let before = with_local_state_dir(&old, || {
            (
                install_identity().unwrap(),
                device_binding("native-store").unwrap(),
                source_store_namespace("codex", "source-store").unwrap(),
            )
        });
        std::fs::rename(&old, &new).unwrap();
        let after = with_local_state_dir(&new, || {
            (
                install_identity().unwrap(),
                device_binding("native-store").unwrap(),
                source_store_namespace("codex", "source-store").unwrap(),
            )
        });
        assert_eq!(before, after);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn directory_replacement_before_lock_admission_cannot_create_identity_in_replacement() {
        let base = tempfile::tempdir().unwrap();
        let state = base.path().join("identity");
        std::fs::create_dir(&state).unwrap();
        let directory = platform::IdentityDirectory::open(&state).unwrap();
        std::fs::rename(&state, base.path().join("previous")).unwrap();
        std::fs::create_dir(&state).unwrap();
        let mut body_ran = false;
        let result = with_locked_directory(directory, |_| {
            body_ran = true;
            Ok(())
        });
        assert!(result.is_err());
        assert!(!body_ran);
        assert_eq!(std::fs::read_dir(&state).unwrap().count(), 0);
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn directory_replacement_inside_locked_transaction_fails_without_touching_replacement() {
        let base = tempfile::tempdir().unwrap();
        let state = base.path().join("identity");
        with_local_state_dir(&state, || {
            install_identity().unwrap();
            let original = std::fs::read(state.join(INSTALL_IDENTITY_FILE)).unwrap();
            let moved = base.path().join("previous");
            let result = with_locked_state(|directory| {
                std::fs::rename(&state, &moved).unwrap();
                std::fs::create_dir(&state).unwrap();
                directory
                    .write_child(INSTALL_IDENTITY_FILE, b"must not land in replacement")
                    .map_err(|_| state_error("test directory was replaced"))
            });
            assert!(result.is_err());
            assert_eq!(std::fs::read_dir(&state).unwrap().count(), 0);
            assert_eq!(
                std::fs::read(moved.join(INSTALL_IDENTITY_FILE)).unwrap(),
                original
            );
        });
    }
}
