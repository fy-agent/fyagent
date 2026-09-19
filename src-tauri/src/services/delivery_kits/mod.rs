mod schema;
#[cfg(test)]
mod tests;
mod validator;

use schema::{canonical, compatible, digest, parse, valid_id, MAX_BYTES};
pub(crate) use schema::{KitError, Manifest, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    time::{Duration, Instant},
};
use uuid::Uuid;
pub(crate) use validator::DemoResult;

const BUILTINS: [&str; 3] = [
    include_str!("../../../resources/delivery-kits/weekly-report.fyagent-kit.json"),
    include_str!("../../../resources/delivery-kits/knowledge-support.fyagent-kit.json"),
    include_str!("../../../resources/delivery-kits/business-query.fyagent-kit.json"),
];
const TTL: Duration = Duration::from_secs(600);

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct KitIdentity {
    pub kit_id: String,
    pub kit_version: String,
    pub manifest_digest: String,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct KitView {
    pub identity: KitIdentity,
    pub manifest: Manifest,
    pub installed: bool,
    pub builtin: bool,
    pub compatible: bool,
    pub exportable: bool,
    pub connections_checked: bool,
}
#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Preview {
    pub preview_id: String,
    pub kind: PreviewKind,
    pub kit: KitView,
    pub conflict: bool,
}
#[derive(Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum PreviewKind {
    Import,
    Export,
}
struct Pending {
    preview: Preview,
    created: Instant,
    bytes: Vec<u8>,
}

pub(crate) struct KitLibrary {
    root: PathBuf,
    pending: HashMap<String, Pending>,
}
impl KitLibrary {
    // No filesystem effects on application startup or catalogue reads when no library exists.
    pub(crate) fn new(root: PathBuf) -> Self {
        Self {
            root,
            pending: HashMap::new(),
        }
    }
    fn builtins() -> Result<Vec<Manifest>> {
        BUILTINS.iter().map(|s| parse(s.as_bytes())).collect()
    }
    fn identity(m: &Manifest) -> Result<KitIdentity> {
        Ok(KitIdentity {
            kit_id: m.id.clone(),
            kit_version: m.version.clone(),
            manifest_digest: digest(&canonical(m)?),
        })
    }
    fn is_builtin(m: &Manifest) -> Result<bool> {
        let id = Self::identity(m)?;
        Ok(Self::builtins()?
            .iter()
            .any(|m| Self::identity(m).as_ref() == Ok(&id)))
    }
    fn path(&self, id: &KitIdentity) -> Result<PathBuf> {
        if !valid_id(&id.kit_id)
            || id.manifest_digest.len() != 64
            || !id
                .manifest_digest
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            return Err(KitError::InvalidPackage);
        }
        let v = semver::Version::parse(&id.kit_version).map_err(|_| KitError::InvalidPackage)?;
        if !v.pre.is_empty() || !v.build.is_empty() || id.kit_version.len() > 32 {
            return Err(KitError::InvalidPackage);
        }
        Ok(self
            .root
            .join(format!("{}--{}.json", id.kit_id, id.kit_version)))
    }
    fn root_exists(&self) -> Result<bool> {
        check_ancestors(&self.root)?;
        match fs::symlink_metadata(&self.root) {
            Ok(m) if m.is_dir() => Ok(true),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
            _ => Err(KitError::LibraryUnavailable),
        }
    }
    fn read_installed(&self, id: &KitIdentity) -> Result<Option<Manifest>> {
        if !self.root_exists()? {
            return Ok(None);
        }
        let path = self.path(id)?;
        if !path
            .try_exists()
            .map_err(|_| KitError::LibraryUnavailable)?
        {
            return Ok(None);
        }
        let m = parse(&read_file(&path)?).map_err(|_| KitError::ReadbackFailed)?;
        if m.id != id.kit_id || m.version != id.kit_version {
            return Err(KitError::ReadbackFailed);
        }
        Ok(Some(m))
    }
    fn view(&self, m: Manifest) -> Result<KitView> {
        let id = Self::identity(&m)?;
        let installed = self
            .read_installed(&id)?
            .is_some_and(|old| Self::identity(&old).as_ref() == Ok(&id));
        let builtin = Self::is_builtin(&m)?;
        Ok(KitView {
            compatible: compatible(&m),
            identity: id,
            manifest: m,
            installed,
            builtin,
            exportable: builtin || installed,
            connections_checked: false,
        })
    }
    pub(crate) fn list(&self) -> Result<Vec<KitView>> {
        let mut manifests = Self::builtins()?;
        let mut total_bytes: usize = BUILTINS.iter().map(|s| s.len()).sum();
        if self.root_exists()? {
            let entries = fs::read_dir(&self.root).map_err(|_| KitError::LibraryUnavailable)?;
            let mut count = 0;
            for entry in entries {
                count += 1;
                if count > 100 {
                    return Err(KitError::LibraryUnavailable);
                }
                let entry = entry.map_err(|_| KitError::LibraryUnavailable)?;
                // Incomplete temporary writes are never published manifests.
                if entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".kit-stage-")
                {
                    continue;
                }
                let bytes = read_file(&entry.path())?;
                total_bytes += bytes.len();
                if total_bytes > 16 * MAX_BYTES {
                    return Err(KitError::LibraryUnavailable);
                }
                let m = parse(&bytes).map_err(|_| KitError::ReadbackFailed)?;
                let id = Self::identity(&m)?;
                if entry.path() != self.path(&id)? {
                    return Err(KitError::ReadbackFailed);
                }
                if !manifests
                    .iter()
                    .any(|x| Self::identity(x).as_ref() == Ok(&id))
                {
                    manifests.push(m);
                }
            }
        }
        manifests.into_iter().map(|m| self.view(m)).collect()
    }
    fn get(&self, id: &KitIdentity) -> Result<Manifest> {
        self.path(id)?;
        if let Some(m) = self.read_installed(id)? {
            if Self::identity(&m)? == *id {
                return Ok(m);
            }
        }
        Self::builtins()?
            .into_iter()
            .find(|m| Self::identity(m).as_ref() == Ok(id))
            .ok_or(KitError::NotFound)
    }
    pub(crate) fn confirm_identity(&self, id: &KitIdentity) -> Result<()> {
        let m = self.get(id)?;
        if !compatible(&m) {
            return Err(KitError::IncompatibleHost);
        }
        Ok(())
    }
    fn preview(&mut self, m: Manifest, kind: PreviewKind) -> Result<Preview> {
        self.pending.retain(|_, p| p.created.elapsed() < TTL);
        if self.pending.len() >= 16 {
            return Err(KitError::Busy);
        }
        let id = Self::identity(&m)?;
        let conflict = self
            .read_installed(&id)?
            .is_some_and(|x| Self::identity(&x).as_ref() != Ok(&id))
            || Self::builtins()?.iter().any(|x| {
                x.id == m.id && x.version == m.version && Self::identity(x).as_ref() != Ok(&id)
            });
        let bytes = canonical(&m)?;
        let preview = Preview {
            preview_id: Uuid::new_v4().to_string(),
            kind,
            kit: self.view(m)?,
            conflict,
        };
        self.pending.insert(
            preview.preview_id.clone(),
            Pending {
                preview: preview.clone(),
                created: Instant::now(),
                bytes,
            },
        );
        Ok(preview)
    }
    pub(crate) fn preview_builtin(&mut self, id: &KitIdentity) -> Result<Preview> {
        let m = self.get(id)?;
        self.preview(m, PreviewKind::Import)
    }
    pub(crate) fn preview_file(&mut self, path: &Path) -> Result<Preview> {
        self.preview_bytes(&read_file(path)?)
    }
    pub(crate) fn preview_bytes(&mut self, bytes: &[u8]) -> Result<Preview> {
        self.preview(parse(bytes)?, PreviewKind::Import)
    }
    pub(crate) fn preview_export(&mut self, id: &KitIdentity) -> Result<Preview> {
        let m = self.get(id)?;
        self.preview(m, PreviewKind::Export)
    }
    fn pending(&self, id: &str, d: &str, kind: PreviewKind) -> Result<&Pending> {
        let p = self.pending.get(id).ok_or(KitError::PreviewExpired)?;
        if p.created.elapsed() >= TTL {
            return Err(KitError::PreviewExpired);
        }
        if p.preview.kind != kind || p.preview.kit.identity.manifest_digest != d {
            return Err(KitError::InvalidPreview);
        }
        Ok(p)
    }
    pub(crate) fn cancel(&mut self, id: &str) {
        self.pending.remove(id);
    }
    pub(crate) fn apply(&mut self, id: &str, d: &str) -> Result<KitView> {
        let p = self.pending(id, d, PreviewKind::Import)?;
        if p.preview.conflict {
            return Err(KitError::ContentConflict);
        }
        if !p.preview.kit.compatible {
            return Err(KitError::IncompatibleHost);
        }
        let m = parse(&p.bytes)?;
        let identity = Self::identity(&m)?;
        let path = self.path(&identity)?;
        self.root_exists()?;
        if let Some(existing) = self.read_installed(&identity)? {
            if Self::identity(&existing)? != identity {
                return Err(KitError::ContentConflict);
            }
            return self.view(existing);
        }
        let catalogue = self.list()?;
        let catalogue_bytes = catalogue.iter().try_fold(0usize, |total, item| {
            canonical(&item.manifest).map(|bytes| total + bytes.len())
        })?;
        if catalogue.len() >= 100 || catalogue_bytes + p.bytes.len() > 16 * MAX_BYTES {
            return Err(KitError::Busy);
        }
        fs::create_dir_all(&self.root).map_err(|_| KitError::WriteFailed)?;
        self.root_exists()?;
        #[cfg(target_os = "macos")]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&self.root, fs::Permissions::from_mode(0o700))
                .map_err(|_| KitError::WriteFailed)?;
        }
        let mut stage = tempfile::Builder::new()
            .prefix(".kit-stage-")
            .tempfile_in(&self.root)
            .map_err(|_| KitError::WriteFailed)?;
        stage
            .write_all(&p.bytes)
            .and_then(|_| stage.as_file().sync_all())
            .map_err(|_| KitError::WriteFailed)?;
        // Atomic create-only publish: another process cannot replace an existing version.
        if let Err(e) = stage.persist_noclobber(&path) {
            if e.error.kind() != std::io::ErrorKind::AlreadyExists {
                return Err(KitError::WriteFailed);
            }
        }
        let actual = self
            .read_installed(&identity)?
            .ok_or(KitError::ReadbackFailed)?;
        if Self::identity(&actual)? != identity {
            return Err(KitError::ContentConflict);
        }
        self.view(actual)
    }
    pub(crate) fn export_bytes(&self, id: &str, d: &str) -> Result<Vec<u8>> {
        let p = self.pending(id, d, PreviewKind::Export)?;
        self.get(&p.preview.kit.identity)?;
        Ok(p.bytes.clone())
    }
    pub(crate) fn run(&self, id: &KitIdentity) -> Result<DemoResult> {
        let m = self.get(id)?;
        if !compatible(&m) {
            return Err(KitError::IncompatibleHost);
        }
        // Only reviewed synthetic fixtures can produce the local_fixture classification.
        // An imported self-declaration is not evidence that data are synthetic.
        if !Self::is_builtin(&m)? {
            return Err(KitError::UnsupportedValidator);
        }
        validator::run(&m)
    }
}

fn check_ancestors(path: &Path) -> Result<()> {
    if !path.is_absolute()
        || path
            .components()
            .any(|c| matches!(c, std::path::Component::ParentDir))
    {
        return Err(KitError::UnsafeContent);
    }
    for ancestor in path.ancestors() {
        match fs::symlink_metadata(ancestor) {
            Ok(m) if m.file_type().is_symlink() => return Err(KitError::UnsafeContent),
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(_) => return Err(KitError::LibraryUnavailable),
        }
    }
    Ok(())
}
fn read_file(path: &Path) -> Result<Vec<u8>> {
    check_ancestors(path)?;
    let metadata = fs::symlink_metadata(path).map_err(|_| KitError::LibraryUnavailable)?;
    if !metadata.is_file() || metadata.len() > MAX_BYTES as u64 {
        return Err(KitError::InvalidPackage);
    }
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|_| KitError::LibraryUnavailable)?
        .take(MAX_BYTES as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| KitError::LibraryUnavailable)?;
    if bytes.len() > MAX_BYTES {
        return Err(KitError::InvalidPackage);
    }
    Ok(bytes)
}
pub(crate) fn export_file(path: &Path, bytes: &[u8]) -> Result<()> {
    check_ancestors(path)?;
    if !path
        .file_name()
        .and_then(|s| s.to_str())
        .is_some_and(|s| s.ends_with(".fyagent-kit.json"))
    {
        return Err(KitError::InvalidPackage);
    }
    if path.exists() {
        return Err(KitError::ContentConflict);
    }
    let parent = path.parent().ok_or(KitError::UnsafeContent)?;
    let mut stage = tempfile::NamedTempFile::new_in(parent).map_err(|_| KitError::WriteFailed)?;
    stage
        .write_all(bytes)
        .and_then(|_| stage.as_file().sync_all())
        .map_err(|_| KitError::WriteFailed)?;
    stage
        .persist_noclobber(path)
        .map_err(|_| KitError::WriteFailed)?;
    if read_file(path)? != bytes {
        return Err(KitError::ReadbackFailed);
    }
    Ok(())
}
