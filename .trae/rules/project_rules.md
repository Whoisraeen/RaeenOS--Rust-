### RaeenOS Project Rules (for all AIs and contributors)

These rules are non-negotiable. They exist to protect system integrity, performance, and long-term maintainability of RaeenOS.

### 1) Non‑negotiables
- No stubs or placeholders. Implement real functionality or do not touch the area.
- Do not comment out/disable modules to “get a build working”. Fix root causes.
- No duplicate implementations. Search the codebase before adding anything new.
- No trivial toy kernels or resets of architecture decisions. Respect `Docs/RaeenOS.md` and `Docs/Choice.md`.
- No duplicate files with advanced, enhanced, or anything else in the name. If the file already exsists under a different name edit that file. 

### 2) Editing discipline
- Keep edits small, focused, and reversible. Do not reformat unrelated code.
- Preserve existing indentation and style; do not mix tabs/spaces or change line endings.
- Add imports explicitly and remove unused imports. Keep modules compiling warning‑free where feasible.
- Do not rename public types/functions, move files, or change module paths without explicit rationale and a migration.

### 3) Build and environment
- Respect `rust-toolchain.toml` (nightly) and `.cargo/config.toml`. Do not change targets or feature gates without approval.
- Do not run build/test commands on the user’s machine unless explicitly asked; the user drives builds [[memory:2535035]].
- Ensure changes compile conceptually (types/traits consistent), and leave the tree in a state that should build cleanly.

 - After adding code, perform a cargo check and fix errors and warnings.

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

### 36) Graphics, compositor, and RaeUI (extensions)
- Render loop budgets: Target ≤ 8.33 ms at 120 Hz and ≤ 16.67 ms at 60 Hz; missed frames must not trigger busy-wait recovery.
- Frame pacing: Use triple buffering by default; enable adaptive vsync where supported; do not block the main/UI thread on GPU fences.
- Zero-copy paths: Prefer zero-copy blits and shared-buffer protocols for app→compositor; avoid unnecessary format conversions.
- Accessibility baseline: All UI components must pass WCAG AA contrast and support screen reader, keyboard navigation, and high DPI scaling.
- Shader safety: Precompile and validate shaders; forbid runtime shader code generation in release builds.

### 37) Package management (.rae) and sandbox
- Signed bundles: All `.rae` packages must be signed; enforce signature verification and metadata freshness before install/upgrade.
- Manifest and capabilities: Require a `manifest.rae` with explicit permissions (filesystem, network, GPU, input, devices); reject unknown or undeclared bits.
- Reproducible builds: Enforce deterministic builds for first-party apps; record exact toolchain and flags in bundle metadata.
- Updates and rollback: Delta updates must be content-addressed and verified; upgrades are atomic with one-shot rollback on failure.
- Post-install hooks: No network or privileged operations without declared permissions; hooks run strictly inside the app sandbox.
- Storage quotas: Enforce per-app disk quotas and private data directories; shared directories require explicit capability.

### 38) AI and privacy (Rae Assistant)
- On-device by default: Sensitive operations run locally; cloud features are opt-in with explicit user consent per session.
- Model governance: Version and sign models; verify integrity before load; document model provenance and evaluation notes.
- Data lifecycle: No silent retention; provide per-scope data deletion; redact PII in any persisted prompt/context stores.
- Offline guarantees: AI features degrade gracefully without network; never block the UI awaiting cloud responses.
- Auditability: Maintain local, user-viewable logs of AI actions and capability uses; rate-limit logs to avoid floods.

### 39) Gaming mode and performance overlay
- Mode isolation: Game Mode settings are scoped to the foreground game and auto-restored on exit; do not persist global changes without consent.
- Power constraints: On battery, disallow settings that meaningfully reduce battery health unless explicitly overridden by the user.
- Overlay budgets: Overlays must stay under 1 ms/frame GPU time and 0.5 ms CPU time; sample at configurable intervals.
- Scheduler policy: Gaming priority must not starve background critical tasks (I/O flush, security updates).

### 40) Compatibility layers (Windows, Android, PWAs)
- Strict userland: Compatibility layers run entirely in user space within sandboxes; no kernel bypass or ring elevation.
- FS/registry virtualization: Provide per-app virtual filesystems/registry; block raw device access unless explicitly granted.
- GPU mediation: Only mediated/virtualized GPU access; forbid direct MMIO from compatibility layers.

### 41) Security hardening (extensions)
- Userland ASLR/CFG: Enable ASLR and control-flow integrity for user processes; forbid self-modifying code in apps.
- Parser fuzzing: All external-input parsers (files, network, packages, GPU binaries) must have coverage-guided fuzzing with corpora checked into `third_party/fuzz_corpora/`.
- Kernel slab hardening: Enable guard patterns/poisoning for freed slabs; validate freelist integrity in debug builds.
- VRAM scrubbing: Zeroize GPU buffers and textures on reuse across processes to prevent data leakage.

### 42) Networking and updates
- Secure transport: Package repositories use signed metadata and QUIC/HTTP3 with pinning; fail closed on signature errors.
- DNS policy: DNS-over-HTTPS by default with a documented fallback policy; captive portal detection must not leak unrelated queries.
- Offline-first: Core system UIs remain functional offline; background update checks are rate-limited and resumable.

### 43) Storage, filesystem, and integrity
- Snapshot semantics: System updates occur within snapshot boundaries; define write ordering and fsync policies for crash consistency.
- Background maintenance: Defragmentation, compression, and deduplication run with strict I/O/CPU budgets and back off under load.
- Key management: Document FDE key hierarchy; enforce secure enclave/TPM usage where available; support emergency recovery keys.

### 44) Observability without telemetry
- Local-only metrics: Allow local performance traces and counters; no external exfiltration. Provide user toggles and redaction.
- Tracepoints: Define stable kernel/user trace events; disabled by default in release builds; bounded buffers with backpressure.

### 45) API governance (Syscalls, RaeKit, SDK)
- Review gates: New syscalls and public SDK APIs require design docs with invariants, error domains, and a versioning plan.
- Deprecation policy: Mark deprecated APIs with time-bound removal plans; provide shims for at least one minor release.
- Threading contracts: SDK APIs must document threading and allocator expectations; forbid hidden global state.

### 46) Internationalization and assets
- i18n baseline: All user-facing strings must be localizable; no hard-coded glyph assumptions; provide font fallbacks.
- Asset licensing: Track licenses for fonts, icons, and sounds; record in `third_party/ATTRIBUTIONS.md` before inclusion.

### 47) Build, CI, and reproducibility
- Pre-commit checks: After adding code, perform a cargo check and fix errors and warnings; enforce no new warnings in CI.
- Determinism: Release artifacts must be reproducible; record build hashes and environments; verify via a CI job.
- Size budgets: Define binary and symbol size budgets for kernel and critical libraries; fail CI on regressions beyond thresholds.

### 48) Power management and timers (extensions)
- Idle policy: No busy-wait loops outside of verified low-latency paths; prefer timers or scheduler yields.
- Per-app profiles: Document and enforce per-app power profiles; the UI must show and allow user override.

### 49) Real-time audio and sound
- Low-latency target: End-to-end audio pipeline ≤ 10 ms; prefer lock-free ring buffers and avoid heap allocations in hot paths.
- Glitch policy: XRuns are logged with bounded rate; recovery must not stall the UI or starve other real-time threads.
- Audio graph: Node scheduling must be deterministic; no user plugins may run with kernel privileges.

### 50) Timekeeping and clocks
- Monotonic source: Prefer TSC with invariants verified; fall back to HPET/PIT where required; never use wall clock for scheduling.
- Clock sync: NTP/PTP runs sandboxed; skew/step limits enforced; the UI reflects unsynchronized state without blocking.

### 51) Boot, recovery, and safe mode
- Safe boot: Provide a minimal safe-mode profile with nonessential drivers disabled; never write to disk in safe mode without explicit consent.
- Boot integrity: Verify boot chain measurements; refuse to boot unsigned kernels unless explicit developer mode is enabled.
- Recovery media: Allow offline, signed recovery images; ensure rollback across A/B slots is atomic.

### 52) Crash handling and diagnostics (local only)
- Kernel/user dumps: Generate local crash dumps with user consent; no network transmission. Dumps are size-bounded and encrypted at rest.
- Watchdogs: Per-subsystem watchdogs may reset only the affected service; whole-system reboot requires explicit policy.

### 53) C/C++ interop and unsafe boundaries
- FFI policy: All C/C++ surfaces are behind safe Rust wrappers with documented invariants and lifetime/aliasing rules.
- Allocation: C++ code must use OS-provided allocators; forbid global new/delete overrides.
- Exceptions: No C++ exceptions may cross FFI boundaries; translate to error codes.

### 54) IPC and ABI versioning
- IPC schemas: Version all IPC messages; reject unknown fields/bits; provide forward-compatible optional fields.
- Stability: No breaking IPC changes without a migration window and compatibility shims.

### 55) Driver and kernel module policy
- In-tree only: No stable external kernel module ABI; third-party drivers are built in-tree against the exact toolchain.
- Signing: All drivers must be signed and measured; developer mode allows unsigned only on developer machines.
- Fault isolation: Driver failures must not panic the kernel; use restartable worker contexts where feasible.

### 56) Rendering fallback and headless
- Software renderer: Provide a pixel-correct software fallback when GPU features are unavailable; keep the UI responsive under fallback.
- Headless mode: Core services must run headless with a virtual framebuffer for CI and end-to-end tests.

### 57) Input/HID security
- Secure input path: Keyboard/IME/password fields use an isolated input channel; block injection from other apps by default.
- Device hotplug: New HID devices default to least privilege until explicitly trusted by the user.

### 58) Update rollouts and staging
- Phased rollout: Stage updates (canary → beta → stable) with kill switches; auto-pause on elevated crash rates.
- Policy locks: Enterprise policy mode can pin versions and defer updates within defined windows.

### 59) Configuration and state
- Defaults: Safe, privacy-first defaults; first-run prompts are minimal and local-only.
- State storage: Separate machine, user, and app state; schema migrations must be idempotent and reversible.

### 60) Secrets and key management
- Secrets API: Central secrets store with per-app isolation; no plaintext secrets on disk; enforce screen-lock requirement for retrieval.
- Key rotation: Support rotation for FDE and signing keys; document operational procedures.

### 61) Performance and regression gates
- Budgets: Subsystem-specific CPU/GPU/memory budgets tracked in CI; regressions beyond defined thresholds fail the build.
- Benchmarks: Fixed-input micro/macro benchmarks are checked into the repo; results compared against a moving median with variance bounds.

### 62) Testing and verification
- Determinism: Tests must not depend on wall clock or network; seed all RNGs; use hermetic test environments.
- Fuzzing gates: Critical parsers and drivers must hit a minimum coverage threshold; periodic corpus minimization is enforced.

### 63) Internationalization and typography (extensions)
- Bidi and shaping: Ensure proper Unicode bidirectional handling and text shaping; font fallback stacks must cover CJK and RTL scripts.
- Locale-aware: Formatting of dates, numbers, and currency respects user locale throughout RaeUI and RaeKit.

### 64) Entropy and RNG
- RNG policy: Initialize CSPRNG only after sufficient entropy; block high-grade crypto until ready; document entropy sources and health checks.

### 65) Kernel architectural excellence (aspirational standard)
- Architectural bar: Microkernel with modular, hot-swappable services communicating via capability-scoped IPC. Enforce strict least-privilege boundaries between drivers, filesystems, networking, compositor, and system services.
- Scheduling: Hybrid scheduler with real-time classes (deadline/priority with priority inheritance) for latency-critical paths (input, audio, compositor), and fair scheduling for best-effort work. NUMA-aware load balancing, CPU affinity, and preemptible kernel with bounded tail latencies.
- Memory safety and isolation: Enforce W^X globally; KASLR; SMEP/SMAP/UMIP/CET when available; KPTI where required. Fine-grained page permissions and guard pages for stacks/critical regions. Zeroization of sensitive buffers and deterministic TLB invalidation on permission changes.
- IPC performance: Zero-copy message passing via shared-memory grant tables and bounded lock-free rings. Copy-in/out helpers validate all user pointers and lengths. Capability handles are unforgeable, revocable, and auditable.
- Concurrency discipline: Prefer wait-free/lock-free structures in hot paths; use RCU where appropriate; explicit global lock hierarchy with static analysis for deadlock prevention. No blocking or heap allocation in ISRs; threaded IRQs with MSI-X and interrupt steering.
- I/O and DMA safety: IOMMU enforced for all DMA-capable devices; bounce buffers where needed. Strict memory barriers for MMIO; coherent cacheability attributes declared for all mappings. Driver crashes isolate and restart service contexts without kernel-wide failure.
- Security posture: Minimize syscall surface; validate all enums/flags and reject unknown bits. Optional per-process syscall filters. Hardened slab allocators with guard/poison in debug, and mitigations for UAF/double-free classes.
- Time and power: Tickless kernel with high-resolution timers and TSC-deadline where available. Energy-aware scheduling and QoS hints, with predictable wakeup latencies for real-time classes.
- Observability (local, low overhead): Static tracepoints backed by a lock-free ring buffer; user-toggleable with bounded memory and rate limits. Deterministic perf counters and microbenchmarks for hot paths.
- Reliability and upgrades: Crash-only, restartable service model for user-space system daemons; watchdogs scoped per subsystem. Support live configuration reload and safe hot-patching of non-critical services; kernel text remains immutable at runtime (W^X preserved).
- Verification culture: Critical parsers, allocators, and context-switch code come with property tests, coverage-guided fuzzing, and documented invariants for every `unsafe` block. Changes must not regress established latency, throughput, or security budgets.

### 66) Capability system semantics
- Object model: All resources are objects referenced by per-process handles with a rights bitmap (read, write, signal, map, exec, duplicate, send, receive). Handles are non-forgeable via table indices with generation counters or 128-bit secrets.
- Delegation: `cap_clone()` can only reduce rights; never expand. Support time-boxed capabilities (expiry) and scoped capabilities (valid only in a process subtree).
- Revocation: Maintain a capability revocation table enabling O(1) revocation per holder via intrusive lists, including bulk revoke by label.
- Revocation SLO: Revocation latency targets p99 < 200 µs to block new mappings/uses and < 2 ms to tear down existing shared mappings.
- Audit: Append-only, bounded audit log with per-PID rate caps, redaction rules for sensitive fields, and tamper-evident hashing/chaining.
- Namespaces: Per-process namespaces (ports, paths) resolve to capabilities; avoid global mutable registries.

### 67) IPC QoS and backpressure
- Queues: Bounded, lock-free MPSC rings with credit-based flow control and zero-copy grants via shared memory with rights-based RO/RW mapping.
- Priority inheritance: Propagate receiver priority to senders when they hold receiver-owned queue space to avoid priority inversion.
- QoS tags: Latency-sensitive (input/audio/compositor), best-effort, and bulk classes; scheduler honors these tags.
- Fan-out: Use copy-on-write fallbacks for multi-receiver delivery; versioned message schemas reject unknown fields.
- Backpressure semantics: When one receiver stalls, define policy per-channel (drop-oldest, sender-park with timeout, or spill-to-bounded-buffer) and expose counters to the Vitals service.

```yaml
ipc_defaults:
  latency_sensitive: {policy: "park_with_timeout", timeout_us: 500, spill_bytes: 0}
  best_effort:       {policy: "drop_oldest",       timeout_us: 0,   spill_bytes: 0}
  bulk:              {policy: "spill_bounded",     timeout_us: 0,   spill_bytes: 262144}
```

### 68) Scheduling SLOs and mechanics
- Targets (to be tuned on reference hardware): Input delivery p99 < 2 ms (p999 < 5 ms); compositor CPU budget < 1.5 ms/frame @ 120 Hz; audio jitter p99 < 200 µs with zero underruns under nominal load; intra-core IPC RTT p99 < 3 µs.
- Mechanics: EDF/CBS for audio and input; RR/fixed-priority for device threads. Reserve isolated cores for RT classes (nohz_full behavior). NUMA-aware per-node runqueues with pull-based balancing; avoid remote wake-ups for RT. RT throttling via CBS to prevent starvation.
- RT isolation: Explicitly configure isolcpus and rcu_nocbs-style behavior for RT cores to shrink tail latencies.

### 69) Memory model edges and JIT policy
- W^X exceptions: For JIT, forbid write+exec co-resident mappings; use dual-mapping (RW for codegen, RX for execute) with TLB shootdown; require signed JIT or in-kernel verifier.
- Hardening: Enable CFI and shadow stacks (CET on x86, BTI/PAC on ARM64) when available; MTE opt-in for userland on ARM64.
- Paging/TLB: Support transparent huge pages (2 MiB/1 GiB) with auto promotion/demotion; use PCID/ASID to reduce flush overhead and batch TLB shootdowns.
- Field diagnostics: Lightweight KFENCE/KASAN-style sampling mode in production to catch UAF/double-free with <1% overhead.
- Runtime budget: Keep diagnostics within ≤ 0.5% CPU and ≤ 64 MiB memory overhead so they can remain enabled in production. Support encrypted swap and opt-in per-process memory encryption (SEV/TDX/TME) when available.

### 70) Concurrency discipline verification
- Lock hierarchy: Enforce a global lock order; add static config and lockdep-like runtime sampling in CI.
- Data structures: Use wait-free only in proven hot paths; elsewhere prefer RCU and fine-grained mutexes to avoid retry cliffs. Prefer per-CPU allocators and counters to minimize cache bounce.
- Sanitizers: Nightly builds run race detectors where feasible and linearizability tests (e.g., Lincheck) for lock-free structures.

### 71) Drivers and DMA (zero-trust model)
- User-space first: Favor user-space drivers; keep kernel drivers only for early-boot and critical paths. Enforce IOMMU SVA/PASID for shared page tables with user drivers; use ATS where safe.
- Virtualization: For SR-IOV, each virtual function receives capability-scoped DMA windows; revoke on misbehavior.
- Interrupts: Route RT device interrupts to isolated cores; assign bulk NIC queues per-queue affinity; use NAPI-like polling for high-PPS paths.
- Health: Drivers run under supervised service sandboxes with crash loops, exponential backoff, and quarantine after repeated failures.

### 72) Security posture integrated with boot and policy
- Attestation: Measured and secure boot with TPM quotes; attest kernel and core services to a userland policy daemon before granting sensitive caps.
- Module/code policy: Only signed modules; no runtime kernel text patching. Per-process syscall allowlists and per-capability policies (e.g., file caps cannot map exec).
- MAC: Capability-aware labeling; labels propagate with capability transfer. Provide a simple default policy with optional richer policy modules.
- Network: Prefer userland protocol stacks via zero-copy NIC queues; kernel networking features gated behind verified programs respecting W^X rules.

### 73) Timekeeping and power determinism
- Clock discipline: Monotonic base with PTP/PHC synchronization; stable sched_clock for tracing and SLO measurement.
- Energy model: Integrate DVFS hints by QoS class; RT buckets are excluded from downclocking. Coalesce best-effort timers; protect RT timers from coalescing.

### 74) Observability as flight recorder
- Flight recorder: Always-on, tiny circular buffer capturing the last N seconds of critical tracepoints; dumped on crash.
- Probes: Provide USDT-style static tracepoints and dynamic probes; maintain frame pointers for accurate unwinding.
- Vitals: Each subsystem exports latency/throughput/error SLOs to a central, local-only "Vitals" service; sampling profilers are rate-limited per PID.
 - Privacy and budgets: Flight recorder is local-only and size-bounded, with PII redaction (paths, hostnames) unless an admin debugging capability is explicitly enabled. Rate-limit tracepoints per PID and per subsystem to prevent self-DoS.

### 75) Reliability, upgrades, and rollback
- Transactions: A/B userland services with transactional configuration (write-validate-switch) and automatic rollback on failure.
- Micro-reboots: Service processes are disposable; externalize state via versioned shared memory so restarts are cheap.
- Crash triage: Generate minidumps with redaction; prefer kexec-style crash kernel for rapid triage while preserving flight recorder.

### 76) Verification and formal methods
- Specs: Provide formal models for IPC, scheduler admission control, and page-table transitions (e.g., TLA+, Iris/Coq) proving key invariants and deadlock-freedom.
- Coverage gates: Reject changes that reduce line/branch or MC/DC coverage for critical code.
- Fuzz tiers: Maintain fuzz corpora for syscalls, driver IOCTLs (with device emulators), and file/network parsers using dictionary-aided fuzzing.
- Unsafe ledger: Every `unsafe` block links to a markdown spec documenting preconditions/postconditions and evidence (tests or proofs).

### 77) Release SLO gates (measurable baselines)
- Latency baselines (tune per reference platform): Input p99 ≤ 2 ms idle / ≤ 4 ms under 90% CPU load; audio callback miss rate 0 per 10 minutes under stress; IPC small-message RTT p99 ≤ 3 µs same-core / ≤ 8 µs cross-core.
- MMU baselines: Anonymous page-fault service time p99 ≤ 15 µs; TLB shootdown p99 ≤ 40 µs for 64-page batches on 16 cores.
- Power baseline: Background idle ≤ 150 mW on the reference laptop with Wi‑Fi enabled.
- Compositor jitter: p99 ≤ 0.3 ms at 120 Hz; missed-frame rate ≤ 0.1% under mixed load.
- Storage NVMe: 4 KiB read @ QD=1 p99 ≤ 120 µs on hot set; flush p99 ≤ 900 µs.
- Networking RTT: Loopback userspace→userspace via NIC queues small-packet RTT p99 ≤ 12 µs.
- Pressure budgets: PSI memory stall time ≤ 5% over 60 s under the standard app mix.
- Stability: 72 h chaos test with zero data-loss events and filesystem consistency verified post-run.
 - Capability revocation: Block new uses p99 ≤ 200 µs; tear down shared mappings p99 ≤ 2 ms.
 - Process lifecycle: Process spawn (empty image) p99 ≤ 650 µs same NUMA node; thread create p99 ≤ 80 µs.
 - Power management: Suspend/resume (S3 or modern standby) resume-to-input-ready p95 ≤ 300 ms on laptop SKU; thermal floor maintained such that compositor jitter and audio SLOs still pass at 95th percentile ambient with throttling.
 - Wi‑Fi roaming (if applicable): Handover p99 ≤ 150 ms without audio underrun.
 - Filesystem durability: rename+fsync atomicity and metadata+data ordering invariants verified.

### 80) User-space ABI policy (future-proofing)
- Stability: Freeze a stable user ABI (syscalls, ioctls, message schemas) with semantic versioning and feature negotiation via per-process bitsets.
- Evolution: Provide compatibility shims and time-bound deprecation windows to evolve interfaces without breaking applications.
 - vDSO: vDSO and its symbols are part of the stable user ABI and are versioned.
 - Compat profiles: Per-process "compat syscalls" profile with an expiry date; CI runs both current and compat profiles.
 - Message schemas: Capabilities tag + length + version fields are mandatory; unknown fields are a hard reject and covered by binary tests.

### 81) Resource control and pressure signals
- Quotas: Hierarchical, cgroup-like quotas for CPU/RT runtime, memory (including page cache), I/O, and IPC queue credits.
- Signals: Export PSI-style CPU/memory/I/O pressure signals to userland so daemons can back off before OOM; integrate with the Vitals service.
 - Defaults: Default hierarchy `system.slice`, `user.slice/<uid>`, `services.slice` with per-slice ceilings for CPU shares, RT runtime, memory.max (including page cache), IO.max, and IPC credits.
 - Auto-throttle: PSI alerts can auto-throttle tasks marked as bulk QoS via the Vitals integration.

### 82) Memory pressure, reclaim, and OOM policy
- Reclaim: Tunables for LLC-aware reclaim, NUMA-local page placement, compaction aggressiveness, and THP promotion heuristics.
- OOM: Cgroup-aware OOM killer with victim selection signals and graceful teardown hooks; optional zswap/zram support.
- Working set: Hot/cold page aging and working-set estimation to avoid thrashing under pressure.
 - THP by class: Best-effort/bulk prefer 2 MiB THP; RT/latency discourage THP unless pages are pinned.
 - NUMA policy: First-touch + mbind hints; reclaim biased to preserve locality.
 - OOM pre-kill: Victims receive a pre-kill hook (≤ 50 ms) to dump state into a sealed buffer.

### 83) Filesystem and storage semantics (crash safety)
- Filesystems: First-class copy-on-write or journaled filesystems with end-to-end checksums or ordered journaling with barriers.
- I/O path: blk-mq with per-device scheduler selection (mq-deadline/none/bfq) and guaranteed write-barrier/FUA correctness.
- NVMe: Multipath, ANA, DULBE/Streams support; TRIM/discard policies; fs-verity/dm-verity for immutable roots; encrypted-at-rest volumes.
 - Semantics: Document that `rename()` is atomic after `fsync(dir)`, `fsync(fd)` guarantees durable data+metadata, and barriers/FUA are honored on power loss.
 - Validation: CI includes crashmonkey-style workloads with fault injection until invariants are proven.

### 84) Networking fast path
- Queues: RSS/RPS/RFS defaults with per-queue IRQ affinity; GRO/GSO tuning; XDP-like zero-copy path where userland stacks can own queues.
- Congestion: Provide CUBIC and BBR with sane defaults; pacing enabled in TX path.
- Offload policy: Prefer userland QUIC/TLS; any in-kernel offload must be behind verifiers and respect W^X/JIT rules.
 - IRQ and pacing: Bind per-queue IRQs to cores; keep NIC TX pacing enabled for BBR.
 - Userland ownership: For userland stacks, pin queues, restrict IOMMU windows to least privilege, and watchdog missed doorbells.
 - 12 µs RTT test recipe: Pin endpoints to the same NUMA node, pre-fault buffers, fix MTU, and report IRQ line and queue IDs.

### 85) Graphics/compositor contract
- Interface: DRM/KMS-like API with atomic modesetting, plane composition, explicit sync fences, and direct scanout for fullscreen surfaces.
- Pacing: Frame pacing SLOs with p99 jitter ≤ 0.3 ms at 120 Hz; zero-copy surface handoff GPU→compositor→scanout.
 - Fences and depths: Use explicit acquire/release fences throughout; present only after acquire fences signal. **MUST:** app→compositor backlog ≤ 2 frames; compositor→scanout backlog = 1.
 - Color management: Default internal pipeline scRGB or linear FP16 with optional ICC/3D LUT.
 - VRR: Enforce a minimum frame time clamp so pacing SLOs remain meaningful under variable refresh.

### 86) Virtualization and containers
- Virt baseline: Virtio device family with vIOMMU; nested-safe time and TSX policies.
- Namespaces: pid/net/ipc/mount/user namespaces with capability-aware bridges and per-namespace syscall filters.
 - Accounting: Provide steal-time accounting to the scheduler so vCPUs do not starve RT classes.

### 87) Supply chain and reproducibility
- Builds: Reproducible builds with pinned toolchains; emit SBOM; sign and attest artifacts (e.g., Sigstore) aiming for defined SLSA levels.
- Microcode: Microcode management with mitigation toggles measured and logged as part of the boot attestation.
 - Attestation: Include a reproducible-build bit in the attestation quote and log CPU mitigation toggles to Audit and Vitals.

### 88) Speculative execution and CPU hardening knobs
- Mitigations: Documented toggles for IBRS/IBPB/STIBP, retpolines, MDS/TAA, SRBDS, and SMT policy with sane defaults.
- Policy: Boot-time mitigation policy participates in attestation and is reflected in the audit and Vitals services.

### 89) Entropy and time at boot
- CSPRNG: Early-boot seeding using jitter entropy plus hardware sources with continuous health tests before marking ready.
- Time: Clocksource watchdog and cross-core TSC synchronization guarantees; SLO measurements use monotonic time only.

### 90) Fault injection and chaos testing
- Failpoints: Kernel-level failpoints (allocation, I/O, IRQ loss), driver crash loops, packet drop/duplication tools, and slab-poison toggles integrated into CI and nightly testing.
 - Coverage goal: ≥ 80% of driver codepaths exercise at least one failpoint nightly; expose progress in Vitals.

### 91) Kernel image safety
- A/B kernel images with measured boot and automatic fallback on boot failure; immutable root option via verity for production.

### 92) Operational kill-switches and incident policy
- Feature flags: Capability-guarded, per-subsystem toggles (e.g., disable THP, switch congestion control, enable safe compositor fallback).
- Emergency capabilities: A "break-glass" capability can freeze a process tree, revoke a class of device capabilities, or force a service micro-reboot. All events are audited and attested.
- Runbooks: Define SEV-1..4 levels, on-box triage steps (collect flight recorder, minidump), off-box upload policy, and auto-rollback rules tied to A/B images and transactional service upgrades.

### 93) CI SLO gates skeleton

```yaml
slo_gates:
  input_latency_p99_ms: {max: 2.0}
  audio_jitter_p99_us: {max: 200}
  compositor_jitter_p99_ms: {max: 0.3}
  ipc_rtt_samecore_p99_us: {max: 3}
  anon_fault_service_p99_us: {max: 15}
  tlb_shootdown_64pg_16c_p99_us: {max: 40}
  nvme_4k_rd_qd1_p99_us: {max: 120}
  idle_power_mw: {max: 150}
  chaos_fs_consistency: {must_equal: "pass"}
```

```yaml
env:
  platform: ["desk-sku-a", "lap-sku-b", "srv-sku-c"]
  cpu_governor: "performance"
  isolated_cores: [2,3]
  numa_node: 0

methodology:
  warmup_seconds: 45
  min_samples: 60000
  require_consecutive_passes: 2
  max_drift_vs_7day_median_pct: 5
```

- Test annotations: Tag SLO tests so CI can auto-map metrics to gates. Example:

```rust
#[test]
#[slo(metric = "input_latency_p99_ms")]
fn input_latency_slo() {
    // ... collect histogram and emit metric ...
}
```

- CI mapping: Tests must emit a JSON results file (e.g., `slo_results.json`) with metric keys matching `slo_gates`. CI enforces two consecutive passes or ≤ 5% drift vs the rolling 7‑day median per 94).
- Profiles: Where applicable, run both "current" and "compat" ABI profiles; report results separately per profile and per reference SKU.

Example `slo_results.json` (copy/paste template):

```json
{
  "platform": "lap-sku-b",
  "metrics": {
    "input_latency_p99_ms": 1.95,
    "audio_jitter_p99_us": 180,
    "ipc_rtt_samecore_p99_us": 2.9
  }
}
```

### 94) SLO measurement methodology (machine-checkable)
- Reference platforms: Enumerate 2–3 SKUs (desktop, laptop, server) with CPU generation, core count, RAM, NVMe, NIC, and GPU details. CI SLO gates run only on known SKUs.
- Warmup and sampling: Warm up for 30–60 s, then collect ≥ 60k samples per metric or ≥ 10k frames for graphics; compute p99 using HdrHistogram-style bins.
- Pinning and isolation: Pin test threads, disable turbo for latency tests unless power-aware, isolate RT cores, and freeze background daemons during runs.
- Time source: Use monotonic clock only, with TSC verified against PHC/NT; include the clocksource ID in reports.
- Variance guard: Require two consecutive passes, or a pass within ≤ 5% drift of the rolling 7-day median.
 - Standard app mix and SKUs: The standard mix is defined in `Docs/Perf/standard_app_mix_v1.yaml` (and `Docs/Perf/standard_app_mix_v1.md`). Reference platforms are tracked in `Docs/Perf/reference_skus.yaml`. CI must load these exact files for gating.


### 78) Anti-goals (explicit non-features)
- No stable external kernel driver ABI across major versions.
- No global registries or ambient authority; all access mediated by capabilities.
- No blocking allocations or filesystem/network I/O in ISRs.
- No live-patching of kernel text; use service micro-restarts or kexec for fixes.

### 79) Operational documentation set
- Capability model specification (objects, rights, delegation, revocation, audit) checked into `Docs/`.
- Scheduling policy document (classes, budgets, isolation, NUMA rules, SLOs).
- Memory hardening guide (W^X, JIT exception protocol, CFI/shadow stacks, huge pages).
- Driver sandbox contract (IOMMU policy, crash/health behavior, required metrics).
- Verification plan (theorems to prove, fuzz matrices, coverage floors) and performance playbook (benchmarks, counters, fail thresholds).

