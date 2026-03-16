// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Stack handling and reconstruction

use std::fmt;
use std::num::NonZeroU64;

use riscv_etrace::instruction::{self, Instruction};

use instruction::info::Info;

/// Call stack with reconstruction state
#[derive(Clone, Default, Debug)]
pub struct Stack {
    frames: Vec<Frame>,
    last: Step,
}

impl Stack {
    /// Drive stack reconstruction with the given instruction retirement
    pub fn process_item(&mut self, pc: u64, insn: &Instruction<impl Info>) -> Result<(), Error> {
        match self.last {
            Step::Regular => (),
            Step::Call {
                origin,
                origin_size,
            } => {
                let frame = Frame {
                    origin,
                    origin_size,
                    entry: pc,
                    size: None,
                };
                self.frames.push(frame);
            }
            Step::Return => {
                let Some(frame) = self.frames.pop() else {
                    return Err(Error::NoFrame);
                };

                let expected = frame.return_addr();
                if expected != pc {
                    return Err(Error::OriginMismatch { have: pc, expected });
                }
            }
        }

        self.last = Step::from_insn(pc, insn);
        Ok(())
    }

    /// Retrieve the current stack depth
    ///
    /// Depth increases after the first instruction in a call is processed via
    /// [`process_item`][Self::process_item] and decreases after a return is
    /// processed.
    pub fn depth(&self) -> usize {
        self.frames.len()
    }

    /// Check whether the current state indicates a fn entry
    ///
    /// This fn will return true after a call but before the first instruction
    /// in the called fn is processed via [`process_item`][Self::process_item].
    pub fn at_fn_entry(&self) -> bool {
        matches!(self.last, Step::Call { .. })
    }

    /// Reset this stack
    pub fn reset(&mut self) {
        *self = Default::default()
    }
}

impl AsRef<[Frame]> for Stack {
    fn as_ref(&self) -> &[Frame] {
        self.frames.as_ref()
    }
}

/// A step in the stack reconstruction
#[derive(Copy, Clone, Default, Debug)]
enum Step {
    #[default]
    Regular,
    Call {
        origin: u64,
        origin_size: instruction::Size,
    },
    Return,
}

impl Step {
    /// Create a step from this instruction retirement
    pub fn from_insn(pc: u64, insn: &Instruction<impl Info>) -> Self {
        if insn.is_call() {
            Step::Call {
                origin: pc,
                origin_size: insn.size,
            }
        } else if insn.is_return() {
            Step::Return
        } else {
            Default::default()
        }
    }
}

/// Errors occuring during stack reconstruction
#[derive(Copy, Clone, Debug)]
pub enum Error {
    /// Encountered a fn return on an empty stack
    NoFrame,
    /// The address after a fn return does not match the call site
    OriginMismatch { have: u64, expected: u64 },
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoFrame => write!(f, "Function return with no call site to return to"),
            Self::OriginMismatch { have, expected } => write!(
                f,
                "Mismtach of return address: have {have}, expected {expected}"
            ),
        }
    }
}

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
