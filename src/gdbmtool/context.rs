use std::path::PathBuf;

use crate::database::Database;

#[derive(Default)]
pub struct Context {
    cache_size: Option<usize>,
    write: bool,
    create: bool,
    block_size: Option<u32>,
    filename: Option<PathBuf>,
    database: Option<Database>,
    prompt: Option<String>,
}

impl Context {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cache_size(self, cache_size: Option<usize>) -> Self {
        Self { cache_size, ..self }
    }

    pub fn write(self, write: bool) -> Self {
        Self { write, ..self }
    }

    pub fn create(self, create: bool) -> Self {
        Self { create, ..self }
    }

    pub fn block_size(self, block_size: Option<u32>) -> Self {
        Self { block_size, ..self }
    }

    pub fn filename(self, filename: Option<PathBuf>) -> Self {
        Self { filename, ..self }
    }

    pub fn prompt(&self) -> String {
        self.prompt.clone().unwrap_or("gdbm> ".to_string())
    }

    pub fn open(&mut self) -> Result<(), String> {
        self.filename
            .as_ref()
            .ok_or_else(|| "no filename to open".to_string())
            .and_then(|filename| {
                if self.write {
                    Database::open_rw(filename, self.cache_size, self.create, self.block_size)
                } else {
                    Database::open_ro(filename, self.cache_size)
                }
            })
            .map(|database| self.database = Some(database))
    }

    pub fn commands() -> Vec<clap::Command> {
        Database::commands()
    }

    pub fn dispatch(
        &mut self,
        name: &str,
        matches: &clap::ArgMatches,
    ) -> Result<Vec<String>, String> {
        match name {
            "set" => unimplemented!(),
            _ => self
                .current_database()
                .and_then(|db| db.dispatch(name, matches)),
        }
    }

    fn current_database(&mut self) -> Result<&mut Database, String> {
        self.database
            .as_mut()
            .ok_or_else(|| "no current database".to_string())
    }
}
