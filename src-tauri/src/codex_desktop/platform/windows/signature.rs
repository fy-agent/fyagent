//! Closed Codex package identity and Authenticode admission. No display-name lock.
use super::command::{CommandInvocation, CommandRunner, MAX_OUTPUT};
use crate::codex_desktop::error::{InstallerError, InstallerErrorCode};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    io::{Read, Seek, SeekFrom},
    path::Path,
};

pub(super) const NAME: &str = "OpenAI.Codex";
pub(super) const PUBLISHER: &str = "CN=50BDFD77-8903-4850-9FFE-6E8522F64D5B";
const FAMILY: &str = "OpenAI.Codex_2p2nqsd0c76g0";
const ROOT_SUBJECT: &str = "CN=Microsoft Root Certificate Authority 2011, O=Microsoft Corporation, L=Redmond, S=Washington, C=US";
// Microsoft PKI repository: Microsoft Root Certificate Authority 2011 (SHA-1).
const ROOT_THUMBPRINT: &str = "8F43288AD272F3103B6FB1428485EA3014C0BCFE";
const MAX_ENTRIES: usize = 65_536;
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const SCRIPT: &str = include_str!("authenticode.ps1");

fn rejected(message: &str) -> InstallerError {
    InstallerError::new(InstallerErrorCode::PackageSignatureInvalid)
        .with_diagnostic_message(message)
}

pub(super) fn publisher_id(publisher: &str) -> String {
    let mut hash = Sha256::new();
    for unit in publisher.encode_utf16() {
        hash.update(unit.to_le_bytes());
    }
    let digest = hash.finalize();
    let alphabet = b"0123456789abcdefghjkmnpqrstvwxyz";
    // MSB-first encoding of 64 bits, with one zero padding bit at the end.
    let value = u64::from_be_bytes(digest[..8].try_into().expect("SHA-256 prefix"));
    (0..13)
        .map(|index| {
            let shift = 59_i32 - index * 5;
            let digit = if shift >= 0 {
                (value >> shift) & 31
            } else {
                (value << 1) & 31
            };
            alphabet[digit as usize] as char
        })
        .collect()
}

pub(super) fn check_identity(name: &str, publisher: &str) -> Result<(), InstallerError> {
    if name != NAME
        || publisher != PUBLISHER
        || format!("{name}_{}", publisher_id(publisher)) != FAMILY
    {
        return Err(
            InstallerError::new(InstallerErrorCode::PackageIdentityMismatch)
                .with_diagnostic_message("安装包名称、发布者或包家族名不匹配，已拒绝安装"),
        );
    }
    Ok(())
}

/// Select by archive contents, because both downloaded and bridged packages have
/// a fixed .msix basename even when the payload is an Appx bundle. Never extract.
fn read_manifest(mut file: impl Read + Seek) -> Result<(Vec<u8>, bool), InstallerError> {
    let invalid = || rejected("安装包清单缺失、重复、超出限制或 ZIP 无效，已拒绝安装");
    let expected_entries = bound_zip_directory(&mut file).map_err(|_| invalid())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|_| invalid())?;
    if archive.len() != expected_entries || archive.len() > MAX_ENTRIES {
        return Err(invalid());
    }
    let mut selected = None;
    let mut names = std::collections::HashSet::new();
    for index in 0..archive.len() {
        let entry = archive.by_index_raw(index).map_err(|_| invalid())?;
        let name = entry.name();
        if !names.insert(name.replace('\\', "/").to_ascii_lowercase()) {
            return Err(invalid());
        }
        let bundle = name == "AppxMetadata/AppxBundleManifest.xml";
        if name.eq_ignore_ascii_case("AppxManifest.xml")
            || name.eq_ignore_ascii_case("AppxMetadata/AppxBundleManifest.xml")
        {
            if selected.is_some()
                || !(bundle || name == "AppxManifest.xml")
                || entry.size() > MAX_MANIFEST_BYTES
                || entry.is_dir()
                || entry.encrypted()
            {
                return Err(invalid());
            }
            selected = Some((index, bundle));
        }
    }
    let (index, bundle) = selected.ok_or_else(invalid)?;
    let mut manifest = Vec::new();
    archive
        .by_index(index)
        .map_err(|_| invalid())?
        .take(MAX_MANIFEST_BYTES + 1)
        .read_to_end(&mut manifest)
        .map_err(|_| invalid())?;
    if manifest.len() as u64 > MAX_MANIFEST_BYTES {
        return Err(invalid());
    }
    Ok((manifest, bundle))
}

/// Bound central-directory allocation BEFORE constructing ZipArchive. ZIP64 is
/// allowed, but multipart archives, prepended/trailing data and contradictory
/// directory lengths are rejected. ZipArchive still owns all entry decoding.
fn bound_zip_directory(file: &mut (impl Read + Seek)) -> std::io::Result<usize> {
    use std::io::{Error, ErrorKind};
    let invalid = || Error::new(ErrorKind::InvalidData, "invalid bounded ZIP directory");
    let length = file.seek(SeekFrom::End(0))?;
    let tail_size = length.min(65_557) as usize;
    let mut tail = vec![0; tail_size];
    file.seek(SeekFrom::End(-(tail_size as i64)))?;
    file.read_exact(&mut tail)?;
    let end = (0..tail.len().saturating_sub(21))
        .rev()
        .find(|&offset| {
            tail[offset..].starts_with(b"PK\x05\x06")
                && offset + 22 + u16::from_le_bytes([tail[offset + 20], tail[offset + 21]]) as usize
                    == tail.len()
        })
        .ok_or_else(invalid)?;
    let eocd = &tail[end..];
    let u16_at = |offset| u16::from_le_bytes([eocd[offset], eocd[offset + 1]]) as u64;
    let u32_at = |offset| u32::from_le_bytes(eocd[offset..offset + 4].try_into().unwrap()) as u64;
    if u16_at(4) != 0 || u16_at(6) != 0 || u16_at(8) != u16_at(10) {
        return Err(invalid());
    }
    let mut count = u16_at(10);
    let mut size = u32_at(12);
    let mut start = u32_at(16);
    let mut directory_end = length - tail_size as u64 + end as u64;
    let mut locator = [0; 20];
    if directory_end >= 20 {
        file.seek(SeekFrom::Start(directory_end - 20))?;
        file.read_exact(&mut locator)?;
    }
    if locator.starts_with(b"PK\x06\x07") {
        if locator[4..8] != [0; 4] || locator[16..20] != [1, 0, 0, 0] {
            return Err(invalid());
        }
        let offset = u64::from_le_bytes(locator[8..16].try_into().unwrap());
        let mut record = [0; 56];
        file.seek(SeekFrom::Start(offset))?;
        file.read_exact(&mut record)?;
        let at = |offset| u64::from_le_bytes(record[offset..offset + 8].try_into().unwrap());
        if !record.starts_with(b"PK\x06\x06")
            || at(4) != 44
            || record[16..24] != [0; 8]
            || at(24) != at(32)
            || offset.checked_add(76) != Some(directory_end)
        {
            return Err(invalid());
        }
        if (count != 0xffff && count != at(32))
            || (size != 0xffff_ffff && size != at(40))
            || (start != 0xffff_ffff && start != at(48))
        {
            return Err(invalid());
        }
        count = at(32);
        size = at(40);
        start = at(48);
        directory_end = offset;
    }
    if count > MAX_ENTRIES as u64
        || size > 32 * 1024 * 1024
        || start.checked_add(size) != Some(directory_end)
        || count.saturating_mul(46) > size
    {
        return Err(invalid());
    }
    // Verify every raw central entry fits the declared bound. This also stops
    // crafted name/extra lengths from making ZipArchive read past that bound.
    file.seek(SeekFrom::Start(start))?;
    let mut directory = vec![0; size as usize];
    file.read_exact(&mut directory)?;
    let mut offset = 0usize;
    for _ in 0..count {
        let header = directory.get(offset..offset + 46).ok_or_else(invalid)?;
        if !header.starts_with(b"PK\x01\x02") {
            return Err(invalid());
        }
        let field = |index| u16::from_le_bytes([header[index], header[index + 1]]) as usize;
        offset = offset
            .checked_add(46 + field(28) + field(30) + field(32))
            .ok_or_else(invalid)?;
        if offset > directory.len() {
            return Err(invalid());
        }
    }
    if offset != directory.len() {
        return Err(invalid());
    }
    file.seek(SeekFrom::Start(0))?;
    Ok(count as usize)
}

pub(super) fn verify_package(
    runner: &dyn CommandRunner,
    program: &Path,
    file: impl Read + Seek,
    path: &Path,
) -> Result<(), InstallerError> {
    let (bytes, bundle) = read_manifest(file)?;
    let (name, publisher) = super::manifest::parse_identity(&bytes, bundle)?;
    check_identity(&name, &publisher)?;
    verify_authenticode(runner, program, path)
}

/// Shared by the native bridge handoff and portable flow tests. Ownership is
/// returned only after the full check; rejection consumes exactly this stage.
pub(super) fn admit_staged<T>(
    staged: T,
    verify: impl FnOnce(&T) -> Result<(), InstallerError>,
    cleanup: impl FnOnce(T) -> Result<(), InstallerError>,
) -> Result<T, InstallerError> {
    if let Err(error) = verify(&staged) {
        // Cleanup is capability-based. A changed identity must be retained rather
        // than falling back to deleting a path that might belong to someone else.
        cleanup(staged).map_err(|_| rejected("签名校验失败，暂存清理未完成，已拒绝安装"))?;
        return Err(error);
    }
    Ok(staged)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SignatureEvidence {
    status: String,
    signer: String,
    chain_valid: bool,
    root_subject: String,
    root_thumbprint: String,
}

fn verify_authenticode(
    runner: &dyn CommandRunner,
    program: &Path,
    package: &Path,
) -> Result<(), InstallerError> {
    if !super::command::is_absolute_system_program(program) {
        return Err(rejected("签名校验工具路径无效，已拒绝安装"));
    }
    let output = runner
        .run(&CommandInvocation {
            program: program.to_path_buf(),
            arguments: [
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                SCRIPT,
            ]
            .into_iter()
            .map(Into::into)
            .collect(),
            package_path: package.as_os_str().to_owned(),
        })
        .map_err(|_| rejected("无法完成 Windows 签名校验，已拒绝安装"))?;
    if output.truncated || output.stdout.len() > MAX_OUTPUT || output.stderr.len() > MAX_OUTPUT {
        return Err(rejected("签名校验输出被截断，已拒绝安装"));
    }
    if output.status != Some(0) || !output.stderr.is_empty() {
        return Err(rejected("Windows 签名校验未通过，已拒绝安装"));
    }
    let evidence: SignatureEvidence = serde_json::from_slice(&output.stdout)
        .map_err(|_| rejected("签名校验结果解析失败，已拒绝安装"))?;
    if evidence.status != "Valid"
        || evidence.signer != PUBLISHER
        || !evidence.chain_valid
        || evidence.root_subject != ROOT_SUBJECT
        || evidence.root_thumbprint != ROOT_THUMBPRINT
    {
        return Err(rejected("安装包签名、签名者或微软根证书不匹配，已拒绝安装"));
    }
    Ok(())
}

#[cfg(test)]
pub(super) mod tests {
    use super::super::command::CommandOutput;
    use super::*;
    use std::sync::Mutex;
    pub(crate) const POWERSHELL: &str =
        r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe";
    struct FakeRunner(Mutex<Option<CommandOutput>>, Mutex<Vec<CommandInvocation>>);
    impl CommandRunner for FakeRunner {
        fn run(&self, invocation: &CommandInvocation) -> Result<CommandOutput, ()> {
            self.1.lock().unwrap().push(invocation.clone());
            self.0.lock().unwrap().take().ok_or(())
        }
    }
    fn evidence() -> serde_json::Value {
        serde_json::json!({"status":"Valid", "signer":PUBLISHER, "chain_valid":true, "root_subject":ROOT_SUBJECT, "root_thumbprint":ROOT_THUMBPRINT})
    }
    fn runner(evidence: serde_json::Value) -> FakeRunner {
        FakeRunner(
            Mutex::new(Some(CommandOutput {
                status: Some(0),
                stdout: serde_json::to_vec(&evidence).unwrap(),
                stderr: vec![],
                truncated: false,
            })),
            Mutex::new(vec![]),
        )
    }
    pub(crate) fn valid_runner() -> impl CommandRunner {
        runner(evidence())
    }
    pub(crate) fn valid_package(bundle: bool) -> Vec<u8> {
        package_with_identity(bundle, NAME, PUBLISHER)
    }
    fn package_with_identity(bundle: bool, name: &str, publisher: &str) -> Vec<u8> {
        let (root, namespace, entry) = if bundle {
            (
                "Bundle",
                "http://schemas.microsoft.com/appx/2013/bundle",
                "AppxMetadata/AppxBundleManifest.xml",
            )
        } else {
            (
                "Package",
                "http://schemas.microsoft.com/appx/manifest/foundation/windows10",
                "AppxManifest.xml",
            )
        };
        let xml = format!(
            r#"<?xml version="1.0" encoding="UTF-8"?><{root} xmlns="{namespace}"><Identity Name="{name}" Publisher="{publisher}"/><Properties><DisplayName>Any display name</DisplayName></Properties></{root}>"#
        );
        archive(&[(entry, xml.as_bytes())])
    }
    #[test]
    fn complete_msix_appx_and_bundle_admission_accepts_valid_packages() {
        for (bundle, basename) in [
            (false, "installer.msix"),
            (false, "installer.appx"),
            (true, "installer.msixbundle"),
            (true, "installer.msix"),
        ] {
            let runner = runner(evidence());
            verify_package(
                &runner,
                Path::new(POWERSHELL),
                std::io::Cursor::new(valid_package(bundle)),
                Path::new(basename),
            )
            .unwrap();
            assert_eq!(runner.1.lock().unwrap().len(), 1);
        }
    }
    #[test]
    fn complete_admission_rejects_wrong_identity_before_running_a_command() {
        for bundle in [false, true] {
            for (name, publisher) in [("OpenAI.Other", PUBLISHER), (NAME, "CN=wrong")] {
                let runner = runner(evidence());
                let error = verify_package(
                    &runner,
                    Path::new(POWERSHELL),
                    std::io::Cursor::new(package_with_identity(bundle, name, publisher)),
                    Path::new("installer.msix"),
                )
                .unwrap_err();
                assert_eq!(error.code(), InstallerErrorCode::PackageIdentityMismatch);
                assert!(runner.1.lock().unwrap().is_empty());
            }
        }
    }
    #[test]
    fn complete_admission_rejects_manifest_and_signer_disagreement() {
        let mut wrong = evidence();
        wrong["signer"] = "CN=other".into();
        let error = verify_package(
            &runner(wrong),
            Path::new(POWERSHELL),
            std::io::Cursor::new(valid_package(false)),
            Path::new("installer.msix"),
        )
        .unwrap_err();
        assert_eq!(error.code(), InstallerErrorCode::PackageSignatureInvalid);
    }
    #[test]
    fn malformed_manifest_never_reaches_authenticode() {
        for xml in [b"<Package>".as_slice(), b"not XML", b""] {
            let runner = runner(evidence());
            let error = verify_package(
                &runner,
                Path::new(POWERSHELL),
                std::io::Cursor::new(archive(&[("AppxManifest.xml", xml)])),
                Path::new("installer.msix"),
            )
            .unwrap_err();
            assert_eq!(error.code(), InstallerErrorCode::PackageParseFailed);
            assert!(runner.1.lock().unwrap().is_empty());
        }
    }
    #[test]
    fn bridge_reverification_failure_cleans_only_stage_and_never_installs() {
        use std::{
            fs,
            sync::atomic::{AtomicBool, Ordering},
        };
        // This is the same admission/ownership boundary used by the native helper
        // after its protected, hash-checked bridge copy and before helper launch.
        for failure in ["manifest", "signature", "root", "truncated", "json"] {
            let root = tempfile::tempdir().unwrap();
            let source = root.path().join("download.msix");
            let installed = root.path().join("installed-Codex");
            fs::write(&source, valid_package(false)).unwrap();
            fs::write(&installed, b"old application").unwrap();
            verify_package(
                &valid_runner(),
                Path::new(POWERSHELL),
                fs::File::open(&source).unwrap(),
                &source,
            )
            .unwrap();
            let stage = tempfile::tempdir_in(root.path()).unwrap();
            let staged_path = stage.path().join("installer.msix");
            fs::copy(&source, &staged_path).unwrap();
            assert_eq!(
                Sha256::digest(fs::read(&source).unwrap()),
                Sha256::digest(fs::read(&staged_path).unwrap())
            );
            let runner = runner(evidence());
            match failure {
                "manifest" => fs::write(
                    &staged_path,
                    package_with_identity(false, "OpenAI.Other", PUBLISHER),
                )
                .unwrap(),
                "signature" | "root" => {
                    let mut bad = evidence();
                    bad[if failure == "signature" {
                        "status"
                    } else {
                        "root_subject"
                    }] = "wrong".into();
                    runner.0.lock().unwrap().as_mut().unwrap().stdout =
                        serde_json::to_vec(&bad).unwrap();
                }
                "truncated" => runner.0.lock().unwrap().as_mut().unwrap().truncated = true,
                _ => runner.0.lock().unwrap().as_mut().unwrap().stdout = b"bad json".to_vec(),
            }
            let stage_path = stage.path().to_path_buf();
            let installed_called = AtomicBool::new(false);
            let result = admit_staged(
                stage,
                |_| {
                    verify_package(
                        &runner,
                        Path::new(POWERSHELL),
                        fs::File::open(&staged_path).unwrap(),
                        &staged_path,
                    )
                },
                |stage| stage.close().map_err(|_| rejected("暂存清理失败")),
            )
            .inspect(|_| {
                installed_called.store(true, Ordering::SeqCst);
            });
            assert!(result.is_err(), "{failure}");
            assert!(!installed_called.load(Ordering::SeqCst));
            assert!(!stage_path.exists());
            assert!(source.exists());
            assert_eq!(fs::read(installed).unwrap(), b"old application");
        }
    }
    #[test]
    fn successful_bridge_reverification_returns_stage_for_installation() {
        let stage = tempfile::tempdir().unwrap();
        let path = stage.path().join("installer.msix");
        std::fs::write(&path, valid_package(true)).unwrap();
        let stage = admit_staged(
            stage,
            |_| {
                verify_package(
                    &valid_runner(),
                    Path::new(POWERSHELL),
                    std::fs::File::open(&path).unwrap(),
                    &path,
                )
            },
            |_| panic!("successful verification must retain the stage"),
        )
        .unwrap();
        assert!(path.exists());
        stage.close().unwrap();
    }
    #[test]
    fn native_handoff_and_download_preparation_are_wired_to_full_admission() {
        let helper = include_str!("helper.rs");
        let copy = helper
            .find("let bridge = ProtectedPackageBridge::create(")
            .unwrap();
        let verify = helper[copy..]
            .find("super::signature::admit_staged(")
            .unwrap()
            + copy;
        let launch = helper[verify..]
            .find("match launch_fyagent_user_helper_as_user(")
            .unwrap()
            + verify;
        assert!(copy < verify && verify < launch);
        let admission = &helper[verify..launch];
        assert!(admission.contains("ProtectedPackageBridge::verify_codex_signature"));
        assert!(admission.contains("ProtectedPackageBridge::cleanup"));
        let adapter = include_str!("mod.rs");
        let prepare = adapter.find("fn prepare_install_package<'a>").unwrap();
        let prepare = &adapter[prepare
            ..adapter[prepare..]
                .find("fn install_current_user<'a>")
                .unwrap()
                + prepare];
        assert!(prepare.contains("signature::verify_package("));
        assert!(
            prepare.find("signature::verify_package(").unwrap()
                < prepare
                    .find("PreparedInstallPackage::from_prepared_artifact")
                    .unwrap()
        );
    }
    #[test]
    fn command_scan_has_no_bare_executable_or_path_interpolation() {
        let source = include_str!("command.rs");
        assert!(source.contains("Command::new(&invocation.program)"));
        assert!(
            source
                .find("if !is_absolute_system_program(&invocation.program)")
                .unwrap()
                < source.find("Command::new(&invocation.program)").unwrap()
        );
        for source in [
            include_str!("helper.rs"),
            include_str!("package_bridge.rs"),
            include_str!("runtime.rs"),
            include_str!("deployment.rs"),
        ] {
            assert!(!source.contains("Command::new("));
        }
        assert!(!SCRIPT.contains("Invoke-Expression"));
        assert!(SCRIPT.contains("-LiteralPath $env:FYAGENT_CODEX_SIGNATURE_PATH"));
    }
    #[test]
    fn publisher_id_known_vectors() {
        assert_eq!(publisher_id(PUBLISHER), "2p2nqsd0c76g0");
        assert_eq!(
            publisher_id(
                "CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond, S=Washington, C=US"
            ),
            "8wekyb3d8bbwe"
        );
        check_identity(NAME, PUBLISHER).unwrap();
    }
    #[test]
    fn wrong_publisher_and_package_name_are_rejected() {
        for (name, publisher) in [
            ("OpenAI.Codex.Beta", PUBLISHER),
            ("openai.codex", PUBLISHER),
            (NAME, "CN=wrong"),
            (NAME, ""),
            ("", PUBLISHER),
        ] {
            assert_eq!(
                check_identity(name, publisher).unwrap_err().code(),
                InstallerErrorCode::PackageIdentityMismatch
            );
        }
    }
    #[test]
    fn valid_signature_uses_absolute_command_and_path_is_not_script_text() {
        let runner = runner(evidence());
        let path = Path::new(r"C:\cache\'; $(exit 0); [x]\installer.msix");
        verify_authenticode(&runner, Path::new(POWERSHELL), path).unwrap();
        let calls = runner.1.lock().unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].program, Path::new(POWERSHELL));
        assert_eq!(calls[0].package_path, path.as_os_str());
        assert_eq!(calls[0].arguments.last().unwrap(), SCRIPT);
        assert!(!SCRIPT.contains(path.to_str().unwrap()));
        assert!(SCRIPT.contains("-LiteralPath $env:FYAGENT_CODEX_SIGNATURE_PATH"));
    }
    #[test]
    fn unsigned_nonvalid_wrong_root_and_mismatched_signer_are_rejected() {
        for (key, value) in [
            ("status", "NotSigned"),
            ("status", "HashMismatch"),
            ("status", "UnknownError"),
            ("signer", "CN=wrong"),
            ("root_subject", "CN=Other Root"),
            (
                "root_thumbprint",
                "0000000000000000000000000000000000000000",
            ),
        ] {
            let mut evidence = evidence();
            evidence[key] = value.into();
            assert!(verify_authenticode(
                &runner(evidence),
                Path::new(POWERSHELL),
                Path::new("package.msix")
            )
            .is_err());
        }
        let mut evidence = evidence();
        evidence["chain_valid"] = false.into();
        assert!(verify_authenticode(
            &runner(evidence),
            Path::new(POWERSHELL),
            Path::new("package.msix")
        )
        .is_err());
    }
    #[test]
    fn truncated_malformed_duplicate_missing_output_and_command_failure_reject() {
        for variant in 0..9 {
            let runner = runner(evidence());
            {
                let mut slot = runner.0.lock().unwrap();
                let output = slot.as_mut().unwrap();
                match variant {
                    0 => output.truncated = true,
                    1 => output.stdout.resize(MAX_OUTPUT + 1, b' '),
                    2 => output.stderr.resize(MAX_OUTPUT + 1, b' '),
                    3 => output.stdout = b"not json".to_vec(),
                    4 => output.stdout = br#"{"status":"Valid","status":"Valid"}"#.to_vec(),
                    5 => output.stdout = b"{}".to_vec(),
                    6 => output.status = Some(1),
                    7 => output.stdout = vec![0xff],
                    _ => {
                        slot.take();
                    }
                }
            }
            assert!(
                verify_authenticode(&runner, Path::new(POWERSHELL), Path::new("package.msix"))
                    .is_err(),
                "variant {variant}"
            );
        }
    }
    fn archive(entries: &[(&str, &[u8])]) -> Vec<u8> {
        use std::io::{Cursor, Write};
        let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
        for (name, contents) in entries {
            writer
                .start_file(*name, zip::write::SimpleFileOptions::default())
                .unwrap();
            writer.write_all(contents).unwrap();
        }
        writer.finish().unwrap().into_inner()
    }
    #[test]
    fn bounded_zip_reads_msix_and_bundle_manifests_without_extracting() {
        for (name, bundle) in [
            ("AppxManifest.xml", false),
            ("AppxMetadata/AppxBundleManifest.xml", true),
        ] {
            let bytes = archive(&[(name, b"xml")]);
            let (manifest, actual_bundle) = read_manifest(std::io::Cursor::new(bytes)).unwrap();
            assert_eq!(manifest, b"xml");
            assert_eq!(actual_bundle, bundle);
        }
    }
    fn as_zip64(mut bytes: Vec<u8>, count: u64) -> Vec<u8> {
        let end = bytes.split_off(bytes.len() - 22);
        let offset = bytes.len() as u64;
        let mut record = b"PK\x06\x06".to_vec();
        record.extend_from_slice(&44u64.to_le_bytes());
        record.extend_from_slice(&[45, 0, 45, 0]);
        record.extend_from_slice(&[0; 8]);
        record.extend_from_slice(&count.to_le_bytes());
        record.extend_from_slice(&count.to_le_bytes());
        for start in [12, 16] {
            record.extend_from_slice(
                &(u32::from_le_bytes(end[start..start + 4].try_into().unwrap()) as u64)
                    .to_le_bytes(),
            );
        }
        bytes.extend_from_slice(&record);
        bytes.extend_from_slice(b"PK\x06\x07");
        bytes.extend_from_slice(&[0; 4]);
        bytes.extend_from_slice(&offset.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        let mut end = end;
        end[8..20].fill(0xff);
        bytes.extend_from_slice(&end);
        bytes
    }
    #[test]
    fn zip64_is_supported_but_excessive_directory_count_and_size_reject() {
        let bytes = archive(&[("AppxManifest.xml", b"xml")]);
        let zip64 = as_zip64(bytes.clone(), 1);
        assert_eq!(
            read_manifest(std::io::Cursor::new(zip64)).unwrap().0,
            b"xml"
        );
        assert!(read_manifest(std::io::Cursor::new(as_zip64(
            bytes.clone(),
            MAX_ENTRIES as u64 + 1
        )))
        .is_err());
        let mut oversized = bytes;
        let length = oversized.len();
        oversized[length - 10..length - 6].copy_from_slice(&(33u32 * 1024 * 1024).to_le_bytes());
        assert!(read_manifest(std::io::Cursor::new(oversized)).is_err());
    }
    #[test]
    fn duplicate_raw_zip_entries_and_truncated_central_headers_reject() {
        let mut bytes = archive(&[("AppxManifest.xml", b"xml")]);
        let end = bytes.split_off(bytes.len() - 22);
        let start = u32::from_le_bytes(end[16..20].try_into().unwrap()) as usize;
        let directory = bytes[start..].to_vec();
        bytes.extend_from_slice(&directory);
        let mut end = end;
        end[8..10].copy_from_slice(&2u16.to_le_bytes());
        end[10..12].copy_from_slice(&2u16.to_le_bytes());
        end[12..16].copy_from_slice(&((directory.len() * 2) as u32).to_le_bytes());
        bytes.extend_from_slice(&end);
        assert!(read_manifest(std::io::Cursor::new(bytes)).is_err());
        let mut bytes = archive(&[("AppxManifest.xml", b"xml")]);
        let start = bytes
            .windows(4)
            .position(|window| window == b"PK\x01\x02")
            .unwrap();
        bytes[start + 28..start + 30].copy_from_slice(&u16::MAX.to_le_bytes());
        assert!(read_manifest(std::io::Cursor::new(bytes)).is_err());
    }
    #[test]
    fn cleanup_failure_remains_rejection_and_never_returns_stage() {
        let result = admit_staged(
            (),
            |_| Err(rejected("签名无效")),
            |_| Err(rejected("暂存身份改变")),
        );
        assert_eq!(
            result.unwrap_err().code(),
            InstallerErrorCode::PackageSignatureInvalid
        );
    }
    #[test]
    fn missing_ambiguous_oversized_and_broken_archives_reject() {
        let oversized = vec![b'x'; MAX_MANIFEST_BYTES as usize + 1];
        for bytes in [
            b"broken".to_vec(),
            archive(&[]),
            archive(&[("appxmanifest.xml", b"x")]),
            archive(&[
                ("AppxManifest.xml", b"x"),
                ("AppxMetadata/AppxBundleManifest.xml", b"x"),
            ]),
            archive(&[("AppxManifest.xml", &oversized)]),
        ] {
            assert!(read_manifest(std::io::Cursor::new(bytes)).is_err());
        }
    }
}
