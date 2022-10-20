use core::arch::asm;
use core::borrow::Borrow;

use conquer_once::spin::Lazy;
use x86_64::registers::rflags::RFlags;
use x86_64::registers::segmentation::{CS, Segment, SS};
use x86_64::structures::idt::InterruptDescriptorTable;
use x86_64::VirtAddr;

pub use user_interrupts::attach_new_interrupt_handler;

use crate::{eprintln, gdt, hlt_loop, println, serial, set_handler, set_handler_error_code, vga_buffer};
use crate::concurrency::mutex::Mutex;
use crate::interrupts::user_interrupts::handle_user_interrupt;
use crate::pic::ChainedPics;

mod idt;

#[macro_use]
mod entry;
mod user_interrupts;

pub const PIC_1_OFFSET: u8 = 32;
pub const PIC_2_OFFSET: u8 = PIC_1_OFFSET + 8;

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum InterruptIndex {
    Timer = PIC_1_OFFSET,
    Keyboard,
}

impl InterruptIndex {
    pub(crate) fn as_u8(self) -> u8 {
        self as u8
    }

    fn as_usize(self) -> usize {
        usize::from(self.as_u8())
    }
}

pub static PICS: Mutex<ChainedPics> =
    Mutex::new(unsafe { ChainedPics::new(PIC_1_OFFSET, PIC_2_OFFSET) });

#[derive(Debug, Default, Copy, Clone)]
#[repr(C)]
pub struct StandardContext {
    pub rax: usize,
    pub rbx: usize,
    pub rcx: usize,
    pub rdx: usize,
    pub rsi: usize,
    pub rdi: usize,
    pub r8: usize,
    pub r9: usize,
    pub r10: usize,
    pub r11: usize,
    pub r12: usize,
    pub r13: usize,
    pub r14: usize,
    pub r15: usize,
}

#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct InterruptFrame {
    pub instruction_pointer: u64,
    pub code_segment: u64,
    pub cpu_flags: RFlags,
    pub stack_pointer: u64,
    pub stack_segment: u64,
}

impl Default for InterruptFrame {
    fn default() -> Self {
        Self {
            cpu_flags: RFlags::INTERRUPT_FLAG.union(unsafe { RFlags::from_bits_unchecked(0x2) }),
            code_segment: CS::get_reg().0 as u64,
            stack_segment: SS::get_reg().0 as u64,
            instruction_pointer: 0,
            stack_pointer: 0,
        }
    }
}

static IDT: Lazy<InterruptDescriptorTable> = Lazy::new(|| {
    use crate::task::executor::timer_interrupt_handler;

    let mut idt = InterruptDescriptorTable::new();
    unsafe {
        set_handler!(idt.breakpoint, breakpoint_handler);
        set_handler_error_code!(idt.page_fault, page_fault_handler);
        set_handler_error_code!(idt.segment_not_present, segment_not_present_handler);
        set_handler_error_code!(idt.double_fault, double_fault_handler).set_stack_index(gdt::DOUBLE_FAULT_IST_INDEX);
        set_handler!(idt[InterruptIndex::Timer.as_usize()], timer_interrupt_handler);
        set_handler!(idt[InterruptIndex::Keyboard.as_usize()], keyboard_interrupt_handler);
        set_handler!(idt[user_interrupts::USER_INTERRUPT_VECTOR as usize], _handle_user_interrupt);
    }
    idt
});

pub fn init_idt() {
    IDT.load();
}

extern "C" fn _handle_user_interrupt(interrupt_frame: &mut InterruptFrame, ctx: &mut StandardContext) {
    handle_user_interrupt(ctx.rax, interrupt_frame, ctx);
}

extern "C" fn breakpoint_handler(interrupt_frame: &mut InterruptFrame, ctx: &mut StandardContext) {
    println!("EXCEPTION: BREAKPOINT\n{:#X?}, {:#X?}", interrupt_frame, ctx);
    serial::flush();
    vga_buffer::flush();
}

extern "C" fn page_fault_handler(
    interrupt_frame: &mut InterruptFrame, ctx: &mut StandardContext,
) {
    use x86_64::registers::control::Cr2;
    unsafe { crate::allocator::ALLOCATOR.inner.force_unlock(); }
    eprintln!(r"EXCEPTION: PAGE FAULT
    Accessed Address: {:X?}
    {:#X?}
    ", Cr2::read(), interrupt_frame);
    serial::flush();
    vga_buffer::flush();
    hlt_loop();
}

extern "C" fn segment_not_present_handler(
    interrupt_frame: &mut InterruptFrame, ctx: &mut StandardContext,
) {
    use x86_64::registers::control::Cr2;

    eprintln!(r"EXCEPTION: SEGMENT NOT PRESENT
    Accessed Address: {:X?}
    {:#X?}
    ", Cr2::read(), interrupt_frame);
    serial::flush();
    vga_buffer::flush();
    hlt_loop();
}

extern "C" fn double_fault_handler(
    interrupt_frame: &mut InterruptFrame, ctx: &mut StandardContext, error_code: u64,
) -> ! {
    panic!("EXCEPTION: DOUBLE FAULT {}, \n{:#X?} {:#X?}", error_code, interrupt_frame, ctx);
}


extern "C" fn keyboard_interrupt_handler() {
    use x86_64::instructions::port::Port;

    let mut port = Port::new(0x60);
    let scancode: u8 = unsafe { port.read() };
    crate::task::keyboard::add_scancode(scancode);

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Keyboard.as_u8());
    }
}

#[test_case]
fn test_breakpoint_exception() {
    // invoke a breakpoint exception
    x86_64::instructions::interrupts::int3();
}
