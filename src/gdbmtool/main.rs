extern crate clap;
extern crate gdbm_native;
extern crate rustyline;
extern crate shellwords;

mod context;
mod database;
mod display;

use clap::{arg, command, value_parser};
use display::display;
use rustyline::error::ReadlineError;
use rustyline::DefaultEditor;
use std::io::{BufRead, IsTerminal};
use std::path::PathBuf;
use std::process::ExitCode;
use std::str;

use context::Context;

fn main() -> ExitCode {
    let cmd = command!()
        .arg(
            arg!(-r --"read-only" "Open database read-only")
                .conflicts_with_all(["create", "block-size"]),
        )
        .arg(arg!(-c --create "Create a new database if the database file is missing"))
        .arg(
            arg!(-b --"block-size" <SIZE> "Block size for new databases")
                .value_parser(value_parser!(u32)),
        )
        .arg(arg!(--"cache-size" <SIZE> "Size of memory cache").value_parser(value_parser!(usize)))
        .arg(
            arg!(<FILE> "Database filename")
                .required(false)
                .value_parser(value_parser!(PathBuf)),
        )
        .subcommands(Context::commands());

    let matches = cmd.get_matches();

    let mut context = Context::new()
        .write(!matches.get_flag("read-only"))
        .create(matches.get_flag("create"))
        .block_size(matches.get_one("block-size").copied())
        .cache_size(matches.get_one("cache-size").copied());

    if let Ok(Some(filename)) = matches.try_get_one::<PathBuf>("FILE") {
        context = context.filename(Some(filename.clone()));
        if let Err(e) = context.open() {
            eprintln!("Failed to open database: {e}");
            return ExitCode::FAILURE;
        }
    }

    match matches.subcommand() {
        Some((name, args)) => single_command(context, name, args),
        None => {
            if std::io::stdin().is_terminal() {
                interactive(context)
            } else {
                command_stream(context, &mut std::io::stdin().lock())
            }
        }
    }
}

fn interactive(mut context: Context) -> ExitCode {
    let mut rl = DefaultEditor::new().expect("failed to get line editor");

    let commands = clap::Command::new("gdbmtool-rs interactive mode")
        .multicall(true)
        .subcommand(clap::Command::new("exit").about("Exit the interpreter"))
        .subcommands(Context::commands());

    loop {
        let args = match rl.readline(&context.prompt()) {
            Ok(line) => {
                rl.add_history_entry(line.as_str()).ok();
                shellwords::split(&line).unwrap_or_else(|_| {
                    eprintln!("Bad command: mismatched quotation marks");
                    vec![]
                })
            }
            Err(ReadlineError::Eof) => vec!["exit".to_string()],
            Err(e) => {
                eprintln!("{e:?}");
                break ExitCode::FAILURE;
            }
        };

        if !args.is_empty() {
            match commands.clone().try_get_matches_from(args) {
                Ok(matches) => match matches.subcommand() {
                    Some(("exit", _)) => break ExitCode::SUCCESS,
                    Some((name, matches)) => context
                        .dispatch(name, matches)
                        .map(display)
                        .unwrap_or_else(|e| eprintln!("{e}")),
                    None => unreachable!(),
                },
                Err(e) => eprintln!("command line error: {e}"),
            }
        }
    }
}

fn single_command(mut context: Context, name: &str, matches: &clap::ArgMatches) -> ExitCode {
    context
        .dispatch(name, matches)
        .map(|lines| {
            lines.into_iter().for_each(|l| println!("{l}"));
            ExitCode::SUCCESS
        })
        .map_err(|e| eprintln!("{e}"))
        .unwrap_or(ExitCode::FAILURE)
}

fn command_stream(mut context: Context, reader: &mut impl BufRead) -> ExitCode {
    let commands = clap::Command::new("pipe")
        .multicall(true)
        .subcommands(Context::commands());

    reader
        .lines()
        .try_for_each(|l| {
            l.map_err(|e| format!("Input failure: {e}"))
                .and_then(|l| {
                    shellwords::split(&l)
                        .map_err(|_| "Bad input: mismatched quotation marks".to_string())
                })
                .and_then(|args| {
                    commands
                        .clone()
                        .try_get_matches_from(args)
                        .map_err(|e| e.to_string())
                })
                .and_then(|matches| {
                    let (name, matches) = matches.subcommand().unwrap();
                    context.dispatch(name, matches)
                })
                .map(|lines| lines.into_iter().for_each(|l| println!("{l}")))
        })
        .map(|_| ExitCode::SUCCESS)
        .map_err(|e| eprintln!("{e}"))
        .unwrap_or(ExitCode::FAILURE)
}
