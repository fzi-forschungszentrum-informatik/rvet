// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Stack handling and reconstruction

use std::fmt;
use std::sync::Arc;

use riscv_etrace::instruction::{self, Instruction};

use instruction::info::Info;

/// Call stack with reconstruction state
#[derive(Clone, Default, Debug)]
pub struct State {
    top: Arc<Frame>,
    last: Step,
}

impl State {
    /// Create a new reconstruction state
    pub fn new(entry: u64) -> Self {
        Self {
            top: Frame::new(entry, Kind::Base).into(),
            last: Default::default(),
        }
    }

    /// Drive stack reconstruction with the given instruction retirement
    pub fn process_item(&mut self, pc: u64, insn: &Instruction<impl Info>) -> Result<(), Error> {
        let last = std::mem::replace(&mut self.last, Step::from_insn(pc, insn));
        match last {
            Step::Regular => (),
            Step::Call {
                origin,
                origin_size,
            } => {
                let frame = Frame::new(
                    pc,
                    Kind::FnCall {
                        origin,
                        origin_size,
                        ctx: self.top.clone(),
                    },
                );
                self.top = frame.into();
            }
            Step::Return => {
                if let Some(expected) = self.top.pop()?.return_addr()
                    && expected != pc
                {
                    return Err(Error::OriginMismatch { have: pc, expected });
                }
            }
        }

        Ok(())
    }

    /// Retrieve the current stack
    pub fn stack(&self) -> &Arc<Frame> {
        &self.top
    }

    /// Check whether the current state indicates a fn entry
    ///
    /// This fn will return true after a call but before the first instruction
    /// in the called fn is processed via [`process_item`][Self::process_item].
    pub fn at_fn_entry(&self) -> bool {
        matches!(self.last, Step::Call { .. })
    }
}

impl AsRef<Arc<Frame>> for State {
    fn as_ref(&self) -> &Arc<Frame> {
        self.stack()
    }
}

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
                    entry: pc,
                    kind: Kind::FnCall {
                        origin,
                        origin_size,
                        ctx: Default::default(),
                    },
                };
                self.frames.push(frame);
            }
            Step::Return => {
                let Some(frame) = self.frames.pop() else {
                    return Err(Error::NoFrame);
                };

                if let Some(expected) = frame.return_addr()
                    && expected != pc
                {
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
#[derive(Clone, Default, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub struct Frame {
    entry: u64,
    kind: Kind,
}

impl Frame {
    /// Create a new frame
    fn new(entry: u64, kind: Kind) -> Self {
        Self { entry, kind }
    }

    /// Retrieve the address of the call
    pub fn origin(&self) -> Option<u64> {
        match &self.kind {
            Kind::FnCall { origin, .. } => Some(*origin),
            _ => None,
        }
    }

    /// Retrieve the address this call returns to
    pub fn return_addr(&self) -> Option<u64> {
        match &self.kind {
            Kind::FnCall {
                origin,
                origin_size,
                ..
            } => Some(origin + u64::from(*origin_size)),
            _ => None,
        }
    }

    /// Retrieve the fn's entry address
    ///
    /// Returns the address the call jumped to.
    pub fn entry(&self) -> u64 {
        self.entry
    }

    /// Retrieve this frame's caller
    pub fn caller(&self) -> Option<&Arc<Self>> {
        match &self.kind {
            Kind::FnCall { ctx, .. } => Some(ctx),
            _ => None,
        }
    }

    /// Create an [`Iterator`] over this frame's call stack
    pub fn iter(self: &Arc<Self>) -> impl Iterator<Item = &Arc<Self>> + Clone {
        std::iter::successors(Some(self), |f| f.caller())
    }

    /// Determine the depth of this frame's call stack
    pub fn depth(&self) -> usize {
        std::iter::successors(self.caller(), |f| f.caller()).count()
    }

    /// Remove the topmost frame from this stack
    pub fn pop(self: &mut Arc<Self>) -> Result<Arc<Self>, Error> {
        let caller = self.caller().ok_or(Error::NoFrame)?;
        Ok(std::mem::replace(self, caller.clone()))
    }
}

/// [`Frame`] kind
#[derive(Clone, Default, Debug, Hash, Eq, Ord, PartialEq, PartialOrd)]
pub enum Kind {
    /// Base/entry frame
    #[default]
    Base,
    /// Function call
    FnCall {
        /// Address of the calling instruction
        origin: u64,
        /// Size of the calling instruction
        origin_size: instruction::Size,
        /// Context of this call
        ctx: Arc<Frame>,
    },
}
