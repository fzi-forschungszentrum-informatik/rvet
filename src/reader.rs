// Copyright (C) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Packet/payload reading and decoding utilities

use riscv_etrace::packet;

use packet::decoder::Decoder;
use packet::unit::Plug;

/// Handler for packets
pub trait PacketHandler {
    /// Type of output resulting from processing a single packet
    type Output;

    /// Decode and handle a single packet
    fn handle(&mut self, decoder: &mut Decoder<'_, Plug>) -> anyhow::Result<Option<Self::Output>>;
}

impl<T: PacketHandler> PacketHandler for &mut T {
    type Output = T::Output;

    fn handle(&mut self, decoder: &mut Decoder<'_, Plug>) -> anyhow::Result<Option<Self::Output>> {
        T::handle(self, decoder)
    }
}

/// A dummy [`PacketHandler`]
#[derive(Copy, Clone, Default, Debug)]
pub struct DefaultHandler;

impl PacketHandler for DefaultHandler {
    type Output = ();

    fn handle(&mut self, _decoder: &mut Decoder<'_, Plug>) -> anyhow::Result<Option<Self::Output>> {
        Ok(None)
    }
}

/// Partial reimpl of [`std::io::BufReader`] with associated `peek` fn
struct BufReader<R: std::io::Read> {
    inner: R,
    buffer: Vec<std::mem::MaybeUninit<u8>>,
    pos: usize,
}

impl<R: std::io::Read> BufReader<R> {
    /// Create a new reader
    pub fn new(inner: R) -> Self {
        let buffer = Box::new_uninit_slice(16 * 1024).into();
        Self {
            inner,
            buffer,
            pos: usize::MAX,
        }
    }

    /// Retrieve some number of bytes without advancing the read position
    pub fn peek(&mut self, n: usize) -> std::io::Result<&[u8]> {
        if self.pos.saturating_add(n) > self.buffer.len() {
            let len = self.buffer.len();
            self.buffer.copy_within(self.pos..len, 0);
            let read_base = len - self.pos;
            self.pos = 0;

            let (_, availible) = self.buffer.split_at_mut(read_base);
            let read_len = self.inner.read(unsafe { availible.assume_init_mut() })?;

            self.buffer.truncate(read_base.saturating_add(read_len));
        }

        let availible = &self.buffer[self.pos..];
        let res = &availible[..n];
        Ok(unsafe { res.assume_init_ref() })
    }

    /// Fill the internal buffer if we ran out of data
    fn fill_buf(&mut self) -> std::io::Result<&[u8]> {
        if self.pos >= self.buffer.len() {
            let buf = self.buffer.as_mut_slice();
            let read_len = self.inner.read(unsafe { buf.assume_init_mut() })?;
            self.pos = 0;
            self.buffer.truncate(read_len);
        }

        let res = &self.buffer[self.pos..];
        Ok(unsafe { res.assume_init_ref() })
    }

    /// Consume the given number of bytes
    pub fn consume(&mut self, amount: usize) {
        self.pos = self.pos.saturating_add(amount);
    }
}
