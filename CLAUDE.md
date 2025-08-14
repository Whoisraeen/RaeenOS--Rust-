# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

RaeenOS is a modern, secure operating system written primarily in Rust. This is a from-scratch OS implementation featuring a microkernel architecture (RaeCore), modern security practices, and ambitious compatibility goals including Windows and Android app support.

## Build System and Commands

### Core Build Commands
- `cargo run --bin raeen-build -- all` - Build all components (kernel, userspace, bootloader, compatibility layers)
- `cargo run --bin raeen-build -- kernel` - Build only the kernel
- `cargo run --bin raeen-build -- userspace` - Build userspace components  
- `cargo run --bin raeen-build -- test` - Run test suite
- `cargo run --bin raeen-build -- check` - Check code without building
- `cargo run --bin raeen-build -- clean` - Clean build artifacts
- `cargo run --bin raeen-build -- docs` - Build documentation

### Direct Kernel Development
- `cd kernel && cargo build --target x86_64-unknown-none` - Build kernel directly
- `cd kernel && cargo check --target x86_64-unknown-none` - Check kernel code
- Uses bootimage runner via `.cargo/config.toml`

### Build Profiles
- `release` (default) - Optimized builds
- `kernel` - Kernel-specific optimizations (`opt-level = "s"`, `lto = "thin"`)
- `kernel-debug` - Debug kernel builds with panic = abort
- `compatibility` - For compatibility layer components

### Toolchain Requirements
- Rust nightly (specified in `rust-toolchain.toml`)
- Components: `rust-src`, `llvm-tools-preview`
- Build targets: `x86_64-unknown-none` (kernel), `x86_64-unknown-linux-gnu` (userspace)

## Architecture and Code Structure

### Kernel Architecture (`kernel/src/`)
The kernel follows a microkernel design with these core modules:
- `main.rs` - Boot entry point and early initialization
- `lib.rs` - Kernel library initialization and module exports
- `memory.rs` - Physical memory management and page allocation
- `vmm.rs` - Virtual memory management and address space handling
- `gdt.rs` - Global Descriptor Table setup
- `interrupts.rs` - Interrupt handling and ISR management
- `process.rs` - Process management and context switching
- `syscall.rs` - System call interface and handlers
- `heap.rs` - Kernel heap allocation
- `serial.rs` - Serial port communication for debugging

### Application Modules
- `raeshell.rs` - Built-in shell implementation
- `rae_assistant.rs` - AI assistant integration
- `raede.rs` - Text/code editor
- `raekit.rs` - Application development framework
- `raepkg.rs` - Package management system

### Development Tools (`tools/`)
- `tools/build/` - Custom build system (`raeen-build` binary)
- `tools/test/` - Testing infrastructure
- `tools/package/` - Package creation and management

## Critical Development Rules

### Build and Environment Discipline
- **Never run build commands without explicit user request** - Users drive builds per project rules
- Always respect `rust-toolchain.toml` (nightly) and `.cargo/config.toml` settings
- Target must remain `x86_64-unknown-none` for kernel builds
- Keep builds warning-free where feasible

### Memory Safety and `no_std` Requirements
- Kernel is `#![no_std]` - use `extern crate alloc;` when heap allocation needed
- **No `unwrap()`, `expect()`, or panics in kernel paths** except fatal boot errors
- Return `Result` types and handle errors properly
- Use existing abstractions (`memory::with_mapper`, frame allocator helpers, `spin::Mutex`) over raw pointers

### Interrupt Service Routines (ISRs)
- Keep ISRs minimal: acknowledge EOI, fixed small work only
- **Never allocate heap memory or call blocking operations in ISRs**
- No filesystem, allocation-heavy, or blocking operations in ISRs
- Defer complex work via scheduler or work queues

### Memory Management Invariants
- Use global mapper and frame allocator accessors
- **Never unmap bootloader-critical mappings**
- Page table changes must include proper TLB invalidation
- When switching address spaces, ensure CR3 correctness and TLB flushing

### Code Quality Standards
- **No stubs or placeholders** - implement real functionality or don't touch the area
- No duplicate implementations - search codebase before adding new code
- Keep edits small, focused, and reversible
- Preserve existing indentation and style
- Remove unused imports, keep modules warning-free

### Security Requirements
- **No telemetry or data exfiltration** - privacy-first design
- Enforce W^X for all mappings (never writable+executable pages)
- Validate all user pointers and lengths in syscalls
- Never trust userspace buffers - copy in/out with dedicated helpers
- Use audited crypto primitives only, respect `no_std` constraints

### Documentation and Testing
- Update module docs for behavioral changes
- Include invariants and safety expectations in documentation
- Add `no_std` compatible unit tests where possible
- Keep QEMU boot/run scripts deterministic and non-interactive

## Important Files and Configuration

### Essential Configuration
- `Cargo.toml` - Workspace configuration with extensive metadata for build system, security, compatibility
- `rust-toolchain.toml` - Nightly toolchain specification
- `.cargo/config.toml` - Build targets and runner configuration
- `kernel/linker.ld` - Kernel linker script
- `.trae/rules/project_rules.md` - Comprehensive project rules (35 detailed rules)

### Project Documentation
- `Docs/Choice.md` - Architecture and language choice rationale
- `Docs/RaeenOS.md` - Complete OS specification (referenced but not read)
- `Docs/Production_Checklist.md` - Production readiness checklist

### Security and Compatibility Goals
- Windows app compatibility via custom Wine-like layer
- Android app runtime without Linux kernel dependency
- Multiple package formats: Flatpak, Snap, AppImage support
- Strict sandboxing with capability-based security model
- Verified boot and secure boot support

## Development Workflow Notes

### When Working on Kernel Code
1. Always work in `kernel/` directory for kernel changes
2. Use `cargo check --target x86_64-unknown-none` for quick validation
3. Follow strict memory management rules - never break bootloader mappings
4. Test ISR changes carefully - keep them minimal and fast

### When Adding New Features
1. Check project rules in `.trae/rules/project_rules.md` first
2. Search for existing implementations before creating new ones
3. Follow the microkernel design - keep kernel minimal
4. Add proper error handling with `Result` types

### Build System Integration
The custom `raeen-build` tool handles complex build orchestration including:
- Dependency ordering (kernel → userspace → bootloader → compatibility)
- Multiple target architectures and build profiles
- ISO/VMDK image creation (planned)
- Comprehensive build reporting and error handling

### Testing and Validation
- Unit tests should be `no_std` compatible where possible
- Integration tests run via custom test runner
- Boot testing uses QEMU with specific hardware emulation settings
- Benchmark infrastructure using Criterion for performance tracking