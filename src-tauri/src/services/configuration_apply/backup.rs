//! Job-owned in-process backup receipt. No wire serialization, no SQLite.

use std::sync::atomic::{AtomicBool, Ordering};

/// Closed resource kinds the fake adapter may declare.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupResourceKind {
    CodexLiveConfig,
}

/// Process-local receipt that a backup bundle was committed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackupReceipt {
    pub resource_count: u16,
}

/// Why a fake backup could not be created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackupError {
    NoResources,
}

/// In-memory backup store used by the fake coordinator.
pub struct FakeBackupStore {
    available: AtomicBool,
}

impl FakeBackupStore {
    pub fn new() -> Self {
        Self {
            available: AtomicBool::new(false),
        }
    }

    pub fn create_from_declared_resources(
        &self,
        resources: &[BackupResourceKind],
    ) -> Result<BackupReceipt, BackupError> {
        let resource_count = match u16::try_from(resources.len()) {
            Ok(0) | Err(_) => return Err(BackupError::NoResources),
            Ok(count) => count,
        };
        self.available.store(true, Ordering::Release);
        Ok(BackupReceipt { resource_count })
    }

    pub fn is_available(&self) -> bool {
        self.available.load(Ordering::Acquire)
    }
}

impl Default for FakeBackupStore {
    fn default() -> Self {
        Self::new()
    }
}
