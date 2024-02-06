extern crate clap;
extern crate rs_gdbm;
extern crate rustyline;
extern crate shellwords;

use clap::Parser;
use rs_gdbm::{Gdbm, GdbmOptions};
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result};
use std::str;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Pathname of GDBM database to open
    name: String,

    /// Open database in read-only mode?
    #[arg(short, long, default_value_t = false)]
    read_only: bool,
}

struct CmdInfo {
    name: &'static str,
    description: &'static str,
    arginfo: &'static str,
    min_args: usize,
}

const CMDINFO: [CmdInfo; 7] = [
    CmdInfo {
        name: "dir",
        description: "Display hash directory",
        arginfo: "",
        min_args: 0,
    },
    CmdInfo {
        name: "exit",
        description: "Exit program",
        arginfo: "",
        min_args: 0,
    },
    CmdInfo {
        name: "get",
        description: "Retrieve and display value for specified KEY",
        arginfo: "KEY",
        min_args: 1,
    },
    CmdInfo {
        name: "header",
        description: "Display database global header",
        arginfo: "",
        min_args: 0,
    },
    CmdInfo {
        name: "help",
        description: "This help message",
        arginfo: "",
        min_args: 0,
    },
    CmdInfo {
        name: "version",
        description: "Display program name and version",
        arginfo: "",
        min_args: 0,
    },
    CmdInfo {
        name: "?",
        description: "This help message",
        arginfo: "",
        min_args: 0,
    },
];

fn get_cmd_metadata(cmd_name: &str) -> Option<CmdInfo> {
    for metadata in CMDINFO {
        if metadata.name == cmd_name {
            return Some(metadata);
        }
    }

    return None;
}

fn cmd_help() {
    println!("Available commands:");

    for metadata in CMDINFO {
        println!("{}\t{}", metadata.name, metadata.description);
    }
}

fn cmd_version() {
    const PKG_NAME: &str = env!("CARGO_PKG_NAME");
    const VERSION: &str = env!("CARGO_PKG_VERSION");

    println!("{} {}", PKG_NAME, VERSION);
}

fn cmd_dir(db: &Gdbm) {
    println!("size {}", db.header.dir_sz);
    println!("bits {}", db.header.dir_bits);

    for n in 0..db.dir.dir.len() {
        println!("{}: {}", n, db.dir.dir[n]);
    }
}

fn cmd_header(db: &Gdbm) {
    let (dir_sz, dir_bits) = rs_gdbm::dir::build_dir_size(db.header.block_sz);

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
}

fn cmd_get(db: &mut Gdbm, args: &[String]) {
    if args.len() < 1 {
        return;
    }

    let res = db.get(args[0].as_bytes());
    match res {
        Ok(opt) => match opt {
            None => {
                println!("ERROR: Key not found");
            }
            Some(val) => match str::from_utf8(&val) {
                Ok(s) => println!("{}", s),
                Err(_e) => println!("{:?}", val),
            },
        },
        Err(_e) => {
            println!("ERROR: Database GET low-level error");
        }
    }
}

fn handle_line(db: &mut Gdbm, line: &String) -> bool {
    let words = shellwords::split(&line).expect("Invalid command syntax");

    if words.len() == 0 {
        return true;
    }

    let cmd_name = &words[0];
    let cmd_args = &words[1..];
    let cmd_metadata_res = get_cmd_metadata(cmd_name);

    if cmd_metadata_res.is_none() {
        println!("Unknown command");
        return true;
    }
    let cmd_metadata = cmd_metadata_res.unwrap();
    if cmd_metadata.min_args > cmd_args.len() {
        println!(
            "Command \"{}\" is missing one or more parameters.\nUsage: {} {}",
            cmd_name, cmd_name, cmd_metadata.arginfo
        );
        return true;
    }

    match cmd_name.as_ref() {
        "dir" => cmd_dir(db),
        "exit" => return false,
        "get" => cmd_get(db, cmd_args),
        "header" => cmd_header(db),
        "?" => cmd_help(),
        "help" => cmd_help(),
        "version" => cmd_version(),
        _ => println!("BUG: CMDINFO out of sync in source"),
    }

    true
}

fn main() -> Result<()> {
    let args = Args::parse();

    let dbcfg = match args.read_only {
        true => GdbmOptions {
            readonly: true,
            creat: false,
        },
        false => GdbmOptions {
            readonly: false,
            creat: true,
        },
    };

    let mut db = Gdbm::open(&args.name, &dbcfg).expect("Unable to open db");

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
