use crate::interrupts::{InterruptFrame, StandardContext};

pub const USER_INTERRUPT_VECTOR: u8 = 0x80;

struct UserIntHandler {
    handlers: [Option<extern "C" fn(&mut InterruptFrame, &mut StandardContext)>; 256]
}

static mut INSTANCE: UserIntHandler = UserIntHandler::new();
impl UserIntHandler {
    const fn new() -> Self {
        Self {
            handlers: [None; 256]
        }
    }
}

impl UserIntHandler {
    fn _set_handler(&mut self, idx: usize, handler: extern "C" fn(&mut InterruptFrame, &mut StandardContext)) -> bool {
        if self.handlers[idx as usize].is_some() {
            return false;
        }
        
        self.handlers[idx as usize] = Some(handler);
        true
    }
}

pub fn attach_new_interrupt_handler(idx: u8, handler: extern "C" fn(&mut InterruptFrame, &mut StandardContext)) {
    unsafe {
        let result = INSTANCE._set_handler(idx as usize, handler);
        if !result {
            panic!("Interrupt handler already attached to idx {:?}", idx);
        }
    }
}

pub fn handle_user_interrupt(idx: usize, frame: &mut InterruptFrame, ctx: &mut StandardContext) {
    unsafe {
        if idx >= INSTANCE.handlers.len() {
            panic!("user interrupt index too high (max is 256):  {}", idx);
        }

        let handle = INSTANCE.handlers[idx];
        match handle {
            Some(func) => func(frame, ctx),
            None => panic!("unhandled user interrupt {}", idx)
        }
    }
}