extern crate clap;
extern crate rs_gdbm;

use clap::{Parser, ValueEnum};
use rs_gdbm::{ExportBinMode, Gdbm};
use std::fs::OpenOptions;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Pathname of GDBM database to open
    dbfn: String,

    /// Output target for export data
    outfn: String,

    /// Select output format, binary or ascii
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Binary)]
    format: OutputFormat,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum OutputFormat {
    /// binary format
    Binary,

    /// ascii format
    Ascii,
}

fn main() {
    let args = Args::parse();

    let mut db = Gdbm::open(&args.dbfn).expect("Unable to open database");
    let mut outf = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&args.outfn)
        .expect("Unable to open output file");

    let _iores = match args.format {
        OutputFormat::Binary => db
            .export_bin(&mut outf, ExportBinMode::ExpNative)
            .expect("Output error"),
        OutputFormat::Ascii => db.export_ascii(&mut outf).expect("Output error"),
    };
}