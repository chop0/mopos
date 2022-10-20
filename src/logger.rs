use core::fmt::Write;
use vga::writers::{Text80x25, TextWriter};

struct Logger {
    mode: Text80x25,
    // buffer: ConstGenericRingBuffer<>
}

impl Write for Logger {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        panic!()
        // self.mode.write_character()
    }

    fn write_char(&mut self, c: char) -> core::fmt::Result {
panic!()
    }
}