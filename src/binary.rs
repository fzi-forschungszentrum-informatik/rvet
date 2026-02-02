// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Utilties for loading and assembling binaries

use riscv_etrace::binary;

/// Type of binary produced by the builder
pub type Binary = binary::boxed::Binary<'static, riscv_isa::Instruction>;
