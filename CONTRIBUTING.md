# Contributing

Bug reports and pull requests are welcome at <https://github.com/jgarzik/gdbmtool-rs>.
Changes to database handling itself belong in
[gdbm-native](https://github.com/jgarzik/gdbm-native-rs).

## Checks

CI runs these; please run them before sending a change:

```sh
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

The minimum supported Rust version is 1.89.

`tests/cli.rs` runs the built binaries. Its C GDBM interoperability test needs `gdbm_load`
and `gdbm_dump` on the `PATH` (e.g. Debian's `gdbmtool` package or Homebrew's `gdbm`); it is
skipped without them unless `REQUIRE_GDBM` is set, as in CI.

## Releases

gdbmtool-rs is released together with gdbm-native. Record
changes in [CHANGELOG.md](CHANGELOG.md).
