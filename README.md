# RISC-V E-Trace format CLI tool

This tool allows inspecting and processing traces in the instruction tracing
format defined in the [Efficient Trace for RISC-V][etrace] specification. It
currently provides subcommands for:

* displaying payloads from a single HART,
* tracing a single HART and
* gathering packet statistics.

When built with the `pager` feature, output is displayed via the pager provided
via the `PAGER` environment variable.

> [!NOTE]
> The disassembler used in the `trace` subcommand is known to be incomplete and
> faulty for some instructions. Furthermore, we currently display compressed
> instructions as their expanded (uncompressed) equivalent.

## Supported formats

Traces are generally read from a trace file containing packets. The tool
supports the following formats (selectable via the `--format` option):

* [Unformatted Trace & Diagnostic Data Packet Encapsulation for RISC-V][encap]
* Siemens Messaging Infrastructure

## Supported trace encoders

The following trace encoders and compatible units are supported (selectable via
the `--unit` option):

* the (original) reference encoder implementation
* the [PULP rv tracer][rv_tracer]

## License

This program is licensed under the [Apache License 2.0](./LICENSE).

[etrace]: <https://github.com/riscv-non-isa/riscv-trace-spec/>
[encap]: <https://github.com/riscv-non-isa/e-trace-encap/>
[rv_tracer]: <https://github.com/pulp-platform/rv_tracer>
