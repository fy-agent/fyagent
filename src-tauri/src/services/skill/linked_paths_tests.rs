//! Definitions only: root runs Windows fixtures serially in synthetic temp homes.
//! No environment failure may be reported as a passed link regression.
use super::*;

fn write_skill(dir: &Path, name: &str) {
    fs::create_dir_all(dir).unwrap();
    fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\n---\nfixture body\n"),
    )
    .unwrap();
}

#[cfg(windows)]
struct HomeGuard(Option<std::ffi::OsString>);
#[cfg(windows)]
impl HomeGuard {
    fn set(home: &Path) -> Self {
        crate::initialize_windows_user_context()
            .expect("initialize the real Windows Shell-user context for linked-path tests");
        let guard = Self(std::env::var_os("FYAGENT_TEST_HOME"));
        std::env::set_var("FYAGENT_TEST_HOME", home);
        crate::settings::reload_settings().unwrap();
        guard
    }
}
#[cfg(windows)]
impl Drop for HomeGuard {
    fn drop(&mut self) {
        match self.0.take() {
            Some(value) => std::env::set_var("FYAGENT_TEST_HOME", value),
            None => std::env::remove_var("FYAGENT_TEST_HOME"),
        }
        crate::settings::reload_settings().unwrap();
    }
}

#[cfg(windows)]
fn directory_link(source: &Path, link: &Path, junction: bool) {
    if junction {
        // Arguments travel via isolated child environment, never shell interpolation.
        let output = std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-Command",
                "New-Item -ItemType Junction -Path $env:I17_LINK -Target $env:I17_SOURCE -ErrorAction Stop | Out-Null"])
            .env("I17_LINK", link).env("I17_SOURCE", source).output().expect("junction fixture helper");
        assert!(
            output.status.success(),
            "Windows junction fixture creation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    } else {
        std::os::windows::fs::symlink_dir(source, link)
            .expect("Windows symlink fixture requires authorized privilege");
    }
}

#[cfg(windows)]
fn assert_windows_parent_link_read_only(junction: bool) {
    let temp = tempfile::tempdir().unwrap();
    let _home = HomeGuard::set(temp.path());
    let outside = temp.path().join("external");
    write_skill(&outside.join("linked-skill"), "Linked Skill");
    fs::write(outside.join("sentinel.txt"), b"sentinel").unwrap();
    let parent = temp.path().join(".claude");
    fs::create_dir_all(&parent).unwrap();
    let link = parent.join("skills");
    directory_link(&outside, &link, junction);
    let db = Arc::new(Database::memory().unwrap());
    let before = SelfSnapshot::capture(&outside);
    let observed = SkillService::get_all_installed(&db).unwrap();
    let skill = observed
        .iter()
        .find(|skill| skill.directory == "linked-skill")
        .expect("readable linked skill");
    assert!(SkillService::observed_read_only(Path::new(
        skill.path.as_deref().unwrap()
    )));
    assert!(
        SkillService::observed_read_only_targets(&skill.directory).contains(&"claude".to_string())
    );
    assert!(
        db.get_all_installed_skills().unwrap().is_empty(),
        "read must not adopt"
    );
    for enabled in [false, true] {
        let error = SkillService::toggle_target(&db, &skill.id, &SkillTargetId::Claude, enabled)
            .unwrap_err();
        assert!(error.to_string().contains("SKILL_LINK_READ_ONLY"));
    }
    assert!(SkillService::uninstall(&db, &skill.id)
        .unwrap_err()
        .to_string()
        .contains("SKILL_LINK_READ_ONLY"));
    assert!(SkillService::remove_from_target("linked-skill", &SkillTargetId::Claude).is_err());
    assert!(SkillService::sync_to_app_dir("linked-skill", &SkillTargetId::Claude).is_err());
    assert!(
        db.get_all_installed_skills().unwrap().is_empty(),
        "rejected actions must not adopt"
    );
    assert_eq!(SelfSnapshot::capture(&outside), before);
    assert!(
        fs::symlink_metadata(&link).is_ok(),
        "link itself must remain"
    );
    assert!(
        !SkillService::get_ssot_dir().unwrap().exists(),
        "rejection must precede SSOT creation"
    );
    assert!(SkillService::observed_read_only_targets("ordinary-absent").is_empty());
    assert!(SkillService::require_writable_skill_resources("ordinary-absent").is_ok());
    SkillService::remove_from_target("ordinary-absent", &SkillTargetId::Claude).unwrap();
    let imported = SkillService::import_from_apps(
        &db,
        vec![ImportSkillSelection {
            directory: "linked-skill".to_string(),
            apps: SkillApps::default(),
        }],
    )
    .unwrap();
    assert_eq!(imported.len(), 1);
    assert!(SkillService::get_ssot_dir()
        .unwrap()
        .join("linked-skill")
        .join("SKILL.md")
        .is_file());
    assert_eq!(
        SelfSnapshot::capture(&outside),
        before,
        "read-only source import must preserve target bytes"
    );
    // Actor cleanup of fixture link only, independent of product acceptance.
    fs::remove_dir(link).unwrap();
    assert_eq!(SelfSnapshot::capture(&outside), before);
}

#[cfg(windows)]
#[test]
#[serial_test::serial]
fn i17_windows_parent_symlink_reads_and_rejects_mutations_without_side_effects() {
    assert_windows_parent_link_read_only(false);
}

#[cfg(windows)]
#[test]
#[serial_test::serial]
fn i17_windows_parent_junction_reads_and_rejects_mutations_without_side_effects() {
    assert_windows_parent_link_read_only(true);
}

#[cfg(windows)]
#[test]
fn i17_windows_leaf_link_is_preserved_and_read_source_can_copy_to_normal_root() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    write_skill(&source, "Source");
    let linked = temp.path().join("leaf");
    directory_link(&source, &linked, false);
    let before = SelfSnapshot::capture(&source);
    assert!(SkillService::remove_path(&linked).is_err());
    assert!(SkillService::copy_dir_recursive(&source, &linked).is_err());
    let copied = temp.path().join("managed-copy");
    SkillService::copy_dir_recursive(&linked, &copied).unwrap();
    assert_eq!(
        fs::read(copied.join("SKILL.md")).unwrap(),
        fs::read(source.join("SKILL.md")).unwrap()
    );
    assert_eq!(SelfSnapshot::capture(&source), before);
    assert!(fs::symlink_metadata(&linked).is_ok());
    fs::remove_dir(linked).unwrap();
}

#[derive(Debug, PartialEq, Eq)]
struct SelfSnapshot(Vec<(PathBuf, Option<Vec<u8>>)>);
impl SelfSnapshot {
    fn capture(root: &Path) -> Self {
        // The authoritative existing reader captures hidden files/empty dirs.
        let tree = SkillService::scan_vendor_tree(root).unwrap();
        Self(
            tree.entries
                .into_iter()
                .map(|entry| {
                    let bytes = entry
                        .content_hash
                        .map(|_| fs::read(root.join(&entry.relative)).unwrap());
                    (entry.relative, bytes)
                })
                .collect(),
        )
    }
}

#[test]
fn i17_ordinary_directory_remains_writable() {
    #[cfg(target_os = "macos")]
    let temp = tempfile::tempdir_in(std::env::temp_dir().canonicalize().unwrap()).unwrap();
    #[cfg(windows)]
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("source");
    write_skill(&source, "Ordinary");
    let destination = temp.path().join("destination");
    assert!(!SkillService::observed_read_only(&source));
    SkillService::copy_dir_recursive(&source, &destination).unwrap();
    assert_eq!(
        fs::read(source.join("SKILL.md")).unwrap(),
        fs::read(destination.join("SKILL.md")).unwrap()
    );
    SkillService::remove_path(&destination).unwrap();
    assert!(!destination.exists());
    assert!(source.join("SKILL.md").exists());
}

#[cfg(windows)]
#[test]
#[serial_test::serial]
fn i17_storage_migration_preflights_owned_leaf_link_before_any_move_or_setting_write() {
    let temp = tempfile::tempdir().unwrap();
    let _home = HomeGuard::set(temp.path());
    let db = Arc::new(Database::memory().unwrap());
    let old = SkillService::get_ssot_dir().unwrap();
    write_skill(&old.join("ordinary"), "Ordinary");
    write_skill(&old.join("linked"), "Linked");
    for directory in ["ordinary", "linked"] {
        let mut row = SkillService::installed_from_unmanaged(
            UnmanagedSkill {
                directory: directory.to_string(),
                name: directory.to_string(),
                description: None,
                found_in: vec!["claude".to_string()],
                path: old.join(directory).display().to_string(),
            },
            &HashMap::new(),
        );
        row.path = None;
        db.save_skill(&row).unwrap();
    }
    let app_dir = SkillService::get_target_skills_dir(&SkillTargetId::Claude).unwrap();
    fs::create_dir_all(&app_dir).unwrap();
    let link = app_dir.join("linked");
    directory_link(&old.join("linked"), &link, false);
    let before = SelfSnapshot::capture(&old);
    let db_before = serde_json::to_value(db.get_all_installed_skills().unwrap()).unwrap();
    let settings = temp.path().join(".fyagent/settings.json");
    let settings_before = fs::read(&settings).ok();
    let destination = temp.path().join(".agents/skills");
    let error = SkillService::migrate_storage(&db, SkillStorageLocation::Unified).unwrap_err();
    assert!(error.to_string().contains("SKILL_LINK_READ_ONLY"));
    assert_eq!(
        SelfSnapshot::capture(&old),
        before,
        "even ordinary source must remain unmoved"
    );
    assert!(
        !destination.exists(),
        "rejection precedes destination creation"
    );
    assert_eq!(fs::read(&settings).ok(), settings_before);
    assert_eq!(
        crate::settings::get_skill_storage_location(),
        SkillStorageLocation::FyAgent
    );
    assert_eq!(
        serde_json::to_value(db.get_all_installed_skills().unwrap()).unwrap(),
        db_before
    );
    assert!(fs::symlink_metadata(&link).is_ok());
    fs::remove_dir(link).unwrap();
}

#[cfg(windows)]
#[test]
#[serial_test::serial]
fn i17_storage_migration_internal_junction_refusal_keeps_all_sources_and_settings() {
    let temp = tempfile::tempdir().unwrap();
    let _home = HomeGuard::set(temp.path());
    let db = Arc::new(Database::memory().unwrap());
    let old = SkillService::get_ssot_dir().unwrap();
    for directory in ["ordinary", "linked"] {
        write_skill(&old.join(directory), directory);
        let mut row = SkillService::installed_from_unmanaged(
            UnmanagedSkill {
                directory: directory.to_string(),
                name: directory.to_string(),
                description: None,
                found_in: vec![],
                path: old.join(directory).display().to_string(),
            },
            &HashMap::new(),
        );
        row.path = None;
        db.save_skill(&row).unwrap();
    }
    let outside = temp.path().join("external");
    write_skill(&outside, "Outside");
    let outside_before = SelfSnapshot::capture(&outside);
    let link = old.join("linked/inside");
    directory_link(&outside, &link, true);
    let bodies: Vec<_> = ["ordinary", "linked"]
        .iter()
        .map(|directory| fs::read(old.join(directory).join("SKILL.md")).unwrap())
        .collect();
    let settings = temp.path().join(".fyagent/settings.json");
    let settings_before = fs::read(&settings).ok();
    let error = SkillService::migrate_storage(&db, SkillStorageLocation::Unified).unwrap_err();
    assert!(error.to_string().contains("SKILL_LINK_READ_ONLY"));
    for (index, directory) in ["ordinary", "linked"].iter().enumerate() {
        assert_eq!(
            fs::read(old.join(directory).join("SKILL.md")).unwrap(),
            bodies[index]
        );
    }
    assert!(!temp.path().join(".agents/skills").exists());
    assert_eq!(fs::read(&settings).ok(), settings_before);
    assert_eq!(SelfSnapshot::capture(&outside), outside_before);
    assert_eq!(db.get_all_installed_skills().unwrap().len(), 2);
    assert!(fs::symlink_metadata(&link).is_ok());
    fs::remove_dir(link).unwrap();
}
