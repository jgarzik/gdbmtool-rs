# gdbmtool-rs

[![CI](https://github.com/jgarzik/gdbmtool-rs/actions/workflows/TestingCI.yml/badge.svg)](https://github.com/jgarzik/gdbmtool-rs/actions/workflows/TestingCI.yml)
[![crates.io](https://img.shields.io/crates/v/gdbmtool-rs.svg)](https://crates.io/crates/gdbmtool-rs)
[![MSRV 1.89](https://img.shields.io/badge/MSRV-1.89-blue)](Cargo.toml)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

Command line tools for inspecting, editing, dumping and repairing
[GNU GDBM](https://www.gnu.org.ua/software/gdbm/) databases, in pure Rust. They are built
on [gdbm-native](https://github.com/jgarzik/gdbm-native-rs), so they need no C library and
read and write GDBM files from any platform: either byte order, 32- or 64-bit file offsets,
with or without numsync.

* `gdbmtool-rs`: query, edit and maintain a database, one command at a time or
  interactively; modeled on C GDBM's `gdbmtool`.
* `gdbmdump-rs`: dump a database in C GDBM's ASCII (`gdbm_dump`) or binary
  (`gdbm_export`) format.

## Install

```sh
cargo install gdbmtool-rs
```

## gdbmtool-rs

```text
gdbmtool-rs [OPTIONS] [FILE] [COMMAND]
```

| Option | |
|---|---|
| `-r`, `--read-only` | Open the database read-only (default: read-write) |
| `-c`, `--create` | Create the database if the file is missing |
| `-b`, `--block-size SIZE` | Block size for a new database, rounded up to a power of two |
| `--cache-size SIZE` | Size in bytes of the bucket cache |

Give a command after `FILE` to run it and exit. Without one, commands are read from standard
input, one per line, with shell-style quoting: interactively with line editing and history
when standard input is a terminal (long output goes through `$PAGER`), or as a script
otherwise, stopping at the first command that fails.

```console
$ gdbmtool-rs -c users.db insert alice alice@example.com
$ gdbmtool-rs users.db insert bob "Bob <bob@example.com>"
$ gdbmtool-rs -r users.db get alice
alice@example.com
$ printf 'len\nentries\n' | gdbmtool-rs -r users.db
2
bob => Bob <bob@example.com>
alice => alice@example.com
$ gdbmtool-rs users.db
gdbm> verify
buckets 1
records 2
free-extents 1
free-bytes 4050
gdbm> exit
```

Keys and values are shown as text, with backslash, control characters and bytes that are not
UTF-8 escaped (`\\`, `\n`, `\t`, `\xNN`), so every record prints on one line.

### Commands

| Command | |
|---|---|
| `get KEY` | Show the value for KEY |
| `insert KEY VALUE` | Insert or replace, showing the old value if there was one |
| `try-insert KEY VALUE` | Insert only if KEY is not present, otherwise showing its value |
| `remove KEY` | Remove KEY, showing its value if it was present |
| `len` | Number of entries |
| `keys`, `values`, `entries` | List the database |
| `dump FILE` | Write an ASCII dump, portable to any platform and to C GDBM's `gdbm_load` |
| `load FILE [--insert]` | Add the records of an ASCII dump |
| `export FILE [--format F]` | Write a binary dump |
| `import FILE [--format F] [--insert]` | Add the records of a binary dump |
| `info` | Database format: magic, layout, byte order, block and bucket sizes, numsync, ... |
| `header`, `dir` | Display the database header and hash directory |
| `verify` | Check the consistency of the database's use of file space |
| `recover [--backup]` | Rebuild the database from whatever records can be read, first copying it to `FILE.bak` with `--backup` |
| `reorganize` | Compact the database, rebuilding it without free space |
| `upgrade`, `downgrade` | Convert to and from the numsync format |
| `sync` | Write changes to disk |

`load` and `import` replace the values of keys that are already present, or with `--insert`
stop with an error at the first such key.

Binary dumps record lengths in C's `long`, so their layout depends on the platform of the
C GDBM that reads or wrote them. `--format` is one of `host` (this platform; the default for
`export`), `long32` (32-bit platforms and Windows), `long64le` (64-bit little-endian, such as
x86-64 and arm64) or `long64be` (64-bit big-endian, such as s390x). `import` detects the
layout when `--format` is not given. ASCII dumps avoid the question.

`gdbmtool-rs help COMMAND` describes each command.

## gdbmdump-rs

```text
gdbmdump-rs [-f binary|ascii] [--bin-format F] DATABASE OUTPUT
```

Dumps `DATABASE` to `OUTPUT` (`-` for standard output) in binary (the default) or ASCII
format; `--bin-format` takes the values of `--format` above.

## Compatibility with C GDBM

Databases and dumps are exchanged with C GDBM 1.26; CI loads each tool's dumps with the
other and reads the resulting databases. See
[gdbm-native](https://github.com/jgarzik/gdbm-native-rs#compatibility-with-c-gdbm) for the
file format variants supported. Commands are named after C `gdbmtool`'s, but their options
and output differ.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) and [CHANGELOG.md](CHANGELOG.md).

## License

MIT; see [LICENSE](LICENSE).
