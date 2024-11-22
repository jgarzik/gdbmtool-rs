extern crate clap;
extern crate gdbm_native;

use clap::{Parser, ValueEnum};
use gdbm_native::{ExportBinMode, OpenOptions};

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
    /// Binary dump format
    Binary,

    /// ASCII dump format
    Ascii,
}

fn main() {
    let args = Args::parse();

    // Open db in read-only mode
    let mut db = OpenOptions::new()
        .open(&args.dbfn)
        .expect("Unable to open db");

    // Open write+create output dump file
    let mut outf = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true)
        .open(&args.outfn)
        .expect("Unable to open output file");

    // Call export API
    match args.format {
        OutputFormat::Binary => db
            .export_bin(&mut outf, ExportBinMode::ExpNative)
            .expect("Output error"),
        OutputFormat::Ascii => db.export_ascii(&mut outf).expect("Output error"),
    };
}
