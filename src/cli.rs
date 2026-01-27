// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! CLI parsing types and utilities

#[derive(clap::Parser)]
#[command(version, about)]
pub struct Cli {
    /// Packet format to assume for the trace
    #[arg(value_enum, short, long, default_value_t, global = true)]
    pub format: PacketFormat,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand)]
pub enum Command {}

/// Packet format
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum PacketFormat {
    /// RISC-V Encapsulation structures
    #[default]
    Encap,
    /// Siemens Messaging Infrastructure (SMI)
    Smi,
}
