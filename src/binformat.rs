// Binary dump length field layouts, shared by the command line tools.

use clap::ValueEnum;
use gdbm_native::BinDumpFormat;

/// Layout of the length fields of a binary dump, which depends on the platform of the C GDBM
/// that wrote it or will read it.
#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum BinFormat {
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
