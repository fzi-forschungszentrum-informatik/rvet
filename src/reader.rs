// Copyright (C) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Packet/payload reading and decoding utilities

use std::fs::File;

use anyhow::Context;
use riscv_etrace::packet;

use packet::decoder::Decoder;
use packet::unit::Plug;

/// Reader/processor for packets
///
/// Instances read and processe packets using a [`PacketHandler`]. It functions
/// as an [`Iterator`] yielding any handler result that is not [`None`].
pub struct Reader<H: PacketHandler = DefaultHandler> {
    reader: BufReader<File>,
    builder: packet::Builder<Plug>,
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
    ) -> anyhow::Result<Self> {
        File::open(trace_file)
            .map(|f| Self {
                reader: BufReader::new(f),
                builder,
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
            handler,
        }
    }

    /// Read/process a single packet
    ///
    /// Calls [`PacketHandler::handle`] with a new [`Decoder`]. If the result
    /// indicates [insufficient data][packet::error::Error::InsufficientData],
    /// the process is repeated with a larger buffer if possible.
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
            match self.handler.handle(&mut decoder) {
                Ok(r) => {
                    let read = buf.len() - decoder.bytes_left();
                    self.reader.consume(read);
                    if let Some(res) = r {
                        return Ok(Some(res));
                    }
                }
                Err(e) => {
                    if let Some(packet::error::Error::InsufficientData(need)) = e.downcast_ref() {
                        // The buffer did not contain a full packet, so we try
                        // to load at least the amount of data needed to carry
                        // on.
                        let need = buf.len().saturating_add(need.get());

                        let buf = self
                            .reader
                            .peek(need)
                            .context("Could not read from trace file")?;
                        if buf.len() < need {
                            anyhow::bail!("Reached end of file while decoding a packet");
                        }
                    } else {
                        return Err(e);
                    }
                }
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

    /// Decode and handle a single packet
    fn handle(&mut self, decoder: &mut Decoder<'_, Plug>) -> anyhow::Result<Option<Self::Output>>;
}

impl<T: PacketHandler> PacketHandler for &mut T {
    type Output = T::Output;

    fn handle(&mut self, decoder: &mut Decoder<'_, Plug>) -> anyhow::Result<Option<Self::Output>> {
        T::handle(self, decoder)
    }
}

/// A [`PacketHandler`] filtering for tracing payloads emitted by a single source
#[derive(Copy, Clone, Debug)]
pub enum SingleHart {
    /// Decode RISC-V Encapsulation structures
    Encap {
        /// Source to filter for
        src_id: u16,
        /// Accepted flow indicator
        flow: u8,
    },
    /// Decode Siemens Messaging Infrastructure (SMI) packets
    Smi {
        /// Source to filter for
        src_id: u64,
    },
}

impl PacketHandler for SingleHart {
    type Output = packet::payload::Payload<
        <Plug as packet::unit::Unit>::IOptions,
        <Plug as packet::unit::Unit>::DOptions,
    >;

    fn handle(&mut self, decoder: &mut Decoder<'_, Plug>) -> anyhow::Result<Option<Self::Output>> {
        let res = match *self {
            Self::Encap { src_id, flow } => decoder
                .decode_encap_packet()?
                .into_normal()
                .filter(|p| p.src_id() == src_id && p.flow() == flow)
                .map(|p| p.decode_payload())
                .transpose(),
            Self::Smi { src_id } => {
                let packet = decoder.decode_smi_packet()?;
                if packet.hart() == src_id && packet.trace_type().is_some() {
                    packet.decode_payload().map(Some)
                } else {
                    Ok(None)
                }
            }
        };
        res.context("Could not decode payload")
    }
}

impl clap::Args for SingleHart {
    fn augment_args(cmd: clap::Command) -> clap::Command {
        Self::augment_args_for_update(cmd)
    }

    fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
        cmd.arg(clap::arg!(--"src-id" <ID> "Process packets originating from this source"))
            .arg(clap::arg!(--"flow" <ID> "Process packets with this flow indicator"))
    }
}

impl clap::FromArgMatches for SingleHart {
    fn from_arg_matches(matches: &clap::ArgMatches) -> Result<Self, clap::Error> {
        use crate::cli::PacketFormat;

        let format = matches.get_one("format").cloned().unwrap_or_default();
        let mut res = match format {
            PacketFormat::Encap => Self::Encap { src_id: 0, flow: 0 },
            PacketFormat::Smi => Self::Smi { src_id: 0 },
        };
        res.update_from_arg_matches(matches)?;
        Ok(res)
    }

    fn update_from_arg_matches(&mut self, matches: &clap::ArgMatches) -> Result<(), clap::Error> {
        let src_id_value = matches.get_one("src-id").cloned().unwrap_or_default();
        match self {
            Self::Encap { src_id, flow } => {
                *src_id = src_id_value;
                if let Some(f) = matches.get_one("flow") {
                    *flow = *f;
                }
            }
            Self::Smi { src_id } => *src_id = src_id_value.into(),
        }
        Ok(())
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
