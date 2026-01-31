// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Pretty printing and general rendering utilities

use std::fmt;
use std::num::NonZeroU8;

use riscv_etrace::instruction::info::Info;
use riscv_etrace::tracer::item::{self, Item};

/// A single tracing item, rendered as a line
pub struct ItemLine<I: Info> {
    item: Item<I>,
    address_width: NonZeroU8,
    show_context: bool,
}

impl<I: Info + fmt::Display> fmt::Display for ItemLine<I> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pc = self.item.pc();
        let addr_width = self.address_width.get().into();

        match self.item.kind() {
            item::Kind::Regular(insn) => write!(f, "{pc:0addr_width$x}  {}", insn.info),
            item::Kind::Trap(info) => write!(f, "{pc:0addr_width$x}  {info}"),
            item::Kind::Context(ctx) => {
                let privilege = ctx.privilege;
                write!(f, "{0:addr_width$}  Context: {privilege}-mode", ' ')?;
                if self.show_context {
                    write!(f, " ctx: {}", ctx.context)?;
                }
                Ok(())
            }
        }
    }
}
