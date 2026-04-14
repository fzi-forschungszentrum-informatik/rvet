// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Callgrind format export utilities

use std::collections::BTreeMap;
use std::io::Write;

use anyhow::Context;
use riscv_etrace::instruction::info::Info;

use super::{Cost, Metrics};
use crate::symbols::Provider;

/// Writer for profiles in the callgrind format
pub struct Writer<W: Write, P: Provider<I>, I: Info> {
    inner: W,
    symbols: P,
    phantom: std::marker::PhantomData<I>,
}

impl<W: Write, P: Provider<I>, I: Info> Writer<W, P, I> {
    /// Create a new writer
    pub fn new(writer: W, symbols: P) -> Self {
        Self {
            inner: writer,
            symbols,
            phantom: Default::default(),
        }
    }

    /// Write a single set of [`Metrics`]
    fn write_metric(
        &mut self,
        metrics: &Metrics,
        mut calls: BTreeMap<(u64, u64), Cost>,
    ) -> anyhow::Result<()> {
        metrics.as_map().iter().try_for_each(|(k, v)| {
            let mut cost = *v;
            calls
                .extract_if(.., |(a, _), _| a == k)
                .try_for_each(|((_, t), c)| {
                    cost -= c;
                    let ticks = c.ticks;
                    writeln!(self.inner, "cfn={}", self.symbols.fn_symbol(t))?;
                    writeln!(self.inner, "calls={ticks} 0x{t:x}")?;
                    writeln!(self.inner, "0x{k:x} {}", c.ticks)
                })
                .context("Could not write call")?;
            if !cost.is_zero() {
                let ticks = cost.ticks;
                writeln!(self.inner, "0x{k:x} {ticks}").context("Could not write cost line")?;
            }
            Ok(())
        })
    }
}
