use std::cmp::max;
use std::io::Write;
use std::thread;
use std::time::Duration;

use bitvec::prelude::*;
/// Thermal Printer from Adafruit interface
///
/// Port of the C++ library at https://github.com/adafruit/Adafruit-Thermal-Printer-Library/
use chrono::Utc;
use serial::prelude::*;
use serial::SystemPort;

type Dots = u8;
type Columns = u8;

pub struct Printer<const BAUDRATE: u32 = 19200> {
    port: SystemPort,
    // TODO(manuel) Might be better to make this a deadline, really
    timeout: Duration,

    last_byte: u8,
    last_column: Columns,
    max_column: Columns,
    char_height: Dots,
    inter_line_spacing: Dots,
    barcode_height: Dots,
    max_chunk_height: u8,

    firmware_version: u16,

    dot_print_time: Duration,
    dot_feed_time: Duration,
}

pub enum Underline {
    None,
    Single,
    Double,
}

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

impl<const BAUDRATE: u32> Printer<BAUDRATE> {
    // a byte is 11 bits. There is no real flow control (although we do use XON/XOFF flow control
    // on unix, so we have to wait an estimation of the time to transmit the bytes over serial.
    // I am not sure what this will be on the hardware itself, since we will have to wait for the
    // peripheral to transmit anyway
    pub const BYTE_DURATION: Duration =
        Duration::from_micros(((11 * 1000000) + BAUDRATE / 2) as u64 / BAUDRATE as u64);

    pub fn new(port: SystemPort) -> Result<Self, anyhow::Error> {
        let mut f = Self {
            port,
            timeout: Duration::from_millis(0),

            last_byte: LF,
            last_column: 0,
            max_column: 32,
            char_height: 24,
            inter_line_spacing: 6,
            barcode_height: 50,
            max_chunk_height: 255,
            firmware_version: 268,
            dot_print_time: Duration::from_millis(30),
            dot_feed_time: Duration::from_micros(2100),
        };

        f.port.reconfigure(&|settings| {
            settings.set_baud_rate(serial::Baud19200)?;
            settings.set_char_size(serial::Bits8);
            settings.set_parity(serial::ParityNone);
            settings.set_stop_bits(serial::Stop1);
            settings.set_flow_control(serial::FlowControl::FlowSoftware);
            Ok(())
        })?;
        // first command should wait a bit
        f.set_timeout(Duration::from_millis(500));

        Ok(f)
    }

    pub fn init(&mut self) -> Result<(), anyhow::Error> {
        self.cmd_init()?;
        self.last_byte = LF;
        self.last_column = 0;
        self.max_column = 32;
        self.char_height = 24;
        self.inter_line_spacing = 6;
        self.barcode_height = 50;

        // TODO configure tab stops
        if self.firmware_version >= 264 {
            self.write_bytes(&[ESC, b'D', 4, 8, 12, 16, 20, 24, 28, 0])?;
        }

        // self.cmd_online()?;
        // self.cmd_justify('L')?;
        // self.cmd_double_height(false)?;
        // self.set_line_height(30)?;
        // self.set_bold(false)?;
        // self.set_underline(Underline::None)?;
        // self.set_barcode_height(50)?;
        // self.set_size('s')?;
        // self.set_charset()?;
        // self.set_code_page()?;
        self.cmd_set_heat_config(11, Duration::from_micros(120), Duration::from_micros(40))?;

        Ok(())
    }

    fn set_timeout(&mut self, timeout: Duration) {
        self.timeout = timeout;
    }

    pub fn wait(&mut self) {
        println!("Waiting for {} ms", self.timeout.as_millis());
        thread::sleep(self.timeout);
        self.timeout = Duration::from_millis(0);
    }

    /// Returns the duration for an empty feed line
    fn feed_duration(&self) -> Duration {
        (self.char_height + self.inter_line_spacing) as u32 * self.dot_feed_time
    }

    /// Returns the duration for a text line to be printed
    fn text_line_duration(&self) -> Duration {
        (self.char_height as u32 * self.dot_print_time)
            + (self.inter_line_spacing as u32 * self.dot_feed_time)
    }

    pub fn write_bytes(&mut self, cmd: &[u8]) -> Result<(), anyhow::Error> {
        self.wait();
        self.port.write(cmd)?;
        self.set_timeout(Self::BYTE_DURATION * cmd.len() as u32);
        Ok(())
    }

    pub fn print_barcode(&mut self, s: &str, barcode_type: Barcode) -> Result<(), anyhow::Error> {
        self.cmd_feed(1)?;
        let mut barcode_type = barcode_type as u8;
        if self.firmware_version >= 264 {
            barcode_type += 65;
        }
        // Select printing position of human readable character
        self.write_bytes(&[GS, b'H', 2])?; // below the barcode

        // Set barcode width
        self.write_bytes(&[GS, b'w', 3])?;

        if self.firmware_version >= 264 {
            self.write_bytes(&[GS, b'k', barcode_type, s.len() as u8])?;
            self.write_bytes(s.as_ref())?;
        } else {
            self.write_bytes(&[GS, b'k', barcode_type])?;
            self.write_bytes(s.as_ref())?;
            self.write_bytes(&[0])?;
        }
        self.set_timeout((self.barcode_height as u32 + 40) * self.dot_print_time);
        self.last_byte = LF;
        Ok(())
    }

    pub fn write_char(&mut self, c: char) -> Result<(), anyhow::Error> {
        let c = c as u8;
        if c == CR {
            return Ok(());
        }

        self.wait();
        self.port.write(&[c as u8])?;
        let mut d = Self::BYTE_DURATION;

        if c == LF || self.last_column >= self.max_column {
            d += if self.last_byte == LF {
                self.feed_duration()
            } else {
                self.text_line_duration()
            };
            self.last_column = 0;
            self.last_byte = LF;
        } else {
            self.last_column += 1;
            self.last_byte = c;
        }

        self.set_timeout(d);
        Ok(())
    }

    pub fn write(&mut self, s: &str) -> Result<(), anyhow::Error> {
        for c in s.chars() {
            self.write_char(c)?;
        }
        Ok(())
    }

    pub fn cmd_feed(&mut self, lines: u8) -> Result<(), anyhow::Error> {
        if self.firmware_version >= 264 {
            self.write_bytes(&[ESC, b'd', lines])?;
            self.set_timeout(self.dot_feed_time * self.char_height as u32);
            self.last_byte = LF;
            self.last_column = 0;
        } else {
            for n in 1..lines {
                self.write_char('\n')?;
            }
        }

        Ok(())
    }

    pub fn cmd_wake(&mut self) -> Result<(), anyhow::Error> {
        self.set_timeout(Duration::from_millis(0));
        self.write_bytes(&[0xFF])?;
        self.set_timeout(Duration::from_millis(50));

        if self.firmware_version > 264 {
            // sleep off
            self.write_bytes(&[ESC, b'8', 0, 0])?;
            self.set_timeout(Duration::from_millis(50));
        } else {
            for i in 0..10 {
                self.write_bytes(&[0])?;
                self.set_timeout(Duration::from_millis(10));
            }
        }
        Ok(())
    }

    pub fn cmd_init(&mut self) -> Result<(), anyhow::Error> {
        self.write_bytes(&[ESC, b'@'])?;
        self.set_timeout(Duration::from_millis(100));
        Ok(())
    }

    pub fn cmd_flush(&mut self) -> Result<(), anyhow::Error> {
        self.write_bytes(&[FF])?;
        // TODO(manuel) compute the duration
        Ok(())
    }

    pub fn cmd_set_heat_config(
        &mut self,
        dots: u8,
        heating_time: Duration,
        heating_interval: Duration,
    ) -> Result<(), anyhow::Error> {
        self.write_bytes(&[
            ESC,
            b'7',
            dots,
            (heating_time.as_micros() / 10).try_into()?,
            (heating_interval.as_micros() / 10).try_into()?,
        ])?;
        Ok(())
    }

    pub fn cmd_set_print_density(
        &mut self,
        density: u8,
        break_time: Duration,
    ) -> Result<(), anyhow::Error> {
        let break_time: u8 = (break_time.as_micros() / 250).try_into()?;
        self.port
            .write(&[27, '#' as u8, density | ((break_time & 0x7) << 5)])?;
        thread::sleep(Duration::from_millis(1));
        Ok(())
    }

    pub fn cmd_set_underline(&mut self, underline: Underline) -> Result<(), anyhow::Error> {
        let underline = match underline {
            Underline::None => 0,
            Underline::Single => 1,
            Underline::Double => 2,
        };
        self.port.write(&[ESC, '-' as u8, underline])?;
        thread::sleep(Duration::from_millis(1));
        Ok(())
    }

    pub fn set_barcode_height(&mut self, val: u8) -> Result<(), anyhow::Error> {
        self.write_bytes(&[GS, b'h', max(1, val)])?;
        Ok(())
    }

    pub fn cmd_test_page(&mut self) -> Result<(), anyhow::Error> {
        self.write_bytes(&[DC2, b'T'])?;
        let test_page_duration = self.dot_print_time * 24 * 26 + // 26 lines with text
            self.dot_feed_time * (6 * 26 + 30); // 26 text lines (feed 6 dots) + blank line
        self.set_timeout(test_page_duration);
        Ok(())
    }

    pub fn print_bitmap(&mut self, w: usize, h: usize, bitmap: &[u8]) -> Result<(), anyhow::Error> {
        // modes are:
        // 0 - 8dots single density, 102 dpi
        // 1 - 8dots double density, 203 dpi
        // 31 - 24dots single density, 102 dpi
        // 32 - 24dots double density, 203 dpi

        // bitmaps use MSB, MSB printed left, data sent first printed left
        for chunk in bitmap.view_bits::<Msb0>().chunks(w).into_iter() {
            let mut b = [0u8; 48];
            for (idx, bit) in chunk.into_iter().enumerate() {
                let byte = idx / 8;
                let shift = 7 - idx % 8;
                if *bit {
                    b[byte] |= 1 << shift;
                }
            }
            let w = (w + 7) / 8;
            self.write_bytes(&[DC2, b'*', w as u8, 1])?;
            self.write_bytes(&b[..w])?;
        }
        self.set_timeout(self.dot_print_time * h as u32);
        self.last_byte = LF;
        Ok(())
    }
}

fn main() {
    let mut port = serial::open("/dev/cu.usbserial-2120").unwrap();
    let mut printer: Printer<19200> = Printer::new(port).unwrap();
    println!("{}: Initializing", Utc::now().time().to_string());
    printer.init().unwrap();
    // println!("{}: Print test page", Utc::now().time().to_string());
    // printer.cmd_test_page().unwrap();
    // printer.wait();
    // println!("{}: Finished test page", Utc::now().time().to_string());

    // printer.cmd_feed(3).unwrap();
    // println!("{}: Print text", Utc::now().time().to_string());
    // for i in 0..10 {
    //     printer.write(&format!("Hello world {}\n", i)).unwrap();
    // }
    // printer.cmd_feed(3).unwrap();
    // printer.wait();

    // printer
    //     .print_barcode("123456789012", Barcode::UpcA)
    //     .unwrap();
    // printer.write("\n---\n").unwrap();
    // printer.init().unwrap();
    // printer.write("\n---\n").unwrap();
    // printer.init().unwrap();
    // printer.write("\n---\n").unwrap();
    // for i in 0..5 {
    //     printer.write(&format!("Hello world {}\n", i)).unwrap();
    // }

    printer.print_bitmap(384, 5, &[0xaa; 48 * 5]).unwrap();

    printer.cmd_feed(3).unwrap();
    printer.wait();

    println!("{}: Finished text", Utc::now().time().to_string());
}
