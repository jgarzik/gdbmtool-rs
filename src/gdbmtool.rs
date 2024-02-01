extern crate clap;
extern crate rs_gdbm;
extern crate rustyline;

use clap::Parser;
use rs_gdbm::Gdbm;
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Pathname of GDBM database to open
    name: String,

    /// Open database in read-only mode?
    #[arg(short, long, default_value_t = false)]
    read_only: bool,
}

fn cmd_help() {
    let helpstr = r#"Available commands:
exit		Exit program.
header		Display database global header
help		This message.
version		Display program name and version."#;

    println!("{}", helpstr);
}

fn cmd_version() {
    const PKG_NAME: &str = env!("CARGO_PKG_NAME");
    const VERSION: &str = env!("CARGO_PKG_VERSION");

    println!("{} {}", PKG_NAME, VERSION);
}

fn cmd_header(db: &Gdbm) -> bool {
    let (dir_sz, dir_bits) = rs_gdbm::dir::build_dir_size(db.header.block_sz);

    println!("{}", "GDBM file header:\n");
    println!("magic {:#x}", db.header.magic);
    println!("dir-offset {}", db.header.dir_ofs);
    println!("dir-size {}", dir_sz);
    println!("dir-bits {}", dir_bits);
    println!("block-size {}", db.header.block_sz);
    println!("bucket-elems {}", db.header.bucket_elems);
    println!("bucket-size {}", db.header.bucket_sz);
    println!("next-block {}", db.header.next_block);
    println!("avail-size {}", db.header.avail.sz);
    println!("avail-count {}", db.header.avail.count);
    println!("avail-next-block {}", db.header.avail.next_block);

    true
}

fn handle_line(db: &mut Gdbm, line: &String) -> bool {
    let mut tsplit = line.split_whitespace();

    let cmd_name = tsplit.next().expect("No command provided");

    match cmd_name {
        "exit" => return false,
        "header" => return cmd_header(&db),
        "help" => cmd_help(),
        "version" => cmd_version(),
        _ => println!("Invalid or unknown command"),
    }

    true
}

fn main() -> Result<()> {
    let args = Args::parse();

    let mut db = Gdbm::open(&args.name).expect("Unable to open database");

    let mut rl = DefaultEditor::new()?;

    loop {
        let readline = rl.readline("gdbm> ");
        match readline {
            Ok(line) => {
                let _ = rl.add_history_entry(line.as_str());
                if !handle_line(&mut db, &line) {
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
