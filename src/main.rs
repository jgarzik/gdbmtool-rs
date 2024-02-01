extern crate clap;
extern crate rustyline;

use clap::Parser;
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Open database in read-only mode?
    #[arg(short, long, default_value_t = false)]
    read_only: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.read_only {
        println!("Option: read-only");
    }

    let mut rl = DefaultEditor::new()?;

    loop {
        let readline = rl.readline("gdbm> ");
        match readline {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());
                println!("Line: {}", line);
            },
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break
            },
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break
            },
            Err(err) => {
                println!("Error: {:?}", err);
                break
            }
        }
    }

    Ok(())
}
