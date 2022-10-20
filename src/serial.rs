use alloc::string::String;
use alloc::sync::Arc;
use core::fmt;
use core::fmt::Write;
use core::sync::atomic::Ordering::SeqCst;

use conquer_once::raw::Lazy;
use conquer_once::spin::Spin;
use crossbeam_queue::SegQueue;

use crate::concurrency::mutex::Mutex;
use crate::concurrency::rcu::RCU;
use crate::uart::SerialPort;
use crate::INITIALISED;

pub static SERIAL1: Lazy<Mutex<SerialPort>, Spin> = Lazy::new(|| {
    let mut serial_port = unsafe { SerialPort::new(0x3F8) };
    serial_port.init();
    Mutex::new(serial_port)
});

struct OutputBuffer(SegQueue<String>);

impl Drop for OutputBuffer {
    fn drop(&mut self) {
        let mut s = SERIAL1.lock();
        while let Ok(line) = self.0.pop() {
            s.write_str(&line).expect("could not write");
        }
    }
}

static BUFFER: Lazy<RCU<SegQueue<String>>, Spin> = Lazy::new(Default::default);

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    if !INITIALISED.load(SeqCst) {
        if let Some(mut s) = SERIAL1.try_lock() {
            s.write_fmt(args).unwrap();
        }
        return;
    }

    let mut result = String::new();
    result.write_fmt(args).unwrap();
    BUFFER.read().push(result);
}

#[doc(hidden)]
pub fn _eprint(args: fmt::Arguments) {
    if let Some(mut writer) = SERIAL1.try_lock() {
        writer.write_fmt(args).unwrap();
    } else {
        let mut writer = {
            let mut serial_port = unsafe { SerialPort::new(0x3F8) };
            serial_port.init();
            serial_port
        };
        writer.write_fmt(args).unwrap();
    }
    
}

pub fn flush() {
    if let Some(mut s) = SERIAL1.try_lock() {
        let old = BUFFER.update_and_get(SegQueue::new());
        while !old.is_empty() || Arc::strong_count(&old) != 1 {
            while let Ok(line) = old.pop() {
                s.write_str(&line).expect("could not write");
            }
        }
    }
}

/// Prints to the host through the serial interface.
#[macro_export]
macro_rules! serial_print {
    ($($arg:tt)*) => {
        $crate::serial::_print(format_args!($($arg)*));
    };
}

#[macro_export]
macro_rules! serial_eprint {
    ($($arg:tt)*) => {
        $crate::serial::_eprint(format_args!($($arg)*));
    };
}

/// Prints to the host through the serial interface, appending a newline.
#[macro_export]
macro_rules! serial_println {
    () => ($crate::serial_print!("\n"));
    ($fmt:expr) => ($crate::serial_print!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_print!(
        concat!($fmt, "\n"), $($arg)*));
}

#[macro_export]
macro_rules! serial_eprintln {
    () => ($crate::serial_eprint!("\n"));
    ($fmt:expr) => ($crate::serial_eprint!(concat!($fmt, "\n")));
    ($fmt:expr, $($arg:tt)*) => ($crate::serial_eprint!(
        concat!($fmt, "\n"), $($arg)*));
}
