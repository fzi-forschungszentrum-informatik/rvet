// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! CLI parsing types and utilities

#[derive(clap::Parser)]
#[command(version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(clap::Subcommand)]
pub enum Command {}
