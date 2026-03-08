// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Utilties for handling symbols

use anyhow::Context;
use riscv_etrace::binary::{self, Binary};
use riscv_etrace::instruction::info::Info;

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
    /// Create a new symbol from an [`elf::symbol::Symbol`]
    pub fn new(
        symbol: elf::symbol::Symbol,
        elf: &elf::ElfBytes<'static, impl elf::endian::EndianParse>,
        strings: &elf::string_table::StringTable<'static>,
    ) -> anyhow::Result<Option<Self>> {
        let address = match symbol.st_shndx {
            elf::abi::SHN_ABS => symbol.st_value,
            elf::abi::SHN_LORESERVE..=elf::abi::SHN_HIRESERVE => return Ok(None),
            _ if elf.ehdr.e_type != elf::abi::ET_REL => symbol.st_value,
            shndx => elf
                .section_headers()
                .context("Could not access section header table")?
                .get(shndx.into())
                .context("Could not access header of section {shndx}")?
                .sh_addr
                .checked_add(symbol.st_value)
                .context("Invalid address")?,
        };

        let name = symbol
            .st_name
            .try_into()
            .context("Could not retrieve name for symbol")?;
        let name = strings
            .get(name)
            .context("Could not retrieve name for symbol")?;
        Ok(Some(Self {
            name,
            symtype: symbol.st_symtype(),
            bind: symbol.st_bind(),
            visibility: symbol.st_vis(),
            address,
            size: symbol.st_size,
        }))
    }

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

    /// Check whether this symbol potentially refers to code objects
    ///
    /// Returns `true` if the symbol's type indicates a function or no type. The
    /// latter covers labels.
    pub fn is_code(&self) -> bool {
        use elf::abi;

        matches!(
            self.symtype(),
            abi::STT_FUNC | abi::STT_GNU_IFUNC | abi::STT_NOTYPE
        )
    }
}

/// Provider for symbols
pub trait Provider<I: Info>: Binary<I> {
    /// Retrieve [`Symbol`]s with the given (start) address
    ///
    /// Returns an [`Iterator`] over [`Symbol`]s with the given start address.
    /// If no symbols can be found, the returned [`Iterator`] will be empty.
    fn get_symbols(&self, addr: u64) -> Box<dyn Iterator<Item = Symbol> + '_>;
}

impl<B, I> Provider<I> for Box<B>
where
    B: Provider<I> + ?Sized,
    I: Info,
{
    fn get_symbols(&self, addr: u64) -> Box<dyn Iterator<Item = Symbol> + '_> {
        B::get_symbols(self.as_ref(), addr)
    }
}

impl<C, B, I> Provider<I> for binary::Multi<C, B>
where
    C: std::borrow::BorrowMut<[B]>,
    B: Provider<I>,
    B::Error: binary::error::Miss,
    I: Info,
{
    fn get_symbols(&self, addr: u64) -> Box<dyn Iterator<Item = Symbol> + '_> {
        Box::new(self.iter().flat_map(move |b| b.get_symbols(addr)))
    }
}

impl<B, I> Provider<I> for binary::Offset<B>
where
    B: Provider<I>,
    B::Error: binary::error::Miss,
    I: Info,
{
    fn get_symbols(&self, addr: u64) -> Box<dyn Iterator<Item = Symbol> + '_> {
        let Some(mapped) = addr.checked_sub(self.offset()) else {
            return Box::new(std::iter::empty());
        };
        let res = self
            .inner()
            .get_symbols(mapped)
            .map(move |s| Symbol { address: addr, ..s });
        Box::new(res)
    }
}
