use crate::interrupts::{InterruptFrame, StandardContext};
use crate::println;
use crate::task::executor::end_curr_task;
use alloc::boxed::Box;
use alloc::vec::Vec;

use core::cell::{Cell, UnsafeCell};

use core::ops::{DerefMut};

use core::{
    mem,
    pin::Pin,
    sync::atomic::{AtomicU64, Ordering},
};

pub mod executor;
pub mod keyboard;

const STACK_SIZE: usize = 8192;

pub type ContextState = (InterruptFrame, StandardContext);

#[derive(Debug)]
pub struct PreemptiveTask {
    id: TaskId,

    complete: Cell<bool>,
    stack: Pin<Box<UnsafeCell<[u8; STACK_SIZE]>>>,

    cont: Option<ContextState>,
    entrypoint: fn(),
}

extern "C" fn run_task(task: Pin<&PreemptiveTask>) {
    println!("BEGINNING TASK!");
    x86_64::instructions::interrupts::enable();
    (task.entrypoint)();
    println!("FINSIHED TASK");
    task.complete.replace(true);
}

fn entry_context_for(task: Pin<&PreemptiveTask>) -> ContextState {
    unsafe {
        let sp = (task.stack.get() as *const u8).add(STACK_SIZE - 8);
        let return_address = sp as *mut extern "C" fn() -> !;
        *return_address = end_curr_task;
        (
            InterruptFrame {
                instruction_pointer: run_task as extern "C" fn(Pin<&PreemptiveTask>) as *const ()
                    as u64,
                stack_pointer: sp as u64,
                ..Default::default()
            },
            StandardContext {
                rdi: &*task as *const PreemptiveTask as usize,
                ..Default::default()
            },
        )
    }
}

fn into_boxed_unsafecell<T, const N: usize>(inp: Box<[T; N]>) -> Box<UnsafeCell<[T; N]>> {
    unsafe { mem::transmute(inp) }
}

fn pinned_array_of_default<T: Default, const N: usize>() -> Pin<Box<UnsafeCell<[T; N]>>> {
    let mut vec = Vec::new();
    vec.resize_with(N, T::default);
    let boxed: Box<[T; N]> = match <Box<[T; N]>>::try_from(vec.into_boxed_slice()) {
        Ok(boxed) => boxed,
        Err(_) => unreachable!(),
    };

    into_boxed_unsafecell(boxed).into()
}

impl PreemptiveTask {
    fn new(entrypoint: fn()) -> Self {
        Self {
            id: TaskId::new(),
            complete: Cell::new(false),
            entrypoint,
            stack: pinned_array_of_default::<u8, STACK_SIZE>(),
            cont: None,
        }
    }

    fn poll(self: &mut Pin<impl DerefMut<Target = Self>>) -> Option<ContextState> {
        if self.complete.get() {
            return None;
        }

        let new_ctx = match self.cont.take() {
            None => entry_context_for(self.as_ref()),
            Some(ctx) => ctx,
        };

        Some(new_ctx)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct TaskId(u64);

impl TaskId {
    fn new() -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        TaskId(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}
