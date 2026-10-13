# Changelog

## 0.9.0 (2026-10-13)

First release on crates.io, alongside gdbm-native 0.9.0.

### Changed

- Built on gdbm-native 0.9 from crates.io instead of a git snapshot: one database type opened
  read-only or read-write at run time, so writes to a read-only database report the library's
  error.
- Keys and values that are not UTF-8 are shown with `\xNN` escapes, and backslash and control
  characters are escaped, instead of failing.
- `load` reports the number of records loaded; ASCII dumps record the database's pathname.
- `gdbmdump-rs` reports errors with a message and exit status 1 instead of panicking, writes
  binary dumps in the local C GDBM's format by default and accepts `-` for standard output.
- Rust 2024 edition; minimum supported Rust version 1.89. clap 4.6, rustyline 18.

### Added

- `gdbmtool-rs` commands `info`, `verify`, `recover [--backup]`, `reorganize`, `sync`,
  `upgrade`, `downgrade`, and binary dump `import`/`export` with `--format`.
- `load` and `import` take `--insert` to fail on keys already in the database.
- `gdbmdump-rs --bin-format` to write binary dumps for C GDBM on another platform.
- Integration tests, including interoperability with C GDBM's `gdbm_load` and `gdbm_dump`.
- CI on Linux and macOS (x86-64 and arm64), with clippy, rustfmt, MSRV and cargo-deny checks.

### Fixed

- Interactive mode no longer panics when standard output is not a terminal, and does not page
  when the terminal reports no size.

## 0.1.0

Unreleased development versions.
