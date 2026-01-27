// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use riscv_etrace::packet;

mod cli;
mod reader;

fn main() -> anyhow::Result<()> {
    let args: cli::Cli = clap::Parser::parse();

    let params = args
        .params
        .map(|path| {
            let toml_str = std::fs::read_to_string(path).context("Could not load parameters")?;
            toml::from_str(toml_str.as_ref()).context("Could not parse parameters")
        })
        .transpose()?
        .unwrap_or_default();
    let builder = packet::builder()
        .with_params(&params)
        .for_unit(packet::unit::Plug::default());

    match args.command {
        cli::Command::Payloads { filter, trace } => reader::Reader::new(trace.as_ref(), builder)?
            .with_handler(filter)
            .try_for_each(|p| {
                println!("{:?}", p?);
                Ok(())
            }),
    }
}
