//! Fake Codex Provider adapter: one writer call, readback, outbound HTTP spy.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

/// Counts Provider HTTP that the adapter would make. Fake path never increments.
#[derive(Debug, Default)]
pub struct ProviderHttpSpy {
    count: AtomicU64,
}

impl ProviderHttpSpy {
    pub fn new() -> Self {
        Self {
            count: AtomicU64::new(0),
        }
    }

    pub fn record_outbound(&self) {
        self.count.fetch_add(1, Ordering::Relaxed);
    }

    pub fn count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }
}

/// How the single fake writer invocation behaves.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FakeWriterMode {
    Succeed,
    Fail,
}

/// Successful local write that has not yet been read back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WriteReceipt {
    pub resource_count: u16,
}

/// Successful non-sensitive readback match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadbackMatch {
    pub resource_count: u16,
}

/// Writer-side failure. Never a network error: the fake does not dial.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteError {
    WriterAlreadyUsed,
    ManagedWriteFailed,
}

/// Readback-side failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReadbackError {
    Mismatch,
}

/// Process-local fake around the "call writer exactly once" contract.
pub struct FakeProviderAdapter {
    mode: FakeWriterMode,
    writer_count: AtomicU64,
    http: Arc<ProviderHttpSpy>,
}

impl FakeProviderAdapter {
    pub fn succeeding() -> Self {
        Self::with_mode(FakeWriterMode::Succeed)
    }

    pub fn failing() -> Self {
        Self::with_mode(FakeWriterMode::Fail)
    }

    fn with_mode(mode: FakeWriterMode) -> Self {
        Self {
            mode,
            writer_count: AtomicU64::new(0),
            http: Arc::new(ProviderHttpSpy::new()),
        }
    }

    pub fn write_once(&self) -> Result<WriteReceipt, WriteError> {
        let prior = self.writer_count.fetch_add(1, Ordering::AcqRel);
        if prior != 0 {
            return Err(WriteError::WriterAlreadyUsed);
        }
        match self.mode {
            FakeWriterMode::Succeed => Ok(WriteReceipt { resource_count: 1 }),
            FakeWriterMode::Fail => Err(WriteError::ManagedWriteFailed),
        }
    }

    pub fn readback(&self) -> Result<ReadbackMatch, ReadbackError> {
        Ok(ReadbackMatch { resource_count: 1 })
    }

    pub fn writer_count(&self) -> u64 {
        self.writer_count.load(Ordering::Acquire)
    }

    pub fn outbound_http_count(&self) -> u64 {
        self.http.count()
    }
}
