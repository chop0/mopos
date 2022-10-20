




#[macro_export]
macro_rules! push_state {
    () => {
        r"
        sub rsp, {size}

        mov [rsp], rax
        mov [rsp + 8], rbx
        mov [rsp + 16], rcx
        mov [rsp + 24], rdx
        mov [rsp + 32], rsi
        mov [rsp + 40], rdi
        mov [rsp + 48], r8
        mov [rsp + 56], r9
        mov [rsp + 64], r10
        mov [rsp + 72], r11
        mov [rsp + 80], r12
        mov [rsp + 88], r13
        mov [rsp + 96], r14
        mov [rsp + 104], r15
        "
    };
}

#[macro_export]
macro_rules! pop_state {
    () => {
        r"
        mov rax, [rsp]
        mov rbx, [rsp + 8]
        mov rcx, [rsp + 16]
        mov rdx, [rsp + 24]
        mov rsi, [rsp + 32]
        mov rdi, [rsp + 40]
        mov r8, [rsp + 48]
        mov r9, [rsp + 56]
        mov r10, [rsp +64 ]
        mov r11, [rsp + 72]
        mov r12, [rsp + 80]
        mov r13, [rsp + 88]
        mov r14, [rsp + 96]
        mov r15, [rsp + 104]

        add rsp, {size}
        "
    };
}

macro_rules! ctx_save_trampoline_error_code {
    ($callback:ident) => {{
        #[naked]
        extern "C" fn handler() {
        unsafe {
            asm!(
            "
cli
// set up fake stack frame
push rbp
mov rbp, rsp
",
push_state!(),
"
lea rdi, [rbp + 16] // interrupt frame
mov rsi, rsp // standard context
mov rdx, [rbp + 8] // error code
call {callback}

",
pop_state!(),
"
mov rsp, rbp
pop rbp

add rsp, 8 // skip error code
iretq
            ",
            callback = sym $callback,
                        size = const core::mem::size_of::<StandardContext>(),
            options(noreturn)
            )
        }
    }
            handler
    }};
}

macro_rules! ctx_save_trampoline {
    ($callback:ident) => {{
        #[naked]
        extern "C" fn handler() {
        unsafe {
            asm!(
            "
cli
// set up fake stack frame
push rbp
mov rbp, rsp
",
push_state!(),
"
lea rdi, [rbp + 8] // interrupt frame
mov rsi, rsp // standard context
call {callback}
",
pop_state!(),
"
mov rsp, rbp
pop rbp

iretq
            ",
            callback = sym $callback,
            size = const core::mem::size_of::<StandardContext>(),

            options(noreturn)
            )
        }
            }
        handler
    }};
}

#[macro_export]
macro_rules! set_handler {
    ($target: expr, $handler: ident) => {
        $target.set_handler_addr(VirtAddr::new(ctx_save_trampoline!($handler) as u64))
    };
}

#[macro_export]
macro_rules! set_handler_error_code {
    ($target: expr, $handler: ident) => {
        $target.set_handler_addr(VirtAddr::new(
            ctx_save_trampoline_error_code!($handler) as u64
        ))
    };
}
