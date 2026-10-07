use super::*;
use crate::{
    app_config::{AppType, SkillApps, SkillTargetId},
    services::skill::{update, SkillService, SkillStorageLocation},
};
use anyhow::{anyhow, Context, Result};
use base64::Engine as _;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fs, io,
    io::Read,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};
use tempfile::{tempdir, TempDir};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio_rustls::TlsAcceptor;

const ZIP_BASE64: &str = include_str!("fixtures/synthetic-skill.zip.b64");
const ZIP_SHA256: &str = "8066be9f3c182d912d61870ef0efec6ee932750c6bb06fc56a3d120a234a6668";
const CA: &str = include_str!("fixtures/ca-cert.pem");
const CERT: &str = include_str!("fixtures/server-cert.pem");
const KEY: &str = include_str!("fixtures/server-key.pem");
const SKILL_ID: &str = "iteration-resources/audit-skills:audit-skill";
const DIRECTORY: &str = "audit-skill";
const OLD_SKILL: &str =
    "---\nname: Old Audit Skill\ndescription: previous fixture\n---\nold body\n";

fn fixture_zip() -> &'static [u8] {
    static DECODED: std::sync::OnceLock<Vec<u8>> = std::sync::OnceLock::new();
    DECODED.get_or_init(|| {
        let encoded: String = ZIP_BASE64.split_ascii_whitespace().collect();
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .expect("valid synthetic ZIP base64");
        assert_eq!(format!("{:x}", Sha256::digest(&bytes)), ZIP_SHA256);
        bytes
    })
}

struct HomeGuard {
    previous_home: Option<std::ffi::OsString>,
    previous_settings: Option<crate::settings::AppSettings>,
}

impl HomeGuard {
    fn set(home: &Path) -> Self {
        let prior = std::env::var_os("FYAGENT_TEST_HOME");
        std::env::set_var("FYAGENT_TEST_HOME", home);
        // Lazy settings initialization must happen under the fixture override.
        // Keep any existing cache snapshot in memory; never persist it to disk.
        let previous_settings = crate::settings::get_settings();
        Self {
            previous_home: prior,
            previous_settings: Some(previous_settings),
        }
    }
}

impl Drop for HomeGuard {
    fn drop(&mut self) {
        if let Some(settings) = self.previous_settings.take() {
            crate::settings::replace_settings_in_memory_for_test(settings);
        }
        match self.previous_home.take() {
            Some(value) => std::env::set_var("FYAGENT_TEST_HOME", value),
            None => std::env::remove_var("FYAGENT_TEST_HOME"),
        }
    }
}

struct Isolated {
    _home: HomeGuard,
    _temp: TempDir,
    db: Arc<Database>,
    service: SkillService,
    before: InstalledSkill,
    ssot: PathBuf,
    claude: PathBuf,
    codex: PathBuf,
}

impl Isolated {
    fn new() -> Self {
        let temp = tempdir().expect("isolated home");
        let home = HomeGuard::set(temp.path());
        assert!(crate::app_store::get_app_config_dir_override().is_none());
        let home_path = crate::config::get_home_dir();
        let config = crate::config::get_app_config_dir();
        let user_temp = crate::config::get_user_temp_dir();
        assert!(home_path.starts_with(temp.path()));
        assert!(config.starts_with(temp.path()));
        assert!(user_temp.starts_with(temp.path()));
        let test_settings = crate::settings::AppSettings {
            skill_sync_method: crate::services::skill::SyncMethod::Copy,
            skill_storage_location: SkillStorageLocation::FyAgent,
            ..crate::settings::AppSettings::default()
        };
        crate::settings::update_settings(test_settings).expect("write isolated settings");

        let db = Arc::new(Database::memory().expect("memory DB"));
        let ssot = SkillService::get_ssot_dir().expect("SSOT");
        let source = ssot.join(DIRECTORY);
        fs::create_dir_all(&source).expect("old SSOT");
        fs::write(source.join("SKILL.md"), OLD_SKILL).expect("old manifest");
        fs::write(source.join("payload.txt"), "old payload").expect("old payload");
        let mut before = InstalledSkill {
            id: SKILL_ID.to_string(),
            name: "Old Audit Skill".to_string(),
            description: Some("previous fixture".to_string()),
            directory: DIRECTORY.to_string(),
            repo_owner: Some("iteration-resources".to_string()),
            repo_name: Some("audit-skills".to_string()),
            repo_branch: Some("main".to_string()),
            readme_url: Some("https://github.com/iteration-resources/audit-skills/blob/main/audit-skill/SKILL.md".to_string()),
            apps: SkillApps { claude: true, codex: true, ..SkillApps::default() },
            installed_at: 1_700_000_000,
            content_hash: None,
            updated_at: 0,
            path: None,
        };
        before.content_hash = Some(SkillService::compute_dir_hash(&source).expect("old hash"));
        db.save_skill(&before).expect("seed DB");
        let claude = SkillService::get_target_skills_dir(&SkillTargetId::Claude)
            .expect("Claude dir")
            .join(DIRECTORY);
        let codex = SkillService::get_target_skills_dir(&SkillTargetId::Codex)
            .expect("Codex dir")
            .join(DIRECTORY);
        assert!(claude.starts_with(temp.path()));
        assert!(codex.starts_with(temp.path()));
        copy_tree(&source, &claude);
        copy_tree(&source, &codex);
        Self {
            _home: home,
            _temp: temp,
            db,
            service: SkillService::new(),
            before,
            ssot,
            claude,
            codex,
        }
    }

    fn install_client(&mut self, addr: SocketAddr) {
        self.service.update_test_client = Some(test_client(addr).expect("isolated TLS client"));
    }

    async fn update(&self) -> Result<InstalledSkill> {
        self.service.update_skill(&self.db, SKILL_ID).await
    }

    fn block_codex_projection(&self) -> Result<PathBuf> {
        let root = SkillService::get_target_skills_dir(&SkillTargetId::Codex)?;
        assert_eq!(root.join(DIRECTORY), self.codex);
        assert!(root.is_absolute() && root.starts_with(self._temp.path()));
        // The managed leaf is replaceable: Copy removes an existing file there.
        // Block the actual app root, where production must create directories.
        // Remove only this fixture's known leaf and then its empty parent.
        fs::remove_dir_all(&self.codex)?;
        fs::remove_dir(&root)?;
        fs::write(&root, "blocked target root")?;
        SkillService::sync_to_app_dir(DIRECTORY, &SkillTargetId::Codex)
            .expect_err("fixture must block the actual production Codex projection");
        assert_eq!(fs::read(&root)?, b"blocked target root");
        Ok(root)
    }
}

struct DenyDns;

impl reqwest::dns::Resolve for DenyDns {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let result: std::result::Result<reqwest::dns::Addrs, Box<dyn Error + Send + Sync>> =
            Err(Box::new(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!("test DNS denied host {}", name.as_str()),
            )));
        Box::pin(async move { result })
    }
}

fn test_client(addr: SocketAddr) -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .use_rustls_tls()
        .tls_built_in_root_certs(false)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .https_only(true)
        .add_root_certificate(reqwest::Certificate::from_pem(CA.as_bytes())?)
        .dns_resolver(Arc::new(DenyDns))
        .resolve("github.com", addr)
        .build()?)
}

fn offline_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .use_rustls_tls()
        .tls_built_in_root_certs(false)
        .no_proxy()
        .redirect(reqwest::redirect::Policy::none())
        .https_only(true)
        .dns_resolver(Arc::new(DenyDns))
        .connect_timeout(std::time::Duration::from_secs(1))
        .timeout(std::time::Duration::from_secs(2))
        .build()?)
}

fn expected_payload() -> Vec<u8> {
    let mut archive =
        zip::ZipArchive::new(std::io::Cursor::new(fixture_zip())).expect("valid fixture zip");
    let mut file = archive
        .by_name("audit-skills-main/audit-skill/payload.txt")
        .expect("fixture payload entry");
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes).expect("read fixture payload");
    bytes
}

fn copy_tree(source: &Path, dest: &Path) {
    fs::create_dir_all(dest).expect("copy target");
    for entry in fs::read_dir(source).expect("read source") {
        let entry = entry.expect("source entry");
        let target = dest.join(entry.file_name());
        if entry.file_type().expect("entry type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).expect("copy file");
        }
    }
}

fn parse_cert(pem: &str) -> CertificateDer<'static> {
    let encoded = pem
        .lines()
        .filter(|line| !line.starts_with("---"))
        .collect::<String>();
    CertificateDer::from(
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .expect("fixture certificate base64"),
    )
}

fn parse_key(pem: &str) -> PrivateKeyDer<'static> {
    let encoded = pem
        .lines()
        .filter(|line| !line.starts_with("---"))
        .collect::<String>();
    PrivateKeyDer::Pkcs8(PrivatePkcs8KeyDer::from(
        base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .expect("fixture private key base64"),
    ))
}

async fn serve_one() -> Result<(SocketAddr, tokio::task::JoinHandle<Result<()>>)> {
    let zip = fixture_zip();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    let config = rustls::ServerConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()?
    .with_no_client_auth()
    .with_single_cert(vec![parse_cert(CERT)], parse_key(KEY))?;
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let task = tokio::spawn(async move {
        let (stream, peer) =
            tokio::time::timeout(std::time::Duration::from_secs(5), listener.accept())
                .await
                .context("loopback fixture TCP accept timed out")?
                .context("loopback fixture TCP accept failed")?;
        assert!(
            peer.ip().is_loopback(),
            "fixture received non-loopback peer"
        );
        let mut stream =
            tokio::time::timeout(std::time::Duration::from_secs(5), acceptor.accept(stream))
                .await
                .context("loopback fixture TLS handshake timed out")?
                .map_err(|error| {
                    // The production branch fallback returns the final connect error.
                    // Keep the first server handshake cause visible even if that error
                    // would otherwise be replaced by a later branch attempt.
                    eprintln!("loopback fixture first TLS handshake failed: {error:?}");
                    anyhow!(error).context("loopback fixture first TLS handshake failed")
                })?;
        let mut request = Vec::new();
        let mut chunk = [0u8; 1024];
        while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
            let read = stream
                .read(&mut chunk)
                .await
                .context("read fixture request")?;
            assert!(
                read > 0 && request.len() + read <= 8192,
                "invalid fixture request"
            );
            request.extend_from_slice(&chunk[..read]);
        }
        let request = String::from_utf8_lossy(&request);
        assert!(
            request
                .starts_with("GET /iteration-resources/audit-skills/archive/refs/heads/main.zip "),
            "unexpected archive request: {request}"
        );
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            zip.len()
        );
        stream
            .write_all(head.as_bytes())
            .await
            .context("write fixture response header")?;
        stream
            .write_all(zip)
            .await
            .context("write exact fixture ZIP")?;
        stream
            .shutdown()
            .await
            .context("close fixture TLS response")?;
        Ok(())
    });
    Ok((addr, task))
}

// Validate the controlled download fixture before interpreting an application
// failure as backup refusal or partial projection. A failed TLS precondition
// must not masquerade as a missing pending receipt or an empty update list.
async fn finish_fixture_request<T>(
    result: Result<T>,
    server: tokio::task::JoinHandle<Result<()>>,
) -> Result<Result<T>> {
    server
        .await
        .context("loopback fixture server task failed")??;
    Ok(result)
}

fn archive_root() -> PathBuf {
    crate::config::get_app_config_dir().join("skill-backups")
}

fn assert_updated(isolated: &Isolated) {
    let current =
        fs::read(isolated.ssot.join(DIRECTORY).join("payload.txt")).expect("SSOT payload");
    assert_eq!(current, expected_payload());
    assert_eq!(
        fs::read(isolated.claude.join("payload.txt")).expect("Claude payload"),
        current
    );
    assert_eq!(
        fs::read(isolated.codex.join("payload.txt")).expect("Codex payload"),
        current
    );
}

#[tokio::test]
#[serial_test::serial]
async fn iteration_resources_skill_update_downloads_backs_up_and_projects_both_targets(
) -> Result<()> {
    let mut isolated = Isolated::new();
    let (addr, server) = serve_one().await?;
    isolated.install_client(addr);
    let denied = isolated
        .service
        .update_test_client
        .as_ref()
        .expect("injected test client")
        .get("https://blocked.invalid/test")
        .send()
        .await
        .expect_err("non-GitHub DNS must be denied");
    assert!(denied.to_string().contains("blocked.invalid"));
    let updated = finish_fixture_request(isolated.update().await, server).await??;

    assert_updated(&isolated);
    assert!(updated.updated_at > 0);
    assert_eq!(
        isolated
            .db
            .get_installed_skill(SKILL_ID)?
            .unwrap()
            .content_hash,
        updated.content_hash
    );
    assert!(!update::is_pending(SKILL_ID)?);
    let backups = archive_root();
    let backup = fs::read_dir(backups)?
        .filter_map(|entry| entry.ok().map(|item| item.path()))
        .find(|path| path.join("skill").join("payload.txt").is_file())
        .expect("old preimage backup");
    assert_eq!(
        fs::read(backup.join("skill").join("payload.txt"))?,
        b"old payload"
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn iteration_resources_skill_update_backup_root_file_refusal_has_zero_writes() -> Result<()> {
    let mut isolated = Isolated::new();
    let ssot = isolated.ssot.join(DIRECTORY);
    let ssot_before = revision(&ssot)?;
    let claude_before = revision(&isolated.claude)?;
    let codex_before = revision(&isolated.codex)?;
    let backup_root = archive_root();
    if backup_root.is_dir() {
        fs::remove_dir_all(&backup_root)?;
    }
    fs::write(&backup_root, "obstruction").context("block backup root with regular file")?;
    let (addr, server) = serve_one().await?;
    isolated.install_client(addr);
    let error = finish_fixture_request(isolated.update().await, server)
        .await?
        .expect_err("backup refusal must stop update");

    assert!(
        error.to_string().contains("UPDATE_BACKUP_FAILED"),
        "{error:#}"
    );
    assert_eq!(fs::read(&backup_root)?, b"obstruction");
    assert_eq!(revision(&ssot)?, ssot_before);
    assert_eq!(revision(&isolated.claude)?, claude_before);
    assert_eq!(revision(&isolated.codex)?, codex_before);
    assert_eq!(
        serde_json::to_value(isolated.db.get_installed_skill(SKILL_ID)?.unwrap())?,
        serde_json::to_value(isolated.before)?
    );
    assert!(!update::is_pending(SKILL_ID)?);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn iteration_resources_skill_update_retry_resumes_only_blocked_target_in_new_service(
) -> Result<()> {
    let mut isolated = Isolated::new();
    let codex_root = isolated.block_codex_projection()?;
    let (addr, server) = serve_one().await?;
    isolated.install_client(addr);
    let error = finish_fixture_request(isolated.update().await, server)
        .await?
        .expect_err("second target should be blocked");
    assert!(error.to_string().contains("UPDATE_INCOMPLETE"), "{error:#}");
    assert!(update::is_pending(SKILL_ID)?);
    let progress = read_pending(SKILL_ID)?.expect("durable progress");
    assert!(
        progress
            .targets
            .iter()
            .find(|target| target.target == SkillTargetId::Claude)
            .expect("Claude progress")
            .applied
    );
    assert!(
        !progress
            .targets
            .iter()
            .find(|target| target.target == SkillTargetId::Codex)
            .expect("Codex progress")
            .applied
    );
    assert_eq!(
        fs::read(isolated.ssot.join(DIRECTORY).join("payload.txt"))?,
        expected_payload()
    );
    assert_eq!(
        fs::read(isolated.claude.join("payload.txt"))?,
        expected_payload()
    );
    assert_eq!(
        isolated
            .db
            .get_installed_skill(SKILL_ID)?
            .unwrap()
            .content_hash,
        SkillService::compute_dir_hash(&isolated.ssot.join(DIRECTORY)).ok()
    );

    // Force retention past its limit with newer unrelated backups. The pending preimage must survive.
    let pending_backup = fs::read_dir(&archive_root())?
        .filter_map(|entry| entry.ok().map(|item| item.path()))
        .find(|path| path.is_dir() && path.join("meta.json").is_file())
        .expect("preimage backup");
    for index in 0..(super::super::SKILL_BACKUP_RETAIN_COUNT + 2) {
        fs::create_dir_all(archive_root().join(format!("retention-fixture-{index}")))?;
        std::thread::sleep(std::time::Duration::from_millis(3));
    }
    SkillService::cleanup_old_skill_backups(&archive_root())?;
    assert!(
        pending_backup.exists(),
        "retention deleted pending recovery preimage"
    );

    let mut checker = SkillService::new();
    checker.update_test_client = Some(offline_client()?);
    assert!(
        checker
            .check_updates(&isolated.db)
            .await?
            .iter()
            .any(|item| item.id == SKILL_ID),
        "pending item must remain retryable after DB metadata advances"
    );
    let first_payload = isolated.claude.join("payload.txt");
    let before_retry = fs::metadata(&first_payload)?.modified()?;
    fs::remove_file(&codex_root)?;
    fs::create_dir_all(&codex_root)?;
    let mut fresh_service = SkillService::new();
    fresh_service.update_test_client = Some(offline_client()?);
    let result = fresh_service.update_skill(&isolated.db, SKILL_ID).await?;
    assert_updated(&isolated);
    assert_eq!(
        fs::metadata(first_payload)?.modified()?,
        before_retry,
        "successful first target was rewritten on retry"
    );
    assert!(!update::is_pending(SKILL_ID)?);
    assert!(result.updated_at > 0);
    let (addr, server) = serve_one().await?;
    let mut checker = SkillService::new();
    checker.update_test_client = Some(test_client(addr)?);
    let checked =
        finish_fixture_request(checker.check_updates(&isolated.db).await, server).await??;
    assert!(
        !checked.iter().any(|item| item.id == SKILL_ID),
        "completed item with matching content must disappear from updates"
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn iteration_resources_skill_update_retry_preserves_external_first_target_edits() -> Result<()>
{
    let mut isolated = Isolated::new();
    let codex_root = isolated.block_codex_projection()?;
    let (addr, server) = serve_one().await?;
    isolated.install_client(addr);
    let error = finish_fixture_request(isolated.update().await, server)
        .await?
        .expect_err("Codex root obstruction must be partial failure");
    assert!(error.to_string().contains("UPDATE_INCOMPLETE"), "{error:#}");

    fs::write(
        isolated.claude.join("payload.txt"),
        "external payload, keep me\n",
    )?;
    fs::write(
        isolated.claude.join(".user-hidden"),
        "external hidden data\n",
    )?;
    fs::remove_file(&codex_root)?;
    fs::create_dir_all(&codex_root)?;
    let mut fresh_service = SkillService::new();
    fresh_service.update_test_client = Some(offline_client()?);
    let error = fresh_service
        .update_skill(&isolated.db, SKILL_ID)
        .await
        .expect_err("external content conflict must remain visible");
    let details: serde_json::Value = serde_json::from_str(&error.to_string())?;
    assert_eq!(details["context"]["conflicted"], "claude", "{details}");
    assert_eq!(
        fs::read(isolated.claude.join("payload.txt"))?,
        b"external payload, keep me\n"
    );
    assert_eq!(
        fs::read(isolated.claude.join(".user-hidden"))?,
        b"external hidden data\n"
    );
    assert_eq!(
        fs::read(isolated.codex.join("payload.txt"))?,
        expected_payload()
    );
    assert!(update::is_pending(SKILL_ID)?);
    let backup = fs::read_dir(archive_root())?
        .filter_map(|entry| entry.ok().map(|item| item.path()))
        .find(|path| path.join("skill").join("payload.txt").is_file())
        .expect("recovery preimage backup was lost");
    assert_eq!(
        fs::read(backup.join("skill").join("payload.txt"))?,
        b"old payload"
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn iteration_resources_skill_retry_respects_changed_assignments_and_finishes_offline(
) -> Result<()> {
    let mut isolated = Isolated::new();
    let codex_root = isolated.block_codex_projection()?;
    let (addr, server) = serve_one().await?;
    isolated.install_client(addr);
    let error = finish_fixture_request(isolated.update().await, server)
        .await?
        .expect_err("Codex update is partial");
    assert!(error.to_string().contains("UPDATE_INCOMPLETE"), "{error:#}");

    fs::remove_file(&codex_root)?;
    fs::create_dir_all(&codex_root)?;
    // Exercise ordinary assignment APIs rather than directly changing the DB.
    SkillService::toggle_app(&isolated.db, SKILL_ID, &AppType::Codex, false)?;
    SkillService::toggle_app(&isolated.db, SKILL_ID, &AppType::Gemini, true)?;
    let gemini = SkillService::get_target_skills_dir(&SkillTargetId::Gemini)?.join(DIRECTORY);
    let first_payload = isolated.claude.join("payload.txt");
    let added_payload = gemini.join("payload.txt");
    let first_mtime = fs::metadata(&first_payload)?.modified()?;
    let added_mtime = fs::metadata(&added_payload)?.modified()?;
    let changed_apps = isolated.db.get_installed_skill(SKILL_ID)?.unwrap().apps;
    assert!(changed_apps.claude && changed_apps.gemini && !changed_apps.codex);

    let mut service = SkillService::new();
    service.update_test_client = Some(offline_client()?);
    assert!(service
        .check_updates(&isolated.db)
        .await?
        .iter()
        .any(|item| item.id == SKILL_ID));
    let result = service.update_skill(&isolated.db, SKILL_ID).await?;
    assert_eq!(result.apps, changed_apps);
    assert_eq!(
        isolated.db.get_installed_skill(SKILL_ID)?.unwrap().apps,
        changed_apps
    );
    assert!(!isolated.codex.exists(), "disabled target was recreated");
    assert_eq!(fs::read(&first_payload)?, expected_payload());
    assert_eq!(fs::read(&added_payload)?, expected_payload());
    assert_eq!(fs::metadata(&first_payload)?.modified()?, first_mtime);
    assert_eq!(fs::metadata(&added_payload)?.modified()?, added_mtime);
    assert!(
        !is_pending(SKILL_ID)?,
        "changed assignments prevented completion"
    );
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn iteration_resources_skill_changed_assignments_preserve_old_and_new_target_edits(
) -> Result<()> {
    let mut isolated = Isolated::new();
    let codex_root = isolated.block_codex_projection()?;
    let (addr, server) = serve_one().await?;
    isolated.install_client(addr);
    let error = finish_fixture_request(isolated.update().await, server)
        .await?
        .expect_err("Codex update is partial");
    assert!(error.to_string().contains("UPDATE_INCOMPLETE"), "{error:#}");

    fs::remove_file(&codex_root)?;
    fs::create_dir_all(&codex_root)?;
    SkillService::toggle_app(&isolated.db, SKILL_ID, &AppType::Codex, false)?;
    SkillService::toggle_app(&isolated.db, SKILL_ID, &AppType::Gemini, true)?;
    let gemini = SkillService::get_target_skills_dir(&SkillTargetId::Gemini)?.join(DIRECTORY);
    fs::write(
        isolated.claude.join("payload.txt"),
        "external Claude edits\n",
    )?;
    fs::write(gemini.join(".user-hidden"), "external Gemini data\n")?;
    let claude_revision = revision(&isolated.claude)?;
    let gemini_revision = revision(&gemini)?;
    let changed_apps = isolated.db.get_installed_skill(SKILL_ID)?.unwrap().apps;

    let mut service = SkillService::new();
    service.update_test_client = Some(offline_client()?);
    let error = service
        .update_skill(&isolated.db, SKILL_ID)
        .await
        .expect_err("external edits must remain visible after assignment changes");
    let details: serde_json::Value = serde_json::from_str(&error.to_string())?;
    assert_eq!(details["code"], "UPDATE_INCOMPLETE");
    assert_eq!(
        details["context"]["conflicted"], "claude,gemini",
        "{details}"
    );
    assert_eq!(
        isolated.db.get_installed_skill(SKILL_ID)?.unwrap().apps,
        changed_apps
    );
    assert!(!isolated.codex.exists(), "disabled target was recreated");
    assert_eq!(revision(&isolated.claude)?, claude_revision);
    assert_eq!(revision(&gemini)?, gemini_revision);
    assert!(is_pending(SKILL_ID)?, "conflict preimage was discarded");
    // A second offline retry must not adopt the newly observed external tree
    // as a preimage and overwrite it on a later attempt.
    service
        .update_skill(&isolated.db, SKILL_ID)
        .await
        .expect_err("conflict remains");
    assert_eq!(revision(&isolated.claude)?, claude_revision);
    assert_eq!(revision(&gemini)?, gemini_revision);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn iteration_resources_skill_assignment_retry_still_rejects_changed_installation(
) -> Result<()> {
    let mut isolated = Isolated::new();
    let codex_root = isolated.block_codex_projection()?;
    let (addr, server) = serve_one().await?;
    isolated.install_client(addr);
    finish_fixture_request(isolated.update().await, server)
        .await?
        .expect_err("Codex update is partial");
    let source = isolated.ssot.join(DIRECTORY);
    let source_revision = revision(&source)?;
    let claude_revision = revision(&isolated.claude)?;
    let mut changed = isolated.db.get_installed_skill(SKILL_ID)?.unwrap();
    changed.installed_at += 1;
    changed.apps.codex = false;
    isolated.db.save_skill(&changed)?;

    let mut service = SkillService::new();
    service.update_test_client = Some(offline_client()?);
    service
        .check_updates(&isolated.db)
        .await
        .expect_err("new installation must not be advertised as a retry");
    service
        .update_skill(&isolated.db, SKILL_ID)
        .await
        .expect_err("assignment reconciliation must not accept a new generation");
    assert_eq!(revision(&source)?, source_revision);
    assert_eq!(revision(&isolated.claude)?, claude_revision);
    assert_eq!(fs::read(&codex_root)?, b"blocked target root");
    assert_eq!(
        serde_json::to_value(isolated.db.get_installed_skill(SKILL_ID)?.unwrap())?,
        serde_json::to_value(changed)?
    );
    assert!(is_pending(SKILL_ID)?);
    Ok(())
}

#[tokio::test]
#[serial_test::serial]
async fn iteration_resources_skill_uninstall_clears_progress_before_restoring_same_id() -> Result<()>
{
    let mut isolated = Isolated::new();
    let codex_root = isolated.block_codex_projection()?;
    let (addr, server) = serve_one().await?;
    isolated.install_client(addr);
    finish_fixture_request(isolated.update().await, server)
        .await?
        .expect_err("update remains partial");
    let preimage = read_pending(SKILL_ID)?.expect("pending preimage").backup_id;
    fs::remove_file(&codex_root)?;
    fs::create_dir_all(&codex_root)?;

    SkillService::uninstall(&isolated.db, SKILL_ID)?;
    assert!(isolated.db.get_installed_skill(SKILL_ID)?.is_none());
    assert!(!is_pending(SKILL_ID)?);
    let restored = SkillService::restore_from_backup_for_target(
        &isolated.db,
        &preimage,
        &SkillTargetId::Claude,
    )?;
    assert_ne!(restored.installed_at, isolated.before.installed_at);
    assert_eq!(
        fs::read(isolated.claude.join("payload.txt"))?,
        b"old payload"
    );

    // A new installation generation must start an ordinary update rather
    // than replay the previous installation's recorded target operations.
    let (addr, server) = serve_one().await?;
    isolated.install_client(addr);
    let updated = finish_fixture_request(isolated.update().await, server).await??;
    assert_eq!(updated.installed_at, restored.installed_at);
    assert_eq!(
        fs::read(isolated.claude.join("payload.txt"))?,
        expected_payload()
    );
    assert!(!is_pending(SKILL_ID)?);
    Ok(())
}
