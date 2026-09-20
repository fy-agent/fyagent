//! Atomic portable-only Provider draft persistence. No live file or secret writes.
use crate::database::Database;
use crate::services::config_pack::{
    digest, project, PackApp, PackDraftWrite, PackError, PackInventory, PackStoredProvider,
    PortableProvider, Result, DRAFT_CATEGORY,
};
use rusqlite::{params, types::ValueRef, Connection, OptionalExtension};
use serde_json::Value;
use sha2::{Digest, Sha256};

fn unavailable<T>(_: T) -> PackError {
    PackError::StorageUnavailable
}

fn inventory_on(conn: &Connection, local_current: &[String]) -> Result<PackInventory> {
    let credentials_exist: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='provider_credentials')",
        [], |r| r.get(0),
    ).map_err(unavailable)?;
    let mut stmt = conn.prepare(
        "SELECT * FROM providers WHERE app_type IN ('claude', 'codex') ORDER BY app_type, id LIMIT 257"
    ).map_err(unavailable)?;
    let names: Vec<String> = stmt.column_names().iter().map(|n| (*n).into()).collect();
    let mut rows = stmt.query([]).map_err(unavailable)?;
    let mut providers = Vec::new();
    let mut revision = Sha256::new();
    for id in local_current {
        revision.update((id.len() as u64).to_le_bytes());
        revision.update(id.as_bytes());
    }
    while let Some(row) = rows.next().map_err(unavailable)? {
        if providers.len() >= 256 {
            return Err(PackError::Busy);
        }
        // Fingerprint every column, including future credential columns, without
        // exposing raw rows, hashes or secret material to the renderer.
        for (index, name) in names.iter().enumerate() {
            revision.update(name.as_bytes());
            let value = row.get_ref(index).map_err(unavailable)?;
            let bytes = match value {
                ValueRef::Null => vec![0],
                ValueRef::Integer(v) => {
                    let mut b = vec![1];
                    b.extend(v.to_le_bytes());
                    b
                }
                ValueRef::Real(v) => {
                    let mut b = vec![2];
                    b.extend(v.to_le_bytes());
                    b
                }
                ValueRef::Text(v) => {
                    let mut b = vec![3];
                    b.extend(v);
                    b
                }
                ValueRef::Blob(v) => {
                    let mut b = vec![4];
                    b.extend(v);
                    b
                }
            };
            revision.update((bytes.len() as u64).to_le_bytes());
            revision.update(bytes);
        }
        let id: String = row.get("id").map_err(unavailable)?;
        let app: String = row.get("app_type").map_err(unavailable)?;
        let app = if app == "codex" {
            PackApp::Codex
        } else {
            PackApp::Claude
        };
        let name: String = row.get("name").map_err(unavailable)?;
        let raw: String = row.get("settings_config").map_err(unavailable)?;
        let settings: Value = serde_json::from_str(&raw).unwrap_or(Value::Null);
        let meta: String = row.get("meta").map_err(unavailable)?;
        let category: Option<String> = row.get("category").map_err(unavailable)?;
        let active: bool = row.get("is_current").map_err(unavailable)?;
        let failover: bool = row.get("in_failover_queue").map_err(unavailable)?;
        let endpoint_count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM provider_endpoints WHERE provider_id=?1 AND app_type=?2",
                params![id, app.as_str()],
                |r| r.get(0),
            )
            .map_err(unavailable)?;
        let credential_count: i64 = if credentials_exist {
            conn.query_row(
                "SELECT COUNT(*) FROM provider_credentials WHERE provider_id=?1 AND status IN ('pending','ready','revoked')",
                [&id], |r| r.get(0),
            ).map_err(unavailable)?
        } else {
            0
        };
        revision.update(endpoint_count.to_le_bytes());
        revision.update(credential_count.to_le_bytes());
        let portable = project(app, &name, &settings);
        let clean_meta = serde_json::from_str::<Value>(&meta)
            .ok()
            .and_then(|v| v.as_object().map(|o| o.is_empty()))
            .unwrap_or(false);
        let overwrite_allowed = category.as_deref() == Some(DRAFT_CATEGORY)
            && !active
            && !failover
            && credential_count == 0
            && endpoint_count == 0
            && clean_meta
            && !local_current.contains(&format!("{}:{id}", app.as_str()))
            && portable.as_ref().and_then(|p| p.settings().ok()).as_ref() == Some(&settings);
        providers.push(PackStoredProvider {
            selection_id: digest(format!("{}:{id}", app.as_str()).as_bytes()),
            id,
            app,
            name,
            portable,
            overwrite_allowed,
        });
    }
    Ok(PackInventory {
        revision: format!("{:x}", revision.finalize()),
        providers,
    })
}
impl Database {
    pub(crate) fn config_pack_inventory(&self, local_current: &[String]) -> Result<PackInventory> {
        let conn = self.conn.lock().map_err(unavailable)?;
        inventory_on(&conn, local_current)
    }

    pub(crate) fn insert_portable_provider_drafts(
        &self,
        expected_revision: &str,
        local_current: &[String],
        writes: &[PackDraftWrite],
    ) -> Result<Vec<PortableProvider>> {
        let mut names = std::collections::HashSet::new();
        let mut ids = std::collections::HashSet::new();
        if writes.len() > crate::services::config_pack::MAX_ENTRIES
            || writes.iter().any(|w| {
                !names.insert((w.provider.app, &w.provider.name))
                    || !ids.insert((w.provider.app, &w.id))
            })
        {
            return Err(PackError::InvalidPack);
        }
        let mut conn = self.conn.lock().map_err(unavailable)?;
        let tx = conn.transaction().map_err(unavailable)?;
        let before = inventory_on(&tx, local_current)?;
        if before.revision != expected_revision {
            return Err(PackError::StalePreview);
        }
        if before.providers.len() + writes.iter().filter(|w| !w.overwrite).count() > 256 {
            return Err(PackError::Busy);
        }
        for write in writes {
            let p = &write.provider;
            p.validate()?;
            let existing = before
                .providers
                .iter()
                .find(|old| old.id == write.id && old.app == p.app);
            if write.overwrite {
                if !existing.is_some_and(|old| old.overwrite_allowed && old.name == p.name) {
                    return Err(PackError::Conflict);
                }
            } else if existing.is_some()
                || before
                    .providers
                    .iter()
                    .any(|old| old.app == p.app && old.name == p.name)
            {
                return Err(PackError::Conflict);
            }
            let settings = serde_json::to_string(&p.settings()?).map_err(unavailable)?;
            if write.overwrite {
                tx.execute(
                    "UPDATE providers SET name=?1, settings_config=?2 WHERE id=?3 AND app_type=?4 AND is_current=0 AND in_failover_queue=0",
                    params![p.name, settings, write.id, p.app.as_str()],
                ).map_err(|_| PackError::WriteFailed)?;
            } else {
                tx.execute(
                    "INSERT INTO providers(id, app_type, name, settings_config, category, meta, is_current, in_failover_queue)
                     VALUES (?1, ?2, ?3, ?4, ?5, '{}', 0, 0)",
                    params![write.id, p.app.as_str(), p.name, settings, DRAFT_CATEGORY],
                ).map_err(|_| PackError::WriteFailed)?;
            }
        }
        let mut readback = Vec::new();
        for write in writes {
            let actual: Option<(String, String, bool, bool)> = tx.query_row(
                "SELECT name, settings_config, is_current, in_failover_queue FROM providers WHERE id=?1 AND app_type=?2",
                params![write.id, write.provider.app.as_str()],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            ).optional().map_err(|_| PackError::ReadbackFailed)?;
            let Some((name, text, false, false)) = actual else {
                return Err(PackError::ReadbackFailed);
            };
            let settings: Value =
                serde_json::from_str(&text).map_err(|_| PackError::ReadbackFailed)?;
            let p =
                project(write.provider.app, &name, &settings).ok_or(PackError::ReadbackFailed)?;
            if p != write.provider || settings != p.settings()? {
                return Err(PackError::ReadbackFailed);
            }
            readback.push(p);
        }
        // SQLite owns whole-batch rollback on all earlier exits and process loss.
        // Readback occurs while the transaction/connection lock is still held.
        tx.commit().map_err(|_| PackError::WriteFailed)?;
        Ok(readback)
    }
}
