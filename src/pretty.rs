// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Pretty printing and general rendering utilities

use std::fmt;
use std::num::NonZeroU8;

use riscv_etrace::config::Parameters;
use riscv_etrace::instruction::info::Info;
use riscv_etrace::tracer::item::{self, Item};
use riscv_etrace::types::Context;

/// Generator for [`ItemLine`]s
pub struct ItemGen {
    context: Option<Context>,
    address_width: NonZeroU8,
    show_context: bool,
}

impl ItemGen {
    /// Create a new [`ItemLine`] generator
    ///
    /// The generator will generate lines with some features (e.g. field widths)
    /// tailored to the given [`Parameters`]
    pub fn new(params: &Parameters) -> Self {
        Self {
            context: Default::default(),
            address_width: params.iaddress_width_p.div_ceil(NonZeroU8::new(4).unwrap()),
            show_context: !params.nocontext_p,
        }
    }

    /// Process a single tracing [`Item`]
    pub fn process_item<I: Info>(&mut self, item: Item<I>) -> Option<ItemLine<I>> {
        if let item::Kind::Context(ctx) = item.kind() {
            if Some(ctx) == self.context.as_ref() {
                return None;
            }

            self.context = Some(*ctx);
        }

        Some(ItemLine {
            item,
            address_width: self.address_width,
            show_context: self.show_context,
        })
    }
}

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
