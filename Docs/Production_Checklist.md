# RaeenOS Production Checklist

This comprehensive checklist tracks the development progress toward a complete, production-ready RaeenOS with all features implemented and thoroughly tested.

## 🚨 NO-FOOT-GUNS HEADER (Ship Discipline)
**These rules are non-negotiable for all kernel code:**
- [ ] **-D warnings** enforced in CI (no warnings allowed)
- [ ] **No unwrap/expect** in kernel code (explicit error handling only)
- [ ] **Unsafe invariants documented** with safety comments
- [ ] **No work in ISRs** beyond EOI/minimal bookkeeping
- [ ] **Don't renumber syscalls** (append-only ABI)
- [ ] **Migration notes required** for any refactoring
- **Acceptance:** CI enforces all rules; code review checklist includes safety verification

---

# 🚀 RUTHLESS GLOW-UP CHECKLIST
*Making RaeenOS the "best OS of all time," not just best vibes*

## 🚦 Core Release Gates (Ship or Skip)

### SLO Harness is Law [MUST v1]
- [ ] SLO tests emit `slo_results.json` and must pass two consecutive runs or be within ≤5% of the 7-day median
- [ ] Acceptance includes: input/compositor/audio/IPC/anon-fault/TLB/NVMe/idle-power/chaos-FS
- **Acceptance:** All gate keys pass on reference SKUs with standard app mix

### Reference SKUs + Standard App Mix [MUST v1]  
- [ ] Reference SKUs + standard app mix checked in and pinned for every perf CI run
- [ ] Results don't drift due to hardware/workload variance
- **Acceptance:** CI runs use consistent hardware profiles; performance baselines stable

### ABI Governance, Frozen Early [MUST v1]
- [ ] Append-only syscall table with no renumbering
- [ ] Versioned vDSO with compatibility profiles and expiry dates
- [ ] CI runs both current and compatibility profiles
- **Acceptance:** ABI doc exists; compatibility tests load both profiles successfully

## 🧩 Microkernel Boundaries & IPC (No Ambient Authority)

### Capabilities & IPC Contract [MUST v1]
- [ ] Per-process handle tables (index+gen+rights) with read/write/signal/map/exec/dup/send/recv rights
- [ ] `cap_clone` can only shrink rights; time-boxed/subtree-scoped capabilities
- [ ] O(1) revoke by label with intrusive lists per holder
- [ ] Per-channel backpressure policies: drop/park/spill with counters
- **Acceptance:** Revoke p99 ≤200 µs (block), ≤2 ms (tear-down); IPC RTT same-core p99 ≤3 µs

### Microkernel Split Now [MUST v1]
- [ ] Move compositor to `rae-compositord` (DRM/KMS-like API, explicit fences, direct scanout)
- [ ] Move networking to `rae-netd` (userland stack owns NIC queues, pacing, BBR/CUBIC defaults)  
- [ ] Move filesystem to `rae-fsd` (VFS front in kernel, FS logic in userspace)
- **Acceptance:** Comp p99 jitter ≤0.3 ms @120 Hz; user↔user NIC RTT p99 ≤12 µs

## 🕒 Scheduling & Time (Tails Matter)

### RT Classes + Isolation [MUST v1]
- [ ] EDF/CBS for input/audio/compositor threads
- [ ] RR/fixed-prio for device threads  
- [ ] RT cores isolated with priority inheritance across IPC
- **Acceptance:** Input p99 <2 ms @90% CPU; compositor <1.5 ms @120Hz; audio jitter p99 <200 µs

### TSC-Deadline + Tickless Groundwork [MUST v1]
- [ ] UEFI+GOP, APIC/MSI-X, SMP per-CPU data structures
- [ ] Invariant TSC with cross-CPU synchronization
- **Acceptance:** Deadline jitter p99 ≤50 µs; TSC skew p99 ≤5 µs

## 🔐 Memory & Hardening (Secure by Default)

### W^X Everywhere, JIT Dual-Map Policy [MUST v1]
- [ ] Guard pages, KASLR, SMEP/SMAP/UMIP enforcement
- [ ] Dual-mapping policy documented for future JIT (RW→RX swap + TLB shootdown)
- **Acceptance:** Anon fault p99 ≤15 µs; 64-page/16-core TLB shootdown p99 ≤40 µs

### CFI/Shadow Stacks on Capable HW [SHOULD v1]
- [ ] Control Flow Integrity and shadow stacks enabled where supported
- [ ] Tracked under "Security Foundation" and reflected in boot attestation
- **Acceptance:** Boot attestation includes CFI enabled; no shadow-stack violations in test suite

## 🖥️ Graphics/Compositor (Feel Buttery, Measured)

### DRM/KMS-like API + Explicit Fences + Backlog Caps [MUST v1]
- [ ] App→comp ≤2 frames, comp→scanout =1 frame buffering
- [ ] Direct scanout for fullscreen applications
- **Acceptance:** Comp jitter p99 ≤0.3 ms @120 Hz; direct scanout verified for fullscreen

## 🌐 Networking Fast Path (User-Space First)

### Userland Stack Owns NIC Queues [MUST v1]
- [ ] Pacing enabled with BBR/CUBIC defaults
- [ ] Per-queue IRQ affinity configured
- [ ] 12 µs RTT test recipe published in tree
- **Acceptance:** User↔user loopback RTT p99 ≤12 µs; pacing keeps queueing delay p99 ≤2 ms

## 💽 Storage + FS (No Data Loss, Ever)

### Crash-Safe Semantics & Validation [MUST v1]
- [ ] CoW/journaled FS with checksums and write barriers
- [ ] Document fsync/rename guarantees explicitly
- [ ] Run crash-monkey-style CI with fault injection
- **Acceptance:** 10k power-fail cycles with 0 metadata corruption; journal replays clean

### NVMe Acceptance [MUST v1]
- [ ] 4 KiB QD1 read p99 ≤120 µs (hot set)
- [ ] Flush p99 ≤900 µs
- **Acceptance:** NVMe performance gates met on reference SKUs

## 🔭 Observability & Reliability (Debug in Minutes, Not Days)

### Flight Recorder + USDT Probes + Crash-Only Services [SHOULD v1]
- [ ] 72h chaos testing: 0 data loss, FS clean, crash dump contains flight-recorder snapshot
- [ ] Always-on flight recorder with bounded, local-only storage
- [ ] Crash-only services with micro-reboots and per-subsystem watchdogs
- **Acceptance:** 72h chaos run passes; flight recorder captures crash context

### Minidumps + Symbolization Pipeline [SHOULD v1]
- [ ] Tie crash dumps to exact build IDs with auto-upload gated by policy
- [ ] Offline symbol server with deterministic build IDs
- **Acceptance:** Minidump ≤16 MB generated ≤1s; symbolization <30s; regression linked to commit

## 🔐 Boot, Attestation, Supply Chain (Trust the Bits)

### Measured/Secure Boot + TPM Quotes + A/B Kernel [SHOULD v1]
- [ ] Immutable root with attestation logs for mitigation toggles
- [ ] A/B kernel images with auto-rollback and reproducible-build bit
- **Acceptance:** Attestation includes mitigation policy; A/B rollback tested and verified

### Repro Builds, SBOM, Signed Artifacts, SLSA Target [SHOULD v1]
- [ ] Wire reproducible builds into RaeenPkg and CI pipeline
- [ ] SBOM emission with artifact signing and verification
- **Acceptance:** Bit-for-bit reproducible across two builders; attestations required by CI

## 🛡️ Security Posture (Least Privilege All Day)

### Syscall Surface Minimization Gate [MUST v1]
- [ ] Every new syscall needs capability mapping + design doc
- [ ] Unknown bits rejected; per-process filters optional
- **Acceptance:** Syscall surface documented; unknown syscalls hard-reject; cap mappings verified

### Driver Zero-Trust [MUST v1]
- [ ] User-space first with IOMMU isolation for all devices
- [ ] Restartable workers; no kernel-wide failure from driver issues
- **Acceptance:** Driver failures don't crash kernel; IOMMU isolation verified

## 📉 Resource Control & Pressure

### PSI Signals + Quotas [SHOULD v1]
- [ ] CPU/RT/mem/IO/IPC credits with PSI-driven backoff
- [ ] Default hierarchy (system/user/services) with quotas
- **Acceptance:** PSI mem stall ≤5%/60s under standard app mix

## 🛠 Platform & Power Polish

### No-Legacy Bring-Up [MUST v1]
- [ ] UEFI+GOP, x2APIC/MSI-X, SMP per-CPU, TSC-deadline tickless groundwork
- **Acceptance:** Legacy VGA/PIC/8259 support removed; modern platform APIs only

### Idle & Wake Budgets [SHOULD v1]
- [ ] Idle ≤150 mW on laptop SKU with predictable RT wake
- [ ] Enforce via SLO harness on reference SKUs
- **Acceptance:** Power budgets met on reference laptop SKUs; wake latency predictable

## 🧑‍💻 DevEx & Ops Guardrails

### Ship Discipline [MUST v1]
- [ ] Forbid renumbering syscalls, enforce -D warnings, no unwrap/expect in kernel
- [ ] Migration notes required for refactors
- **Acceptance:** CI enforces coding standards; syscall ABI stability maintained

### Kill-Switches + Runbooks [SHOULD v1]
- [ ] Per-subsystem feature flags with break-glass capabilities
- [ ] SEV-level runbooks tied to A/B rollback procedures
- **Acceptance:** Feature flags tested; runbooks validated; rollback procedures work

## 🎮 Gaming-First Receipts (Make it Measurable)

### Game Mode Acceptance [SHOULD v1]
- [ ] Define input→present budget and minimum 1%-low uplift vs baseline
- [ ] Storefront bridges marked as [LATER] priority
- **Acceptance:** Input-to-present budget ≤X ms; 1% low fps uplift ≥Y% vs baseline

## 🌍 UX, A11y, Intl (Polish That Shows)

### A11y/Intl Test Bullets [SHOULD v1]
- [ ] Screen reader flows, contrast presets, IME smoke tests as checklist items
- **Acceptance:** Screen reader end-to-end flows pass; contrast AA/AAA validated; CJK IME tested

---

# 🔥 FLAGSHIP FEATURES (User-Visible with Testable Gates)

## 📊 Latency Lab (End-to-End) [SHOULD v1]
- [ ] Live timeline from input → app → compositor → scanout
- [ ] Per-frame p50/p95/p99 metrics with frame pacing heatmap
- [ ] "What spiked" hints for latency analysis
- [ ] Export `latency_lab.json` for CI integration
- **Acceptance:** p99 input→present ≤X ms in Game Mode on desk-sku-a; ties to 120 Hz polish + compositor pacing

## 🎮 Game Mode 2.0 (Hard Isolation) [MUST v1]
- [ ] One toggle pins game process to isolated RT cores
- [ ] Assigns dedicated NIC queues with IRQ affinity
- [ ] Throttles background daemons to minimal CPU share
- [ ] Locks compositor backlog caps (app→comp ≤2, comp→scanout=1)
- **Acceptance:** Input→present p99 ≤X ms; compositor jitter p99 ≤0.3 ms @120 Hz; NIC loopback RTT p99 ≤12 µs; background CPU share ≤Y% while active

## 💾 DirectStorage-Style Asset Streaming [MUST v1]
- [ ] Kernel exposes minimal I/O + DMA capabilities only
- [ ] User-space storage service streams game assets
- [ ] Async NVMe with zero-copy grants via shared memory
- [ ] Asset pipeline bypasses kernel VFS for hot paths
- **Acceptance:** NVMe 4 KiB @ QD=1 p99 ≤120 µs (hot set); NVMe FLUSH/fsync p99 ≤900 µs

## 🛡️ Safe Graphics Mode [MUST v1]
- [ ] GOP scanout fallback when GPU/driver fails
- [ ] Minimal compositor keeps desktop reachable
- [ ] Flight recorder captures GPU failure context
- [ ] Auto-recovery attempts with progressive fallback
- **Acceptance:** Enter safe mode within 1s; keep input responsive; crash dump captured and symbolized

## 🔗 RaeLink (Low-Latency Game Streaming) [SHOULD v1]
- [ ] Host/Client pairing with capability-gated permissions
- [ ] Hardware encode path with GPU acceleration
- [ ] Input capture with minimal latency overhead
- [ ] Network optimization for real-time streaming
- **Acceptance:** Host→Client glass-to-glass ≤35 ms p99; zero frame drops for 10 min continuous streaming

## 🔒 Per-App Network Firewall (Cap-Aware) [MUST v1]
- [ ] Default-deny network access for all processes
- [ ] Grant net:udp:port-range via explicit capabilities
- [ ] No ambient network authority for any process
- [ ] Audit trail for all network capability grants/revokes
- **Acceptance:** Net-cap violations blocked with audit; verified policy enforcement; no ambient authority bypass

## 🔍 Privacy Dashboard [MUST v1]
- [ ] View per-app capabilities (files, net, mic, camera)
- [ ] Revoke capabilities live with immediate effect
- [ ] Audit trail shows all capability operations
- [ ] Bulk revoke by label with O(1) performance
- **Acceptance:** Bulk revoke by label succeeds in ≤2 ms; UI refresh reflects revocation instantly

## 🔄 Crash-Only Services + Micro-Reboots [MUST v1]
- [ ] All daemons designed as disposable processes
- [ ] State externalized via versioned shared memory
- [ ] Watchdogs auto-restart failed services
- [ ] Zero data loss during service restarts
- **Acceptance:** 72h chaos = 0 data-loss; FS check clean; flight recorder persisted across restarts

## 🔐 Measured/Secure Boot + A/B Kernel + Immutable Root [SHOULD v1]
- [ ] Attestation logs mitigation toggles (SMEP/SMAP/UMIP, CFI, W^X)
- [ ] Build provenance with reproducible-build bit
- [ ] Auto-rollback on boot failure with A/B partitions
- [ ] Immutable root filesystem with integrity verification
- **Acceptance:** Boot report shows reproducible build bit + toggles; rollback rehearsal passes; attestation includes mitigation policy

## 📦 Hermetic Builds + SBOM Everywhere [MUST v1]
- [ ] `cargo vendor` for reproducible dependencies
- [ ] in-toto attestations with cosign verification
- [ ] `cargo deny` gates for license whitelist and CVE blocking
- [ ] SBOM generation for all artifacts
- **Acceptance:** Bit-for-bit reproducible across 2 builders; 0 disallowed licenses; 0 critical vulns; all artifacts signed

## 🖥️ AppVMs for Untrusted Apps [LATER]
- [ ] Lightweight micro-VMs using user-space drivers
- [ ] Performance mode bypasses for signed + sandboxed apps
- [ ] Copy-paste and window sharing between VMs
- [ ] Audio/video passthrough with zero underruns
- **Acceptance:** AppVM cold-start ≤800 ms; copy-paste + window sharing functional; audio underrun = 0

## 🎨 RaeUI Effects (SLO-Aware) [SHOULD v1]
- [ ] Glass/mica/blur effects with performance budgeting
- [ ] Compositor disables effects if they break jitter targets
- [ ] Runtime SLO monitoring for UI effects
- [ ] Graceful degradation when performance drops
- **Acceptance:** Effect toggles are SLO-aware at runtime; compositor jitter ≤0.3 ms @120 Hz maintained

## ♿ A11y First-Run & IME Smoke [MUST v1]
- [ ] First-boot flow checks screen reader functionality
- [ ] Contrast preset validation during setup
- [ ] IME (CJK) smoke test suite in QA pipeline
- [ ] Accessibility features discoverable and functional
- **Acceptance:** All a11y flows pass; IME round-trips functional; contrast presets validated

## 💻 RaeShell: Pipelines + GPU Text [SHOULD v1]
- [ ] Inline graphs and images with GPU acceleration
- [ ] AI fix-its with explicit capability grants
- [ ] Zero-copy handoff to compositor for rendering
- [ ] Command palette with audited actions
- **Acceptance:** Scroll jank p99 <2 ms; command palette actions audited; GPU text rendering functional

## ⚡ One-Click Performance Profiles [SHOULD v1]
- [ ] Per-app presets: RT budget, THP policy, CPU governor
- [ ] NIC queue pinning and background throttling controls
- [ ] Profile switching without SLO violations
- [ ] Performance diff reporting in Vitals
- **Acceptance:** Profile switch never violates SLO gates; report diffs in Vitals; presets applied instantly

---

# 🖥️ DESKTOP & WINDOWING FEATURES
*Modern desktop experience with advanced window management and productivity features*

## 🗂️ Desktop Stacks & Organization [MUST v1]

### RaeStacks (Desktop Stacks++ from macOS) [MUST v1]
- [ ] Auto-group files on desktop by type/date/project with AI assistance
- [ ] Per-folder rules and "Project Mode" that pulls related emails/notes
- [ ] Customizable grouping rules, exclude lists, animation styles
- [ ] Grid density controls and visual organization options
- **Acceptance:** Auto-grouping accuracy ≥90%; project mode integration functional; customization settings preserved

## 🪟 Virtual Desktops & Window Management [MUST v1]

### RaeSpaces (Mission Control + Task View) [MUST v1]
- [ ] Virtual desktops with profiles (apps, wallpaper, focus rules, power settings)
- [ ] Saved "Scenes" recallable per project/game with one hotkey
- [ ] Hot corners, gesture count, per-display spaces, per-space Dock
- [ ] Cross-space window management and preview capabilities
- **Acceptance:** Scene switching ≤500ms; profile application instant; gesture recognition ≤100ms

### Snap Designer (Win11 Snap Layouts Pro) [MUST v1]
- [ ] Visual editor to draw custom snap grids
- [ ] Remember per-app snap preferences automatically
- [ ] Per-monitor templates, edge resistance, alt-drag grid toggle
- [ ] Advanced snapping rules and window positioning logic
- **Acceptance:** Grid creation intuitive; per-app preferences saved; snap detection ≤50ms

### App Exposé in Alt-Tab [SHOULD v1]
- [ ] Hold Alt-Tab on app to fan out its windows with type-to-filter
- [ ] Thumbnail size, order (MRU vs spatial), cross-space peek options
- [ ] Window preview and quick navigation capabilities
- **Acceptance:** Window fan-out ≤200ms; filtering responsive; preview generation ≤50ms

### Stage Bins (Stage Manager, Enhanced) [SHOULD v1]
- [ ] "Bins" of window groups at side for context switching
- [ ] Drag to swap entire window contexts between bins
- [ ] Bin width, count, auto-collapse rules, keyboard cycles
- [ ] Multi-monitor stage management support
- **Acceptance:** Context switching ≤300ms; drag operations smooth; keyboard navigation functional

### Always-On-Top + Picture-in-Picture [MUST v1]
- [ ] One keystroke to pin any window as always-on-top
- [ ] PIP mode for videos/calls/document cameras
- [ ] Opacity, border style, click-through toggle customization
- [ ] Per-app PIP behavior and window management rules
- **Acceptance:** Pin/unpin instant; PIP mode stable; customization options functional

## 📁 File Management & Navigation [MUST v1]

### RaeFinder (Quick Look Enhanced) [MUST v1]
- [ ] Spacebar preview for any file type with interactive capabilities
- [ ] Scrub videos, annotate PDFs, 3D model viewing, code diff display
- [ ] Plugin chain order, preview shortcuts, sandboxed preview system
- [ ] Advanced file operations and batch processing
- **Acceptance:** Preview loads ≤200ms; annotation tools functional; plugin system secure

### Smart Folders & Rules [SHOULD v1]
- [ ] Live folders built from queries/tags with action triggers
- [ ] Rule editor for automated file organization
- [ ] Action library (resize images, re-tag, notify) with customization
- [ ] Dynamic folder content updates and smart categorization
- **Acceptance:** Query execution ≤100ms; rule triggers reliable; action library extensible

### Power Rename & Bulk Operations [MUST v1]
- [ ] Regex + presets with live preview for batch renaming
- [ ] Batch convert media, extract text via OCR
- [ ] Preset library, per-project presets, conflict resolution
- [ ] Advanced pattern matching and transformation rules
- **Acceptance:** Rename preview instant; OCR accuracy ≥95%; conflict resolution robust

### Libraries 2.0 [SHOULD v1]
- [ ] Virtual folders merging multiple locations (Windows Libraries style)
- [ ] Per-app views and per-library sorting/view modes
- [ ] Offline pinning and synchronization capabilities
- [ ] Cross-device library access and management
- **Acceptance:** Library creation seamless; per-app views functional; offline access reliable

## 🚀 Dock, Start, & Launchers [MUST v1]

### RaeDock (Dock/Taskbar Hybrid) [MUST v1]
- [ ] Live badges, progress bars, stacks, per-desktop docks
- [ ] Vertical/horizontal orientation with customizable zones
- [ ] Magnification curve, badge theming, middle-click actions
- [ ] Multi-monitor support and positioning rules
- **Acceptance:** Badge updates ≤100ms; magnification smooth; multi-monitor sync functional

### RaeStart (Start Menu + Launchpad) [MUST v1]
- [ ] Pinned grid + recent documents with app folders
- [ ] Type-to-search like Spotlight with intelligent ranking
- [ ] Row/column count, grid density, recommendations toggle
- [ ] Customizable layout and organization options
- **Acceptance:** Search results ≤50ms; grid layout responsive; customization preserved

### RaeSpot (Spotlight + PowerToys Run) [MUST v1]
- [ ] Commands, math, unit conversion, clipboard items, quick toggles
- [ ] Plugin system with ranking and privacy scope controls
- [ ] Skin/theme customization and result source management
- [ ] Advanced search capabilities and action execution
- **Acceptance:** Search response ≤50ms; plugin execution sandboxed; privacy controls enforced

## 🔗 Continuity, Sharing, & Devices [SHOULD v1]

### Universal Control++ [SHOULD v1]
- [ ] Mouse/keyboard flow across RaeenOS devices with file drag-drop
- [ ] Window teleport between machines and per-app follow-me mode
- [ ] Trust list, corner barrier strength, clipboard policy controls
- [ ] Cross-device input and display management
- **Acceptance:** Device pairing ≤10s; input latency ≤5ms; window teleport functional

### SidePanel (Universal Second Display) [SHOULD v1]
- [ ] Any tablet/phone as second display (wired/wireless)
- [ ] Low-latency ink support with pressure sensitivity
- [ ] Compression quality, color profile, latency vs quality slider
- [ ] Multi-device display management and configuration
- **Acceptance:** Connection latency ≤100ms; ink latency ≤20ms; quality controls functional

### Continuity Camera Pro [SHOULD v1]
- [ ] Phone as webcam with background blur/green-screen
- [ ] Desk view capability and effect pipeline customization
- [ ] Per-app profiles and auto-switch on proximity
- [ ] Advanced camera controls and image processing
- **Acceptance:** Camera switching ≤2s; effects processing real-time; proximity detection reliable

### RaeDrop (Enhanced File Sharing) [MUST v1]
- [ ] Offline queue, QR join, "trusted circle" auto-accept
- [ ] Visibility modes, auto-delete policies, bandwidth caps
- [ ] Cross-platform compatibility and security controls
- [ ] Large file transfer optimization and resume capability
- **Acceptance:** Transfer initiation ≤3s; offline queue reliable; security controls enforced

## 🔍 Search, Text, & Intelligence [SHOULD v1]

### Live Text Everywhere [SHOULD v1]
- [ ] Select/copy text from images, paused videos, screen shares
- [ ] Multi-language support with on-device processing option
- [ ] Privacy redactions and selective text recognition
- [ ] Integration with system clipboard and search
- **Acceptance:** Text recognition accuracy ≥95%; processing ≤2s; privacy controls functional

### Visual Lookup & Object Capture [SHOULD v1]
- [ ] Right-click image identification (plants, products, landmarks)
- [ ] 3D capture from camera sweep with model generation
- [ ] Offline models, result sources, "never send to cloud" mode
- [ ] Integration with knowledge bases and search engines
- **Acceptance:** Object recognition accuracy ≥85%; 3D capture functional; offline mode complete

### Quick Actions in Spotlight [MUST v1]
- [ ] Natural language commands ("Resize 10 images to 1080p")
- [ ] Custom verbs, permissions per action, audit trail
- [ ] Integration with system functions and third-party apps
- [ ] Workflow automation and scripting capabilities
- **Acceptance:** Command parsing accurate; action execution secure; audit trail complete

## 🎮 Gaming & Media Experience [MUST v1]

### Game Mode 2.0 (Enhanced) [MUST v1]
- [ ] Core isolation, NIC queue pinning, background throttling
- [ ] Strict compositor caps with performance monitoring
- [ ] Customizable core allocation and resource management
- [ ] Fan curve integration and thermal management
- **Acceptance:** Input→present p99 ≤X ms; background CPU ≤5%; thermal controls functional

### Auto-HDR & Color Pipeline [SHOULD v1]
- [ ] System-wide tone mapping for SDR→HDR conversion
- [ ] Per-app whitelist and curve presets
- [ ] Game profiles and per-display disable options
- [ ] Color accuracy validation and calibration tools
- **Acceptance:** HDR conversion quality verified; per-app controls functional; calibration accurate

### RaeBar (Enhanced Game Bar) [SHOULD v1]
- [ ] Instant replay buffer with configurable duration
- [ ] FPS/frametime monitoring and per-app EQ controls
- [ ] Stream/chat widgets and customizable overlay layout
- [ ] Telemetry controls and capture bitrate management
- **Acceptance:** Overlay latency ≤5ms; capture quality maintained; privacy controls enforced

### Audio Routing Matrix [SHOULD v1]
- [ ] Per-app to per-device routing with spatialization
- [ ] "Duck non-voice" toggle and scene management
- [ ] Auto-route rules and device priority configuration
- [ ] Advanced audio processing and effects pipeline
- **Acceptance:** Audio routing ≤10ms; spatial accuracy verified; scene switching instant

## 🌐 Networking & Connectivity [MUST v1]

### Per-App Firewall (Capability-Aware) [MUST v1]
- [ ] Default-deny with granular rules (net:tcp:443, net:udp:27015-27020)
- [ ] Profile-based configurations (Home/Work/Café)
- [ ] Learning mode and rule bundle management
- [ ] Real-time monitoring and violation reporting
- **Acceptance:** Rule enforcement verified; profile switching instant; violation detection accurate

### RaeLink (Low-Latency Streaming) [SHOULD v1]
- [ ] Host/client with hardware encode and controller passthrough
- [ ] 35ms p99 glass-to-glass latency target
- [ ] Codec selection, bitrate control, quality vs latency tuning
- [ ] Input mapping and device forwarding capabilities
- **Acceptance:** Latency target met; stream quality maintained; input forwarding functional

### Wi-Fi Sense & Auto-Roam [SHOULD v1]
- [ ] Prefer low-latency APs with seamless roaming
- [ ] Audio glitch prevention during network transitions
- [ ] Latency vs throughput bias and per-SSID rules
- [ ] Metered connection policies and bandwidth management
- **Acceptance:** Roaming seamless; audio continuity maintained; policy enforcement functional

## 🔔 Notifications, Widgets, & Control [MUST v1]

### Unified Control Center [MUST v1]
- [ ] Quick toggles + sliders + context tiles (battery, VPN status)
- [ ] Tile layout editor and long-press actions
- [ ] Per-profile tile sets and customization options
- [ ] Integration with system settings and third-party controls
- **Acceptance:** Control response ≤100ms; customization preserved; integration functional

### Widgets/Glance Panel [SHOULD v1]
- [ ] Calendar, weather, todo, stocks, performance widgets
- [ ] Third-party widgets with budget limits and permissions
- [ ] Refresh cadence control and desktop pinning
- [ ] Snap zones and layout management
- **Acceptance:** Widget updates ≤1s; budget enforcement functional; layout management smooth

### Focus Modes & Filters [SHOULD v1]
- [ ] Filter apps/people + route content (e.g., only "Work" email)
- [ ] Schedule-based activation and app override rules
- [ ] Wallpaper/sound changes per focus mode
- [ ] Integration with notification system and app behavior
- **Acceptance:** Mode switching instant; filtering accurate; integration seamless

## 🔐 Security, Privacy, & Accounts [MUST v1]

### RaeVault (Credential Management) [MUST v1]
- [ ] Passkeys, passwords, SSH keys with biometric unlock
- [ ] Windows Hello-style presence detection and FIDO2 support
- [ ] 2FA policy enforcement and per-app key scopes
- [ ] On-device only option and cloud sync controls
- **Acceptance:** Unlock latency ≤1s; FIDO2 compliance verified; privacy controls enforced

### Gatekeeper x SmartScreen [MUST v1]
- [ ] Signed/notarized app verification with quarantine system
- [ ] Sandbox trial mode and trust level management
- [ ] Organization policy support and one-time bypass with audit
- [ ] Real-time threat detection and response
- **Acceptance:** Verification accurate; sandbox isolation functional; audit trail complete

### Family Sharing & Screen Time [SHOULD v1]
- [ ] App limits, purchase approval, shared storage management
- [ ] Activity reports and per-child capability controls
- [ ] Time windows and "homework mode" restrictions
- [ ] Parental controls and content filtering
- **Acceptance:** Limits enforcement reliable; reports accurate; controls effective

## 🛠️ Development & Power User Tools [SHOULD v1]

### RaeShortcuts (Automation Platform) [SHOULD v1]
- [ ] Workflows with triggers (time, hotkey, file drop, network join)
- [ ] Per-workflow capability restrictions and sandbox levels
- [ ] Marketplace integration and workflow sharing
- [ ] Advanced scripting and integration capabilities
- **Acceptance:** Workflow execution reliable; sandbox isolation functional; marketplace secure

### Terminal That Slaps [SHOULD v1]
- [ ] GPU-accelerated text rendering with pane management
- [ ] SSH/Serial support and task management
- [ ] AI copilots with capability-scoped access
- [ ] Keymaps, themes, per-project environments, shell profiles
- **Acceptance:** Rendering smooth; AI integration secure; customization preserved

### Rae Subsystem for Linux [SHOULD v1]
- [ ] Development tools in fast user-space VM
- [ ] File portal with safe performance optimization
- [ ] Distro images, GPU/USB forwarding, resource caps
- [ ] Integration with host system and development workflows
- **Acceptance:** VM performance optimized; forwarding functional; integration seamless

### Keyboard Manager & Key Remap [SHOULD v1]
- [ ] System-wide remaps, chords, per-app layers
- [ ] Vim-style navigation and advanced key combinations
- [ ] Tap-dance timing and per-app override support
- [ ] Profile-based configurations and hotkey management
- **Acceptance:** Remapping reliable; timing accurate; profile switching instant

## 💾 Backup, Updates, & System Management [MUST v1]

### RaeHistory (Enhanced Versioning) [SHOULD v1]
- [ ] Continuous versioning to local/cloud with timeline UI
- [ ] Per-folder cadence, retention, encryption, bandwidth caps
- [ ] Visual diff support and intelligent snapshot management
- [ ] Integration with project workflows and collaboration tools
- **Acceptance:** Snapshot creation ≤5s; timeline navigation smooth; encryption verified

### A/B Updates + Immutable Root [MUST v1]
- [ ] Background updates with instant rollback capability
- [ ] "Safe Graphics Mode" fallback for GPU stack failures
- [ ] Update channels (Stable/Beta/Canary) with maintenance windows
- [ ] Metered connection rules and bandwidth management
- **Acceptance:** Update process seamless; rollback functional; safe mode reliable

### Storage Sense & Cleanup [SHOULD v1]
- [ ] Auto-clear caches/temp/duplicated files with smart detection
- [ ] Big file finder and storage usage analysis
- [ ] Customizable thresholds, exclusions, scheduled cleanup
- [ ] Integration with cloud storage and backup systems
- **Acceptance:** Cleanup effective; analysis accurate; scheduling reliable

### Display Sets [SHOULD v1]
- [ ] Save monitor arrangements/resolutions/scales by location
- [ ] Per-app DPI rules and per-set wallpapers
- [ ] Input source mapping and automatic profile switching
- [ ] Multi-monitor configuration management
- **Acceptance:** Profile switching ≤2s; DPI scaling accurate; input mapping functional

## ♿ Accessibility & UX Polish [MUST v1]

### First-Run Accessibility [MUST v1]
- [ ] Guided setup for screen reader, captions, color filters
- [ ] Pointer size adjustment and interaction customization
- [ ] Profile-based configurations and quick toggle access
- [ ] App-level accessibility overrides and preferences
- **Acceptance:** Setup flow intuitive; features functional; overrides effective

### Dictation & Live Captions [SHOULD v1]
- [ ] System-wide dictation with multi-language support
- [ ] Live captions for any audio with on-device processing option
- [ ] Caption style/position customization and profanity filtering
- [ ] Auto-punctuation and voice command integration
- **Acceptance:** Dictation accuracy ≥95%; caption latency ≤500ms; customization preserved

### Dynamic Desktop [SHOULD v1]
- [ ] Time-of-day/ambient-light wallpapers (macOS Dynamic Desktop style)
- [ ] Schedule customization and per-space themes
- [ ] True Tone/Night Light integration and color temperature tuning
- [ ] Automatic adaptation based on environment and usage patterns
- **Acceptance:** Transitions smooth; color accuracy maintained; adaptation intelligent

## 🌈 RGB & Device Integration [SHOULD v1]

### RaeGlow (Unified RGB Control) [SHOULD v1]
- [ ] Unified RGB control for keyboards, mice, fans, LED strips
- [ ] Sync to music or game events with customizable effects
- [ ] Per-app profiles and power-saving dim rules
- [ ] Privacy controls (no app can read devices without capabilities)
- **Acceptance:** Device sync accurate; effects responsive; privacy controls enforced

---

# 🖥️ DESKTOP, WINDOWS, & NAVIGATION
*Modern desktop experience with microkernel advantages*

## 📁 File Management & Navigation [MUST v1]

### RaeFinder (Finder/Explorer Fusion) [MUST v1]
- [ ] One-window tabs + dual-pane, column/tree views
- [ ] Blazing Quick Look (Space) with live, interactive previews
- [ ] Media scrub, Markdown render, 3D spin, OCR text copy in previews
- [ ] Layout presets, sidebar sections, tag colors customization
- [ ] Per-folder view defaults and scriptable actions
- [ ] Custom preview plugins with capability restrictions
**Acceptance:** Quick Look loads ≤200ms; preview plugins sandboxed; OCR accuracy ≥95%

### RaeDrop (AirDrop/Nearby Share++) [MUST v1]
- [ ] Peer-to-peer over LAN/BT/Ultra-Wideband with QR join
- [ ] Offline drop caches until peer returns
- [ ] Cap-scoped transfers (no ambient file access)
- [ ] Visibility rules, speed vs. battery profiles
- [ ] Auto-accept from trusted contacts
**Acceptance:** Transfer initiation ≤3s; offline cache reliable; capability isolation verified

### RaeHistory (Time Machine x File History) [SHOULD v1]
- [ ] Continuous file versioning to local or cloud storage
- [ ] Friendly timeline UI with visual diff support
- [ ] Pre-update snapshots + one-click rollback
- [ ] Per-folder frequency, retention, bandwidth caps
- [ ] Encryption level customization
**Acceptance:** Snapshot creation ≤5s; rollback functional; encryption verified

### Quick Look Pro [MUST v1]
- [ ] Spacebar preview for any file type
- [ ] Annotate PDFs, trim audio/video, extract OCR text
- [ ] Unzip/inspect archives with security scanning
- [ ] Preview hotkeys and plugin pipeline customization
**Acceptance:** Preview loads ≤200ms; OCR accuracy ≥95%; security scan blocks malware

## 🪟 Window Management & Desktop [MUST v1]

### RaeDock (Dock/Taskbar Hybrid) [MUST v1]
- [ ] Pinned apps + live progress badges
- [ ] Stack folders with hover fan-out
- [ ] Window thumbnails and per-desktop docks
- [ ] Vertical/horizontal orientation with group zones
- [ ] Size, magnification, autohide logic customization
- [ ] Middle-click behavior, badge styles, hotkeys
- [ ] Multi-monitor rules and positioning
**Acceptance:** Badge updates ≤100ms; thumbnail generation ≤50ms; multi-monitor sync

### RaeOverview (Mission Control + Task View) [MUST v1]
- [ ] Exposé grid with search-filter windows
- [ ] Group by app/workspace with timeline of recent windows
- [ ] Drag to virtual desktop with peek on hover
- [ ] Hot corners, gesture count, background blur customization
- [ ] Grouping heuristics and layout algorithms
**Acceptance:** Grid layout ≤200ms; search response ≤50ms; smooth animations @120Hz

### Snap Grids + Stage Scenes [MUST v1]
- [ ] Windows snapping like Win11 Snap Layouts
- [ ] Stage Manager-style scene bins
- [ ] Save/restore named layouts per project/game
- [ ] Grid designer and per-app preferred tile settings
- [ ] Keyboard choreography for window management
**Acceptance:** Snap detection ≤50ms; scene restore ≤500ms; keyboard shortcuts responsive

### Hot Corners & Edge Gestures [SHOULD v1]
- [ ] Every corner/edge bindable (Overview, Peek Desktop, Quick Note, Screenshot)
- [ ] Per-display mappings with modifier key support
- [ ] Profile-based configurations (Work/Gaming/Editing)
**Acceptance:** Gesture recognition ≤100ms; profile switching instant; no false triggers

## 🔍 Search & Launcher [MUST v1]

### RaeSpot (Spotlight x PowerToys) [MUST v1]
- [ ] Instant launcher with fuzzy search across apps/files/settings
- [ ] Quick actions (toggle Wi-Fi, dark mode, system controls)
- [ ] Math/unit/emoji calculations and conversions
- [ ] Dev snippets and shell command execution
- [ ] Plugin store with result ranking customization
- [ ] Privacy scope controls and theme options
- [ ] Inline previews for files and content
**Acceptance:** Search results ≤50ms; plugin execution sandboxed; privacy controls enforced

## 🎮 Gaming & Media Experience [MUST v1]

### 120–240 Hz UI with SLO-aware Effects [MUST v1]
- [ ] Glass/mica/blur effects only if jitter p99 ≤0.3ms maintained
- [ ] Auto-simplify effects when performance drops
- [ ] Per-app effect budget and global performance slider
**Acceptance:** Compositor jitter p99 ≤0.3ms @120Hz; effect degradation smooth; budget enforcement

### Game Mode 2.0 (Hard Isolation) [MUST v1]
- [ ] Pin game to dedicated cores with NIC queue assignment
- [ ] Throttle background daemons to minimal CPU share
- [ ] Lock compositor backlog caps (app→comp ≤2, comp→scanout=1)
- [ ] Customize cores/NIC queues, background CPU %, overlay widgets
**Acceptance:** Input→present p99 ≤X ms; background CPU ≤5%; NIC latency optimized

### RaeBar (Game Bar++) [SHOULD v1]
- [ ] Overlay for capture, FPS/frametime/latency graphs
- [ ] Per-app volume/effects controls
- [ ] Stream integrations and instant replay buffer
- [ ] Widget layout, hotkeys, HUD themes customization
- [ ] Telemetry opt-in controls
**Acceptance:** Overlay latency ≤5ms; capture quality maintained; privacy controls functional

### DirectStorage-style Asset Streaming [MUST v1]
- [ ] Zero-copy grants from storage service to game
- [ ] Async NVMe with I/O priorities
- [ ] Auto-HDR/tone-map path integration
- [ ] Cache size, per-title I/O ceilings customization
- [ ] Shader pre-warm policy configuration
**Acceptance:** Asset load time reduced 50%; zero-copy verified; I/O priorities enforced

### Spatial Audio Mixer [SHOULD v1]
- [ ] Per-app volume and per-device routing
- [ ] HRTF/spatial profiles with scene presets
- [ ] Quick "duck everything but voice" toggle
- [ ] App rules and device priority customization
**Acceptance:** Audio routing ≤10ms; spatial accuracy verified; ducking responsive

## 🌐 Networking & Connectivity [MUST v1]

### Per-App Firewall (Cap-aware) [MUST v1]
- [ ] Default-deny with net:udp:port-range rules per app
- [ ] DoH/DoT by default with QUIC path support
- [ ] Profiles (Home/Café/Work) and rule bundles
- [ ] Auto-prompts for network access requests
**Acceptance:** Rule enforcement verified; DoH/DoT functional; profile switching instant

### RaeLink (Low-latency Game/Desktop Streaming) [SHOULD v1]
- [ ] Host↔client pairing with hardware encode
- [ ] Controller passthrough with glass-to-glass ≤35ms p99
- [ ] Quality vs. latency, codec, bitrate customization
- [ ] Input mapping and device compatibility
**Acceptance:** Latency target met; controller lag ≤5ms; quality maintained

### Personal Hotspot & Casting [SHOULD v1]
- [ ] AirPlay/Miracast-like casting to TVs
- [ ] Guest network with device allowlist
- [ ] Printer/scanner auto-setup
- [ ] Codec, resolution/refresh limits customization
**Acceptance:** Casting latency ≤100ms; guest network isolated; auto-setup functional

## 🔔 Notifications & Control [MUST v1]

### Unified Control Center + Action Center [MUST v1]
- [ ] Quick toggles (Wi-Fi/BT/Focus/Do Not Disturb)
- [ ] Sliders (brightness/volume) and context tiles
- [ ] Tile layout, long-press actions customization
- [ ] Focus schedules and automation rules
**Acceptance:** Toggle response ≤100ms; tile customization saved; focus rules enforced

### Widgets/Glance [SHOULD v1]
- [ ] Panel + desktop-pin widgets
- [ ] Calendar, weather, todo, perf, stock tickers
- [ ] Third-party widgets with update-rate budgets
- [ ] Refresh cadence, permissions, snap regions customization
**Acceptance:** Widget updates ≤1s; third-party sandboxed; budget enforcement

### Focus Modes [MUST v1]
- [ ] Filter notifications by app/people/time
- [ ] Auto-enable for full-screen apps or meetings
- [ ] Rulesets, app overrides, per-mode wallpapers/sounds
**Acceptance:** Focus activation ≤200ms; notification filtering accurate; rules persistent

## 🔐 Security & Privacy [MUST v1]

### RaeVault (Keychain/Credential Manager++) [MUST v1]
- [ ] Passwords, passkeys, SSH keys, tokens storage
- [ ] Auto-fill with on-device ML anti-phish
- [ ] Unlock methods, per-app key scopes customization
- [ ] Hardware-key requirement options
**Acceptance:** Auto-fill ≤500ms; phishing detection ≥99%; hardware keys supported

### Gatekeeper x SmartScreen [MUST v1]
- [ ] Signed apps by default with notarized bundles
- [ ] Quarantine prompts and one-click sandbox trial run
- [ ] Trust levels, org policy profiles customization
- [ ] Temporary overrides with audit trail
**Acceptance:** Signature verification functional; sandbox isolation verified; audit complete

### Privacy Dashboard [MUST v1]
- [ ] View and revoke capabilities live (camera/mic/files/net)
- [ ] Schedule auto-revokes with "break-glass" emergency log
- [ ] Auto-revoke timers, per-cap prompts customization
- [ ] Audit retention and compliance reporting
**Acceptance:** Capability revoke ≤100ms; dashboard real-time; audit trail complete

### Parental & Screen Time [SHOULD v1]
- [ ] App limits, web filtering, bedtime controls
- [ ] Activity reports and usage analytics
- [ ] Per-child caps, temporary boosts, school mode
**Acceptance:** Limits enforced accurately; reports generated; parental controls bypass-proof

## ⚡ Productivity & Automation [SHOULD v1]

### RaeShortcuts (Shortcuts/Automator + PowerToys) [SHOULD v1]
- [ ] Drag-and-drop workflows with time/hotkey/event triggers
- [ ] File actions and window arranger
- [ ] Scriptable via JS/Rust/WASM with capability restrictions
- [ ] Per-workflow caps, run sandboxes, store-shareable recipes
**Acceptance:** Workflow execution ≤1s; scripting sandboxed; sharing functional

### Universal Clipboard & Handoff [SHOULD v1]
- [ ] Copy/paste across devices (text/images/files)
- [ ] Continue work on another device/app
- [ ] Clipboard history depth, content types customization
- [ ] Network scope (LAN/VPN) configuration
**Acceptance:** Sync latency ≤2s; handoff seamless; privacy controls enforced

### Clipboard History + Universal Snipping [MUST v1]
- [ ] OCR anywhere with rich paste (strip formatting/code block)
- [ ] Per-app clipboard rules and privacy exclusions
- [ ] History size, auto-expire, and privacy exclusions customization
- [ ] Universal snipping with annotation and sharing
**Acceptance:** OCR accuracy ≥95%; clipboard history instant; snipping tools responsive

### Terminal that Slaps [MUST v1]
- [ ] GPU text rendering with tabs/panes support
- [ ] SSH/Serial connections and task management
- [ ] AI assist (opt-in) with cap-scoped project access
- [ ] Fonts/themes, keybindings, shell profiles customization
**Acceptance:** Text rendering smooth @120Hz; AI assist sandboxed; SSH connections stable

### Migration Assistant [SHOULD v1]
- [ ] Pull from Windows/macOS accounts and browser data
- [ ] Import docs, keychains where possible
- [ ] Selection filters and conflict policies customization
**Acceptance:** Migration completes without data loss; conflict resolution functional

## 🔄 Backup, Updates, & System [MUST v1]

### A/B OS Updates + Immutable Root [MUST v1]
- [ ] Background updates with reboot slot flipping
- [ ] Easy rollback and safe graphics mode on GPU crashes
- [ ] Update channels, maintenance windows, metered data rules customization
**Acceptance:** Update/rollback ≤30s; safe mode functional; background updates seamless

### Store & Packages [MUST v1]
- [ ] Signed/reproducible bundles with delta updates
- [ ] Staged rollouts, SBOM, and key rotation
- [ ] Update cadence, beta flags, org catalogs customization
**Acceptance:** Package integrity verified; delta updates efficient; rollouts staged safely

### Virtual Desktops with Profiles [SHOULD v1]
- [ ] Work/Home/Gaming sets with auto-switch by location/time/app
- [ ] Per-desktop wallpapers, apps, and focus rules
- [ ] Triggers, per-desktop docks, keyboard maps customization
**Acceptance:** Profile switching ≤500ms; auto-triggers reliable; desktop isolation maintained

### Accessibility First [MUST v1]
- [ ] Voice Control, screen reader, high-contrast, color filters
- [ ] Live captions, magnifier, switch control with per-app overrides
- [ ] Profiles, quick toggles, gesture packs customization
**Acceptance:** All a11y features functional; per-app overrides work; quick toggles responsive

## ⭐ Bonus: Tiny Differentiators [SHOULD v1]

### Latency Lab Overlay [SHOULD v1]
- [ ] Per-frame input→present timing display
- [ ] "What spiked" hints for latency analysis
- [ ] Real-time performance metrics and bottleneck identification
**Acceptance:** Overlay latency ≤1ms; spike detection accurate; metrics real-time

### Live Activity Tiles [SHOULD v1]
- [ ] Media, downloads, device battery status in Dock
- [ ] Timer and progress indicators
- [ ] Customizable tile layout and information density
**Acceptance:** Tile updates ≤100ms; battery status accurate; layout customization saved

### Smart Rename & Bulk Ops [SHOULD v1]
- [ ] PowerRename-style with presets and regex preview
- [ ] Bulk operations with undo support
- [ ] Pattern templates and operation history
**Acceptance:** Rename preview instant; bulk ops ≤1s per 100 files; undo functional

### Safe Mode UX [MUST v1]
- [ ] Instant fallback to GOP + functional desktop on graphics failure
- [ ] Crash report ready with diagnostic information
- [ ] Recovery options and driver rollback capabilities
**Acceptance:** Safe mode activation ≤1s; desktop functional; crash reports complete

### One-Click Profiles [SHOULD v1]
- [ ] Work/Gaming/Travel profiles with OS-wide reconfiguration
- [ ] Focus, power, NIC queues, effects adjustment
- [ ] Profile switching with visual feedback
**Acceptance:** Profile switch ≤2s; all settings applied; visual feedback clear

---

# 🍎 MACOS-COMPETITIVE FEATURES
*Beating Apple at their own game with microkernel advantages*

## 🔗 Port/Right Semantics (Mach-Inspired) [MUST v1]
- [ ] Mirror Mach's "port right == capability" model
- [ ] Send/receive/send-once rights with generation counters
- [ ] Expiry/scope extensions beyond classic Mach
- [ ] Service discovery via port namespace
- [ ] Familiar API for Mac developers transitioning
- **Acceptance:** Port rights behave like Mach; service discovery functional; Mac dev documentation complete

## 🚀 Service Manager + High-Level IPC (launchd/XPC Vibes) [MUST v1]
- [ ] Single service manager atop low-latency rings
- [ ] High-level IPC with namespaces and service bootstrap
- [ ] XPC-like convenience APIs for common patterns
- [ ] Service lifecycle management (start/stop/restart)
- [ ] Dependency resolution and ordering
- **Acceptance:** Service manager functional; XPC-style APIs documented; lifecycle management robust

## 🔧 DriverKit-Like User-Space Drivers [MUST v1]
- [ ] Drivers user-space by default with narrow kernel bridge
- [ ] Apple DriverKit-inspired architecture
- [ ] IOMMU isolation for all user-space drivers
- [ ] Restartable driver workers with state preservation
- [ ] Hardware abstraction layer in user-space
- **Acceptance:** User-space drivers functional; IOMMU isolation verified; driver crashes don't affect kernel

## 🎨 Framework-First UX (Cocoa/AppKit Polish) [MUST v1]
- [ ] RaeUI emulates Cocoa/AppKit consistency and polish
- [ ] Explicit fences and frame pacing throughout UI stack
- [ ] Consistent theming engine across all applications
- [ ] Animation system with SLO-aware performance budgeting
- [ ] macOS-level smoothness with measurable guarantees
- **Acceptance:** Compositor jitter p99 ≤0.3 ms @120 Hz; UI feels as smooth as macOS; theming consistent

## 🐧 POSIX Compatibility Layer [SHOULD v1]
- [ ] Tight, well-documented POSIX subset (not full POSIX)
- [ ] Common tooling ports easily (like macOS BSD userland)
- [ ] Shell utilities and development tools compatibility
- [ ] File system semantics match POSIX expectations
- [ ] Process model compatibility for porting
- **Acceptance:** Common Unix tools compile and run; shell scripting functional; development workflow smooth

## 🏆 BEATING MACOS (Microkernel Advantages)

### 🔒 Superior Isolation & Crash Containment [MUST v1]
- [ ] Kernel stays tiny with all services restartable
- [ ] Better isolation than XNU's monolithic approach
- [ ] Service crashes never affect other services or kernel
- [ ] Micro-reboots faster than macOS service recovery
- [ ] Fault isolation at capability boundaries
- **Acceptance:** Service crashes isolated; recovery time <500ms; kernel never crashes from driver issues

### 🖥️ User-Space Drivers Everywhere [MUST v1]
- [ ] IOMMU isolation by policy, not exception
- [ ] All drivers user-space (beat Apple's partial DriverKit)
- [ ] No kernel drivers needed for standard hardware
- [ ] Driver sandboxing with capability restrictions
- [ ] Hot-pluggable driver updates without kernel changes
- **Acceptance:** 95% of hardware uses user-space drivers; IOMMU enforced; no kernel driver crashes

### 🛡️ Capability-Native Security [MUST v1]
- [ ] Every resource is a capability (beyond macOS entitlements)
- [ ] Bulk capability revoke in ≤2 ms (faster than macOS)
- [ ] Full audit trail for all capability operations
- [ ] No ambient authority anywhere in the system
- [ ] Fine-grained permissions beyond macOS sandbox
- **Acceptance:** Bulk revoke p99 ≤2 ms; audit trail complete; no ambient authority verified

### 📊 SLO Receipts (Prove Performance) [MUST v1]
- [ ] Publish performance guarantees Apple won't
- [ ] IPC RTT p99 ≤3 µs (measure and prove it)
- [ ] Input latency p99 <2 ms (beat macOS claims)
- [ ] NVMe performance p99 ≤120 µs (storage advantage)
- [ ] Compositor jitter p99 ≤0.3 ms @120 Hz (smoother than macOS)
- **Acceptance:** All SLO targets met and published; performance dashboard shows real-time metrics

### 🔐 A/B Images + Measured Boot (Day One) [MUST v1]
- [ ] A/B system images with automatic rollback
- [ ] Measured boot with TPM attestation
- [ ] Supply chain provenance out of the box
- [ ] Reproducible builds with verification
- [ ] Boot-time security better than macOS Secure Boot
- **Acceptance:** A/B rollback functional; measured boot attestation working; supply chain verified

### ⚡ Real-Time Guarantees [SHOULD v1]
- [ ] Hard real-time scheduling classes (macOS has soft RT)
- [ ] RT core isolation with guaranteed latency
- [ ] Priority inheritance across IPC boundaries
- [ ] Deterministic interrupt handling
- [ ] Audio/video with zero underruns guaranteed
- **Acceptance:** RT scheduling meets hard deadlines; audio underruns = 0; video frame drops = 0

### 🔄 Live System Updates [SHOULD v1]
- [ ] Update kernel and services without reboot
- [ ] Capability-based update permissions
- [ ] Rollback individual services without system restart
- [ ] Live patching with formal verification
- [ ] Better than macOS staged updates
- **Acceptance:** Live updates functional; service rollback works; no reboot required for most updates

---

## Phasing and Criticality Tags
- Use tags to drive priorities and CI gates:
  - [MUST v1]: required for the first microkernel release and SLO gating
  - [SHOULD v1]: strongly recommended for v1 stability and ops
  - [LATER]: post-v1 enhancements

## Microkernel & SLO Alignment (additions)

### 🧩 Capabilities & IPC [MUST v1]
- [ ] Per-process handle table {index, gen, rights}; rights: read/write/signal/map/exec/dup/send/recv
- [ ] cap_clone (rights can only shrink); time-boxed / subtree-scoped caps
- [ ] Revocation table with O(1) revoke per holder; intrusive lists by label
- [ ] Audit log (append-only, bounded, per-PID rate caps)
- [ ] MPSC rings + credit-based flow control; shared-mem grant tables (RO/RW by rights)
- [ ] Backpressure policy per channel (drop-oldest / park_with_timeout / spill_bounded) and counters export
- Acceptance: cap revoke p99 ≤ 200 µs (block new); ≤ 2 ms (tear shared maps). IPC RTT same-core p99 ≤ 3 µs
 - [ ] Per-process namespaces (ports/paths) resolve to capabilities; support bulk revoke by label

### 🔬 Reliability engineering & formal methods [MUST v1]
- [ ] IPC/capabilities model checked (state machine: create→grant→clone→revoke; no use‑after‑revoke)
- [ ] Invariants doc: no blocking in IRQ path; bounded queues; backpressure policy per channel
- Acceptance: model proofs attached; 72h chaos with 0 invariant violations (flight recorder verified)

### 🔒 Lock ordering and deadlock detection [MUST v1]
- [ ] Static lock levels; runtime lockdep
- Acceptance: 0 lockdep warnings over 72h chaos; contention metrics exported

### 🧠 Scheduler & RT [MUST v1]
- [ ] Classes: EDF/CBS for input/audio/compositor; RR/fixed-prio for device threads
- [ ] RT core isolation (nohz_full-like) + priority inheritance across IPC
- [ ] NUMA-aware runqueues; avoid remote wakeups for RT; CBS throttling to prevent starvation
- Acceptance: input p99 < 2 ms @ 90% CPU; compositor CPU < 1.5 ms @120Hz; audio jitter p99 < 200 µs

### 🛠 Platform bring-up [MUST v1]
- [ ] Switch to UEFI+GOP (no legacy VGA); x2APIC/LAPIC & MSI-X
- [ ] SMP bring-up + per-CPU data
- [ ] TSC-deadline timers; HPET fallback; tickless groundwork
- Acceptance: deadline timer jitter p99 ≤ 50 µs (interim); IRQ EOI and steering verified

### ⏰ Timekeeping & clocks [MUST v1]
- [ ] Invariant TSC enable; cross‑CPU TSC sync; TSC‑deadline timers default
- [ ] NTP client; PTP optional on wired
- Acceptance: cross‑core TSC skew p99 ≤ 5 µs; monotonic drift ≤ 1 ms/hour; NTP offset p99 ≤ 20 ms (Wi‑Fi), PTP ≤ 200 µs (wired)

### ⚡ Power states & S0ix residency [MUST v1]
- [ ] Deep C‑states/C‑stops policy; S0ix entry/exit tracing
- Acceptance: laptop idle (screen on) ≤ 0.8 W; screen off ≤ 0.4 W; S0ix residency ≥ 90%; resume‑to‑first‑frame ≤ 500 ms

### 🔐 Memory & hardening [MUST v1]
- [ ] Enforce W^X globally; guard pages for stacks
- [ ] KASLR; SMEP/SMAP/UMIP toggles; document exceptions
- [ ] Dual-mapping policy for future JIT (RW→RX swap + TLB shootdown)
- Acceptance: anon page-fault service p99 ≤ 15 µs; TLB shootdown (64 pages/16c) p99 ≤ 40 µs

### 🧪 SLO harness & CI [MUST v1]
- [ ] Reference SKUs + standard app mix checked in and used by CI
- [ ] Tests emit slo_results.json (schema-conformant) and tag metrics with #[slo(...)]
- [ ] CI gate: two consecutive passes or ≤ 5% drift vs rolling 7-day median
- Acceptance: pass all gate keys (input, audio, compositor, IPC RTT, faults, TLB, NVMe, idle power, chaos FS)

### 🧰 Microkernel split [MUST v1]
- [ ] Move compositor to rae-compositord (DRM/KMS-like API; explicit fences; direct scanout for fullscreen)
- [ ] Move networking to rae-netd (userland stack owning NIC queues; pacing, BBR/CUBIC defaults)
- [ ] Move filesystem to rae-fsd (VFS front in kernel, FS logic in user)
- Acceptance: compositor p99 jitter ≤ 0.3 ms @120Hz; user↔user NIC RTT p99 ≤ 12 µs

### 🔒 Boot, attestation, & image safety [SHOULD v1]
- [ ] Secure/Measured boot + TPM quotes; record mitigation toggles
- [ ] A/B kernel images with auto fallback; immutable root (verity)
- Acceptance: attestation includes reproducible-build bit and mitigation policy; A/B rollback tested

### 📉 Resource control & pressure [SHOULD v1]
- [ ] Cgroup-like quotas for CPU/RT/mem/IO/IPC credits; default hierarchy (system/user/services)
- [ ] PSI signals exported; Vitals can auto-throttle bulk QoS on alerts
- Acceptance: PSI memory stall ≤ 5% over 60 s under standard app mix

### 📜 Observability & crash-only ops [SHOULD v1]
- [ ] Always-on flight recorder (bounded, local-only, redaction); USDT-style tracepoints
- [ ] Crash-only services with micro-reboots; watchdogs per subsystem
- Acceptance: 72h chaos run, 0 data-loss; FS check clean; flight recorder dump on crash

### 🔗 Unified trace correlation [MUST v1]
- [ ] 128‑bit trace ID propagated across IPC; parent/child span linkage
- Acceptance: single timeline spans app→compositor→scanout; correlation rate ≥ 95% in CI scenario

### 💥 Minidumps + symbolization pipeline [MUST v1]
- [ ] Per‑service crash handler; offline symbol server; deterministic build IDs
- Acceptance: minidump ≤ 16 MB generated ≤ 1 s; symbolization job < 30 s; regression linked to commit

### 📜 ABI governance [MUST v1]
- [ ] Freeze syscall numbering; add only at tail; version vDSO symbols
- [ ] Add compat profiles with expiry; CI runs both
- Acceptance: ABI doc and tests that load current + compat profiles

## 🛠 Core Kernel - RaeCore

### Memory Management
- [x] Physical memory manager with frame allocation
- [x] Virtual memory manager with page table management
- [x] Heap allocator with memory safety guarantees
- [x] Memory protection and isolation
- [ ] Memory compression with zstd
- [ ] Intelligent page caching
- [ ] Memory defragmentation
- [ ] Swap prediction algorithms
- [ ] Copy-on-write for instant snapshots

### Process Management
- [x] Process creation and termination
- [ ] Process scheduling with real-time support
- [ ] Gaming priority modes
- [x] Context switching
- [ ] Inter-process communication (IPC)
- [x] Process isolation and sandboxing
- [ ] CPU core parking control
- [ ] Background process throttling

### System Calls → Services (Microkernel Flip) [MUST v1]
- [x] Basic syscall interface (proc/mem/caps/IPC/time only)
- [x] Process management syscalls
- [x] Memory management syscalls
- [x] File system syscalls (minimal VFS interface)
- [x] Security syscalls (capability management)
- [ ] **Network/Graphics/AI IPC contracts** owned by user-space daemons (`rae-netd`/`rae-compositord`/`rae-assistantd`)
- [ ] Kernel exposes only proc/mem/caps/IPC/time syscalls
- [ ] Schema-versioned IPC contracts with unknown field rejection
- [ ] Performance optimization syscalls (RT scheduling, memory policies)
- [ ] Gaming-specific syscalls (process pinning, priority inheritance)
- **Acceptance:** IPC RTT same-core p99 ≤3 µs; cap revoke p99 ≤200 µs; shared-map teardown ≤2 ms; syscall surface minimized

### Hardware Abstraction
- [ ] CPU feature detection (CPUID)
- [ ] Multi-core support
- [ ] NUMA awareness
- [x] Hardware interrupt handling
- [x] Timer management (PIT/HPET/TSC)
- [ ] Power management (ACPI)
- [ ] Thermal management
- [ ] Hardware ray tracing acceleration
- [ ] Variable rate shading support

### Security Foundation
- [ ] Secure boot with TPM 2.0
- [ ] Measured boot attestation
- [ ] Anti-rollback protection
- [ ] SMEP/SMAP/UMIP support
- [x] W^X memory protection
- [ ] Address space layout randomization (ASLR)
- [ ] Control flow integrity (CFI)
- [ ] Kernel guard pages

### 🛡️ CET/Shadow stacks + IBT [SHOULD v1]
- [ ] Control-flow Enforcement Technology (CET) with shadow stacks and Indirect Branch Tracking (IBT)
- Acceptance: boot attestation includes CET enabled; no shadow‑stack violations in test suite

### 🔐 PKU/PKS exploration for userspace sandboxes [LATER]
- [ ] Protection Keys for Userspace (PKU) and Protection Keys for Supervisor (PKS) evaluation
- Acceptance: microbench shows < 2% overhead; cap transitions mapped to PKU domains

### 🧹 Zero‑on‑free + heap quarantine [MUST v1]
- [ ] Memory zeroing on deallocation and heap quarantine implementation
- Acceptance: memory reuse tests observe zeroed pages; no stale secrets recovered

### 🔒 Side‑channel mitigations policy [MUST v1]
- [ ] Comprehensive side-channel attack mitigations (Spectre/Meltdown/etc.)
- Acceptance: spectre/meltdown test suite passes; mitigations recorded in attestation

### 📋 fs‑verity/verity‑like for immutable system image [SHOULD v1]
- [ ] File system integrity verification for immutable system components
- Acceptance: root verified; tamper detection blocks boot; recovery path validated

## 📁 File System

### Core File System
- [ ] Virtual File System (VFS) layer
- [ ] Native RaeenFS implementation
- [ ] File and directory operations
- [ ] Metadata management
- [ ] Permission system
- [ ] Symbolic and hard links
- [ ] File locking mechanisms
- [ ] Journaling for crash consistency

### 📊 Crash consistency & fs‑fuzz gates [MUST v1]
- [ ] Power‑fail injection testing; journal replay verification
- Acceptance: 10k power‑fail cycles, 0 metadata corruption; journal replays clean; scrub passes

### 📊 NVMe health & wear metrics [SHOULD v1]
- [ ] SMART data monitoring and wear leveling metrics
- Acceptance: SMART data exported; pre‑failure alerting wired to Vitals

### Advanced Features
- [ ] Automatic defragmentation
- [ ] Compression and deduplication
- [ ] RAID support
- [ ] Snapshot functionality
- [ ] Backup and restore
- [ ] Encryption at rest
- [ ] File system quotas
- [ ] Extended attributes

### Storage I/O [MUST v1]
- [ ] NVMe optimization with queue depth management
- [ ] Prioritized I/O queuing for RT workloads
- [ ] Predictive prefetching for game assets
- [ ] DirectStorage equivalent with zero-copy DMA
- [ ] Storage I/O priority boost for active applications
- [ ] Asynchronous I/O with completion callbacks
- [ ] I/O scheduler optimization for mixed workloads
- **Acceptance:** NVMe 4 KiB @ QD=1 p99 ≤120 µs (hot set); NVMe FLUSH/fsync p99 ≤900 µs; DirectStorage assets stream without VFS overhead

## 🎨 Graphics and Display

### Graphics Foundation
- [ ] Framebuffer management
- [ ] GPU driver interface
- [ ] Hardware acceleration
- [ ] OpenGL/Vulkan support
- [ ] DirectX compatibility layer
- [ ] Multi-GPU configurations
- [ ] GPU power limit control

### Display Management [MUST v1]
- [ ] Multi-monitor support with per-display configuration
- [ ] Variable refresh rate (VRR/G-Sync/FreeSync) with frame pacing
- [ ] HDR10+ and Dolby Vision when compositor jitter ≤0.3 ms
- [ ] 8K resolution at 120Hz supported when comp jitter ≤0.3 ms
- [ ] Independent scaling per display with SLO compliance
- [ ] Display profiles with performance impact monitoring
- [ ] Seamless window migration without frame drops
- **Acceptance:** VRR/HDR/8K@120Hz functional when compositor jitter p99 ≤0.3 ms maintained; multi-monitor setup stable

### Compositor
- [ ] GPU-accelerated rendering pipeline
- [ ] Triple buffering
- [ ] Tear-free rendering
- [ ] Minimal input latency
- [ ] Window composition
- [ ] Transparency and blur effects
- [ ] Animation system
- [ ] Smooth 120Hz+ animations
 - [MUST v1] Explicit sync fences; present only after acquire fences
 - [MUST v1] Backlog caps: app→compositor ≤ 2 frames; compositor→scanout = 1
 - [ ] Safe compositor mode (GOP/scanout + minimal UI) for recovery/telemetry
**Acceptance:** Compositor jitter p99 ≤ 0.3ms @ 120Hz on reference SKUs

## 🌐 Networking

### Network Stack
- [ ] TCP/IP implementation
- [ ] UDP support
- [ ] IPv6 support
- [ ] DNS resolution
- [ ] DHCP client
- [ ] Network interface management
- [ ] Routing table management
- [ ] ARP protocol
**Acceptance:** user↔user loopback via NIC queues p99 RTT ≤ 12µs (same NUMA); basic routing/ARP/DHCP verified

### 🔐 TLS 1.3/QUIC path in `rae-netd` [SHOULD v1]
- [ ] TLS 1.3 and QUIC protocol implementation in userspace network daemon
- Acceptance: localhost TLS handshake p95 ≤ 5 ms; QUIC 1‑RTT establish p95 ≤ 3 ms; pacing keeps queueing delay p99 ≤ 2 ms

### ⚙️ NIC queue pinning & busy‑poll policies [MUST v1]
- [ ] IRQ/queue affinity and busy polling optimization
- Acceptance: IRQ/queues affined to cores; tail lat p99 improves ≥ 20% under load (CI scenario)

### Advanced Networking
- [ ] Network packet prioritization
- [ ] Quality of Service (QoS)
- [ ] Firewall with learning mode
- [ ] DNS-over-HTTPS by default
- [ ] VPN integration with kill switch
- [ ] Network monitoring
- [ ] Bandwidth management
- [ ] Network security scanning
 - [ ] WireGuard stack; QUIC path in `rae-netd`
 - [ ] Per-app firewall enforced by capabilities
 - [ ] DoH/DoT default for DNS
**Acceptance:** Connectivity passes with filters enabled and RTT gates still met

### Socket Interface
- [x] Socket creation and management
- [x] TCP socket operations
- [x] UDP socket operations
- [ ] Unix domain sockets
- [ ] Raw sockets
- [ ] Socket security policies
- [ ] Socket performance optimization

## 🔊 Audio System

### Audio Foundation
- [ ] Audio driver interface
- [ ] Low-latency audio pipeline
- [ ] Multiple audio device support
- [ ] Audio mixing and routing
- [ ] Sample rate conversion
- [ ] Audio format support (PCM, compressed)
- [ ] MIDI support
- [ ] VST plugin support

### Audio Features
- [ ] Audio device profiles with EQ presets
- [ ] Spatial audio support
- [ ] Audio effects processing
- [ ] Real-time audio processing
- [ ] Audio recording and playback
- [ ] System sounds
- [ ] Haptic feedback patterns

## 🎮 Gaming Optimizations

### RaeenGame Mode
- [ ] Automatic game detection (10,000+ games)
- [ ] CPU optimization for gaming
- [ ] GPU power limit removal
- [ ] Background process throttling
- [ ] Network packet prioritization
- [ ] Storage I/O priority boost
- [ ] Game-specific optimization profiles
- [ ] Performance monitoring

### Gaming Integration
- [ ] Steam integration
- [ ] Epic Games Store integration
- [ ] GOG Galaxy integration
- [ ] Xbox Game Pass integration
- [ ] PlayStation Plus integration
- [ ] Battle.net integration
- [ ] EA App integration
- [ ] Ubisoft Connect integration

### Performance Tools
- [ ] RaeMetrics performance overlay
- [ ] FPS and frame time monitoring
- [ ] CPU/GPU usage monitoring
- [ ] Temperature monitoring
- [ ] RAM and VRAM usage tracking
- [ ] Network latency measurement
- [ ] Input latency measurement
- [ ] AI-powered bottleneck detection

### Recording and Streaming
- [ ] Built-in game recording
- [ ] Instant replay buffer
- [ ] One-click streaming setup
- [ ] Twitch integration
- [ ] YouTube integration
- [ ] OBS integration
- [ ] Multi-stream support

## 🖥 Desktop Environment - RaeenDE

### Window Management
- [ ] Floating windows with magnetic snapping
- [ ] Tiling mode with customizable grids
- [ ] Tabbed windows
- [ ] Picture-in-picture for any app
- [ ] Virtual desktops
- [ ] 3D cube transitions
- [ ] Per-desktop wallpapers and themes
- [ ] Activity-based workspace templates

### Theming Engine
- [ ] System-wide theme store
- [ ] One-click theme application
- [ ] Color schemes with gradient support
- [ ] Blur intensity and transparency controls
- [ ] Window corner customization
- [ ] Animation speed controls
- [ ] Icon packs with adaptive coloring
- [ ] Live theme preview
- [ ] Scheduled themes

### Widgets and Live Content
- [ ] Desktop widgets with live data
- [ ] Interactive wallpapers
- [ ] Wallpaper Engine compatibility
- [ ] Audio-reactive visualizations
- [ ] System monitoring overlays
- [ ] Weather and time-based changes
- [ ] Notification center
- [ ] Quick settings panel

## 🧠 AI Integration - Rae Assistant

### Core AI Framework
- [ ] Natural language processing
- [ ] System control via voice/text
- [ ] Predictive features
- [ ] App pre-loading based on usage
- [ ] Smart file organization
- [ ] Automated backup scheduling
- [ ] Resource allocation predictions

### Creative AI Tools
- [ ] Code generation and debugging
- [ ] Image generation and editing
- [ ] Document summarization
- [ ] Language translation
- [ ] Workflow automation
- [ ] Visual scripting interface
- [ ] Context-aware suggestions

### Privacy-First AI
- [ ] On-device processing for sensitive data
- [ ] Opt-in cloud features with encryption
- [ ] Clear AI decision explanations
- [ ] User control over AI training data
- [ ] AI usage analytics
- [ ] Privacy dashboard

## 🔒 Security and Privacy

### Application Security
- [ ] Mandatory sandboxing
- [ ] File system isolation
- [ ] Network filtering
- [ ] Hardware access control
- [ ] Inter-process communication limits
- [ ] Granular permission system
- [ ] Time-limited access grants
- [ ] Permission usage analytics

### System Security
- [ ] Full-disk encryption
- [ ] Hardware acceleration for encryption
- [ ] Multiple user keys
- [ ] Emergency recovery options
- [ ] Automatic security updates
- [ ] Vulnerability scanning
- [ ] Intrusion detection
- [ ] Security audit logging

### Privacy Features
- [ ] Privacy dashboard
- [ ] App data collection monitoring
- [ ] Network connection tracking
- [ ] Telemetry settings control
- [ ] Ad blocking statistics
- [ ] Built-in tracking protection
- [ ] Webcam and microphone indicators
- [ ] Encrypted clipboard history

## 📦 Package Management - RaeenPkg

### Core Package System
- [ ] .rae package format
- [ ] Sandboxed app bundles
- [ ] Delta updates
- [ ] Rollback capabilities
- [ ] System snapshots
- [ ] Dependency resolution
- [ ] Conflict prevention
- [ ] Package verification

### 🔐 Supply chain hardening (SLSA) [MUST v1]
- [ ] Hermetic builds; `cargo vendor`; provenance attestations (in‑toto); cosign verification
- Acceptance: bit‑for‑bit reproducible artifacts across two builders; attestations required by CI

### 🛡️ License and vuln policy gates [MUST v1]
- [ ] `cargo deny` for license whitelist; critical CVEs block
- Acceptance: 0 disallowed licenses; 0 critical vulns; unsound crates banned

### 🔑 Key management & rotation [MUST v1]
- [ ] Release keys in HSM/TPM; annual rotation; emergency revoke path
- Acceptance: rotation rehearsal passes; all artifacts verify after rotation

### Application Support
- [ ] Native RaeenOS apps
- [ ] Windows app compatibility layer
- [ ] Progressive Web Apps (PWAs)
- [ ] Android app runtime
- [ ] Wine integration
- [ ] Package repository management
- [ ] Automatic updates
- [ ] Package security scanning

## 🛠 Development Framework - RaeKit

### Core Framework
- [ ] Modern API for native development
- [ ] Rust language support
- [ ] C++ language support
- [ ] Swift language support
- [ ] TypeScript language support
- [ ] Built-in state management
- [ ] Reactive patterns
- [ ] Cross-device sync capabilities

### Development Tools
- [ ] Raeen Code IDE
- [ ] AI pair programming
- [ ] Debugging tools
- [ ] Profiling tools
- [ ] Testing framework
- [ ] Documentation generator
- [ ] Package builder
- [ ] Deployment tools

## 💻 Terminal Environment - RaeShell

### Core Terminal
- [ ] GPU-accelerated terminal
- [ ] Smooth scrolling
- [ ] Rich text support
- [ ] Inline images and graphs
- [ ] AI-powered command suggestions
- [ ] Error corrections
- [ ] Visual pipeline builder
- [ ] Command history with search

### Advanced Features
- [ ] Native SSH support
- [ ] Git integration
- [ ] Container management
- [ ] Tab support
- [ ] Split panes
- [ ] Customizable themes
- [ ] Plugin system
- [ ] Scripting support

## 📱 Pre-installed Applications

### Productivity Suite - Raeen Studio
- [ ] Raeen Write (word processor)
- [ ] Raeen Sheets (spreadsheet)
- [ ] Raeen Present (presentations)
- [ ] Raeen Code (IDE)
- [ ] Raeen Design (graphics editor)
- [ ] Raeen Video (video editor)
- [ ] Collaboration features
- [ ] Cloud sync

### System Utilities
- [ ] Raeen Files (file manager)
- [ ] Raeen Capture (screenshots/recording)
- [ ] Raeen Notes (note-taking)
- [ ] Raeen Mail (email client)
- [ ] Raeen Browser (web browser)
- [ ] System monitor
- [ ] Task manager
- [ ] Registry editor

### Entertainment
- [ ] Raeen Media (video player)
- [ ] Raeen Music (audio player)
- [ ] Raeen Photos (photo management)
- [ ] Raeen Game Hub (gaming center)
- [ ] Streaming apps integration
- [ ] Media codec support
- [ ] DRM support

## 🎛 Input and Peripherals

### Input Devices
- [ ] Keyboard support
- [ ] Mouse support
- [ ] Touchpad support
- [ ] Touchscreen support
- [ ] Stylus support
- [ ] Voice input
- [ ] Eye tracking
- [ ] Neural interface support

### Gaming Controllers
- [ ] Xbox controller support
- [ ] PlayStation controller support
- [ ] Nintendo controller support
- [ ] Generic controller support
- [ ] Custom button mapping
- [ ] Gyro and touchpad support
- [ ] Adaptive triggers
- [ ] Haptic feedback
- [ ] Macro recording

### RGB and Lighting
- [ ] RGB device detection
- [ ] Lighting synchronization
- [ ] Custom lighting profiles
- [ ] Game-based lighting
- [ ] System event lighting
- [ ] Third-party RGB software compatibility

## 🔧 Hardware Support

### CPU Support
- [ ] Intel x86_64 support
- [ ] AMD x86_64 support
- [ ] ARM64 support (future)
- [ ] Multi-core optimization
- [ ] Hyperthreading support
- [ ] CPU frequency scaling
- [ ] Thermal throttling
- [ ] Power management

### GPU Support
- [ ] NVIDIA driver support
- [ ] AMD driver support
- [ ] Intel integrated graphics
- [ ] Multi-GPU configurations
- [ ] GPU switching
- [ ] Hardware acceleration
- [ ] Compute shader support
- [ ] Ray tracing support

### Storage Support
- [ ] SATA SSD/HDD support
- [ ] NVMe SSD support
- [ ] USB storage support
- [ ] SD card support
- [ ] Network storage (NAS)
- [ ] Cloud storage integration
- [ ] RAID configurations
- [ ] Hot-swappable drives

### Network Hardware
- [ ] Ethernet adapter support
- [ ] Wi-Fi adapter support
- [ ] Bluetooth support
- [ ] Cellular modem support
- [ ] Network card drivers
- [ ] Wake-on-LAN
- [ ] Network boot (PXE)

## 🧪 Testing and Quality Assurance

### Unit Testing
- [ ] Kernel module tests
- [ ] System call tests
- [ ] Driver tests
- [ ] File system tests
- [ ] Network stack tests
- [ ] Graphics tests
- [ ] Audio tests
- [ ] Security tests

### 🔍 Kernel and syscall fuzzing [MUST v1]
- [ ] Syscall/IPC fuzzers; FS fuzz with power‑fail injection; network packet fuzz
- Acceptance: 24h continuous fuzz, 0 unreproducible crashes; all crashers minimized and filed

### 🧪 Property‑based tests for invariants [SHOULD v1]
- [ ] Proptest/QuickCheck for cap lifetimes, scheduler budgets, VFS refcounts
- Acceptance: 1000s of generated cases pass per invariant

### ⚡ Fault‑injection framework [SHOULD v1]
- [ ] Deterministic failure points in alloc/IO/IPC paths
- Acceptance: ≥ 80% error branches exercised; no panics; graceful degradation verified

### 🎬 Deterministic record‑and‑replay for UI/input [LATER]
- [ ] Input→present traces replay bit‑exact; jitter within 0.1 ms vs original
- Acceptance: input→present traces replay bit‑exact; jitter within 0.1 ms vs original

### Integration Testing
- [ ] Boot sequence tests
- [ ] Multi-process tests
- [ ] Hardware compatibility tests
- [ ] Performance benchmarks
- [ ] Stress tests
- [ ] Memory leak tests
- [ ] Security penetration tests
- [ ] Compatibility tests

### User Experience Testing
- [ ] UI/UX testing
- [ ] Accessibility testing
- [ ] Usability testing
- [ ] Performance testing
- [ ] Gaming performance tests
- [ ] Application compatibility
- [ ] Hardware compatibility
- [ ] Installation testing

## 📚 Documentation and Support

### Technical Documentation
- [ ] Kernel API documentation
- [ ] Driver development guide
- [ ] Application development guide
- [ ] System administration guide
- [ ] Security best practices
- [ ] Performance optimization guide
- [ ] Troubleshooting guide

### User Documentation
- [ ] Installation guide
- [ ] User manual
- [ ] Feature tutorials
- [ ] Gaming optimization guide
- [ ] Customization guide
- [ ] Privacy and security guide
- [ ] FAQ
- [ ] Video tutorials

### Developer Resources
- [ ] SDK documentation
- [ ] API reference
- [ ] Code examples
- [ ] Best practices
- [ ] Migration guides
- [ ] Community forums
- [ ] Bug reporting system
- [ ] Feature request system

## 🚀 Performance and Optimization

### 📏 Binary size budgets [SHOULD v1]
- [ ] Kernel and system image size constraints
- Acceptance: kernel ≤ X MB; base image ≤ Y GB; size diffs reported in CI with thresholds

### 🏗️ NUMA locality checks [SHOULD v1]
- [ ] Non-Uniform Memory Access optimization verification
- Acceptance: remote wakeups < 5%; cross‑NUMA IPC p99 penalty ≤ 1.5× local

### Boot Performance
- [ ] Sub-5 second cold boot
- [ ] Instant wake from sleep (<0.5s)
- [ ] Parallel service initialization
- [ ] Lazy loading optimization
- [ ] Boot time profiling
- [ ] Startup optimization

### Runtime Performance
- [ ] Memory usage optimization
- [ ] CPU usage optimization
- [ ] I/O performance optimization
- [ ] Network performance optimization
- [ ] Graphics performance optimization
- [ ] Power efficiency optimization
- [ ] Thermal management

### Gaming Performance
- [ ] Frame rate optimization
- [ ] Input latency minimization
- [ ] Memory allocation optimization
- [ ] GPU utilization optimization
- [ ] CPU scheduling optimization
- [ ] Storage I/O optimization
- [ ] Network latency optimization

## 🌍 Internationalization and Accessibility

### Language Support
- [ ] Multi-language UI
- [ ] Right-to-left language support
- [ ] Font rendering for all languages
- [ ] Input method support
- [ ] Locale-specific formatting
- [ ] Translation management
- [ ] Language packs

### Accessibility Features
- [ ] Screen reader support
- [ ] High contrast modes
- [ ] Dyslexia-friendly fonts
- [ ] Keyboard navigation
- [ ] Voice control
- [ ] Magnification tools
- [ ] Color blind support
- [ ] Motor impairment support

## 🔄 Update and Maintenance

### 🛡️ Recovery image + factory reset [SHOULD v1]
- [ ] Recovery boot environment and factory reset capability
- Acceptance: recovery boot < 10 s; restore to latest good A/B slot < 5 min; user data preservation options tested

### 📦 Offline update bundles [SHOULD v1]
- [ ] Detached signature verification and offline update capability
- Acceptance: detached signature verify; rollback after failure tested; no partial boots

### Update System
- [ ] Automatic update checking
- [ ] Delta updates
- [ ] Rollback capability
- [ ] Update scheduling
- [ ] Security update prioritization
- [ ] Update verification
- [ ] Staged rollouts
- [ ] Update notifications

### System Maintenance
- [ ] Automatic cleanup
- [ ] Disk space management
- [ ] Registry optimization
- [ ] Cache management
- [ ] Log rotation
- [ ] Performance monitoring
- [ ] Health checks
- [ ] Diagnostic tools

## 📊 Analytics and Telemetry

### System Analytics
- [ ] Performance metrics collection
- [ ] Crash reporting
- [ ] Usage statistics
- [ ] Hardware compatibility data
- [ ] Feature usage tracking
- [ ] Error reporting
- [ ] Privacy-compliant analytics

### User Analytics
- [ ] Opt-in telemetry
- [ ] Anonymous usage data
- [ ] Performance feedback
- [ ] Feature requests
- [ ] Bug reports
- [ ] User satisfaction surveys
- [ ] Beta testing feedback

## 🖥️ Virtualization & Containers

### 📦 OCI container service (`rae-containd`) [LATER]
- [ ] Open Container Initiative (OCI) container runtime implementation
- Acceptance: run a standard OCI image; CPU/mem/IO caps enforced; overhead ≤ 5%

### 🖥️ KVM host service (`rae-vmd`) [LATER]
- [ ] Kernel-based Virtual Machine host service implementation
- Acceptance: boot Linux guest; virtio net/blk; host p99 input latency degradation ≤ 0.3 ms during idle VM

---

## 🚀 Priority Development Roadmap

### Phase 1: Core Kernel Foundation (Highest Priority)

#### 1. Kernel Threading and Preemption
- [x] Add idle thread implementation
- [x] Implement spawn_kernel_thread function
- [x] Add demo kernel thread for testing
- [x] Verify time-sliced switching in QEMU
- [x] Add simple time-slice budget in PIT tick
- [x] Test preemptive multitasking
- [x] Validate thread context switching
- [x] Ensure proper thread cleanup

#### 2. Real Address Spaces
- [x] Allocate per-address-space PML4
- [x] Map kernel higher-half into every address space
- [x] Implement switch_address_space function
- [x] Add Cr3::write(new_pml4) with TLB considerations
- [x] Implement protect_memory function
- [x] Update page flags with proper TLB flush
- [x] Test address space isolation
- [x] Validate memory protection

#### 3. Ring3 Bring-up + Minimal Userspace
- [x] Add user code/data selectors to GDT
- [x] Build user stack allocation
- [x] Implement iretq transition to ring3
- [x] Choose syscall entry mechanism (SYSCALL/SYSRET or INT 0x80)
- [x] Wire syscall entry/exit paths
- [x] Implement minimal syscalls end-to-end:
  - [x] sys_write (serial/console output)
  - [x] sys_sleep (process suspension)
  - [x] sys_getpid (process identification)
  - [x] sys_exit (process termination)
- [x] Test ring3 execution
- [x] Validate syscall interface

#### 4. Framebuffer Compositor
- [ ] Use bootloader linear framebuffer
- [ ] Implement Framebuffer target
- [ ] Add blit path for graphics operations
- [ ] Implement double buffering
- [ ] Route keyboard input to shell in focused window
- [ ] Add basic mouse support
- [ ] Test graphics rendering
- [ ] Validate input handling

#### 5. ELF Loader
- [ ] Parse ELF headers and sections
- [ ] Load static ELF test binary into new address space
- [ ] Start ELF binary in ring3
- [ ] Validate syscalls from loaded binary
- [ ] Test binary execution
- [ ] Handle ELF loading errors

#### 6. VFS and Persistence
- [ ] Keep RAMFS as root filesystem
- [ ] Add simple read-only tar/romfs loader
- [ ] Preload user binaries and assets
- [ ] Test file system operations
- [ ] Validate data persistence

#### 7. Shell and UI Glue
- [ ] Finish shell builtins on VFS:
  - [ ] ls (list directory)
  - [ ] cd (change directory)
  - [ ] pwd (print working directory)
  - [ ] cat (display file contents)
  - [ ] touch (create file)
  - [ ] mkdir (create directory)
  - [ ] rm (remove file/directory)
- [ ] Implement draw_pixel syscall
- [ ] Implement draw_rect syscall
- [ ] Add window composition
- [ ] Test shell functionality
- [ ] Validate UI operations

#### 8. Hardening and Invariants
- [ ] Enforce W^X in memory mappings
- [ ] Add basic permission checks in syscalls
- [ ] Sanitize user pointers
- [ ] Validate syscall arguments
- [ ] Test security measures
- [ ] Audit kernel interfaces

---

## 🎯 Release Milestones

### Minimal Viable Kernel (Phase 1 Complete)
- [ ] Threading and preemption working
- [ ] Address space isolation
- [ ] Ring3 userspace execution
- [ ] Basic graphics compositor
- [ ] ELF binary loading
- [ ] Simple file system
- [ ] Functional shell
- [ ] Basic security hardening

### Alpha Release
- [ ] All Phase 1 features complete
- [ ] Network stack basics
- [ ] Audio system foundation
- [ ] Device driver framework
- [ ] Basic application support
- [ ] Developer documentation

### Beta Release
- [ ] Complete feature set
- [ ] Gaming optimizations
- [ ] AI integration
- [ ] Full hardware support
- [ ] Application ecosystem
- [ ] User documentation

### Release Candidate
- [ ] Performance optimization
- [ ] Bug fixes
- [ ] Security hardening
- [ ] Compatibility testing
- [ ] User experience polish
- [ ] Final documentation

### Production Release
- [ ] All features implemented
- [ ] Zero critical bugs
- [ ] Performance targets met
- [ ] Security audit passed
- [ ] Compatibility verified
- [ ] Documentation complete
- [ ] Support infrastructure ready
- [ ] Distribution channels prepared

---

**Progress Tracking:**
- ✅ Completed
- 🔄 In Progress
- ❌ Blocked
- ⏳ Planned

**Last Updated:** December 2024
**Version:** 1.0
**Total Items:** ~500+
**Completed:** ~25
**Remaining:** ~475
**Progress:** ~5%

**Major Milestones Completed:**
- ✅ Kernel Threading and Preemption (Phase 1.1)
- ✅ Ring3 Userspace Support (Phase 1.3)
- ✅ Basic Memory Management
- ✅ Hardware Interrupt Handling
- ✅ Timer Management
- ✅ Context Switching with Inline Assembly
- ✅ Syscall Interface (sys_write, sys_sleep, sys_getpid, sys_exit)
- ✅ Process Isolation and Sandboxing
- ✅ W^X Memory Protection

# 🔧 Checklist upgrades (add these)

## 1) Tagging + acceptance everywhere (turn vibes into gates)

* Add tags to **every** major item: **[MUST v1] / [SHOULD v1] / [LATER]** (you started this—apply it across the whole file). Each bullet should end with a one-line **Acceptance** criterion using your p99 SLO targets, reference SKUs, and app mix.
* Example pattern to paste under any section:

  * **Acceptance:** “Runs on `desk-sku-a`; emits `slo_results.json`; p99 meets gate(s).”
* Wire it to CI (two consecutive passes or ≤5% drift vs 7-day median).

## 2) Microkernel purity: move kernel “syscalls” → service IPC

* In **System Calls**, flip “Network/Graphics/AI” from kernel syscalls to **IPC contracts owned by user-space daemons** (`rae-netd`, `rae-compositord`, `rae-assistantd`). Keep the kernel ABI minimal (proc/mem/caps/IPC/time).
* Add a new **Services: IPC Contracts** block with: endpoint IDs, rights bits required, queue sizes, backpressure policy, and schema versioning. **Acceptance:** IPC ping-pong same-core p99 ≤ 3 µs; revocation ≤ 200 µs; shared-map teardown ≤ 2 ms.

## 3) Platform bring-up musts (stop fighting legacy)

* Make **UEFI+GOP**, **x2APIC/LAPIC & MSI-X**, **SMP per-CPU areas**, and **TSC-deadline timers** explicit **[MUST v1]** items under Hardware/Timers. **Acceptance:** deadline-timer jitter p99 ≤ 50 µs (interim), IRQ steering verified.

## 4) Memory hardening + JIT policy up front

* Enforce **W^X**, guard pages, **KASLR**, **SMEP/SMAP/UMIP**; document the dual-map RW→RX swap flow for future JIT (with TLB shootdown). **Acceptance:** anon fault p99 ≤ 15 µs; 64-page/16-core shootdown p99 ≤ 40 µs. Mark the whole block **[MUST v1]**.

## 5) SLO harness baked into the checklist

* Add a tiny “**SLO Tests**” subsection under **Testing & QA** listing the exact gates you expect from runs: input latency, compositor jitter @120 Hz, IPC RTT, anon fault, TLB shootdown, NVMe, idle power, chaos FS. Require **schema-conformant** `slo_results.json`.
* Keep the **reference SKUs** + **standard app mix** files as checklist artifacts (must exist for CI to load).

## 6) Graphics/compositor contract (service-side, not kernel)

* Under **Compositor**, add: **explicit sync fences**, **direct scanout for fullscreen**, and strict **backlog caps** (app→comp ≤ 2 frames; comp→scanout = 1). **Acceptance:** compositor p99 jitter ≤ 0.3 ms @ 120 Hz.

## 7) Networking reality check + targets

* Move low-level stack to `rae-netd` (user-space owns NIC queues). Add **pacing**, **QoS**, and per-queue IRQ affinity. **Acceptance:** user↔user loopback RTT via NIC queues p99 ≤ 12 µs; routing/ARP/DHCP basics pass.

## 8) Boot, attestation, and image safety

* Create a **Boot & Image** section: **Secure/Measured Boot + TPM quotes**, **A/B kernel images** with auto-rollback, and **immutable root (verity)**. Add “mitigation toggles recorded in attestation.” Mark **[SHOULD v1]**.

## 9) Package system hardening

* Under **RaeenPkg**, expand “Package verification” into: **SBOM emission**, **artifact signing/verify**, **reproducible builds**, **staged rollouts**, **rollback tests**, **trust root & key rotation**. Keep `.rae` bundles and delta updates—tie to A/B.

## 10) Compatibility scope = LATER (be ruthless)

* Mark **Windows compatibility** and **Android runtime** as **[LATER]**, with an explicit spike plan. Note in checklist that Windows API re-impl is a **monumental** effort and Android ART is Linux-tied (translation layer required). Don’t block kernel v1 on these.

## 11) Security & privacy: align with your rules

* Add a “**Syscall surface minimization**” guardrail (new syscalls require design doc + cap mapping). Enforce **no stubs**, strict **unsafe invariants**, and **minimal ISRs** directly in the checklist’s “How to ship” preamble.  
* Move **ASLR/CFI/shadow stacks** into **[MUST v1] Security Foundation** with acceptance (“boot attestation shows mitigations enabled”).

## 12) Observability & crash-only ops

* Add an **Observability** block: always-on **flight recorder**, **USDT-style probes**, and **crash-only services** with watchdogs + micro-restarts. **Acceptance:** 72 h chaos run, 0 data-loss, FS check clean, flight-recorder dump on crash. Mark **[SHOULD v1]**.

## 13) ABI governance (freeze it early)

* Create **ABI Governance [MUST v1]**: freeze syscall numbers (append-only), version **vDSO** symbols, provide **compat profiles** with expiry; CI runs both. (This also protects your RaeKit and app store story.)

## 14) Storage I/O acceptance right where you list features

* In **Storage I/O**, add **Acceptance** bullets next to NVMe/flush (your production bar): 4 KiB QD1 read p99 ≤ 120 µs (hot set), flush p99 ≤ 900 µs.

## 15) Power + wake targets

* Under **Hardware/Power**, add: **idle power** budget on laptop SKU and **predictable wake** for RT classes. Tie to your CI gates via the standard app mix. (Acceptance goes to SLO harness so it’s machine-checked.)

## 16) Gaming sections: scope + measurable wins

* Keep **RaeenGame Mode** features, but give it hard gates: “input-to-present budget ≤ X ms in game scene; 1% low fps uplift ≥ Y% vs baseline.” Mark storefront integrations (**Steam/Epic**, etc.) **[LATER]**; ship core OS first. 

## 17) Accessibility + intl = testable items

* Where you list accessibility/theming, add acceptance like “screen reader end-to-end flows pass,” “contrast AA/AAA presets validated,” “IME support for CJK smoke tests.” (Matches your UX polish pitch.)

## 18) DevEx & docs as gates

* Add a short **“Ship discipline”** block at the top of the checklist: forbid renumbering syscalls, no massive refactors without migration notes, enforce `-D warnings`, forbid `unwrap/expect` in kernel. (Mirror your rules so PRs can’t skip them.) 

---

# 🔁 Concrete edits (paste-ready into your checklist)

* **System Calls → Services**
  Change:

  > “Network/Graphics/AI syscalls”
  > to
  > “**Network/Graphics/AI IPC contracts** (owned by `rae-netd`/`rae-compositord`/`rae-assistantd`; kernel exposes only IPC + caps). **Acceptance:** IPC RTT same-core p99 ≤ 3 µs.” 

* **Hardware/Timers**
  Add bullets: “UEFI+GOP, x2APIC/LAPIC & MSI-X, SMP per-CPU, TSC-deadline (tickless groundwork). **Acceptance:** jitter p99 ≤ 50 µs.”

* **Storage I/O**
  Append **Acceptance**: “NVMe 4 KiB @ QD=1 p99 ≤ 120 µs (hot); flush p99 ≤ 900 µs.”

* **Graphics/Compositor**
  Add: “Explicit fences; **backlog caps** (app→comp ≤ 2; comp→scanout = 1); **Acceptance:** jitter p99 ≤ 0.3 ms @ 120 Hz.”

* **RaeenPkg**
  Replace “Package verification” with: “signing + verify, SBOM, reproducible builds, staged rollouts, rollback drills, trust-root + key rotation.”

* **Compatibility layers**
  Tag **[LATER]** and add note: “Huge scope—Windows API re-impl; Android ART needs translation layer.”

---

# 🧭 Why this makes the list hit different

* It enforces **microkernel boundaries** and **minimizes kernel surface**, exactly what your concept promises.
* It converts “nice ideas” into **measurable, CI-gated outcomes** (SLOs), not vibes.
* It balances ambition (Windows/Android, wild UI polish) with **phase discipline** so v1 ships.