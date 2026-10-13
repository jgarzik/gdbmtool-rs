use crate::binformat::BinFormat;
use clap::builder::EnumValueParser;
use clap::{arg, value_parser};
use gdbm_native::{Gdbm, ImportMode, OpenOptions};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

pub struct Database(Gdbm);

impl Database {
    pub fn open_ro(filename: &Path, cache_size: Option<usize>) -> Result<Self, String> {
        Self::open(filename, cache_size, false, false, None)
    }

    pub fn open_rw(
        filename: &Path,
        cache_size: Option<usize>,
        create: bool,
        block_size: Option<u32>,
    ) -> Result<Self, String> {
        Self::open(filename, cache_size, true, create, block_size)
    }

    fn open(
        filename: &Path,
        cache_size: Option<usize>,
        write: bool,
        create: bool,
        block_size: Option<u32>,
    ) -> Result<Self, String> {
        let mut options = OpenOptions::new();
        options.write(write).create(create);
        if let Some(cache_size) = cache_size {
            options.cachesize(cache_size);
        }
        if let Some(block_size) = block_size {
            options.block_size(gdbm_native::BlockSize::Roughly(block_size));
        }
        options.open(filename).map(Self).map_err(|e| e.to_string())
    }

    pub fn commands() -> Vec<clap::Command> {
        vec![
            clap::Command::new("header").about("Display database header"),
            clap::Command::new("dir").about("Display database directory"),
            clap::Command::new("len").about("Show the number of entries in the database"),
            clap::Command::new("get")
                .about("Show the value for KEY")
                .arg(arg!(<KEY> "Key to look up").required(true)),
            clap::Command::new("insert")
                .about("Insert or replace VALUE for KEY, showing the old value if there was one")
                .arg(arg!(<KEY> "Key to insert").required(true))
                .arg(arg!(<VALUE> "Value to set").required(true)),
            clap::Command::new("try-insert")
                .about(
                    "Insert VALUE for KEY only if KEY is not present, otherwise showing its value",
                )
                .arg(arg!(<KEY> "Key to insert").required(true))
                .arg(arg!(<VALUE> "Value to set").required(true)),
            clap::Command::new("remove")
                .about("Remove KEY, showing its value if it was present")
                .arg(arg!(<KEY> "Key to look up").required(true)),
            clap::Command::new("keys").about("List database keys"),
            clap::Command::new("values").about("List database values"),
            clap::Command::new("entries").about("List database entries"),
            clap::Command::new("load")
                .about("Import entries from an ASCII dump file")
                .arg(file_arg())
                .arg(insert_arg()),
            clap::Command::new("dump")
                .about("Export database entries to an ASCII dump file")
                .arg(file_arg()),
            clap::Command::new("import")
                .about("Import entries from a binary dump file")
                .arg(file_arg())
                .arg(format_arg().help("Length field layout of the dump [default: detected]"))
                .arg(insert_arg()),
            clap::Command::new("export")
                .about("Export database entries to a binary dump file")
                .arg(file_arg())
                .arg(format_arg().help("Length field layout of the dump [default: host]")),
            clap::Command::new("info").about("Show the database's format and size"),
            clap::Command::new("verify")
                .about("Check the consistency of the database's use of file space"),
            clap::Command::new("recover")
                .about("Rebuild the database from the records that can still be read")
                .arg(arg!(--backup "First copy the database file to FILE.bak")),
            clap::Command::new("reorganize")
                .about("Compact the database, rebuilding it without free space"),
            clap::Command::new("sync").about("Write changes to disk"),
            clap::Command::new("upgrade").about("Convert the database to numsync format"),
            clap::Command::new("downgrade")
                .about("Convert the database to the format without numsync"),
        ]
    }

    pub fn dispatch(
        &mut self,
        name: &str,
        matches: &clap::ArgMatches,
    ) -> Result<Vec<String>, String> {
        let sarg = |name| matches.get_one::<String>(name);
        let parg = |name| matches.get_one::<PathBuf>(name);
        let format = || matches.get_one::<BinFormat>("format").copied();
        let mode = || {
            if matches.get_flag("insert") {
                ImportMode::Insert
            } else {
                ImportMode::Replace
            }
        };
        match name {
            "header" => Ok(self.header()),
            "dir" => Ok(self.directory()),
            "len" => self.len().map(|l| vec![format!("{l}")]),
            "get" => self.get(sarg("KEY").unwrap()),
            "insert" => self.insert(sarg("KEY").unwrap(), sarg("VALUE").unwrap()),
            "try-insert" => self.try_insert(sarg("KEY").unwrap(), sarg("VALUE").unwrap()),
            "remove" => self.remove(sarg("KEY").unwrap()),
            "keys" => self.keys(),
            "values" => self.values(),
            "entries" => self.entries(),
            "load" => self.load(parg("FILE").unwrap(), mode()),
            "dump" => self.dump(parg("FILE").unwrap()),
            "import" => self.import(parg("FILE").unwrap(), format(), mode()),
            "export" => self.export(parg("FILE").unwrap(), format()),
            "info" => Ok(self.info()),
            "verify" => self.verify(),
            "recover" => self.recover(matches.get_flag("backup")),
            "reorganize" => self.reorganize(),
            "sync" => self.sync(),
            "upgrade" => self.set_numsync(true),
            "downgrade" => self.set_numsync(false),
            _ => unreachable!("no such command"),
        }
    }

    fn header(&self) -> Vec<String> {
        let mut buffer = vec![];
        let _ = self.0.show_header(&mut buffer);
        buffer.lines().map(|l| l.unwrap()).collect()
    }

    fn directory(&self) -> Vec<String> {
        let mut buffer = vec![];
        let _ = self.0.show_directory(&mut buffer);
        buffer.lines().map(|l| l.unwrap()).collect()
    }

    fn len(&self) -> Result<usize, String> {
        self.0.len().map_err(|e| e.to_string())
    }

    fn get(&self, key: &str) -> Result<Vec<String>, String> {
        Self::old_value(self.0.get(key))
    }

    // value, if any, as text
    fn old_value(old: gdbm_native::Result<Option<Vec<u8>>>) -> Result<Vec<String>, String> {
        old.map(|value| value.as_deref().map(text).into_iter().collect())
            .map_err(|e| e.to_string())
    }

    fn insert(&mut self, key: &str, value: &str) -> Result<Vec<String>, String> {
        Self::old_value(self.0.insert(key, value))
    }

    fn try_insert(&mut self, key: &str, value: &str) -> Result<Vec<String>, String> {
        Self::old_value(self.0.try_insert(key, value))
    }

    fn remove(&mut self, key: &str) -> Result<Vec<String>, String> {
        Self::old_value(self.0.remove(key))
    }

    fn keys(&self) -> Result<Vec<String>, String> {
        self.0
            .keys::<Vec<u8>>()
            .map(|key| key.map(|key| text(&key)))
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())
    }

    fn values(&self) -> Result<Vec<String>, String> {
        self.0
            .values::<Vec<u8>>()
            .map(|value| value.map(|value| text(&value)))
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())
    }

    fn entries(&self) -> Result<Vec<String>, String> {
        self.0
            .iter::<Vec<u8>, Vec<u8>>()
            .map(|element| {
                element.map(|(key, value)| format!("{} => {}", text(&key), text(&value)))
            })
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())
    }

    fn load(&mut self, filename: &Path, mode: ImportMode) -> Result<Vec<String>, String> {
        std::fs::File::open(filename)
            .map_err(|e| e.to_string())
            .and_then(|f| {
                self.0
                    .import_ascii(&mut std::io::BufReader::new(f), mode)
                    .map_err(|e| e.to_string())
            })
            .map(|info| vec![format!("{} records loaded", info.count)])
    }

    fn dump(&self, filename: &Path) -> Result<Vec<String>, String> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(filename)
            .map_err(|e| e.to_string())?;

        self.0
            .export_ascii(&mut file, self.0.path())
            .map(|_| vec![])
            .map_err(|e| e.to_string())
    }

    fn import(
        &mut self,
        filename: &Path,
        format: Option<BinFormat>,
        mode: ImportMode,
    ) -> Result<Vec<String>, String> {
        std::fs::File::open(filename)
            .map_err(|e| e.to_string())
            .and_then(|f| {
                self.0
                    .import_bin(
                        &mut std::io::BufReader::new(f),
                        format.map(Into::into),
                        mode,
                    )
                    .map_err(|e| e.to_string())
            })
            .map(|count| vec![format!("{count} records imported")])
    }

    fn export(&self, filename: &Path, format: Option<BinFormat>) -> Result<Vec<String>, String> {
        let file = std::fs::File::create(filename).map_err(|e| e.to_string())?;
        let mut out = std::io::BufWriter::new(file);

        self.0
            .export_bin(&mut out, format.unwrap_or(BinFormat::Host).into())
            .map_err(|e| e.to_string())?;
        out.flush().map(|_| vec![]).map_err(|e| e.to_string())
    }

    fn info(&self) -> Vec<String> {
        let db = &self.0;
        let path = db
            .path()
            .map_or("(temporary)".into(), |p| p.display().to_string());
        let yes_no = |b: bool| if b { "yes" } else { "no" };
        vec![
            format!("file {path}"),
            format!("writable {}", yes_no(db.is_writable())),
            format!("magic {}", db.magic()),
            format!("layout {:?}", db.layout()),
            format!("endian {:?}", db.endian()),
            format!("block-size {}", db.block_size()),
            format!("bucket-size {}", db.bucket_size()),
            format!("dir-depth {}", db.dir_depth()),
            format!("bucket-count {}", db.bucket_count()),
            format!(
                "numsync {}",
                db.numsync().map_or("none".into(), |n| n.to_string())
            ),
            format!(
                "char-sign {}",
                db.char_sign()
                    .map_or("undetermined".into(), |c| format!("{c:?}"))
            ),
        ]
    }

    fn verify(&self) -> Result<Vec<String>, String> {
        self.0
            .verify()
            .map(|report| {
                vec![
                    format!("buckets {}", report.buckets),
                    format!("records {}", report.records),
                    format!("free-extents {}", report.free_extents),
                    format!("free-bytes {}", report.free_bytes),
                ]
            })
            .map_err(|e| e.to_string())
    }

    fn recover(&mut self, backup: bool) -> Result<Vec<String>, String> {
        self.0
            .recover(backup)
            .map(|report| {
                let mut lines = vec![
                    format!("recovered-keys {}", report.recovered_keys),
                    format!("failed-keys {}", report.failed_keys),
                    format!("duplicate-keys {}", report.duplicate_keys),
                    format!("failed-buckets {}", report.failed_buckets),
                ];
                lines.extend(
                    report
                        .backup
                        .map(|path| format!("backup {}", path.display())),
                );
                lines
            })
            .map_err(|e| e.to_string())
    }

    fn reorganize(&mut self) -> Result<Vec<String>, String> {
        self.0.compact().map(|_| vec![]).map_err(|e| e.to_string())
    }

    fn sync(&mut self) -> Result<Vec<String>, String> {
        self.0.sync().map(|_| vec![]).map_err(|e| e.to_string())
    }

    fn set_numsync(&mut self, numsync: bool) -> Result<Vec<String>, String> {
        self.0
            .set_numsync(numsync)
            .and_then(|_| self.0.sync())
            .map(|_| vec![])
            .map_err(|e| e.to_string())
    }
}

fn file_arg() -> clap::Arg {
    arg!(<FILE> "Filename of dump file")
        .required(true)
        .value_parser(value_parser!(PathBuf))
}

fn format_arg() -> clap::Arg {
    arg!(--format <FORMAT>).value_parser(EnumValueParser::<BinFormat>::new())
}

fn insert_arg() -> clap::Arg {
    arg!(--insert "Fail if a key in the dump is already in the database, instead of replacing it")
}

/// Bytes as a line of text: UTF-8 is shown as is, except that backslash, control characters and
/// bytes that are not UTF-8 are escaped (`\\`, `\n`, `\t`, `\xNN`), so binary keys and values
/// display unambiguously on one line.
pub fn text(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len());
    for chunk in bytes.utf8_chunks() {
        for c in chunk.valid().chars() {
            match c {
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                '\t' => out.push_str("\\t"),
                c if c.is_control() => {
                    // control characters are all single bytes or U+0080..U+009F
                    let mut buf = [0; 4];
                    for b in c.encode_utf8(&mut buf).bytes() {
                        out.push_str(&format!("\\x{b:02x}"));
                    }
                }
                c => out.push(c),
            }
        }
        for b in chunk.invalid() {
            out.push_str(&format!("\\x{b:02x}"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::text;

    #[test]
    fn text_escapes() {
        assert_eq!(text(b"plain"), "plain");
        assert_eq!(text("caf\u{e9}".as_bytes()), "caf\u{e9}");
        assert_eq!(text(b"a\\b"), "a\\\\b");
        assert_eq!(text(b"a\nb\tc\0"), "a\\nb\\tc\\x00");
        assert_eq!(text(b"\xff\xfe ok"), "\\xff\\xfe ok");
        assert_eq!(text("\u{85}".as_bytes()), "\\xc2\\x85");
    }
}
