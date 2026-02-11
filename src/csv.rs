// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! CSV export utilities

/// CSV field to include
#[derive(Copy, Clone, Debug, clap::ValueEnum)]
pub enum Field {
    /// Source or HART id
    Source,
    /// PC or EPC
    Address,
    /// Instruction
    Instruction,
    /// Privilege
    Privilege,
    /// Debug/trace context
    Context,
    /// Exception flag (also set for interrupts)
    Exception,
    /// Interrupt flag
    Interrupt,
    /// Exception/interrupt cause
    ECause,
    /// Value of the privilege specific `tval` register
    TVal,
}

impl Field {
    /// Retrieve the header to use for this field
    pub fn header(self) -> &'static str {
        match self {
            Self::Source => "SOURCE",
            Self::Address => "ADDRESS",
            Self::Instruction => "INSTRUCTION",
            Self::Privilege => "PRIVILEGE",
            Self::Context => "CONTEXT",
            Self::Exception => "EXCEPTION",
            Self::Interrupt => "INTERRUPT",
            Self::ECause => "ECAUSE",
            Self::TVal => "TVAL",
        }
    }
}
