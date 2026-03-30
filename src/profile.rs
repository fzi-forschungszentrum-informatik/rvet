// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Profile creation and handling utilities

use std::collections::BTreeMap;
use std::sync::Arc;

use crate::stack::{self, Frame};

/// Profile
///
/// Profiling data for a single call tree.
#[derive(Clone, Default, Debug)]
pub struct Profile {
    inner: BTreeMap<Arc<Frame>, Metrics>,
}

impl Profile {
    /// Retrieve the profiling data as a mapping of stacks to [`Metrics`]
    pub fn as_map(&self) -> &BTreeMap<Arc<Frame>, Metrics> {
        &self.inner
    }

    /// Retrieve the profiling data as a mut mapping of stacks to [`Metrics`]
    pub fn as_map_mut(&mut self) -> &mut BTreeMap<Arc<Frame>, Metrics> {
        &mut self.inner
    }

    /// Convert into a mapping of stacks to [`Metrics`]
    pub fn into_map(self) -> BTreeMap<Arc<Frame>, Metrics> {
        self.inner
    }
}

impl From<Accumulator> for Profile {
    fn from(acc: Accumulator) -> Self {
        acc.inner
    }
}

/// Metrics for a single stack [`Frame`]
#[derive(Clone, Default, Debug)]
pub struct Metrics {
    map: BTreeMap<u64, Cost>,
}

impl Metrics {
    /// Increase the [`Cost`] associated with a PC by single tick
    pub fn tick(&mut self, pc: u64) {
        self.map.entry(pc).or_default().tick();
    }

    /// Merge these metrics with a different one
    pub fn merge(&mut self, other: &Self) {
        self.merge_iter(other.map.iter().map(|(k, v)| (*k, *v)))
    }

    /// Add costs provided via an [`Iterator`]
    pub fn merge_iter(&mut self, map: impl IntoIterator<Item = (u64, Cost)>) {
        map.into_iter().for_each(|(k, v)| {
            let value = self.as_map_mut().entry(k).or_default();
            *value += v;
        });
    }

    /// Retrieve a mapping from PCs to [`Cost`]s
    pub fn as_map(&self) -> &BTreeMap<u64, Cost> {
        &self.map
    }

    /// Retrieve a mutable mapping from PCs to [`Cost`]s
    pub fn as_map_mut(&mut self) -> &mut BTreeMap<u64, Cost> {
        &mut self.map
    }
}

/// Cost associated with a profiled item (e.g. an instruction)
#[derive(Copy, Clone, Default, Debug)]
pub struct Cost {
    /// Ticks
    ///
    /// Ticks are effectively instruction counts.
    pub ticks: u64,
}

impl Cost {
    /// Increase this cost by a single tick
    pub fn tick(&mut self) {
        self.ticks = self.ticks.saturating_add(1);
    }

    /// Check whether this cost is zero
    pub fn is_zero(&self) -> bool {
        self.ticks == 0
    }
}

impl std::ops::Add for Cost {
    type Output = Cost;

    fn add(self, other: Cost) -> Self::Output {
        let ticks = self.ticks.saturating_add(other.ticks);
        Self { ticks }
    }
}

impl std::ops::AddAssign for Cost {
    fn add_assign(&mut self, other: Cost) {
        self.ticks = self.ticks.saturating_add(other.ticks);
    }
}

impl std::ops::Sub for Cost {
    type Output = Cost;

    fn sub(self, other: Cost) -> Self::Output {
        let ticks = self.ticks.saturating_sub(other.ticks);
        Self { ticks }
    }
}

impl std::ops::SubAssign for Cost {
    fn sub_assign(&mut self, other: Cost) {
        self.ticks = self.ticks.saturating_sub(other.ticks);
    }
}

/// Accumulator for profiling data
///
/// This type holds the state for combining fragments of profiling data.
#[derive(Clone, Debug)]
pub struct Accumulator {
    inner: Profile,
    stack: Arc<Frame>,
}

impl Accumulator {
    /// Create a new accumulator with the given starting point
    pub fn new(inner: Profile, stack: Arc<Frame>) -> Self {
        Self { inner, stack }
    }

    /// Absorb a new fragment into the accumulated profile
    pub fn absorb(&mut self, prof: Profile, end: FragmentEnd) -> Result<(), stack::Error> {
        prof.inner.into_iter().for_each(|(k, v)| {
            self.inner
                .as_map_mut()
                .entry(k.rebase(self.stack.clone()))
                .or_default()
                .merge(&v);
        });

        match end {
            FragmentEnd::Stack(stack) => {
                self.stack = stack.rebase(self.stack.clone());
                Ok(())
            }
            FragmentEnd::StepOut(start) => {
                if let Some(expected) = self.stack.pop()?.return_addr()
                    && expected != start
                {
                    return Err(stack::Error::OriginMismatch {
                        have: start,
                        expected,
                    });
                }
                Ok(())
            }
        }
    }
}

/// Specifier of a [`Profile`] fragment's end condition
#[derive(Clone, Debug)]
pub enum FragmentEnd {
    /// The execution for this fragment ended on this stack [`Frame`]
    Stack(Arc<Frame>),
    /// The fragment was ended by returning from the supposed base [`Frame`]
    StepOut(u64),
}
