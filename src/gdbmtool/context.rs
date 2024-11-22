use std::path::PathBuf;

use crate::database::Database;

#[derive(Default)]
pub struct Context {
    write: bool,
    filename: Option<PathBuf>,
    database: Option<Database>,
    prompt: Option<String>,
}

impl Context {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn write(self, write: bool) -> Self {
        Self { write, ..self }
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
                    Database::open_rw(filename)
                } else {
                    Database::open_ro(filename)
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
