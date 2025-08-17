.section .text
.global _start

_start:
    # Test sys_getpid (syscall 5)
    mov $5, %rax        # syscall number for getpid
    syscall
    
    # Test sys_write (syscall 13) - write "Hello from Ring3!\n" to stdout
    mov $13, %rax       # syscall number for write
    mov $1, %rdi        # fd = 1 (stdout)
    mov $hello_msg, %rsi # buffer pointer
    mov $18, %rdx       # length
    syscall
    
    # Test sys_sleep (syscall 7) - sleep for 1 second
    mov $7, %rax        # syscall number for sleep
    mov $1000, %rdi     # sleep for 1000ms
    syscall
    
    # Test sys_exit (syscall 0) - exit with code 42
    mov $0, %rax        # syscall number for exit
    mov $42, %rdi       # exit code
    syscall
    
    # Should never reach here
    jmp .

.section .rodata
hello_msg:
    .ascii "Hello from Ring3!\n"