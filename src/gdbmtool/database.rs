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
            clap::Command::new("get")
                .about("Retrieve and display value for specified KEY")
                .arg(arg!(<KEY> "Key to look up").required(true)),
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
            "get" => self
                .get(matches.get_one::<String>("KEY").unwrap())
                .map(|value| value.into_iter().collect()),
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

    fn get(&mut self, key: &str) -> Result<Option<String>, String> {
        match self {
            Self::ReadOnly(db) => db.get::<&str, String>(key),
            Self::ReadWrite(db) => db.get::<&str, String>(key),
        }
        .map_err(|e| e.to_string())
    }
}
