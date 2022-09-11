use libc::c_int;
use std::ffi::CString;
use std::os::unix::prelude::*;
use termios::{tcflush, tcsetattr, Termios};

#[cfg(target_os = "linux")]
const O_NOCTTY: c_int = 0x00000100;

#[cfg(target_os = "macos")]
const O_NOCTTY: c_int = 0x00020000;

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
const O_NOCTTY: c_int = 0;

pub fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("Please provide a serial port");
    println!("Opening {}", path);

    use libc::{EINVAL, F_SETFL, O_NONBLOCK, O_RDWR};

    let cstr = CString::new(path.as_str().as_bytes()).expect("Could not convert path");

    let fd = unsafe { libc::open(cstr.as_ptr(), O_RDWR | O_NOCTTY | O_NONBLOCK, 0) };
    if fd < 0 {
        panic!("Could not open {}", path);
    }

    println!("Getting termios from {:?}", fd);

    let mut t = Termios::from_fd(fd.as_raw_fd()).expect("Failed to get termios");
    println!("Before {:?}", t.c_iflag);
    t.c_iflag |= libc::IXON | libc::IXOFF;
    println!("Setting {:?}", t.c_iflag);

    tcsetattr(fd.as_raw_fd(), libc::TCSANOW, &t).unwrap();
    tcflush(fd.as_raw_fd(), libc::TCIOFLUSH).unwrap();

    t = Termios::from_fd(fd.as_raw_fd()).expect("Failed to get termios");
    println!("After {:?}", t.c_iflag);

    t.c_iflag |= libc::IXON | libc::IXOFF | libc::IXANY | libc::INLCR;
    println!("Setting {:?}", t.c_iflag);

    tcsetattr(fd.as_raw_fd(), libc::TCSANOW, &t).unwrap();
    tcflush(fd.as_raw_fd(), libc::TCIOFLUSH).unwrap();
    t = Termios::from_fd(fd.as_raw_fd()).expect("Failed to get termios");
    println!("After {:?}", t.c_iflag);
}
