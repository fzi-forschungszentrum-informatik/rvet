// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Callgrind format export utilities

use std::io::Write;

use riscv_etrace::instruction::info::Info;

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
}
