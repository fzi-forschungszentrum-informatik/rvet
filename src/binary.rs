// Copyright (c) 2026 FZI Forschungszentrum Informatik
// SPDX-License-Identifier: Apache-2.0
//! Utilties for loading and assembling binaries

use anyhow::Context;
use riscv_etrace::binary;
use riscv_isa::Target;

/// Type of binary produced by the builder
pub type Binary = binary::boxed::Binary<'static, riscv_isa::Instruction>;

/// Specifications of program binaries to load
#[derive(Clone, Debug)]
pub struct Specs(Vec<Spec>);

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
