mod printer;

use clap::ValueEnum;
pub use printer::Printer;
mod serial;
pub use crate::printer::serial::{SerialPort, UnixSerialPort};

/// Thermal Printer from Adafruit interface
///
/// Port of the C++ library at https://github.com/adafruit/Adafruit-Thermal-Printer-Library/

type Dots = usize;
type Columns = u8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum Underline {
    None,
    Single,
    Double,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum Charset {
    Usa = 0,
    France = 1,
    Germany = 2,
    Uk = 3,
    Denmark1 = 4,
    Sweden = 5,
    Italy = 6,
    Spain1 = 7,
    Japan = 8,
    Norway = 9,
    Denmark2 = 10,
    Spain2 = 11,
    LatinAmerica = 12,
    Korea = 13,
    Slovenia = 14,
    China = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum CodePage {
    Cp437C = 0,
    Katakana = 1,
    Cp850 = 2,
    Cp860 = 3,
    Cp863 = 4,
    Cp865 = 5,
    WPC1251 = 6,
    Cp866 = 7,
    Mik = 8,
    Cp755 = 9,
    Iran = 10,
    Cp862 = 15,
    WPC1252 = 16,
    WPC1253 = 17,
    Cp852 = 18,
    Cp858 = 19,
    Iran2 = 20,
    Latvian = 21,
    Cp864 = 22,
    Iso8859_1 = 23,
    Cp737 = 24,
    WPC1257 = 25,
    Thai = 26,
    Cp720 = 27,
    Cp855 = 28,
    Cp857 = 29,
    WPC1250 = 30,
    Cp775 = 31,
    WPC1254 = 32,
    WPC1255 = 33,
    WPC1256 = 34,
    WPC1258 = 35,
    Iso8859_2 = 36,
    Iso8859_3 = 37,
    Iso8859_4 = 38,
    Iso8859_5 = 39,
    Iso8859_6 = 40,
    Iso8859_7 = 41,
    Iso8859_8 = 42,
    Iso8859_9 = 43,
    Iso8859_15 = 44,
    Thai2 = 45,
    Cp856 = 46,
    Cp874 = 47,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum Barcode {
    UpcA,
    UpcE,
    Ean13,
    Ean8,
    Code39,
    Itf,
    Codabar,
    Code93,
    Code128,
}

const LF: u8 = b'\n';
const TAB: u8 = b'\t';
const FF: u8 = 12;
const CR: u8 = b'\r';
const DC2: u8 = 18;
const ESC: u8 = 27;
const FS: u8 = 28;
const GS: u8 = 29;
