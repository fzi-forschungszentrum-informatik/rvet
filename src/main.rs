// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0

use std::collections::BTreeMap;
use std::io::Write;
use std::num::NonZeroU8;

use anyhow::Context;
use cli_table::{Cell, Table};
use riscv_etrace::instruction::decode::MakeDecode;
use riscv_etrace::{packet, tracer};

mod binary;
mod cli;
mod pretty;
mod reader;
mod stat;

fn main() -> anyhow::Result<()> {
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
        .unwrap_or_else(|| {
            let mut res = riscv_etrace::config::Parameters::default();
            match args.target {
                Some(cli::Target::Rv32I) => res.iaddress_width_p = NonZeroU8::new(32).unwrap(),
                Some(cli::Target::Rv64I) => res.iaddress_width_p = NonZeroU8::new(64).unwrap(),
                None => {}
            }
            res
        });

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
    let tracer = tracer::builder().with_params(&params);

    let res = match args.command {
        cli::Command::Payloads { filter, trace } => {
            let mut out = std::io::stdout().lock();
            reader::Reader::new(trace.as_ref(), decoder)?
                .with_handler(filter)
                .try_for_each(|p| writeln!(out, "{}", p?).map_err(Into::into))
        }
        cli::Command::Trace {
            filter,
            trace,
            program,
        } => {
            let mut reader = reader::Reader::new(trace.as_ref(), decoder)?.with_handler(filter);
            let mut tracer = tracer
                .with_binary(program.build(target)?)
                .build::<riscv_etrace::types::stack::NoStack, _>()
                .context("Could not set up tracer")?;

            let mut printer = pretty::Printer::new(std::io::stdout().lock(), &params);
            reader.try_for_each(|p| {
                let payload = p?;
                tracer
                    .process_payload(&payload)
                    .context("Could not process payload")?;
                tracer.by_ref().try_for_each(|i| {
                    let item = i.context("Error during trace")?;
                    printer.process_item(item).map_err(anyhow::Error::from)
                })?;

                let status = tracer
                    .qual_status()
                    .filter(|s| *s != packet::sync::QualStatus::NoChange)
                    .into_iter();
                printer.report(status).map_err(Into::into)
            })
        }
        cli::Command::Stat { trace } => {
            let reader = reader::Reader::new(trace.as_ref(), decoder)?;
            match args.format {
                cli::PacketFormat::Encap => encap_stat(reader),
                cli::PacketFormat::Smi => smi_stat(reader),
            }
        }
    };

    if res
        .as_ref()
        .err()
        .and_then(|e| e.downcast_ref::<std::io::Error>())
        .map(|e| e.kind())
        == Some(std::io::ErrorKind::BrokenPipe)
    {
        // The output may be piped into some other program that may exit before
        // we are done producing output. This is expected, especially when we
        // are paging output and produce way more data than the user is actually
        // interested in.
        return Ok(());
    }

    res
}

/// Collect and display stats for a trace file containint encap packets
fn encap_stat(reader: reader::Reader) -> anyhow::Result<()> {
    let mut normal: BTreeMap<_, u64> = Default::default();
    let mut null: BTreeMap<_, u64> = Default::default();
    reader.with_handler(stat::EncapHandler).try_for_each(|h| {
        match h? {
            stat::EncapHeader::Null(n) => *null.entry(n).or_default() += 1,
            stat::EncapHeader::Normal(n) => *normal.entry(n).or_default() += 1,
        };
        anyhow::Ok(())
    })?;
    if !normal.is_empty() {
        println!("Normal encapsulation structures:");
        let table = normal
            .into_iter()
            .map(|(h, v)| [h.flow.cell(), h.src_id.cell(), h.timestamp.cell(), v.cell()])
            .table()
            .title(["Flow", "SrcId", "Timestamp", "Count"]);
        cli_table::print_stdout(table)?;
    }
    if !null.is_empty() {
        println!("Null structures:");
        let table = null
            .into_iter()
            .map(|(h, v)| [h.flow.cell(), h.align.cell(), v.cell()])
            .table()
            .title(["Flow", "Align", "Count"]);
        cli_table::print_stdout(table)?;
    }
    Ok(())
}

/// Collect and display stats for a trace file containint SMI packets
fn smi_stat(reader: reader::Reader) -> anyhow::Result<()> {
    let mut packets: BTreeMap<_, u64> = Default::default();
    reader.with_handler(stat::SmiHandler).try_for_each(|h| {
        *packets.entry(h?).or_default() += 1;
        anyhow::Ok(())
    })?;
    if !packets.is_empty() {
        let table = packets
            .into_iter()
            .map(|(h, v)| {
                [
                    h.trace_type.cell(),
                    h.hart.cell(),
                    h.time_tag.cell(),
                    v.cell(),
                ]
            })
            .table()
            .title(["Trace type", "HART", "Time tag", "Count"]);
        cli_table::print_stdout(table)?;
    }
    Ok(())
}
