// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0

mod cli;
mod reader;

fn main() -> anyhow::Result<()> {
    let args: cli::Cli = clap::Parser::parse();

    match args.command {}
}
