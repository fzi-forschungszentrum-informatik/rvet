// Copyright (C) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Packet/payload reading and decoding utilities

use std::collections::{HashMap, hash_map};
use std::fmt;
use std::fs::File;
use std::sync::mpsc;

use anyhow::Context;
use riscv_etrace::packet::{self, encap, smi};

use crate::cli;
use crate::util::Selector;

use packet::decoder::Decoder;
use packet::unit::Plug;

/// Reader/processor for packets
///
/// Instances read and processe packets using a [`PacketHandler`]. It functions
/// as an [`Iterator`] yielding any handler result that is not [`None`].
pub struct Reader<H: PacketHandler = DefaultHandler> {
    reader: BufReader<File>,
    builder: packet::Builder<Plug>,
    format: cli::PacketFormat,
    handler: H,
}

impl Reader {
    /// Create a new reader
    ///
    /// Attempts to open the trace file at `trace_file`. On success a reader is
    /// constructed with the opened file, `builder` and a default handler.
    pub fn new(
        trace_file: &std::path::Path,
        builder: packet::Builder<Plug>,
        format: cli::PacketFormat,
    ) -> anyhow::Result<Self> {
        File::open(trace_file)
            .map(|f| Self {
                reader: BufReader::new(f),
                builder,
                format,
                handler: Default::default(),
            })
            .with_context(|| format!("Could not open trace file {}", trace_file.display()))
    }
}

impl<H: PacketHandler> Reader<H> {
    /// Replaces the handler of this reader
    pub fn with_handler<N: PacketHandler>(self, handler: N) -> Reader<N> {
        Reader {
            reader: self.reader,
            builder: self.builder,
            format: self.format,
            handler,
        }
    }

    /// Read/process a single packet
    ///
    /// Performs initial packet decode with a new [`Decoder`] and passes it to
    /// the appropriate [`PacketHandler`] fn. If the packet decode errors due to
    /// [insufficient data][packet::error::Error::InsufficientData], the process
    /// is repeated with a larger buffer if possible.
    ///
    /// # Note
    ///
    /// Users are expected to use the [`Iterator`] interface instead if
    /// applicable.
    pub fn read_packet(&mut self) -> anyhow::Result<Option<H::Output>> {
        loop {
            let buf = self
                .reader
                .fill_buf()
                .context("Could not read from trace file")?;
            if buf.is_empty() {
                return Ok(None);
            }

            let mut decoder = self.builder.decoder(buf);
            let res = match self.format {
                cli::PacketFormat::Encap => decoder.decode().map(|p| self.handler.handle_encap(p)),
                cli::PacketFormat::Smi => decoder.decode().map(|p| self.handler.handle_smi(p)),
            };
            match res {
                Ok(r) => {
                    let read = buf.len() - decoder.bytes_left();
                    self.reader.consume(read);
                    if !matches!(r, Ok(None)) {
                        return r;
                    }
                }
                Err(packet::error::Error::InsufficientData(need)) => {
                    // The buffer did not contain a full packet, so we try to
                    // load at least the amount of data needed to carry on.
                    let need = buf.len().saturating_add(need.get());

                    let buf = self
                        .reader
                        .peek(need)
                        .context("Could not read from trace file")?;
                    if buf.len() < need {
                        anyhow::bail!("Reached end of file while decoding a packet");
                    }
                }
                Err(e) => return Err(anyhow::Error::new(e).context("Could not decode packet")),
            };
        }
    }
}

impl<H: PacketHandler> Iterator for Reader<H> {
    type Item = anyhow::Result<H::Output>;

    fn next(&mut self) -> Option<Self::Item> {
        self.read_packet().transpose()
    }
}

/// Handler for packets
pub trait PacketHandler {
    /// Type of output resulting from processing a single packet
    type Output;

    /// Handle an [`encap::Packet`]
    fn handle_encap(
        &mut self,
        packet: encap::Packet<Decoder<'_, Plug>>,
    ) -> anyhow::Result<Option<Self::Output>>;

    /// Handle an [`smi::Packet`]
    fn handle_smi(
        &mut self,
        packet: smi::Packet<Decoder<'_, Plug>>,
    ) -> anyhow::Result<Option<Self::Output>>;

    /// Decode and handle a single packet
    fn handle(&mut self, decoder: &mut Decoder<'_, Plug>) -> anyhow::Result<Option<Self::Output>>;
}

impl<T: PacketHandler> PacketHandler for &mut T {
    type Output = T::Output;

    fn handle_encap(
        &mut self,
        packet: encap::Packet<Decoder<'_, Plug>>,
    ) -> anyhow::Result<Option<Self::Output>> {
        T::handle_encap(self, packet)
    }

    fn handle_smi(
        &mut self,
        packet: smi::Packet<Decoder<'_, Plug>>,
    ) -> anyhow::Result<Option<Self::Output>> {
        T::handle_smi(self, packet)
    }

    fn handle(&mut self, decoder: &mut Decoder<'_, Plug>) -> anyhow::Result<Option<Self::Output>> {
        T::handle(self, decoder)
    }
}

/// A [`PacketHandler`] filtering for tracing payloads emitted by a single source
#[derive(Copy, Clone, Debug)]
pub struct SingleHart {
    selector: cli::CommonSelector,
    src_id: u64,
}

impl SingleHart {
    /// Create a handler from configuration
    pub fn new(selector: cli::CommonSelector, src_id: u64) -> Self {
        Self { selector, src_id }
    }
}

impl PacketHandler for SingleHart {
    type Output = Payload;

    fn handle_encap(
        &mut self,
        packet: encap::Packet<Decoder<'_, Plug>>,
    ) -> anyhow::Result<Option<Self::Output>> {
        packet
            .into_normal()
            .filter(|p| self.selector.matches(p) && self.src_id.matches(p))
            .map(|p| p.decode_payload())
            .transpose()
            .context("Could not decode payload")
    }

    fn handle_smi(
        &mut self,
        packet: smi::Packet<Decoder<'_, Plug>>,
    ) -> anyhow::Result<Option<Self::Output>> {
        if self.selector.matches(&packet) && self.src_id.matches(&packet) {
            packet
                .decode_payload()
                .map(Some)
                .context("Could not decode payload")
        } else {
            Ok(None)
        }
    }

    fn handle(&mut self, decoder: &mut Decoder<'_, Plug>) -> anyhow::Result<Option<Self::Output>> {
        match self.format {
            cli::PacketFormat::Encap => self.handle_encap(decoder.decode()?),
            cli::PacketFormat::Smi => self.handle_smi(decoder.decode()?),
        }
    }
}

/// A [`PacketHandler`] dispatching to a separate threads for each source id
#[derive(Clone, Debug)]
pub struct ThreadDispatch {
    targets: HashMap<u64, mpsc::SyncSender<Payload>>,
    selector: cli::CommonSelector,
    src_id: Vec<u64>,
}

impl ThreadDispatch {
    /// Create a handler from configuration
    pub fn new(selector: cli::CommonSelector, src_id: Vec<u64>) -> Self {
        Self {
            targets: Default::default(),
            selector,
            src_id,
        }
    }

    /// Retrieve the target to which dispatch payloads with the given source id
    fn dispatch(
        &mut self,
        src_id: u64,
        payload: impl TryInto<Payload, Error = packet::error::Error>,
    ) -> anyhow::Result<Option<(u64, mpsc::Receiver<Payload>)>> {
        use hash_map::Entry;

        match self.targets.entry(src_id) {
            Entry::Occupied(e) => {
                let payload = payload.try_into()?;
                e.into_mut().send(payload).map_err(|_| EarlyWorkerExit)?;
                Ok(None)
            }
            Entry::Vacant(e) => {
                if !self.src_id.is_empty() && !self.src_id.contains(&src_id) {
                    return Ok(None);
                }

                let (sender, receiver) = mpsc::sync_channel(1024);
                let payload = payload.try_into()?;
                e.insert(sender)
                    .send(payload)
                    .map_err(|_| EarlyWorkerExit)?;
                Ok(Some((src_id, receiver)))
            }
        }
    }
}

impl PacketHandler for ThreadDispatch {
    type Output = (u64, mpsc::Receiver<Payload>);

    fn handle_encap(
        &mut self,
        packet: encap::Packet<Decoder<'_, Plug>>,
    ) -> anyhow::Result<Option<Self::Output>> {
        let Some(packet) = packet.into_normal().filter(|p| self.selector.matches(p)) else {
            return Ok(None);
        };
        let src_id = packet.src_id().into();
        self.dispatch(src_id, packet)
    }

    fn handle_smi(
        &mut self,
        packet: smi::Packet<Decoder<'_, Plug>>,
    ) -> anyhow::Result<Option<Self::Output>> {
        if !self.selector.matches(&packet) {
            return Ok(None);
        }
        let src_id = packet.hart();
        self.dispatch(src_id, packet)
    }

    fn handle(&mut self, decoder: &mut Decoder<'_, Plug>) -> anyhow::Result<Option<Self::Output>> {
        match self.format {
            cli::PacketFormat::Encap => self.handle_encap(decoder.decode()?),
            cli::PacketFormat::Smi => self.handle_smi(decoder.decode()?),
        }
    }
}

/// Kind of [`ThreadDispatch`]
#[derive(Copy, Clone, Debug)]
pub enum TDKind {
    /// Decode RISC-V Encapsulation structures
    Encap(u8),
    /// Decode Siemens Messaging Infrastructure (SMI) packets
    Smi,
}

/// A dummy [`PacketHandler`]
#[derive(Copy, Clone, Default, Debug)]
pub struct DefaultHandler;

impl PacketHandler for DefaultHandler {
    type Output = ();

    fn handle_encap(
        &mut self,
        _packet: encap::Packet<Decoder<'_, Plug>>,
    ) -> anyhow::Result<Option<Self::Output>> {
        Ok(None)
    }

    fn handle_smi(
        &mut self,
        _packet: smi::Packet<Decoder<'_, Plug>>,
    ) -> anyhow::Result<Option<Self::Output>> {
        Ok(None)
    }

    fn handle(&mut self, _decoder: &mut Decoder<'_, Plug>) -> anyhow::Result<Option<Self::Output>> {
        Ok(None)
    }
}

/// Error type signalling early worker exit
#[derive(Copy, Clone, Default, Debug)]
pub struct EarlyWorkerExit;

impl std::error::Error for EarlyWorkerExit {}

impl fmt::Display for EarlyWorkerExit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Worker thread existted prematurely")
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
        let res = availible
            .split_at_checked(n)
            .map(|(b, _)| b)
            .unwrap_or(availible);
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

pub type Payload = packet::payload::Payload<
    <Plug as packet::unit::Unit>::IOptions,
    <Plug as packet::unit::Unit>::DOptions,
>;
