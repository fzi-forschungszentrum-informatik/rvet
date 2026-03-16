// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0

mod binary;
mod cli;
mod csv;
mod pretty;
mod reader;
mod stack;
mod stat;
mod symbols;
mod util;

use std::collections::BTreeMap;
use std::io::{Write, stdout};
use std::num::NonZeroU8;

use anyhow::Context;
use cli_table::{Cell, Table};
use riscv_etrace::instruction::decode::MakeDecode;
use riscv_etrace::{packet, tracer, types};

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
            let mut out = stdout().lock();
            reader::Reader::new(trace.as_ref(), decoder)?
                .with_handler(filter)
                .try_for_each(|p| writeln!(out, "{}", p?).map_err(Into::into))
        }
        cli::Command::Trace {
            filter,
            show_payloads,
            trace,
            program,
        } => {
            let mut reader = reader::Reader::new(trace.as_ref(), decoder)?.with_handler(filter);
            let mut tracer = tracer
                .with_binary(program.build(target)?)
                .build::<riscv_etrace::types::stack::NoStack, _>()
                .context("Could not set up tracer")?;

            let mut printer = pretty::Printer::new(stdout().lock(), &params);
            reader.try_for_each(|p| {
                let payload = p?;
                printer.report(show_payloads.then_some(&payload), false)?;
                let res = tracer
                    .process_payload(&payload)
                    .context("Could not process payload");
                if let Err(err) = res {
                    printer.report(err.chain(), true)?;
                    return tracer.is_recovering().then_some(()).ok_or(err);
                }

                if let Err(err) = process_items(&mut tracer, |i, b| printer.process_item(i, b)) {
                    printer.report(err.chain(), true)?;
                    return tracer.is_recovering().then_some(()).ok_or(err);
                }

                let status = tracer
                    .qual_status()
                    .filter(|s| *s != packet::sync::QualStatus::NoChange);
                printer.report(status, true).map_err(Into::into)
            })
        }
        cli::Command::Csv {
            dispatch,
            trace,
            program,
            csv,
            fields,
        } => std::thread::scope(|scope| {
            let reader = reader::Reader::new(trace.as_ref(), decoder)?.with_handler(dispatch);
            let make_write = csv
                .map(cli::Output::open)
                .transpose()?
                .map(|file| {
                    let file = util::LockingFile::new(file);
                    Box::new(move || Box::new(file.clone()) as Box<dyn Write + Send>)
                        as Box<dyn Fn() -> Box<dyn Write + Send>>
                })
                .unwrap_or(Box::new(|| Box::new(stdout())));
            let mut out = make_write();
            let fields: csv::Fields = fields.into();
            fields.write_header(&mut out)?;

            let res = reader.map(|r| {
                let (src_id, receiver) = r?;
                let mut tracer = tracer
                    .with_binary(program.clone().build(target)?)
                    .build::<riscv_etrace::types::stack::NoStack, _>()
                    .context("Could not set up tracer")?;
                let mut writer = fields.writer(make_write(), src_id);
                anyhow::Ok(scope.spawn(move || {
                    receiver.into_iter().try_for_each(|p| {
                        tracer
                            .process_payload(&p)
                            .context("Could not process payload")?;
                        tracer.by_ref().try_for_each(|i| {
                            let item = i.context("Error during trace")?;
                            writer.feed(item)
                        })
                    })?;
                    writer.flush()
                }))
            });

            let res = collect_threads(res);
            out.flush()?;
            res
        }),
        cli::Command::Stat { trace } => {
            let reader = reader::Reader::new(trace.as_ref(), decoder)?;
            match args.format {
                cli::PacketFormat::Encap => encap_stat(reader),
                cli::PacketFormat::Smi => smi_stat(reader),
            }
        }
        cli::Command::About => {
            let bin_name = env!("CARGO_BIN_NAME");
            let version = env!("CARGO_PKG_VERSION");
            let description = env!("CARGO_PKG_DESCRIPTION");
            let mut out = stdout().lock();
            writeln!(out, "{bin_name} version {version}")?;
            writeln!(out, "{description}")?;
            writeln!(out, "Licensed under the Apache License, Version 2.0")?;
            writeln!(out)?;
            out.write_all(include_str!("../NOTICE").as_bytes())?;
            anyhow::Ok(())
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

/// Process all items produced by this trcer
fn process_items<B, F, E>(
    tracer: &mut tracer::Tracer<B, types::stack::NoStack, binary::Instruction>,
    mut func: F,
) -> anyhow::Result<()>
where
    B: symbols::Provider<binary::Instruction>,
    B::Error: std::error::Error + Send + Sync + 'static,
    F: FnMut(tracer::item::Item<binary::Instruction>, &B) -> Result<(), E>,
    anyhow::Error: From<E>,
{
    while let Some(item) = tracer.next() {
        let item = item.context("Error during trace")?;
        func(item, tracer.binary())?;
    }
    Ok(())
}

/// Collect join handlers and handle anny errors
fn collect_threads<'s>(
    mut threads: impl Iterator<
        Item = anyhow::Result<std::thread::ScopedJoinHandle<'s, anyhow::Result<()>>>,
    >,
) -> anyhow::Result<()> {
    let mut join_handles = Vec::new();
    let res = threads.try_for_each(|t| {
        join_handles.push(t?);
        anyhow::Ok(())
    });

    if res.as_ref().map_err(|e| e.is::<reader::EarlyWorkerExit>()) == Err(false) {
        // We ran into some error during dispatch. Make sure that we
        // report _that_ rather than whtever the workers will report.
        return res;
    }

    // Make sure that all threads finish, and look through them for
    // errors.
    drop(threads);
    join_handles.into_iter().try_for_each(|t| {
        t.join()
            .map_err(|_| anyhow::anyhow!("Error in worker thread"))
            .flatten()
    })?;
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
