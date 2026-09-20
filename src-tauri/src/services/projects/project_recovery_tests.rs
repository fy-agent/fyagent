use super::*;

fn fixture() -> (tempfile::TempDir, ProjectsService, Project) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().canonicalize().unwrap().join("projects");
    let service = ProjectsService::new(
        Arc::new(Database::memory().unwrap()),
        root,
        Arc::new(UnavailableKitReader),
    );
    let customer = service.create_customer("Customer").unwrap();
    let project = service.create(&customer.customer_id, "Project").unwrap();
    (temp, service, project)
}
fn request(p: &Project) -> ProjectMutation {
    ProjectMutation {
        project_id: p.project_id.clone(),
        expected_revision: p.project_revision,
    }
}

#[test]
fn explicit_recovery_preserves_damaged_generation_and_checks_revision() {
    let (_temp, service, p) = fixture();
    let original = service.write_context(&request(&p), "original").unwrap();
    let p = service.get(&p.project_id).unwrap();
    let old_file = PathBuf::from(original.directory.unwrap()).join("context.md");
    std::fs::write(&old_file, "external changes").unwrap();
    assert!(service.write_context(&request(&p), "replacement").is_err());
    let stale = ProjectMutation {
        expected_revision: 0,
        ..request(&p)
    };
    assert!(service
        .write_context_with_recovery(&stale, "replacement", true)
        .is_err());
    let restored = service
        .write_context_with_recovery(&request(&p), "replacement", true)
        .unwrap();
    assert_eq!(restored.content, "replacement");
    assert_eq!(restored.project_revision, p.project_revision + 1);
    assert_ne!(
        PathBuf::from(restored.directory.unwrap()).join("context.md"),
        old_file
    );
    assert_eq!(
        std::fs::read_to_string(old_file).unwrap(),
        "external changes"
    );
    let restored_project = service.get(&p.project_id).unwrap();
    let archived = service
        .update(&request(&restored_project), &p.name, true)
        .unwrap();
    assert!(service
        .write_context_with_recovery(&request(&archived), "forbidden", true)
        .is_err());
}

#[test]
fn explicit_recovery_never_follows_a_replaced_generation_or_project() {
    use std::os::unix::fs::symlink;
    let (temp, service, p) = fixture();
    let original = service.write_context(&request(&p), "original").unwrap();
    let p = service.get(&p.project_id).unwrap();
    let generation = PathBuf::from(original.directory.unwrap());
    let outside = temp.path().join("outside");
    std::fs::create_dir(&outside).unwrap();
    std::fs::write(outside.join("context.md"), "untouched").unwrap();
    std::fs::remove_dir_all(&generation).unwrap();
    symlink(&outside, &generation).unwrap();
    service
        .write_context_with_recovery(&request(&p), "new", true)
        .unwrap();
    assert_eq!(
        std::fs::read_to_string(outside.join("context.md")).unwrap(),
        "untouched"
    );
    assert!(generation
        .symlink_metadata()
        .unwrap()
        .file_type()
        .is_symlink());
    let current = service.get(&p.project_id).unwrap();
    let project_dir = service.root.join(&p.project_id);
    std::fs::rename(&project_dir, temp.path().join("preserved")).unwrap();
    symlink(&outside, project_dir).unwrap();
    assert!(service
        .write_context_with_recovery(&request(&current), "forbidden", true)
        .is_err());
    assert_eq!(
        std::fs::read_to_string(outside.join("context.md")).unwrap(),
        "untouched"
    );
}
