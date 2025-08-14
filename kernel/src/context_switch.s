.section .text
.global switch_context

# switch_context(old_context: *mut ProcessContext, new_context: *const ProcessContext)
# RDI = old_context pointer
# RSI = new_context pointer
switch_context:
    # Save current context if old_context is not null
    test %rdi, %rdi
    jz load_new_context
    
    # Save general purpose registers
    movq %rax, 0(%rdi)   # rax
    movq %rbx, 8(%rdi)   # rbx
    movq %rcx, 16(%rdi)  # rcx
    movq %rdx, 24(%rdi)  # rdx
    movq %rsi, 32(%rdi)  # rsi
    movq %rdi, 40(%rdi)  # rdi (save original value)
    movq %rbp, 48(%rdi)  # rbp
    movq %rsp, 56(%rdi)  # rsp
    movq %r8, 64(%rdi)   # r8
    movq %r9, 72(%rdi)   # r9
    movq %r10, 80(%rdi)  # r10
    movq %r11, 88(%rdi)  # r11
    movq %r12, 96(%rdi)  # r12
    movq %r13, 104(%rdi) # r13
    movq %r14, 112(%rdi) # r14
    movq %r15, 120(%rdi) # r15
    
    # Save return address as RIP
    movq (%rsp), %rax
    movq %rax, 128(%rdi) # rip
    
    # Save RFLAGS
    pushfq
    popq %rax
    movq %rax, 136(%rdi) # rflags
    
    # Save segment selectors
    movq $0x08, 144(%rdi) # cs (kernel code segment)
    movq $0x10, 152(%rdi) # ss (kernel data segment)

load_new_context:
    # Load new context from new_context pointer (RSI)
    # Restore general purpose registers
    movq 0(%rsi), %rax   # rax
    movq 8(%rsi), %rbx   # rbx
    movq 16(%rsi), %rcx  # rcx
    movq 24(%rsi), %rdx  # rdx
    # Skip rsi and rdi for now
    movq 48(%rsi), %rbp  # rbp
    movq 56(%rsi), %rsp  # rsp
    movq 64(%rsi), %r8   # r8
    movq 72(%rsi), %r9   # r9
    movq 80(%rsi), %r10  # r10
    movq 88(%rsi), %r11  # r11
    movq 96(%rsi), %r12  # r12
    movq 104(%rsi), %r13 # r13
    movq 112(%rsi), %r14 # r14
    movq 120(%rsi), %r15 # r15
    
    # Push new RIP onto stack for return
    movq 128(%rsi), %rax # rip
    pushq %rax
    
    # Restore RFLAGS
    movq 136(%rsi), %rax # rflags
    pushq %rax
    popfq
    
    # Finally restore rsi and rdi
    movq 40(%rsi), %rdi  # rdi
    movq 32(%rsi), %rsi  # rsi (this must be last)
    
    # Return to new context
    ret