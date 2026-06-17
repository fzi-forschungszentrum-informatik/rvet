// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! General utils

use std::fs::File;
use std::io::{self, Write};
use std::sync::{Arc, Mutex, MutexGuard};

use riscv_etrace::packet::{encap, smi};

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

/// Packet selector
pub trait Selector<P> {
    /// Check whether the given packet matches this selector
    fn matches(&self, packet: &P) -> bool;
}

impl<T> Selector<encap::Normal<T>> for u64 {
    fn matches(&self, packet: &encap::Normal<T>) -> bool {
        u64::from(packet.src_id()) == *self
    }
}

impl<T> Selector<smi::Packet<T>> for u64 {
    fn matches(&self, packet: &smi::Packet<T>) -> bool {
        packet.hart() == *self
    }
}

impl<T> Selector<encap::Normal<T>> for Vec<u64> {
    fn matches(&self, packet: &encap::Normal<T>) -> bool {
        self.is_empty() || self.contains(&packet.src_id().into())
    }
}

impl<T> Selector<smi::Packet<T>> for Vec<u64> {
    fn matches(&self, packet: &smi::Packet<T>) -> bool {
        self.is_empty() || self.contains(&packet.hart())
    }
}

/// A [`Selector`] accepting all packets
#[derive(Copy, Clone, Default, Debug)]
pub struct All;

impl<P> Selector<P> for All {
    fn matches(&self, _packet: &P) -> bool {
        true
    }
}
