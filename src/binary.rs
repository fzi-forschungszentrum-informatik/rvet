// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Utilties for loading and assembling binaries

use anyhow::Context;
use riscv_etrace::binary;
use riscv_isa::Target;

/// Type of binary produced by the builder
pub type Binary = binary::boxed::Binary<'static, riscv_isa::Instruction>;

/// Specification of a binary to load
#[derive(Clone, Debug)]
struct Spec {
    path: std::path::PathBuf,
    kind: Option<Kind>,
    offset: Option<u64>,
}

impl Spec {
    /// Construct a [`Binary`] based on this specification
    fn build(self, target: Target, data: &'static [u8]) -> anyhow::Result<Binary> {
        use binary::Binary;

        let path = self.path;

        let kind = self.kind.unwrap_or_else(|| {
            if data.starts_with(elf::abi::ELFMAGIC.as_ref()) {
                Kind::Elf
            } else {
                Kind::Bin
            }
        });
        match kind {
            Kind::Bin => {
                let offset = self.offset.with_context(|| {
                    format!("Need offset for (raw) binary '{}'", path.display())
                })?;
                let res = binary::basic::from_segment(data, target)
                    .with_offset(offset)
                    .boxed();
                Ok(res)
            }
            Kind::Elf => {
                let elf = elf::ElfBytes::<elf::endian::LittleEndian>::minimal_parse(data)
                    .with_context(|| format!("Could not parse ELF file '{}'", path.display()))?;
                let elf = std::sync::Arc::new(elf);
                let res = binary::elf::Elf::<_, _, Target>::new(elf.clone())
                    .with_context(|| format!("Could not process ELF file '{}'", path.display(),))?;

                if let Some(offset) = self.offset {
                    Ok(res.with_offset(offset).boxed())
                } else if elf.ehdr.e_type != elf::abi::ET_DYN {
                    Ok(res.boxed())
                } else {
                    Err(anyhow::anyhow!(
                        "Need offset for shared object '{}'",
                        path.display(),
                    ))
                }
            }
        }
    }
}

/// Binary kind
#[derive(Copy, Clone, Debug)]
enum Kind {
    /// Raw binary (e.g. mem dump)
    Bin,
    /// ELF data
    Elf,
}

impl std::str::FromStr for Kind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bin" | "binary" | "raw" => Ok(Self::Bin),
            "elf" => Ok(Self::Elf),
            s => Err(format!("Invalid type: {s}")),
        }
    }
}
