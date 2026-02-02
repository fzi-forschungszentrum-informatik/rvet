// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Utilities for gathering of "statistics"

use riscv_etrace::packet::encap;

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
