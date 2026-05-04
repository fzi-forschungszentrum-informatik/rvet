// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Callgrind format export utilities

use std::collections::BTreeMap;
use std::io::Write;

use anyhow::Context;
use riscv_etrace::instruction::info::Info;

use super::{Cost, Metrics, Profile};
use crate::stack;
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

    /// Write a [`Profile`]
    pub fn write_profile(&mut self, profile: Profile) -> anyhow::Result<()> {
        // We start with a provile with `Metrics` ordered by stack depth,
        // ascending, and with the _immediate_ accumulated cost for each
        // `Metrics`.
        let mut profile: Vec<_> = profile
            .into_map()
            .into_iter()
            .map(|(s, m)| {
                let cost = m.as_map().iter().fold(Cost::default(), |a, (_, c)| a + *c);
                (s, m, cost)
            })
            .collect();
        profile.shrink_to_fit();
        profile.sort_unstable_by_key(|(s, ..)| s.depth());

        // We accumulate the costs from the leaves into the parent elements,
        // which of course have a lower stack depth.
        let mut search_space = profile.as_mut_slice();
        while let Some((stack, _, cost)) = search_space.split_off_last_mut() {
            if let Some((.., acc)) = stack
                .caller()
                .and_then(|p| search_space.iter_mut().rev().find(|(s, ..)| s == p))
            {
                *acc += *cost;
            }
        }

        // We assemble a map from fn entries to call sites
        let mut calls: BTreeMap<_, BTreeMap<_, Cost>> = Default::default();
        profile.iter().for_each(|(s, _, c)| {
            if let stack::Kind::FnCall { origin, ctx, .. } = s.kind() {
                let cost = calls
                    .entry(ctx.entry())
                    .or_default()
                    .entry((*origin, s.entry()))
                    .or_default();
                *cost += *c;
            }
        });

        // We choose one representative for each fn, merge all other ocurrances
        // into it and generate the corresponding output.
        while !profile.is_empty() {
            let (frame, mut metrics, ..) = profile.remove(0);
            let entry = frame.entry();
            profile
                .extract_if(.., |(s, ..)| s.entry() == entry)
                .for_each(|(_, m, _)| metrics.merge(&m));
            let calls = calls.remove(&entry).unwrap_or_default();
            writeln!(self.inner)?;
            writeln!(self.inner, "fn={}", self.symbols.fn_symbol(entry))
                .context("Could not write fn name")?;
            self.write_metric(&metrics, calls)?;
        }

        Ok(())
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
