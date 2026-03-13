// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Pretty printing and general rendering utilities

use std::fmt;
use std::io::Write;
use std::num::NonZeroU8;

use riscv_etrace::config::Parameters;
use riscv_etrace::instruction;
use riscv_etrace::types::{Context, trap};

use crate::symbols::Symbol;

use instruction::bits::Bits;
use instruction::info::Info;

/// Pretty-printer for [`Item`]s
pub struct Printer<W: Write> {
    out: W,
    context: Option<Context>,
    address_width: NonZeroU8,
    insn_width: NonZeroU8,
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
            insn_width: NonZeroU8::new(32).unwrap(),
            show_context: !params.nocontext_p,
            feed_blank: false,
        }
    }

    /// Process a single regular tracing item
    pub fn process_insn<I: Info + fmt::Display>(
        &mut self,
        pc: u64,
        insn: &instruction::Instruction<(I, Bits)>,
        stack_depth: usize,
        at_fn_entry: bool,
        symbols: impl Iterator<Item = Symbol>,
    ) -> std::io::Result<()> {
        self.feed_blank()?;

        let addr_width = self.address_width.get().into();
        let (insn, bits) = &insn.info;
        let insn_width = self.insn_width.get().into();
        write!(
            self.out,
            "{pc:addr_width$x}  {:<8}  {:<insn_width$}",
            bits.to_string(),
            insn.to_string()
        )?;

        if stack_depth >= 1 {
            let stack_sym = if at_fn_entry {
                '>'
            } else if insn.is_return() {
                '<'
            } else {
                '|'
            };
            write!(self.out, "{stack_sym:|>stack_depth$}")?;
        }

        let mut symbols = symbols.map(|s| s.name()).filter(|n| !n.is_empty());
        if let Some(sym) = symbols.next() {
            write!(self.out, " {sym}")?;
            symbols.try_for_each(|s| write!(self.out, ", {s}"))?;
        }
        writeln!(self.out)
    }

    /// Process a single trap tracing item
    pub fn process_trap(&mut self, pc: u64, info: &trap::Info) -> std::io::Result<()> {
        self.feed_blank()?;

        let addr_width = self.address_width.get().into();
        writeln!(self.out, "{pc:addr_width$x}  {info}")
    }

    /// Process a single context tracing item
    pub fn process_ctx(&mut self, ctx: &Context) -> std::io::Result<()> {
        if self.context.as_ref() == Some(ctx) {
            return Ok(());
        }
        self.context = Some(*ctx);

        self.feed_blank()?;

        let addr_width = self.address_width.get().into();
        let privilege = ctx.privilege;
        write!(self.out, "{:addr_width$}  Context: {privilege}-mode", "")?;
        if self.show_context {
            write!(self.out, ", ctx: {}", ctx.context)?;
        }
        writeln!(self.out)
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

    /// Feed a blank line if pre-scheduled
    fn feed_blank(&mut self) -> std::io::Result<()> {
        if self.feed_blank {
            self.feed_blank = false;
            writeln!(self.out)
        } else {
            Ok(())
        }
    }
}
