# Changelog

All notable changes to this project will be documented in this file. Its format
is based on https://keepachangelog.com/en/1.1.0/.

## Unreleased

### Added

- `prof` subcommand for generating a profile in the callgrind format from a
  trace file.
- The `clap` dependency was upgraded to version `4.6`.
- The `riscv-etrace` dependency was upgraded to version `0.10`.
- The `either` dependency was upgraded to version `1.16`.

### Fixed

- Instructions following a return from exception or interrupt are no longer
  obscurred. This happened when the return was not coinciding with a privilege
  (mode) change.

## 0.1.0 - 2026-03-19

### Added

- Initial version of the `rvet` command line utility, featuring subcommands for
  displaying payloads, tracing, conversion to CSV and gathering packet
  statistics.
