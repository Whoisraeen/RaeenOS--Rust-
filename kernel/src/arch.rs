#![allow(dead_code)]

// Low-level architecture support (x86_64)
// Provides the `switch_context` symbol used by the scheduler to switch kernel contexts.

use core::arch::global_asm;

// Context layout (offsets in bytes) must match `process::ProcessContext`:
//  0  rax, 8  rbx, 16 rcx, 24 rdx, 32 rsi, 40 rdi, 48 rbp, 56 rsp,
//  64 r8,  72 r9,  80 r10, 88 r11, 96 r12, 104 r13, 112 r14, 120 r15,
//  128 rip, 136 rflags, 144 cs, 152 ss

global_asm!(r#"
    .globl switch_context
    .type switch_context,@function
switch_context:
    // SysV: rdi = old_context ptr, rsi = new_context ptr
    // Save callee-saved regs and stack into old context
    test rdi, rdi
    jz 1f
    mov [rdi + 8], rbx
    mov [rdi + 48], rbp
    mov [rdi + 96], r12
    mov [rdi + 104], r13
    mov [rdi + 112], r14
    mov [rdi + 120], r15
    mov [rdi + 56], rsp
    // Save return RIP from top of stack
    mov rax, [rsp]
    mov [rdi + 128], rax
1:
    // Load callee-saved regs and stack from new context
    mov rbx, [rsi + 8]
    mov rbp, [rsi + 48]
    mov r12, [rsi + 96]
    mov r13, [rsi + 104]
    mov r14, [rsi + 112]
    mov r15, [rsi + 120]
    mov rsp, [rsi + 56]
    // Jump to new RIP (kernel ring0 threads only)
    jmp qword ptr [rsi + 128]
"#);

extern "C" {
    pub fn switch_context(old_context: *mut crate::process::ProcessContext, new_context: *const crate::process::ProcessContext);
}


