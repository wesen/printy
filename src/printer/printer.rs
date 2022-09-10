use crate::printer::{Barcode, Columns, Dots, Underline, CR, DC2, ESC, FF, GS, LF};
use crate::{SerialPort, SerialPortSettings};
use bitvec::order::Msb0;
use bitvec::view::BitView;
use serial::SystemPort;
use std::cmp::max;
use std::io::Write;
use std::thread;
use std::time::Duration;

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
            dot_print_time: Duration::from_millis(20),
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
        // self.set_timeout(Self::BYTE_DURATION * cmd.len() as u32);
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

    pub fn print_bitmap(&mut self, w: Dots, h: Dots, bitmap: &[u8]) -> Result<(), anyhow::Error> {
        const CHUNK_SIZE: usize = 512;
        let w_in_bytes = (w + 7) / 8;
        let max_rows_in_chunk = (CHUNK_SIZE * 8) / w;

        println!(
            "w: {}, h: {}, w in bytes {}, max rows in chunk: {}",
            w, h, w_in_bytes, max_rows_in_chunk
        );

        self.dot_print_time = Duration::from_millis(5);
        bitmap.view_bits::<Msb0>()[..w * h]
            .chunks(w)
            .for_each(|row| {
                println!("{:?}", row);
            });

        // bitmaps use MSB, MSB printed left, data sent first printed left
        for chunk in bitmap.view_bits::<Msb0>()[..w * h]
            .chunks(max_rows_in_chunk * w)
            .into_iter()
        {
            let brows = chunk.len() / w;

            println!("{:?}", &[DC2, b'*', brows as u8, w_in_bytes as u8]);
            self.write_bytes(&[DC2, b'*', brows as u8, w_in_bytes as u8])?;
            let mut iter = chunk.into_iter();

            for _ in 0..brows {
                let mut b = [0u8; 48];
                for idx in 0..w {
                    let bit = iter.next().unwrap();
                    let byte = idx / 8;
                    let shift = 7 - idx % 8;
                    if *bit {
                        b[byte] |= 1 << shift;
                    }
                    print!("{}", if *bit { "1" } else { "0" });
                }
                println!("");
                self.write_bytes(&b[..w_in_bytes])?;
            }

            self.set_timeout(self.dot_print_time * brows as u32);
        }

        self.last_byte = LF;
        Ok(())
    }
}