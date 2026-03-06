// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Utilties for handling symbols

/// A symbol covering a range of addresses
#[derive(Copy, Clone, Debug)]
pub struct Symbol {
    name: &'static str,
    symtype: u8,
    bind: u8,
    visibility: u8,
    address: u64,
    size: u64,
}

#[allow(unused)]
impl Symbol {
    /// Retrieve the symbol's name
    pub fn name(&self) -> &'static str {
        self.name
    }

    /// Retrieve the symbol's type
    pub fn symtype(&self) -> u8 {
        self.symtype
    }

    /// Retrieve the symbol's binding attributes
    pub fn bind(&self) -> u8 {
        self.bind
    }

    /// Retrieve the symbol's visibility
    pub fn visibility(&self) -> u8 {
        self.visibility
    }

    /// Retrieve the symbol's (virtual) address
    pub fn address(&self) -> u64 {
        self.address
    }

    /// Retrieve the symbol's size
    pub fn size(&self) -> u64 {
        self.size
    }
}
