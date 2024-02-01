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

fn cmd_help() {
    let helpstr = r#"Available commands:
exit		Exit program.
help		This message.
version		Display program name and version."#;

    println!("{}", helpstr);
}

fn cmd_version() {
    const PKG_NAME: &str = env!("CARGO_PKG_NAME");
    const VERSION: &str = env!("CARGO_PKG_VERSION");

    println!("{} {}", PKG_NAME, VERSION);
}

fn handle_line(line: &String) -> bool {
    let mut tsplit = line.split_whitespace();

    let cmd_name = tsplit.next().expect("No command provided");

    match cmd_name {
        "exit" => return false,
        "help" => cmd_help(),
        "version" => cmd_version(),
        _ => println!("Invalid or unknown command"),
    }

    true
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
                if !handle_line(&line) {
                    break;
                }
            }
            Err(ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(err) => {
                println!("Error: {:?}", err);
                break;
            }
        }
    }

    Ok(())
}
