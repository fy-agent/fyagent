//! Process-local cancel handles, one-worker-per-job guard, and event sink.
//! Does not retain snapshot, plan, secret, or backup bytes.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};

/// Cooperative cancel flag shared by the coordinator and cancel command.
#[derive(Clone, Debug)]
pub struct CancellationHandle {
    requested: Arc<AtomicBool>,
}

impl CancellationHandle {
    pub fn new() -> Self {
        Self {
            requested: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn request(&self) {
        self.requested.store(true, Ordering::Release);
    }

    pub fn is_requested(&self) -> bool {
        self.requested.load(Ordering::Acquire)
    }
}

impl Default for CancellationHandle {
    fn default() -> Self {
        Self::new()
    }
}

/// Why a worker could not start.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerAcquireError {
    AlreadyActive,
    Poisoned,
}

/// Process-local apply runtime: cancel + single worker + sink, no durable state.
pub struct ApplyRuntime {
    cancel: CancellationHandle,
    worker: Mutex<()>,
    emissions: std::sync::atomic::AtomicU64,
}

impl ApplyRuntime {
    pub fn new() -> Self {
        Self {
            cancel: CancellationHandle::new(),
            worker: Mutex::new(()),
            emissions: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn request_cancel(&self) {
        self.cancel.request();
    }

    pub fn cancel_requested(&self) -> bool {
        self.cancel.is_requested()
    }

    /// One worker per coordinator instance. Does not persist job identity.
    pub fn try_acquire_worker(&self) -> Result<MutexGuard<'_, ()>, WorkerAcquireError> {
        match self.worker.try_lock() {
            Ok(guard) => Ok(guard),
            Err(TryLockError::WouldBlock) => Err(WorkerAcquireError::AlreadyActive),
            Err(TryLockError::Poisoned(_)) => Err(WorkerAcquireError::Poisoned),
        }
    }

    /// Sink records only that a terminal transition was published, not the body.
    pub fn emit_terminal(&self) {
        self.emissions.fetch_add(1, Ordering::Relaxed);
    }
}

impl Default for ApplyRuntime {
    fn default() -> Self {
        Self::new()
    }
}
