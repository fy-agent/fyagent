use super::*;
use serde_json::{json, Value};

fn library() -> (tempfile::TempDir, KitLibrary) {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path().canonicalize().unwrap().join("kits");
    (dir, KitLibrary::new(root))
}
fn value() -> Value {
    serde_json::from_str(BUILTINS[0]).unwrap()
}
fn bytes(v: Value) -> Vec<u8> {
    serde_json::to_vec(&v).unwrap()
}
fn first(l: &KitLibrary) -> KitIdentity {
    l.list().unwrap()[0].identity.clone()
}

#[test]
fn delivery_kits_builtin_catalog_is_complete_and_zero_write() {
    let (_dir, l) = library();
    let all = l.list().unwrap();
    assert_eq!(all.len(), 3);
    for k in all {
        assert!(!k.installed);
        assert!(k.builtin && k.exportable && k.compatible);
        assert!(!k.connections_checked);
        assert!(!k.manifest.prompts.is_empty());
        assert!(!k.manifest.skills.is_empty());
        assert!(k.manifest.fixtures.len() >= 3);
    }
    assert!(!l.root.exists());
}
#[test]
fn delivery_kits_native_import_replay_restart_export_round_trip() {
    let (dir, mut l) = library();
    let id = first(&l);
    let p = l.preview_builtin(&id).unwrap();
    assert!(!l.root.exists());
    assert!(
        l.apply(&p.preview_id, &id.manifest_digest)
            .unwrap()
            .installed
    );
    assert!(
        l.apply(&p.preview_id, &id.manifest_digest)
            .unwrap()
            .installed
    );
    assert_eq!(fs::read_dir(&l.root).unwrap().count(), 1);
    let mut restarted = KitLibrary::new(l.root.clone());
    assert!(restarted.list().unwrap()[0].installed);
    assert!(matches!(
        restarted.apply(&p.preview_id, &id.manifest_digest),
        Err(KitError::PreviewExpired)
    ));
    let e = restarted.preview_export(&id).unwrap();
    let data = restarted
        .export_bytes(&e.preview_id, &id.manifest_digest)
        .unwrap();
    let path = dir
        .path()
        .canonicalize()
        .unwrap()
        .join("share.fyagent-kit.json");
    export_file(&path, &data).unwrap();
    assert_eq!(export_file(&path, &data), Err(KitError::ContentConflict));
    let (_other, mut other) = library();
    let imported = other.preview_file(&path).unwrap();
    assert_eq!(imported.kit.identity, id);
    other
        .apply(&imported.preview_id, &id.manifest_digest)
        .unwrap();
    assert_eq!(other.run(&id).unwrap().cases.len(), 6);
}
#[test]
fn delivery_kits_cancel_expiry_kind_digest_and_bounds() {
    let (_dir, mut l) = library();
    let id = first(&l);
    let p = l.preview_builtin(&id).unwrap();
    assert!(matches!(
        l.apply(&p.preview_id, "bad"),
        Err(KitError::InvalidPreview)
    ));
    l.pending.get_mut(&p.preview_id).unwrap().created = Instant::now() - TTL;
    assert!(matches!(
        l.apply(&p.preview_id, &id.manifest_digest),
        Err(KitError::PreviewExpired)
    ));
    l.cancel(&p.preview_id);
    assert!(!l.root.exists());
    let e = l.preview_export(&id).unwrap();
    assert!(matches!(
        l.apply(&e.preview_id, &id.manifest_digest),
        Err(KitError::InvalidPreview)
    ));
    for _ in 0..15 {
        l.preview_builtin(&id).unwrap();
    }
    assert!(matches!(l.preview_builtin(&id), Err(KitError::Busy)));
    assert!(matches!(
        parse(&vec![b' '; MAX_BYTES + 1]),
        Err(KitError::InvalidPackage)
    ));
}
#[test]
fn delivery_kits_strict_rejection_and_secret_errors_never_echo_input() {
    let mut v = value();
    v["unknown"] = json!("SECRET-CANARY");
    assert_eq!(parse(&bytes(v)).unwrap_err(), KitError::InvalidPackage);
    let raw = BUILTINS[0].replacen(
        "\"schemaVersion\":",
        "\"title\":\"duplicate\",\"schemaVersion\":",
        1,
    );
    assert_eq!(parse(raw.as_bytes()).unwrap_err(), KitError::InvalidPackage);
    for forbidden in ["script", "command", "secretRef", "path", "enabled"] {
        let mut v = value();
        v[forbidden] = json!("SECRET-CANARY");
        assert_eq!(parse(&bytes(v)).unwrap_err(), KitError::InvalidPackage);
    }
    for unsafe_text in [
        "sec_00112233445546778899aabbccddeeff",
        "sk-SECRET-CANARY",
        "/Users/customer/private",
        "Bearer SECRET-CANARY",
    ] {
        let mut v = value();
        v["summary"] = json!(unsafe_text);
        let e = parse(&bytes(v)).unwrap_err();
        assert_eq!(e, KitError::UnsafeContent);
        assert!(!serde_json::to_string(&e).unwrap().contains("CANARY"));
    }
    for id in ["../escape", "a/b", "a\\b", "/absolute", "a:stream"] {
        let mut v = value();
        v["id"] = json!(id);
        assert_eq!(parse(&bytes(v)).unwrap_err(), KitError::InvalidPackage);
    }
    let mut v = value();
    v["resources"][0]["text"] = json!("altered");
    assert_eq!(parse(&bytes(v)).unwrap_err(), KitError::InvalidPackage);
    let mut v = value();
    v["connections"][0]["credentialSlotIds"] = json!(["missing"]);
    assert_eq!(parse(&bytes(v)).unwrap_err(), KitError::InvalidPackage);
    assert!(schema::strict_json(br#"{"a":{"x":1,"x":2}}"#).is_err());
    assert!(schema::strict_json(br#"{"__proto__":{}}"#).is_err());
    let deep = format!("{}0{}", "[".repeat(34), "]".repeat(34));
    assert!(schema::strict_json(deep.as_bytes()).is_err());
}
#[test]
fn delivery_kits_unknown_schema_and_incompatible_host() {
    let (_dir, mut l) = library();
    let mut v = value();
    v["schemaVersion"] = json!("future");
    assert_eq!(parse(&bytes(v)).unwrap_err(), KitError::UnsupportedSchema);
    let mut v = value();
    v["version"] = json!("2.0.0");
    v["compatibility"]["minFyAgentVersion"] = json!("999.0.0");
    let p = l.preview_bytes(&bytes(v)).unwrap();
    assert!(!p.kit.compatible);
    assert!(matches!(
        l.apply(&p.preview_id, &p.kit.identity.manifest_digest),
        Err(KitError::IncompatibleHost)
    ));
    assert!(!l.root.exists());
}
#[test]
fn delivery_kits_version_conflict_and_imported_sharing_preserve_identity() {
    let (_dir, mut l) = library();
    let mut v = value();
    v["summary"] = json!("different");
    let p = l.preview_bytes(&bytes(v.clone())).unwrap();
    assert!(p.conflict);
    assert!(matches!(
        l.apply(&p.preview_id, &p.kit.identity.manifest_digest),
        Err(KitError::ContentConflict)
    ));
    v["id"] = json!("custom-kit");
    let p = l.preview_bytes(&bytes(v)).unwrap();
    let id = p.kit.identity.clone();
    assert!(!p.kit.exportable);
    assert!(matches!(l.preview_export(&id), Err(KitError::NotFound)));
    assert!(matches!(l.confirm_identity(&id), Err(KitError::NotFound)));
    let imported = l.apply(&p.preview_id, &id.manifest_digest).unwrap();
    assert!(imported.exportable && imported.installed && !imported.builtin);
    let original = fs::read(l.path(&id).unwrap()).unwrap();
    let e = l.preview_export(&id).unwrap();
    assert_eq!(fs::read(l.path(&id).unwrap()).unwrap(), original);
    assert_eq!(fs::read_dir(&l.root).unwrap().count(), 1);
    assert!(matches!(
        l.export_bytes(&p.preview_id, &id.manifest_digest),
        Err(KitError::InvalidPreview)
    ));
    l.cancel(&e.preview_id);
    assert!(matches!(
        l.export_bytes(&e.preview_id, &id.manifest_digest),
        Err(KitError::PreviewExpired)
    ));
    let e = l.preview_export(&id).unwrap();
    let data = l.export_bytes(&e.preview_id, &id.manifest_digest).unwrap();
    assert_eq!(data, original);
    let (_other, mut other) = library();
    let incoming = other.preview_bytes(&data).unwrap();
    assert_eq!(incoming.kit.identity, id);
    let received = other
        .apply(&incoming.preview_id, &id.manifest_digest)
        .unwrap();
    assert!(received.exportable && !received.builtin);
    assert_eq!(other.confirm_identity(&id), Ok(()));
    assert!(matches!(
        other.run(&id),
        Err(KitError::UnsupportedValidator)
    ));
    assert!(matches!(l.run(&id), Err(KitError::UnsupportedValidator)));
    fs::remove_file(l.path(&id).unwrap()).unwrap();
    assert!(matches!(
        l.export_bytes(&e.preview_id, &id.manifest_digest),
        Err(KitError::NotFound)
    ));
}
#[test]
fn delivery_kits_identity_confirmation_is_exact_compatible_and_read_only() {
    let (_dir, l) = library();
    let id = first(&l);
    assert_eq!(l.confirm_identity(&id), Ok(()));
    let mut wrong = id.clone();
    wrong.manifest_digest = "0".repeat(64);
    assert_eq!(l.confirm_identity(&wrong), Err(KitError::NotFound));
    wrong = id;
    wrong.kit_version = "999.0.0".into();
    assert_eq!(l.confirm_identity(&wrong), Err(KitError::NotFound));
    assert!(!l.root.exists());

    let mut v = value();
    v["id"] = json!("future-kit");
    v["compatibility"]["minFyAgentVersion"] = json!("999.0.0");
    let manifest = parse(&bytes(v)).unwrap();
    let id = KitLibrary::identity(&manifest).unwrap();
    let data = canonical(&manifest).unwrap();
    fs::create_dir(&l.root).unwrap();
    fs::write(l.path(&id).unwrap(), &data).unwrap();
    assert_eq!(l.confirm_identity(&id), Err(KitError::IncompatibleHost));
    assert_eq!(fs::read(l.path(&id).unwrap()).unwrap(), data);
    assert_eq!(fs::read_dir(&l.root).unwrap().count(), 1);
}
#[test]
fn delivery_kits_validator_computes_actual_numbers_and_business_failures() {
    let (_dir, l) = library();
    let r = l.run(&first(&l)).unwrap();
    assert_eq!(r.source_class, "local_fixture");
    assert!(r.cases.iter().all(|x| x.matches_expectation));
    let m = r.cases[0].metrics.as_ref().unwrap();
    assert_eq!(m.current_minor, 18_000_000);
    assert_eq!(m.previous_minor, 15_000_000);
    assert_eq!(m.growth_bps, 2000);
    assert_eq!(m.target_bps, 9000);
    assert_eq!(r.cases[0].source_row_ids.len(), 6);
    assert_eq!(r.cases[2].code, validator::BusinessCode::DuplicateRow);
    assert_eq!(r.cases[5].code, validator::BusinessCode::ZeroPrevious);
    let mut m = parse(BUILTINS[0].as_bytes()).unwrap();
    let e = m
        .resources
        .iter_mut()
        .find(|x| x.id == "baseline-expected")
        .unwrap();
    e.text = e.text.replace("18000000", "1");
    e.sha256 = digest(e.text.as_bytes());
    let changed = validator::run(&m).unwrap();
    assert!(!changed.cases[0].matches_expectation);
    assert_eq!(
        changed.cases[0].metrics.as_ref().unwrap().current_minor,
        18_000_000
    );
    assert!(matches!(
        l.run(&l.list().unwrap()[1].identity),
        Err(KitError::UnsupportedValidator)
    ));
}
#[test]
fn delivery_kits_canonical_digest_ignores_key_order_not_array_order() {
    let m = parse(BUILTINS[0].as_bytes()).unwrap();
    let a = digest(&canonical(&m).unwrap());
    let golden: Value = serde_json::from_str(include_str!(
        "../../../../tests/fixtures/deliveryKitContract.v1.json"
    ))
    .unwrap();
    assert_eq!(a, golden["identity"]["manifestDigest"].as_str().unwrap());
    let mut v = value();
    let obj = v
        .as_object_mut()
        .unwrap()
        .clone()
        .into_iter()
        .rev()
        .collect();
    v = Value::Object(obj);
    assert_eq!(a, digest(&canonical(&parse(&bytes(v)).unwrap()).unwrap()));
    let mut changed = m;
    changed.fixtures.reverse();
    assert_ne!(a, digest(&canonical(&changed).unwrap()));
}
#[cfg(target_os = "macos")]
#[test]
fn delivery_kits_symlinks_and_partial_writes_do_not_escape() {
    use std::os::unix::fs::symlink;
    let (dir, mut l) = library();
    let root = dir.path().canonicalize().unwrap();
    let other = root.join("other");
    fs::create_dir(&other).unwrap();
    symlink(&other, &l.root).unwrap();
    assert!(matches!(l.list(), Err(KitError::UnsafeContent)));
    assert_eq!(fs::read_dir(&other).unwrap().count(), 0);
    fs::remove_file(&l.root).unwrap();
    let id = first(&l);
    let p = l.preview_builtin(&id).unwrap();
    fs::create_dir(&l.root).unwrap();
    symlink(other.join("missing"), l.path(&id).unwrap()).unwrap();
    assert!(l.apply(&p.preview_id, &id.manifest_digest).is_err());
    assert_eq!(fs::read_dir(&other).unwrap().count(), 0);
    let target = root.join("share.fyagent-kit.json");
    symlink(other.join("missing"), &target).unwrap();
    assert_eq!(export_file(&target, b"test"), Err(KitError::UnsafeContent));
}
#[test]
fn delivery_kits_concurrent_imports_publish_one_immutable_file() {
    let (_dir, l) = library();
    let root = l.root.clone();
    let threads: Vec<_> = (0..4)
        .map(|_| {
            let root = root.clone();
            std::thread::spawn(move || {
                let mut l = KitLibrary::new(root);
                let id = first(&l);
                let p = l.preview_builtin(&id).unwrap();
                l.apply(&p.preview_id, &id.manifest_digest)
                    .unwrap()
                    .installed
            })
        })
        .collect();
    for t in threads {
        assert!(t.join().unwrap());
    }
    assert_eq!(fs::read_dir(root).unwrap().count(), 1);
}
