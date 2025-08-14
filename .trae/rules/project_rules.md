### RaeenOS Project Rules (for all AIs and contributors)

These rules are non-negotiable. They exist to protect system integrity, performance, and long-term maintainability of RaeenOS.

### 1) Non‑negotiables
- No stubs or placeholders. Implement real functionality or do not touch the area.
- Do not comment out/disable modules to “get a build working”. Fix root causes.
- No duplicate implementations. Search the codebase before adding anything new.
- No trivial toy kernels or resets of architecture decisions. Respect `Docs/RaeenOS.md` and `Docs/Choice.md`.

### 2) Editing discipline
- Keep edits small, focused, and reversible. Do not reformat unrelated code.
- Preserve existing indentation and style; do not mix tabs/spaces or change line endings.
- Add imports explicitly and remove unused imports. Keep modules compiling warning‑free where feasible.
- Do not rename public types/functions, move files, or change module paths without explicit rationale and a migration.

### 3) Build and environment
- Respect `rust-toolchain.toml` (nightly) and `.cargo/config.toml`. Do not change targets or feature gates without approval.
- Do not run build/test commands on the user’s machine unless explicitly asked; the user drives builds [[memory:2535035]].
- Ensure changes compile conceptually (types/traits consistent), and leave the tree in a state that should build cleanly.

### 4) Language and style
- Rust is primary. `#![no_std]` kernel: keep `extern crate alloc;` available where needed.
- No `unwrap()`/`expect()`/panics in kernel paths (except fatal boot errors). Return `Result` and handle errors.
- Avoid inline TODOs; if something must be deferred, implement a safe, minimal, correct version.
- Follow clear naming (no 1–2 character names). Prefer explicit, readable code over cleverness.

### 5) Unsafe code policy
- Use `unsafe` only with necessity and document the invariants above the block.
- Prefer existing abstractions (`memory::with_mapper`, frame allocator helpers, `spin::Mutex`) over raw pointers.
- No `unsafe` in interrupt handlers beyond what is strictly necessary for port I/O or EOI.

### 6) Concurrency, interrupts, and timing
- Keep ISRs minimal: acknowledge EOI, perform fixed small work, never block or allocate on the heap inside ISRs.
- Protect shared state with `spin::Mutex`/`RwLock`; avoid nested locks and define a strict lock order.
- Never call filesystem, allocation-heavy, or blocking operations from ISRs. Defer via scheduler or work queues.

### 7) Memory/VMM rules
- Use the global mapper and frame allocator accessors. Do not create ad‑hoc allocators.
- When switching address spaces, ensure CR3 write correctness and flush TLB as required. Do not drop bootloader‑critical mappings.
- Update page table flags consistently when changing `VmPermissions`. Reflect changes with appropriate invalidations.
- No identity mapping of large regions beyond what bootloader mandates. Keep mappings minimal and explicit.

### 8) Scheduler and context switching
- Timer ISR must be quick: tick time, signal scheduler/yield, EOI. No heavy work here.
- Switching processes: save/restore context, switch CR3 (when per‑AS PML4 is live), then resume. Keep invariants consistent.
- Round‑robin queues must remain fair; gaming priority is allowed but must not starve other queues.

### 9) Syscall ABI and user/kernel boundary
- Do not renumber existing syscalls. Add only at the end with clear semantics and argument validation.
- Validate all user pointers and lengths. Copy in/out with dedicated helpers. Never trust userspace buffers.
- Maintain kernel/user permission checks. Enforce sandbox/capability decisions inside syscalls.

### 10) Filesystem
- Respect VFS invariants and borrowing rules. Use owned `String` where required to avoid aliasing.
- Implement `open/close/read/write/seek/create/remove/metadata` semantics consistently. No side effects from read operations.
- Avoid global mutable state; route through `VirtualFileSystem` and mounted FS abstractions.

### 11) Graphics/Compositor
- Use double/triple buffering where possible. Do not render from ISRs.
- Prefer well-defined blit/primitive operations; do not introduce busy-wait loops that hog CPU.
- Any GPU or framebuffer init must be capability-detected and fail-safe.

### 12) Networking
- Keep the stack modular: parsing, routing/ARP, sockets, NIC drivers. No allocation in hot RX/TX paths beyond fixed pools.
- Validate packet lengths and checksums. Never trust external input.

### 13) Security and privacy
- No telemetry or data exfiltration, period. Privacy-first AI and system behavior.
- Use audited primitives only (when integrated) and respect `no_std`. No homegrown crypto.
- Enforce permissions/sandbox levels consistently across syscalls, drivers, and subsystems.

### 14) Testing and validation
- Add unit tests where possible (no_std compatible). For kernel logic, prefer small pure functions that are testable.
- Keep QEMU boot/run scripts deterministic and non-interactive when applicable (the user runs them).

### 15) Documentation and changelog
- Update module docs for any behavioral change. Include invariants and safety expectations.
- Note syscall additions/behavior changes in a CHANGELOG and in module headers.

### 16) Prohibited changes
- No sweeping refactors across unrelated modules in a single edit.
- Do not change initialization order in `lib.rs` without explicit reasoning and validation.
- Do not alter bootloader expectations or linker behavior without a full plan and rollback path.

### 17) When blocked
- If a safe, production implementation is not achievable in the current edit, stop and request guidance with concrete options. Do not insert placeholders.

### 18) Change checklist (must pass before finishing an edit)
- Code compiles conceptually; types and traits line up across modules.
- No new lints or warnings in touched files; unused imports removed.
- ISR paths minimal; EOI is sent; no allocations or blocking in ISRs.
- VMM mappings/permissions updated coherently; no accidental global changes.
- Syscall arguments validated; no unchecked user memory access.
- Docs/comments updated; no TODOs left behind.

### 19) ABI and versioning
- Maintain syscall ABI stability; never renumber existing IDs. Add new at the end with clear semantics.
- Version any new ABIs and document structure layouts, alignments, and endianness for shared memory/IPC.

### 20) Error handling and logging
- Use consistent error domains; map to a kernel-wide `ErrorCode`. Avoid magic integers.
- Rate-limit kernel logs; avoid unbounded logging in hot paths. Put verbose logs behind feature flags.

### 21) Performance and latency budgets
- Define and respect ISR maximum duration and syscall latency targets. Justify any regression and add tests.
- Avoid heap allocations in hot loops; prefer arenas or preallocated pools in kernel subsystems.

### 22) SMP and CPU feature policy
- Detect CPU features via CPUID; provide safe fallbacks. Never assume AVX/TSX/etc.
- Establish and document a global lock hierarchy; avoid circular waits. Use explicit atomic orderings.

### 23) FPU/SIMD usage in kernel
- No FPU/SIMD in ISRs. If used in kernel threads, save/restore state explicitly; prefer integer math when possible.

### 24) Security hardening
- Enforce W^X for all mappings; never map writable+executable pages.
- Enable SMEP/SMAP/UMIP where available; document any exceptions.
- Zeroize secrets and scrub freed pages that contained sensitive data.
- Validate all user pointers and lengths; guard against integer over/underflow in size math.

### 25) Memory management invariants
- Page-table flag changes must be accompanied by correct TLB invalidation; avoid inconsistent intermediate states.
- Never unmap bootloader-critical or kernel text/data regions.
- User mappings should receive zeroed pages; kernel mappings must declare cacheability explicitly.

### 26) Drivers, MMIO, and DMA
- MMIO must be volatile and properly ordered; insert memory barriers where required.
- DMA requires IOMMU configuration or bounce buffers; never DMA into kernel text/data.
- Detect device capabilities; fail gracefully when hardware is absent.

### 27) Power management and timers
- Centralize timers (PIT/HPET/TSC). Expose monotonic time only to kernel logic.
- No busy-waiting for long delays; prefer sleep/yield or timer callbacks.

### 28) Filesystem data integrity
- Define write ordering guarantees; aim for crash-consistent behavior until journaling lands.
- Never block ISRs on FS operations; route through worker queues.

### 29) Syscall surface and permissions
- Enforce least-privilege via capability checks inside syscalls.
- Validate all enums/flags; reject unknown bits with consistent errors.

### 30) Dependency policy
- Kernel allowlist only: `no_std` compatible crates; review transitive dependencies.
- Pin crate versions; document rationale for security-sensitive dependencies.

### 31) Feature flags and configuration
- Gate experimental features behind `cfg` flags; default off. Release builds must not change behavior compared to debug except for instrumentation removal.

### 32) Documentation and review
- Public APIs require rustdoc covering invariants and safety. Document changes to init order, VMM, scheduler, or ISRs with a short design note.

### 33) Testing, profiling, and CI (conceptual)
- Add property tests for parsers (packet formats, filesystems) where feasible.
- Provide microbenchmarks for hot paths with reproducible inputs; enforce thresholds for regressions.

### 34) Licensing and compliance
- Avoid GPL-incompatible code in the kernel unless explicitly approved by project licensing. Track origins for third-party code.
- Maintain copyright headers and LICENSE references for imported code.

### 35) Open‑source sourcing and attribution
- Use web searches (date-filtered to August 2025) when considering external code. Integrate only if license is compatible (e.g., MIT/Apache‑2.0/BSD; avoid GPL for kernel unless approved).
- Give proper credit: add attribution and license text to `LICENSE`/`NOTICE` and maintain `third_party/ATTRIBUTIONS.md` with source URLs.
- Preserve SPDX headers in vendored files and record exact versions/commits.
- Do not copy code without a license review; prefer original implementations if compatibility is unclear.
