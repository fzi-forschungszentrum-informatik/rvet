// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Profile creation and handling utilities

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
