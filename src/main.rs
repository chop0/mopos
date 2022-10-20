#![no_std]
#![no_main]
#![feature(custom_test_frameworks, naked_functions)]
#![test_runner(barefuzz::test_runner)]
#![reexport_test_harness_main = "test_main"]

extern crate alloc;

use core::panic::PanicInfo;
use core::sync::atomic::Ordering;

use bootloader::{BootInfo, entry_point};

use barefuzz::{
    allocator, eprintln, INITIALISED, LOCKS, memory, println, serial, vga_buffer,
};
use barefuzz::interrupts::PICS;
use barefuzz::memory::BootInfoFrameAllocator;
use barefuzz::serial::SERIAL1;
use barefuzz::task::executor;
use barefuzz::vga_buffer::WRITER;

entry_point!(_kernel_entry);
fn _kernel_entry(boot_info: &'static BootInfo) -> ! {
    use x86_64::VirtAddr;

    barefuzz::init();

    let phys_mem_offset = VirtAddr::new(boot_info.physical_memory_offset);
    let mut mapper = unsafe { memory::init(phys_mem_offset) };
    let mut frame_allocator = unsafe { BootInfoFrameAllocator::init(&boot_info.memory_map) };

    allocator::init_heap(&mut mapper, &mut frame_allocator).expect("heap initialisation failed");
    LOCKS.lock().push(&SERIAL1.semaphore);
    LOCKS.lock().push(&WRITER.semaphore);
    LOCKS.lock().push(&PICS.semaphore);

    executor::init();
    INITIALISED.store(true, Ordering::SeqCst);
    // kernel_main()
    unwinding::panic::catch_unwind(kernel_main).unwrap()
}

fn kernel_main() -> ! {
    {
        let mut executor = executor::INSTANCE.get().unwrap().lock();
        executor.spawn(|| loop {
            serial::flush();
            vga_buffer::flush();
        });

        executor.spawn(|| {
            let mut i = 10_000;
            loop {
                i += 1;
                if i % 10000 == 0 {
                    println!("t1 {}", i);
                }
            }
        });

        executor.spawn(|| {
            let mut i = 0;
            loop {
                i += 1;
                if i % 10000 == 0 {
                    println!("t2 {}", i);
                }
            }
        });
    }
    loop {
        // yield_();
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    unsafe {
        LOCKS.force_unlock();
    }
    for lock in LOCKS.lock().iter() {
        if lock.try_acquire(1).is_none() {
            lock.release(1);
        }
    }

    extern "C" {
        // Symbol defined by the linker
        static __executable_start: [u8; 0];
    }

    serial::flush();
    vga_buffer::flush();
    eprintln!("{}", info);

    serial::flush();
    vga_buffer::flush();
    barefuzz::hlt_loop();
}
