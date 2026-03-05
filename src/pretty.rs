// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Pretty printing and general rendering utilities

use std::fmt;
use std::io::Write;
use std::num::NonZeroU8;

use riscv_etrace::config::Parameters;
use riscv_etrace::instruction;
use riscv_etrace::tracer::item::{self, Item};
use riscv_etrace::types::Context;

use instruction::bits::Bits;
use instruction::info::Info;

/// Pretty-printer for [`Item`]s
pub struct Printer<W: Write> {
    out: W,
    context: Option<Context>,
    address_width: NonZeroU8,
    show_context: bool,
    feed_blank: bool,
}

impl<W: Write> Printer<W> {
    /// Create a new pretty-printer
    pub fn new(out: W, params: &Parameters) -> Self {
        Self {
            out,
            context: Default::default(),
            address_width: params.iaddress_width_p.div_ceil(NonZeroU8::new(4).unwrap()),
            show_context: !params.nocontext_p,
            feed_blank: false,
        }
    }

    /// Process a single tracing [`Item`]
    pub fn process_item(
        &mut self,
        item: Item<(impl Info + fmt::Display, Bits)>,
    ) -> std::io::Result<()> {
        let pc = item.pc();
        let addr_width = self.address_width.get().into();

        if self.feed_blank {
            self.feed_blank = false;
            writeln!(self.out)?;
        }

        match item.kind() {
            item::Kind::Regular(insn) => {
                let (insn, bits) = &insn.info;
                writeln!(
                    self.out,
                    "{pc:addr_width$x}  {:<8}  {insn}",
                    bits.to_string()
                )
            }
            item::Kind::Trap(info) => writeln!(self.out, "{pc:addr_width$x}  {info}"),
            item::Kind::Context(ctx) if self.context.as_ref() != Some(ctx) => {
                self.context = Some(*ctx);
                let privilege = ctx.privilege;
                write!(self.out, "{0:addr_width$}  Context: {privilege}-mode", "")?;
                if self.show_context {
                    write!(self.out, ", ctx: {}", ctx.context)?;
                }
                writeln!(self.out)
            }
            _ => Ok(()),
        }
    }

    /// Report in a way that mixes well with the pretty output
    pub fn report<L>(&mut self, lines: L, feed_blank: bool) -> std::io::Result<()>
    where
        L: IntoIterator,
        L::Item: fmt::Display,
    {
        let mut lines = lines.into_iter();
        if let Some(first) = lines.next() {
            if self.feed_blank && !feed_blank {
                writeln!(self.out)?;
            }

            self.feed_blank = feed_blank;
            writeln!(self.out, "--- {first}")?;
            lines.try_for_each(|e| writeln!(self.out, "    {e}"))?;
        }
        Ok(())
    }
}
