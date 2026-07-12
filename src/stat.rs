// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Utilities for gathering of "statistics"

use riscv_etrace::packet::{self, encap, smi};

use crate::reader::PacketHandler;

use packet::decoder::Decoder;
use packet::unit::Plug;

/// Header fields of a RISC-V encapsulation packet
#[derive(Copy, Clone, Debug)]
pub enum EncapHeader {
    Null(EncapNullHeader),
    Normal(EncapNormalHeader),
}

impl<P> From<encap::Packet<P>> for EncapHeader {
    fn from(packet: encap::Packet<P>) -> Self {
        match packet {
            encap::Packet::NullIdle { flow } => Self::Null(EncapNullHeader { flow, align: false }),
            encap::Packet::NullAlign { flow } => Self::Null(EncapNullHeader { flow, align: true }),
            encap::Packet::Normal(n) => Self::Normal(n.into()),
        }
    }
}

/// Header fields of a RISC-V null encapsulation structure
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EncapNullHeader {
    pub flow: u8,
    pub align: bool,
}

/// Header fields of a RISC-V normal encapsulation structure
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct EncapNormalHeader {
    pub flow: u8,
    pub src_id: u16,
    pub timestamp: bool,
}

impl<P> From<encap::Normal<P>> for EncapNormalHeader {
    fn from(packet: encap::Normal<P>) -> Self {
        Self {
            flow: packet.flow(),
            src_id: packet.src_id(),
            timestamp: packet.timestamp().is_some(),
        }
    }
}

/// [`PacketHandler`] for extracting [`EncapHeader`]s
#[derive(Copy, Clone, Debug, Default)]
pub struct EncapHandler;

impl PacketHandler for EncapHandler {
    type Output = EncapHeader;

    fn handle_encap(
        &mut self,
        packet: encap::Packet<Decoder<'_, Plug>>,
    ) -> anyhow::Result<Option<Self::Output>> {
        Ok(Some(packet.into()))
    }

    fn handle_smi(
        &mut self,
        _packet: smi::Packet<Decoder<'_, Plug>>,
    ) -> anyhow::Result<Option<Self::Output>> {
        Err(anyhow::anyhow!("SMI stat handler cannot handle SMI packet"))
    }
}

/// Header fields of a SMI packet
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SmiHeader {
    pub trace_type: u8,
    pub hart: u64,
    pub time_tag: bool,
}

impl<P> From<smi::Packet<P>> for SmiHeader {
    fn from(packet: smi::Packet<P>) -> Self {
        Self {
            trace_type: packet.raw_trace_type(),
            hart: packet.hart(),
            time_tag: packet.time_tag().is_some(),
        }
    }
}

/// [`PacketHandler`] for extracting [`EncapHeader`]s
#[derive(Copy, Clone, Debug, Default)]
pub struct SmiHandler;

impl PacketHandler for SmiHandler {
    type Output = SmiHeader;

    fn handle_encap(
        &mut self,
        _packet: encap::Packet<Decoder<'_, Plug>>,
    ) -> anyhow::Result<Option<Self::Output>> {
        Err(anyhow::anyhow!(
            "SMI stat handler cannot handle encap packet"
        ))
    }

    fn handle_smi(
        &mut self,
        packet: smi::Packet<Decoder<'_, Plug>>,
    ) -> anyhow::Result<Option<Self::Output>> {
        Ok(Some(packet.into()))
    }
}
