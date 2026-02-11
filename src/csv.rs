// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! CSV export utilities

use std::fmt;
use std::io::{Cursor, Write};
use std::sync::Arc;

use anyhow::Context;
use riscv_etrace::instruction::bits::Bits;
use riscv_etrace::instruction::info::Info;
use riscv_etrace::{tracer, types};

/// CSV formatter/printer
#[derive(Debug)]
pub struct Writer<W: Write> {
    inner: W,
    buffer: Cursor<Box<[u8]>>,
    fields: Arc<[Field]>,
    src_id: u64,
    context: types::Context,
}

impl<W: Write> Writer<W> {
    pub fn feed(&mut self, item: tracer::Item<(impl Info, Bits)>) -> anyhow::Result<()> {
        use tracer::item::Kind;

        let mut line = Line {
            fields: self.fields.as_ref(),
            src_id: self.src_id,
            pc: item.pc(),
            ..Default::default()
        };

        match item.kind() {
            Kind::Context(c) => {
                self.context = *c;
                return Ok(());
            }
            Kind::Regular(i) => {
                line.insn = i.info.1;
            }
            Kind::Trap(t) => {
                line.exception = true;
                line.interrupt = t.is_interrupt();
                line.cause = t.ecause;
                line.tval = t.tval.unwrap_or_default();
            }
        }

        line.context = self.context;

        let pos = self.buffer.position().try_into()?;
        if write!(self.buffer, "{line}").is_ok() {
            return Ok(());
        }

        self.inner
            .write_all(&self.buffer.get_ref()[..pos])
            .context("Could not write CSV lines to underlying file")?;
        self.buffer.set_position(0);
        write!(self.buffer, "{line}").context("Could not write CSV line to fresh buffer")
    }

    pub fn flush(&mut self) -> anyhow::Result<()> {
        let pos = self.buffer.position().try_into()?;
        if pos != 0 {
            self.inner
                .write_all(&self.buffer.get_ref()[..pos])
                .context("Could not write CSV lines to underlying file")?;
            self.buffer.set_position(0);
        }
        Ok(())
    }
}

/// Helper for writing a single CSV line, including the newline
#[derive(Copy, Clone, Default, Debug)]
struct Line<'f> {
    fields: &'f [Field],
    src_id: u64,
    context: types::Context,
    pc: u64,
    insn: Bits,
    exception: bool,
    interrupt: bool,
    cause: u16,
    tval: u64,
}

impl fmt::Display for Line<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let write = |s: &_, f: &mut fmt::Formatter<'_>| match s {
            Field::Source => write!(f, "{}", self.src_id),
            Field::Address => write!(f, "{:x}", self.pc),
            Field::Instruction => write!(f, "{}", self.insn),
            Field::Privilege => write!(f, "{}", u8::from(self.context.privilege)),
            Field::Context => write!(f, "{}", self.context.context),
            Field::Exception => write!(f, "{}", self.exception as u8),
            Field::Interrupt => write!(f, "{}", self.interrupt as u8),
            Field::ECause => write!(f, "{:x}", self.cause),
            Field::TVal => write!(f, "{:x}", self.tval),
        };

        let mut line = self.fields.iter();
        if let Some(first) = line.next() {
            write(first, f)?;
            line.try_for_each(|s| {
                write!(f, ",")?;
                write(s, f)
            })?;
        };

        writeln!(f)
    }
}

/// CSV field to include
#[derive(Copy, Clone, Debug, clap::ValueEnum)]
pub enum Field {
    /// Source or HART id
    Source,
    /// PC or EPC
    Address,
    /// Instruction
    Instruction,
    /// Privilege
    Privilege,
    /// Debug/trace context
    Context,
    /// Exception flag (also set for interrupts)
    Exception,
    /// Interrupt flag
    Interrupt,
    /// Exception/interrupt cause
    ECause,
    /// Value of the privilege specific `tval` register
    TVal,
}

impl Field {
    /// Retrieve the header to use for this field
    pub fn header(self) -> &'static str {
        match self {
            Self::Source => "SOURCE",
            Self::Address => "ADDRESS",
            Self::Instruction => "INSTRUCTION",
            Self::Privilege => "PRIVILEGE",
            Self::Context => "CONTEXT",
            Self::Exception => "EXCEPTION",
            Self::Interrupt => "INTERRUPT",
            Self::ECause => "ECAUSE",
            Self::TVal => "TVAL",
        }
    }
}

/// Default fields to include in a CSV
pub const DEFAULT_FIELDS: [Field; 8] = [
    Field::Source,
    Field::Address,
    Field::Instruction,
    Field::Privilege,
    Field::Exception,
    Field::ECause,
    Field::TVal,
    Field::Interrupt,
];
