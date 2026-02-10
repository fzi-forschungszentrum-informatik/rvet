// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Utilties for loading and assembling binaries

use anyhow::Context;
use riscv_etrace::binary;
use riscv_etrace::instruction::bits::Bits;
use riscv_isa::Target;

/// Type of binary produced by the builder
pub type Binary = binary::boxed::Binary<'static, Instruction>;

/// Type of instruction info produced by binaries produced by the builder
pub type Instruction = (
    either::Either<riscv_isa::Compressed, riscv_isa::Instruction>,
    Bits,
);

#[derive(Clone, Debug, clap::Args)]
pub struct Args {
    #[command(flatten)]
    specs: Specs,

    /// Include an additional ROM
    #[clap(long)]
    rom: Option<Rom>,
}

impl Args {
    /// Construct a [`Binary`][binary::Binary] based on these arguments
    pub fn build(self, target: Target) -> anyhow::Result<binary::Multi<Vec<Binary>, Binary>> {
        self.rom
            .map(|r| Ok(r.build(target)))
            .into_iter()
            .chain(self.specs.build(target)?)
            .collect()
    }
}

/// Specifications of program binaries to load
#[derive(Clone, Debug)]
pub struct Specs(Vec<Spec>);

impl Specs {
    /// Construct [`Binary`][binary::Binary]s from these specs
    fn build(self, target: Target) -> anyhow::Result<impl Iterator<Item = anyhow::Result<Binary>>> {
        /// `elf::ElfBytes`, and therefore `binary::elf::Elf`, depend on the
        /// underlying data, which is external. We thus need to load the data
        /// and keep it availible for the `Binary`'s lifetime. And since we need
        /// to do so for ELFs anyway, we also do so for raw binaries.
        static DATA: std::sync::OnceLock<Vec<Vec<u8>>> = std::sync::OnceLock::new();
        let data = self
            .0
            .iter()
            .map(|s| {
                std::fs::read(&s.path)
                    .with_context(|| format!("Could not load file '{}'", s.path.display()))
            })
            .collect::<anyhow::Result<_>>()?;
        DATA.set(data)
            .map_err(|_| anyhow::anyhow!("Could not initialize binary data store"))?;

        let res = Iterator::zip(
            self.0.into_iter(),
            DATA.get().context("Could not retrieve loaded data")?.iter(),
        )
        .map(move |(s, d)| s.build(target, d.as_ref()));

        Ok(res)
    }
}

impl clap::Args for Specs {
    fn augment_args(cmd: clap::Command) -> clap::Command {
        Self::augment_args_for_update(cmd)
    }

    fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
        let arg = clap::Arg::new("code")
            .long("code")
            .num_args(1..)
            .required(true)
            .action(clap::ArgAction::Append)
            .value_names(["FILE", "SPEC"])
            .value_terminator(";")
            .help("Trace this program")
            .long_help(
                "Trace this program

Each FILE is loaded and handled according to (optional) SPECs:
\"type=<type>\" forces FILE to be handled as <type>. Valid types are
\"elf\" for ELF files and \"bin\" for raw binaries. If not type is
specified, it will be discovered from the file contents.
\"offset=<hex>\" places the object at the given (base) address. This
option is mandatory for raw binaries and relocatable ELF files.",
            );
        cmd.arg(arg)
    }
}

impl clap::FromArgMatches for Specs {
    fn from_arg_matches(matches: &clap::ArgMatches) -> Result<Self, clap::Error> {
        let mut res = Self(Default::default());
        res.update_from_arg_matches(matches)?;
        Ok(res)
    }

    fn update_from_arg_matches(&mut self, matches: &clap::ArgMatches) -> Result<(), clap::Error> {
        use clap::error::ErrorKind;

        matches
            .get_raw_occurrences("code")
            .ok_or(clap::Error::new(ErrorKind::TooFewValues))?
            .try_for_each(|mut s| {
                let path = s
                    .next()
                    .ok_or(clap::Error::new(ErrorKind::TooFewValues))?
                    .into();
                let mut kind = None;
                let mut offset = None;
                s.try_for_each(|p| {
                    let (key, value) = p
                        .to_str()
                        .ok_or_else(|| clap::Error::new(ErrorKind::InvalidUtf8))?
                        .split_once('=')
                        .ok_or_else(|| {
                            clap::Error::raw(ErrorKind::ValueValidation, "Expected key-value pair")
                        })?;
                    match key {
                        "type" => {
                            let value = value
                                .parse()
                                .map_err(|e| clap::Error::raw(ErrorKind::ValueValidation, e))?;
                            kind = Some(value);
                        }
                        "offset" => {
                            let value = u64::from_str_radix(value, 16)
                                .map_err(|e| clap::Error::raw(ErrorKind::ValueValidation, e))?;
                            offset = Some(value);
                        }
                        key => {
                            return Err(clap::Error::raw(
                                ErrorKind::ValueValidation,
                                format!("not a valid code option: {key}"),
                            ));
                        }
                    };
                    Ok(())
                })?;
                self.0.push(Spec { path, kind, offset });
                Ok(())
            })
    }
}

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
        use binary::Adaptable;

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

/// Supported special ROMs
#[derive(Copy, Clone, Debug, clap::ValueEnum)]
pub enum Rom {
    /// Bootrom of the spike instruction set simulator
    Spike,
}

impl Rom {
    /// Construct the specified [`Binary`][binary::Binary]s
    fn build(self, target: Target) -> Binary {
        use riscv_isa::Xlen;

        use binary::Adaptable;

        match self {
            Self::Spike => {
                let code = match target.xlen {
                    Xlen::Rv32 => {
                        b"\x97\x02\x00\x00\x93\x85\x02\x02\x73\x25\x40\xf1\x83\xa2\x82\x01\x82\x82"
                    }
                    Xlen::Rv64 => {
                        b"\x97\x02\x00\x00\x93\x85\x02\x02\x73\x25\x40\xf1\x83\xb2\x82\x01\x82\x82"
                    }
                };
                binary::from_segment(code, target)
                    .with_offset(0x1000)
                    .boxed()
            }
        }
    }
}
