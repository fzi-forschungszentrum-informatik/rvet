// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! General utils

use std::fs::File;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, MutexGuard};

/// A [`File`], encapsulated for exclusive writes
///
/// This wrapper's [`Write::write_all`] is implemented to be thread-safe.
#[derive(Clone, Debug)]
pub struct LockingFile {
    inner: Arc<Mutex<File>>,
}

impl LockingFile {
    /// Create a new locking [`File`]
    pub fn new(inner: File) -> Self {
        Self {
            inner: Arc::new(inner.into()),
        }
    }

    /// Acquire a locked reference to the inner [`File`]
    pub fn lock(&self) -> io::Result<MutexGuard<'_, File>> {
        self.inner.lock().map_err(|_| io::ErrorKind::Other.into())
    }
}

impl Write for LockingFile {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.lock()?.write(buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        self.lock()?.write_all(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.lock()?.flush()
    }
}
