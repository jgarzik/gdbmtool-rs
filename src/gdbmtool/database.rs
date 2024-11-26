use clap::arg;
use gdbm_native::{Gdbm, OpenOptions, ReadOnly, ReadWrite};
use std::io::BufRead;
use std::path::Path;

pub enum Database {
    ReadOnly(Gdbm<ReadOnly>),
    ReadWrite(Gdbm<ReadWrite>),
}

impl Database {
    pub fn open_ro(filename: &Path) -> Result<Self, String> {
        Ok(Self::ReadOnly(
            OpenOptions::new()
                .open(filename)
                .map_err(|e| e.to_string())?,
        ))
    }

    pub fn open_rw(filename: &Path) -> Result<Self, String> {
        Ok(Self::ReadWrite(
            OpenOptions::new()
                .write()
                .open(filename)
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
            clap::Command::new("elements").about("List database elements"),
        ]
    }

    pub fn dispatch(
        &mut self,
        name: &str,
        matches: &clap::ArgMatches,
    ) -> Result<Vec<String>, String> {
        match name {
            "header" => Ok(self.header()),
            "dir" => Ok(self.directory()),
            "len" => self.len().map(|l| vec![format!("{l}")]),
            "get" => self
                .get(matches.get_one::<String>("KEY").unwrap())
                .map(|value| value.into_iter().collect()),
            "insert" => self
                .insert(
                    matches.get_one::<String>("KEY").unwrap(),
                    matches.get_one::<String>("VALUE").unwrap(),
                )
                .map(|value| value.into_iter().collect()),
            "try-insert" => self
                .try_insert(
                    matches.get_one::<String>("KEY").unwrap(),
                    matches.get_one::<String>("VALUE").unwrap(),
                )
                .map(|value| value.into_iter().collect()),
            "remove" => self
                .remove(matches.get_one::<String>("KEY").unwrap())
                .map(|value| value.into_iter().collect()),
            "keys" => self.keys(),
            "values" => self.values(),
            "elements" => self.elements(),
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

    fn get(&mut self, key: &str) -> Result<Option<String>, String> {
        match self {
            Self::ReadOnly(db) => db.get::<&str, String>(key),
            Self::ReadWrite(db) => db.get::<&str, String>(key),
        }
        .map_err(|e| e.to_string())
    }

    fn insert(&mut self, key: &str, value: &str) -> Result<Option<String>, String> {
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
    }

    fn try_insert(&mut self, key: &str, value: &str) -> Result<Option<String>, String> {
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
    }

    fn remove(&mut self, key: &str) -> Result<Option<String>, String> {
        match self {
            Self::ReadOnly(_) => Err("readonly database".to_string()),
            Self::ReadWrite(db) => db.remove(key).map_err(|e| e.to_string()).and_then(|old| {
                old.map(|v| std::str::from_utf8(v.as_ref()).map(|v| v.to_string()))
                    .transpose()
                    .map_err(|e| e.to_string())
            }),
        }
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

    fn elements(&mut self) -> Result<Vec<String>, String> {
        let format = |element: Result<(String, String), _>| {
            element.map(|(key, value)| format!("{key} => {value}"))
        };

        match self {
            Self::ReadOnly(db) => db.iter().map(format).collect::<Result<_, _>>(),
            Self::ReadWrite(db) => db.iter().map(format).collect::<Result<_, _>>(),
        }
        .map_err(|e| e.to_string())
    }
}
