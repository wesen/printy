use serial::core::SerialDevice;
use serial::SerialPort as unix_SerialPort;
use serial::SystemPort;
use std::io::Write;
use std::thread;
use std::time::Duration;

type SerialError = anyhow::Error;

pub trait SerialPort {
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), SerialError>;
    fn wait(&mut self, d: Duration) -> Result<(), SerialError>;
}

pub struct UnixSerialPort<const BAUDRATE: u32 = 19200> {
    port: SystemPort,
}

impl<const BAUDRATE: u32> UnixSerialPort<BAUDRATE> {
    // a byte is 11 bits. There is no real flow control (although we do use XON/XOFF flow control
    // on unix, so we have to wait an estimation of the time to transmit the bytes over serial.
    // I am not sure what this will be on the hardware itself, since we will have to wait for the
    // peripheral to transmit anyway
    pub const BYTE_DURATION: Duration =
        Duration::from_micros(((11 * 1000000) + BAUDRATE / 2) as u64 / BAUDRATE as u64);

    pub fn new(mut port: SystemPort) -> Result<Self, SerialError> {
        port.reconfigure(&|settings| {
            settings.set_baud_rate(serial::Baud19200)?;
            settings.set_char_size(serial::Bits8);
            settings.set_parity(serial::ParityNone);
            settings.set_stop_bits(serial::Stop1);
            settings.set_flow_control(serial::FlowControl::FlowSoftware);
            Ok(())
        })?;
        <SystemPort as serial::SerialPort>::set_timeout(&mut port, Duration::from_millis(100))?;

        let settings = port.read_settings()?;
        println!("settings: {:?}", settings);
        // port.set_timeout(Duration::from_millis(100000))?;
        Ok(Self { port })
    }
}

impl<const BAUDRATE: u32> SerialPort for UnixSerialPort<BAUDRATE> {
    fn write_bytes(&mut self, bytes: &[u8]) -> Result<(), SerialError> {
        let res = self.port.write(bytes)?;
        if res != bytes.len() {
            anyhow::bail!("Could not write all bytes");
        }
        // manual flow control, if necessary
        // self.set_timeout(Self::BYTE_DURATION * cmd.len() as u32);
        Ok(())
    }

    fn wait(&mut self, d: Duration) -> Result<(), SerialError> {
        if d > Duration::from_millis(0) {
            println!("Waiting for {} ms", d.as_millis());
            thread::sleep(d);
            println!("Finished waiting");
        }
        Ok(())
    }
}
