// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! CLI parsing types and utilities

use std::fmt;
use std::path::PathBuf;

use riscv_etrace::packet::unit;

use crate::reader::SingleHart;

#[derive(clap::Parser)]
#[command(version, about)]
pub struct Cli {
    /// Packet format to assume for the trace
    #[arg(value_enum, short, long, default_value_t, global = true)]
    pub format: PacketFormat,

    /// Encoder parameters
    #[arg(short, long, global = true)]
    pub params: Option<PathBuf>,

    /// Packet-format specific width for the hart index/src-id field
    #[arg(long, default_value_t, hide_default_value(true), global = true)]
    pub hart_id_width: u8,

    /// Packet-format specific width for the packet's timestamp field
    #[arg(long, default_value_t, hide_default_value(true), global = true)]
    pub ts_width: u8,

    /// Width for the trace type field in SMI packets
    #[arg(long, default_value_t, hide_default_value(true), global = true)]
    pub trace_type_width: u8,

    /// Encoder unit to assume
    #[arg(long, default_value_t, global = true)]
    pub unit: Unit,

    /// Target to assume for raw binaries
    #[arg(value_enum, short, long, global = true)]
    pub target: Option<Target>,

    /// Always display output directly, do not use a pager
    #[cfg(feature = "pager")]
    #[arg(long = "no-pager", action = clap::ArgAction::SetFalse, global = true)]
    pub pager: bool,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand)]
pub enum Command {
    /// Dump the payloads emitted by a single source
    Payloads {
        #[command(flatten)]
        filter: SingleHart,

        #[arg()]
        trace: PathBuf,
    },
}

/// Packet format
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum PacketFormat {
    /// RISC-V Encapsulation structures
    #[default]
    Encap,
    /// Siemens Messaging Infrastructure (SMI)
    Smi,
}

/// Encoder unit (type) representation
#[derive(Copy, Clone, Debug)]
pub struct Unit {
    name: &'static str,
    ctor: fn() -> unit::Plug,
}

impl Default for Unit {
    fn default() -> Self {
        *clap::ValueEnum::value_variants()
            .first()
            .expect("No plugs exist")
    }
}

impl From<Unit> for unit::Plug {
    fn from(unit: Unit) -> Self {
        (unit.ctor)()
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.name, f)
    }
}

impl clap::ValueEnum for Unit {
    fn value_variants<'a>() -> &'a [Self] {
        static UNITS: std::sync::OnceLock<Vec<Unit>> = std::sync::OnceLock::new();
        UNITS.get_or_init(|| {
            unit::PLUGS
                .iter()
                .map(|(name, ctor)| Unit { name, ctor: *ctor })
                .collect()
        })
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.name))
    }
}

/// Target specification
#[derive(Copy, Clone, Debug, PartialEq, Eq, clap::ValueEnum)]
#[value(rename_all = "lower")]
pub enum Target {
    Rv32I,
    Rv64I,
}

impl From<Target> for riscv_isa::Target {
    fn from(target: Target) -> Self {
        use riscv_etrace::instruction::info::MakeDecode;

        match target {
            Target::Rv32I => Self::rv32i_full(),
            Target::Rv64I => Self::rv64i_full(),
        }
    }
}
