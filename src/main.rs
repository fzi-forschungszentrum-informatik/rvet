// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0

use anyhow::Context;
use riscv_etrace::packet;

mod binary;
mod cli;
mod pretty;
mod reader;

fn main() -> anyhow::Result<()> {
    use std::io::Write;

    use riscv_etrace::instruction::info::MakeDecode;

    let args: cli::Cli = clap::Parser::parse();

    #[cfg(feature = "pager")]
    if args.pager {
        pager::Pager::new().setup();
    }

    let params = args
        .params
        .map(|path| {
            let toml_str = std::fs::read_to_string(path).context("Could not load parameters")?;
            toml::from_str(toml_str.as_ref()).context("Could not parse parameters")
        })
        .transpose()?
        .unwrap_or_default();

    let target: riscv_isa::Target = args
        .target
        .map(Into::into)
        .or_else(|| MakeDecode::infer_from_params(&params))
        .unwrap_or_else(MakeDecode::rv32i_full);

    let decoder = packet::builder()
        .with_params(&params)
        .with_hart_index_width(args.hart_id_width)
        .with_timestamp_width(args.ts_width)
        .with_trace_type_width(args.trace_type_width)
        .for_unit(args.unit.into());

    match args.command {
        cli::Command::Payloads { filter, trace } => {
            let mut out = std::io::stdout().lock();
            reader::Reader::new(trace.as_ref(), decoder)?
                .with_handler(filter)
                .try_for_each(|p| writeln!(out, "{}", p?).map_err(Into::into))
        }
    }
}
