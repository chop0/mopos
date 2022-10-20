use alloc::{collections::BTreeMap, sync::Arc, task::Wake};
use alloc::boxed::Box;
use alloc::collections::LinkedList;
use core::arch::asm;
use core::future::Future;
use core::pin::Pin;
use core::sync::atomic::Ordering::SeqCst;
use core::task::Waker;

use conquer_once::spin::OnceCell;
use crossbeam_queue::ArrayQueue;
use x86_64::instructions::interrupts::without_interrupts;

use crate::interrupts::{InterruptIndex, PICS, StandardContext, InterruptFrame, attach_new_interrupt_handler};
use crate::{INITIALISED, println, println_immediate, serial, vga_buffer};
use crate::concurrency::mutex::{Mutex, MutexGuard};
use crate::task::PreemptiveTask;

use super::TaskId;

pub struct Executor {
    tasks: BTreeMap<TaskId, Pin<Box<PreemptiveTask>>>,
    task_queue: LinkedList<TaskId>,
    active_task: Option<TaskId>,
}

const TASK_DONE_INTERRUPT: u8 = 0;
const YIELD_INTERRUPT: u8 = 1;

fn _on_task_done(_: &mut StandardContext) {
    let mut guard = INSTANCE.get().unwrap().lock();
    if let Some(task) = guard.active_task {
        guard.tasks.remove(&task);
    }

    x86_64::instructions::hlt();
}

pub extern "C" fn end_curr_task() -> ! {
    unsafe {
        asm!(r"
        mov rax, 0
        int 0x80
        ", options(noreturn))
    }
}

pub extern "C" fn yield_()  {
    unsafe {
        // asm!(r"
        // mov rax, 1
        // int 0x80
        // ", options(noreturn))
    }
}

pub extern "C" fn timer_interrupt_handler(
    interrupt_frame: &mut InterruptFrame, ctx: &mut StandardContext,
) {
    if !INITIALISED.load(SeqCst) {
        unsafe {
            PICS.lock()
                .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
        }
        return
    }

    if let Some(Some(mut guard)) = INSTANCE.get().map(|x| x.try_lock()) {
        if let Some(current_task) = guard.active_task.take() {
            if let Some(current_task_) = guard.tasks.get_mut(&current_task) {
                current_task_.cont = Some((*interrupt_frame, *ctx));
                guard.task_queue.push_back(current_task);
            }
        }

        guard.scheduler_loop(interrupt_frame, ctx);
    };

    unsafe {
        PICS.lock()
            .notify_end_of_interrupt(InterruptIndex::Timer.as_u8());
    }
}


pub static INSTANCE: OnceCell<Mutex<Executor>> = OnceCell::uninit();

pub fn init() {
    INSTANCE.get_or_init(|| Mutex::new(Executor::new()));
    extern "C" fn handle(_: &mut InterruptFrame, ctx: &mut StandardContext) {
        _on_task_done(ctx);
    }
    attach_new_interrupt_handler(TASK_DONE_INTERRUPT, handle);
    attach_new_interrupt_handler(YIELD_INTERRUPT, timer_interrupt_handler);

    println!("init");
}

impl Executor {
    fn timeslice_expired(&self) -> bool {
        true
    }

    fn new() -> Self {
        Executor {
            tasks: BTreeMap::new(),
            task_queue: LinkedList::new(),
            active_task: None,
        }
    }

    pub fn spawn(&mut self, task: fn()) -> Option<TaskId> {
        let id = TaskId::new();
        println!("1 {:?}", task as *const core::ffi::c_void);

        self.tasks.insert(id, Box::pin(PreemptiveTask::new(task)));
        println!("2");

        self.task_queue.push_back(id);
        println!("3");
        Some(id)
    }

    pub fn scheduler_loop(mut self: MutexGuard<Self>, ictx: &mut InterruptFrame, sctx: &mut StandardContext) {
        if let Some(next_task) = self.task_queue.pop_front() &&
        let Some(task) = self.tasks.get_mut(&next_task) &&
        let Some(ctx) = task.poll()
        {
            self.active_task = Some(next_task);
            (*ictx, *sctx) = ctx;
        }
    }
}


struct TaskWaker {
    task_id: TaskId,
    task_queue: Arc<ArrayQueue<TaskId>>,
}

impl TaskWaker {
    fn new(task_id: TaskId, task_queue: Arc<ArrayQueue<TaskId>>) -> Waker {
        Waker::from(Arc::new(TaskWaker {
            task_id,
            task_queue,
        }))
    }

    fn wake_task(&self) {
        self.task_queue.push(self.task_id).expect("task_queue full");
    }
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_task();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.wake_task();
    }
}
