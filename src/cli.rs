// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! CLI parsing types and utilities

use std::path::PathBuf;

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
    #[arg(long, default_value_t, global = true)]
    pub hart_id_width: u8,

    /// Packet-format specific width for the packet's timestamp field
    #[arg(long, default_value_t, global = true)]
    pub ts_width: u8,

    /// Width for the trace type field in SMI packets
    #[arg(long, default_value_t, global = true)]
    pub trace_type_width: u8,

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
