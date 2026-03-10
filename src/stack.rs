// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Stack handling and reconstruction

use std::num::NonZeroU64;

use riscv_etrace::instruction;

/// A single stack frame
#[derive(Copy, Clone, Debug)]
pub struct Frame {
    origin: u64,
    origin_size: instruction::Size,
    entry: u64,
    size: Option<NonZeroU64>,
}

impl Frame {
    /// Retrieve the address of the call
    pub fn origin(&self) -> u64 {
        self.origin
    }

    /// Retrieve the address this call returns to
    pub fn return_addr(&self) -> u64 {
        self.origin() + u64::from(self.origin_size)
    }

    /// Retrieve the fn's entry address
    ///
    /// Returns the address the call jumped to.
    pub fn fn_entry(&self) -> u64 {
        self.entry
    }

    /// Retrieve the fns code size in bytes
    pub fn fn_size(&self) -> Option<NonZeroU64> {
        self.size
    }
}
