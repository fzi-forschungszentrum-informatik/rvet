# RISC-V E-Trace format CLI tool

This tool allows inspecting and processing traces in the instruction tracing
format defined in the [Efficient Trace for RISC-V][etrace] specification. It
currently provides subcommands for:

* displaying payloads from a single HART,
* tracing a single HART,
* converting a trace into a CSV and
* gathering packet statistics.

When built with the `pager` feature, output is displayed via the pager provided
via the `PAGER` environment variable.

> [!NOTE]
> The disassembler used in the `trace` subcommand is known to be incomplete and
> faulty for some instructions.

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

## Parameter files

Encoder parameters need to be provided as a TOML file with parameters (under
their specification names) at the root. For example:

    cache_size_p=0
    call_counter_size_p=0
    context_width_p=32
    time_width_p=1
    ecause_width_p=5
    f0s_width_p=0
    iaddress_lsb_p=1
    iaddress_width_p=64
    nocontext_p=0
    notime_p=1
    privilege_width_p=2
    return_stack_size_p=0
    sijump_p=0

> [!NOTE]
> Not all parameters described in the [specification][etrace] are processed.
> Unknown parameters are ignored.

## License

This program is licensed under the [Apache License 2.0](./LICENSE).

[etrace]: <https://github.com/riscv-non-isa/riscv-trace-spec/>
[encap]: <https://github.com/riscv-non-isa/e-trace-encap/>
[rv_tracer]: <https://github.com/pulp-platform/rv_tracer>
