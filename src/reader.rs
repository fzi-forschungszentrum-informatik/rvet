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
