use clap::{Parser, ValueEnum};
use gdbm_native::{BinDumpFormat, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

/// Export a GDBM database to a binary or ASCII dump file
#[derive(Parser, Debug)]
#[command(author, version)]
struct Args {
    /// Pathname of GDBM database to open
    dbfn: PathBuf,

    /// Output target for export data, or "-" for standard output
    outfn: PathBuf,

    /// Select output format, binary or ascii
    #[arg(short, long, value_enum, default_value_t = OutputFormat::Binary)]
    format: OutputFormat,

    /// Length field layout of binary dumps, for the platform of the C GDBM that will read them
    #[arg(long, value_enum, default_value_t = BinFormat::Host)]
    bin_format: BinFormat,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum OutputFormat {
    /// Binary dump format (C GDBM gdbm_export)
    Binary,

    /// ASCII dump format (C GDBM gdbm_dump), portable between platforms
    Ascii,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum BinFormat {
    /// The platform this program runs on
    Host,

    /// 4-byte lengths: 32-bit platforms and Windows
    Long32,

    /// 8-byte lengths: 64-bit little-endian platforms such as x86-64 and arm64
    Long64le,

    /// 8-byte lengths: 64-bit big-endian platforms such as s390x
    Long64be,
}

impl From<BinFormat> for BinDumpFormat {
    fn from(format: BinFormat) -> Self {
        match format {
            BinFormat::Host => BinDumpFormat::host(),
            BinFormat::Long32 => BinDumpFormat::Long32,
            BinFormat::Long64le => BinDumpFormat::Long64Le,
            BinFormat::Long64be => BinDumpFormat::Long64Be,
        }
    }
}

fn main() -> ExitCode {
    match dump(&Args::parse()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("gdbmdump-rs: {e}");
            ExitCode::FAILURE
        }
    }
}

fn dump(args: &Args) -> Result<(), String> {
    // Open db in read-only mode
    let db = OpenOptions::new()
        .open(&args.dbfn)
        .map_err(|e| format!("{}: {e}", args.dbfn.display()))?;

    // Write to stdout, or create/truncate the output dump file
    let mut out: Box<dyn Write> = if args.outfn.as_os_str() == "-" {
        Box::new(std::io::stdout().lock())
    } else {
        Box::new(
            std::fs::File::create(&args.outfn)
                .map_err(|e| format!("{}: {e}", args.outfn.display()))?,
        )
    };
    let mut out = std::io::BufWriter::new(&mut out);

    // Call export API
    match args.format {
        OutputFormat::Binary => db.export_bin(&mut out, args.bin_format.into()),
        OutputFormat::Ascii => db.export_ascii(&mut out, Some(&args.dbfn)),
    }
    .map_err(|e| e.to_string())?;

    out.flush().map_err(|e| e.to_string())
}
