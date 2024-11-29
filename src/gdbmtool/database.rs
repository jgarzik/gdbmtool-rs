use clap::{arg, value_parser};
use gdbm_native::{Gdbm, OpenOptions, ReadOnly, ReadWrite};
use std::io::BufRead;
use std::path::{Path, PathBuf};

pub enum Database {
    ReadOnly(Gdbm<ReadOnly>),
    ReadWrite(Gdbm<ReadWrite>),
}

impl Database {
    pub fn open_ro(filename: &Path, cache_size: Option<usize>) -> Result<Self, String> {
        Ok(Self::ReadOnly(
            OpenOptions::new()
                .cachesize(cache_size)
                .open(filename)
                .map_err(|e| e.to_string())?,
        ))
    }

    pub fn open_rw(
        filename: &Path,
        cache_size: Option<usize>,
        create: bool,
        block_size: Option<u32>,
    ) -> Result<Self, String> {
        Ok(Self::ReadWrite(
            if create {
                if let Some(block_size) = block_size {
                    OpenOptions::new()
                        .cachesize(cache_size)
                        .write()
                        .create()
                        .block_size(gdbm_native::BlockSize::Roughly(block_size))
                        .open(filename)
                } else {
                    OpenOptions::new()
                        .cachesize(cache_size)
                        .write()
                        .create()
                        .open(filename)
                }
            } else {
                OpenOptions::new()
                    .cachesize(cache_size)
                    .write()
                    .open(filename)
            }
            .map_err(|e| e.to_string())?,
        ))
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
        let _ = match self {
            Self::ReadOnly(db) => db.show_header(&mut buffer),
            Self::ReadWrite(db) => db.show_header(&mut buffer),
        };
        buffer.lines().map(|l| l.unwrap()).collect()
    }

    fn directory(&self) -> Vec<String> {
        let mut buffer = vec![];
        let _ = match self {
            Self::ReadOnly(db) => db.show_directory(&mut buffer),
            Self::ReadWrite(db) => db.show_directory(&mut buffer),
        };
        buffer.lines().map(|l| l.unwrap()).collect()
    }

    fn len(&mut self) -> Result<usize, String> {
        match self {
            Self::ReadOnly(db) => db.len(),
            Self::ReadWrite(db) => db.len(),
        }
        .map_err(|e| e.to_string())
    }

    fn get(&mut self, key: &str) -> Result<Vec<String>, String> {
        match self {
            Self::ReadOnly(db) => db.get::<&str, String>(key),
            Self::ReadWrite(db) => db.get::<&str, String>(key),
        }
        .map(|value| value.into_iter().collect())
        .map_err(|e| e.to_string())
    }

    fn insert(&mut self, key: &str, value: &str) -> Result<Vec<String>, String> {
        match self {
            Self::ReadOnly(_) => Err("readonly database".to_string()),
            Self::ReadWrite(db) => db
                .insert(key.as_ref(), value.as_ref())
                .map_err(|e| e.to_string())
                .and_then(|old| {
                    old.map(|v| std::str::from_utf8(v.as_ref()).map(|v| v.to_string()))
                        .transpose()
                        .map_err(|e| e.to_string())
                }),
        }
        .map(|value| value.into_iter().collect())
    }

    fn try_insert(&mut self, key: &str, value: &str) -> Result<Vec<String>, String> {
        match self {
            Self::ReadOnly(_) => Err("readonly database".to_string()),
            Self::ReadWrite(db) => db
                .try_insert(key.as_ref(), value.as_ref())
                .map_err(|e| e.to_string())
                .map(|(_, old)| old)
                .and_then(|old| {
                    old.map(|v| std::str::from_utf8(v.as_ref()).map(|v| v.to_string()))
                        .transpose()
                        .map_err(|e| e.to_string())
                }),
        }
        .map(|value| value.into_iter().collect())
    }

    fn remove(&mut self, key: &str) -> Result<Vec<String>, String> {
        match self {
            Self::ReadOnly(_) => Err("readonly database".to_string()),
            Self::ReadWrite(db) => db.remove(key).map_err(|e| e.to_string()).and_then(|old| {
                old.map(|v| std::str::from_utf8(v.as_ref()).map(|v| v.to_string()))
                    .transpose()
                    .map_err(|e| e.to_string())
            }),
        }
        .map(|value| value.into_iter().collect())
    }

    fn keys(&mut self) -> Result<Vec<String>, String> {
        match self {
            Self::ReadOnly(db) => db.keys().collect::<Result<_, _>>(),
            Self::ReadWrite(db) => db.keys().collect::<Result<_, _>>(),
        }
        .map_err(|e| e.to_string())
    }

    fn values(&mut self) -> Result<Vec<String>, String> {
        match self {
            Self::ReadOnly(db) => db.values().collect::<Result<_, _>>(),
            Self::ReadWrite(db) => db.values().collect::<Result<_, _>>(),
        }
        .map_err(|e| e.to_string())
    }

    fn entries(&mut self) -> Result<Vec<String>, String> {
        let format = |element: Result<(String, String), _>| {
            element.map(|(key, value)| format!("{key} => {value}"))
        };

        match self {
            Self::ReadOnly(db) => db.iter().map(format).collect::<Result<_, _>>(),
            Self::ReadWrite(db) => db.iter().map(format).collect::<Result<_, _>>(),
        }
        .map_err(|e| e.to_string())
    }

    fn load(&mut self, filename: &Path) -> Result<Vec<String>, String> {
        match self {
            Self::ReadOnly(_) => Err("readonly database".to_string()),
            Self::ReadWrite(db) => std::fs::File::open(filename)
                .map_err(|e| e.to_string())
                .and_then(|mut f| db.import_ascii(&mut f).map_err(|e| e.to_string()))
                .map(|_| vec![]),
        }
    }

    fn dump(&mut self, filename: &Path) -> Result<Vec<String>, String> {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(filename)
            .map_err(|e| e.to_string())?;

        match self {
            Self::ReadOnly(db) => db.export_ascii(&mut file),
            Self::ReadWrite(db) => db.export_ascii(&mut file),
        }
        .map(|_| vec![])
        .map_err(|e| e.to_string())
    }
}
