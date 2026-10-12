use clap::{arg, value_parser};
use gdbm_native::{Gdbm, ImportMode, OpenOptions};
use std::io::BufRead;
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
                .about("Retrieve and display value for specified KEY")
                .arg(arg!(<KEY> "Key to look up").required(true)),
            clap::Command::new("insert")
                .about("Insert VALUE for specified KEY, showing the old value if there was one")
                .arg(arg!(<KEY> "Key to insert").required(true))
                .arg(arg!(<VALUE> "Value to set").required(true)),
            clap::Command::new("try-insert")
                .about("Try inserting VALUE for specified KEY, failing if the key is already used")
                .arg(arg!(<KEY> "Key to insert").required(true))
                .arg(arg!(<VALUE> "Value to set").required(true)),
            clap::Command::new("remove")
                .about("Remove VALUE for specified KEY, showing the old value if there was one")
                .arg(arg!(<KEY> "Key to look up").required(true)),
            clap::Command::new("keys").about("List database keys"),
            clap::Command::new("values").about("List database values"),
            clap::Command::new("entries").about("List database entries"),
            clap::Command::new("load")
                .about("Import entries from an ASCII dump file")
                .arg(
                    arg!(<FILE> "Filename of dump file")
                        .required(true)
                        .value_parser(value_parser!(PathBuf)),
                ),
            clap::Command::new("dump")
                .about("Export database entries to an ASCII dump file")
                .arg(
                    arg!(<FILE> "Filename of dump file")
                        .required(true)
                        .value_parser(value_parser!(PathBuf)),
                ),
        ]
    }

    pub fn dispatch(
        &mut self,
        name: &str,
        matches: &clap::ArgMatches,
    ) -> Result<Vec<String>, String> {
        let sarg = |name| matches.get_one::<String>(name);
        let parg = |name| matches.get_one::<PathBuf>(name);
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
            "load" => self.load(parg("FILE").unwrap()),
            "dump" => self.dump(parg("FILE").unwrap()),
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
        self.0
            .get::<str, String>(key)
            .map(|value| value.into_iter().collect())
            .map_err(|e| e.to_string())
    }

    // previous value, if any, as text
    fn old_value(old: gdbm_native::Result<Option<Vec<u8>>>) -> Result<Vec<String>, String> {
        old.map_err(|e| e.to_string()).and_then(|old| {
            old.map(|v| String::from_utf8(v).map_err(|e| e.to_string()))
                .transpose()
                .map(|value| value.into_iter().collect())
        })
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
            .keys()
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())
    }

    fn values(&self) -> Result<Vec<String>, String> {
        self.0
            .values()
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())
    }

    fn entries(&self) -> Result<Vec<String>, String> {
        self.0
            .iter::<String, String>()
            .map(|element| element.map(|(key, value)| format!("{key} => {value}")))
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())
    }

    fn load(&mut self, filename: &Path) -> Result<Vec<String>, String> {
        std::fs::File::open(filename)
            .map_err(|e| e.to_string())
            .and_then(|mut f| {
                self.0
                    .import_ascii(&mut f, ImportMode::Replace)
                    .map_err(|e| e.to_string())
            })
            .map(|_| vec![])
    }

    fn dump(&self, filename: &Path) -> Result<Vec<String>, String> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(filename)
            .map_err(|e| e.to_string())?;

        self.0
            .export_ascii(&mut file, None::<&Path>)
            .map(|_| vec![])
            .map_err(|e| e.to_string())
    }
}
