// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! CLI parsing types and utilities

use std::fmt;
use std::path::PathBuf;

use anyhow::Context;
use clap::builder::TypedValueParser;
use riscv_etrace::packet::unit;

use crate::binary;
use crate::csv;
use crate::reader::{SingleHart, ThreadDispatch};

#[derive(clap::Parser)]
#[command(version, about)]
pub struct Cli {
    /// Packet format to assume for the trace
    #[arg(value_enum, short, long, default_value_t, global = true)]
    pub format: PacketFormat,

    /// Encoder parameters
    #[arg(short, long, global = true, value_name("TOML"))]
    pub params: Option<PathBuf>,

    /// Packet-format specific width for the hart index/src-id field
    #[arg(
        long,
        default_value_t,
        hide_default_value(true),
        global = true,
        value_name("NUM")
    )]
    pub hart_id_width: u8,

    /// Packet-format specific width for the packet's timestamp field
    #[arg(
        long,
        default_value_t,
        hide_default_value(true),
        global = true,
        value_name("NUM")
    )]
    pub ts_width: u8,

    /// Width for the trace type field in SMI packets
    #[arg(
        long,
        default_value_t,
        hide_default_value(true),
        global = true,
        value_name("NUM")
    )]
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

        /// Trace file
        trace: PathBuf,
    },
    /// Trace a single source
    Trace {
        #[command(flatten)]
        filter: SingleHart,

        /// Show playloads in between trace output
        #[arg(long, action = clap::ArgAction::SetTrue)]
        show_payloads: bool,

        /// Trace file
        trace: PathBuf,

        /// Program binaries to trace
        #[command(flatten)]
        program: binary::Args,
    },
    Csv {
        #[command(flatten)]
        dispatch: ThreadDispatch,

        /// Trace file
        trace: PathBuf,

        /// Program binaries to trace
        #[command(flatten)]
        program: binary::Args,

        /// Path of output file
        #[arg(long, value_name("PATH"), value_parser = Output::value_parser())]
        csv: Option<Output>,

        /// CSV fields
        #[arg(long, value_name("FIELD"))]
        fields: Vec<csv::Field>,
    },
    /// List number of packets for different sources, destinations and types
    Stat {
        /// Trace file
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
#[derive(Copy, Clone, Default, Debug)]
pub struct Unit(unit::PlugsEntry<'static>);

impl From<Unit> for unit::Plug {
    fn from(unit: Unit) -> Self {
        unit.0.plug()
    }
}

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.0.name(), f)
    }
}

impl clap::ValueEnum for Unit {
    fn value_variants<'a>() -> &'a [Self] {
        static UNITS: std::sync::OnceLock<Vec<Unit>> = std::sync::OnceLock::new();
        UNITS.get_or_init(|| unit::PLUGS.iter().copied().map(Unit).collect())
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.0.name()).help(self.0.description()))
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
        use riscv_etrace::instruction::decode::MakeDecode;

        match target {
            Target::Rv32I => Self::rv32i_full(),
            Target::Rv64I => Self::rv64i_full(),
        }
    }
}

/// File to write output to
#[derive(Clone, Debug)]
pub struct Output(PathBuf);

impl Output {
    /// Open this file
    pub fn open(self) -> anyhow::Result<std::fs::File> {
        std::fs::File::create(&self.0)
            .with_context(|| format!("Could not open file '{}' for writing", self.0.display()))
    }

    /// Create a [`TypedValueParser`] for this type
    pub fn value_parser() -> impl TypedValueParser<Value = Self> {
        clap::builder::PathBufValueParser::new().map(Output)
    }
}
