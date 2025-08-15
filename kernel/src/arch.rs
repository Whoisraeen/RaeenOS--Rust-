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

/// CPU vendor identification
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CpuVendor {
    Intel,
    Amd,
    Unknown,
}

/// CPU feature enumeration for type-safe feature checking
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CpuFeature {
    Fpu,
    Vme,
    De,
    Pse,
    Tsc,
    Msr,
    Pae,
    Mce,
    Cx8,
    Apic,
    Sep,
    Mtrr,
    Pge,
    Mca,
    Cmov,
    Pat,
    Pse36,
    Psn,
    Clfsh,
    Ds,
    Acpi,
    Mmx,
    Fxsr,
    Sse,
    Sse2,
    Ss,
    Htt,
    Tm,
    Ia64,
    Pbe,
    // ECX features
    Sse3,
    Pclmulqdq,
    Dtes64,
    Monitor,
    DsCpl,
    Vmx,
    Smx,
    Est,
    Tm2,
    Ssse3,
    CnxtId,
    Sdbg,
    Fma,
    Cx16,
    Xtpr,
    Pdcm,
    Pcid,
    Dca,
    Sse4_1,
    Sse4_2,
    X2apic,
    Movbe,
    Popcnt,
    TscDeadline,
    Aes,
    Xsave,
    Osxsave,
    Avx,
    F16c,
    Rdrand,
    Hypervisor,
    // Extended features (EBX)
    Fsgsbase,
    TscAdjust,
    Sgx,
    Bmi1,
    Hle,
    Avx2,
    FdpExcptnOnly,
    Smep,
    Bmi2,
    Erms,
    Invpcid,
    Rtm,
    Pqm,
    Fpcsds,
    Mpx,
    Pqe,
    Avx512f,
    Avx512dq,
    Rdseed,
    Adx,
    Smap,
    Avx512ifma,
    Pcommit,
    Clflushopt,
    Clwb,
    IntelPt,
    Avx512pf,
    Avx512er,
    Avx512cd,
    Sha,
    Avx512bw,
    Avx512vl,
}

/// CPU feature flags
#[derive(Debug, Clone, Copy)]
pub struct CpuFeatures {
    pub fpu: bool,
    pub vme: bool,
    pub de: bool,
    pub pse: bool,
    pub tsc: bool,
    pub msr: bool,
    pub pae: bool,
    pub mce: bool,
    pub cx8: bool,
    pub apic: bool,
    pub sep: bool,
    pub mtrr: bool,
    pub pge: bool,
    pub mca: bool,
    pub cmov: bool,
    pub pat: bool,
    pub pse36: bool,
    pub psn: bool,
    pub clfsh: bool,
    pub ds: bool,
    pub acpi: bool,
    pub mmx: bool,
    pub fxsr: bool,
    pub sse: bool,
    pub sse2: bool,
    pub ss: bool,
    pub htt: bool,
    pub tm: bool,
    pub ia64: bool,
    pub pbe: bool,
    // Extended features (ECX)
    pub sse3: bool,
    pub pclmulqdq: bool,
    pub dtes64: bool,
    pub monitor: bool,
    pub ds_cpl: bool,
    pub vmx: bool,
    pub smx: bool,
    pub est: bool,
    pub tm2: bool,
    pub ssse3: bool,
    pub cnxt_id: bool,
    pub sdbg: bool,
    pub fma: bool,
    pub cx16: bool,
    pub xtpr: bool,
    pub pdcm: bool,
    pub pcid: bool,
    pub dca: bool,
    pub sse4_1: bool,
    pub sse4_2: bool,
    pub x2apic: bool,
    pub movbe: bool,
    pub popcnt: bool,
    pub tsc_deadline: bool,
    pub aes: bool,
    pub xsave: bool,
    pub osxsave: bool,
    pub avx: bool,
    pub f16c: bool,
    pub rdrand: bool,
    pub hypervisor: bool,
    // Extended features (leaf 7)
    pub fsgsbase: bool,
    pub tsc_adjust: bool,
    pub sgx: bool,
    pub bmi1: bool,
    pub hle: bool,
    pub avx2: bool,
    pub fdp_excptn_only: bool,
    pub smep: bool,
    pub bmi2: bool,
    pub erms: bool,
    pub invpcid: bool,
    pub rtm: bool,
    pub pqm: bool,
    pub fpcsds: bool,
    pub mpx: bool,
    pub pqe: bool,
    pub avx512f: bool,
    pub avx512dq: bool,
    pub rdseed: bool,
    pub adx: bool,
    pub smap: bool,
    pub avx512ifma: bool,
    pub pcommit: bool,
    pub clflushopt: bool,
    pub clwb: bool,
    pub intel_pt: bool,
    pub avx512pf: bool,
    pub avx512er: bool,
    pub avx512cd: bool,
    pub sha: bool,
    pub avx512bw: bool,
    pub avx512vl: bool,
}

/// CPU information structure
#[derive(Debug, Clone)]
pub struct CpuInfo {
    pub vendor: CpuVendor,
    pub vendor_string: [u8; 12],
    pub brand_string: [u8; 48],
    pub family: u32,
    pub model: u32,
    pub stepping: u32,
    pub features: CpuFeatures,
    pub logical_cores: u32,
    pub physical_cores: u32,
    pub cache_line_size: u32,
    pub max_cpuid_leaf: u32,
    pub max_extended_leaf: u32,
}

/// Execute CPUID instruction safely
fn cpuid(leaf: u32, subleaf: u32) -> (u32, u32, u32, u32) {
    let mut eax: u32;
    let mut ebx: u32;
    let mut ecx: u32;
    let mut edx: u32;
    
    unsafe {
        core::arch::asm!(
            "push rbx",
            "cpuid",
            "mov {ebx_out:e}, ebx",
            "pop rbx",
            inout("eax") leaf => eax,
            ebx_out = out(reg) ebx,
            inout("ecx") subleaf => ecx,
            out("edx") edx,
        );
    }
    
    (eax, ebx, ecx, edx)
}

/// Detect CPU vendor from CPUID leaf 0
fn detect_cpu_vendor() -> (CpuVendor, [u8; 12], u32) {
    let (max_leaf, ebx, ecx, edx) = cpuid(0, 0);
    
    let mut vendor_string = [0u8; 12];
    vendor_string[0..4].copy_from_slice(&ebx.to_le_bytes());
    vendor_string[4..8].copy_from_slice(&edx.to_le_bytes());
    vendor_string[8..12].copy_from_slice(&ecx.to_le_bytes());
    
    let vendor = match &vendor_string {
        b"GenuineIntel" => CpuVendor::Intel,
        b"AuthenticAMD" => CpuVendor::Amd,
        _ => CpuVendor::Unknown,
    };
    
    (vendor, vendor_string, max_leaf)
}

/// Get CPU brand string from extended CPUID leaves
fn get_brand_string() -> [u8; 48] {
    let mut brand = [0u8; 48];
    
    // Check if extended leaves are available
    let (max_extended, _, _, _) = cpuid(0x80000000, 0);
    if max_extended >= 0x80000004 {
        let (eax1, ebx1, ecx1, edx1) = cpuid(0x80000002, 0);
        let (eax2, ebx2, ecx2, edx2) = cpuid(0x80000003, 0);
        let (eax3, ebx3, ecx3, edx3) = cpuid(0x80000004, 0);
        
        brand[0..4].copy_from_slice(&eax1.to_le_bytes());
        brand[4..8].copy_from_slice(&ebx1.to_le_bytes());
        brand[8..12].copy_from_slice(&ecx1.to_le_bytes());
        brand[12..16].copy_from_slice(&edx1.to_le_bytes());
        brand[16..20].copy_from_slice(&eax2.to_le_bytes());
        brand[20..24].copy_from_slice(&ebx2.to_le_bytes());
        brand[24..28].copy_from_slice(&ecx2.to_le_bytes());
        brand[28..32].copy_from_slice(&edx2.to_le_bytes());
        brand[32..36].copy_from_slice(&eax3.to_le_bytes());
        brand[36..40].copy_from_slice(&ebx3.to_le_bytes());
        brand[40..44].copy_from_slice(&ecx3.to_le_bytes());
        brand[44..48].copy_from_slice(&edx3.to_le_bytes());
    }
    
    brand
}

/// Parse CPU features from CPUID leaf 1
fn parse_cpu_features() -> CpuFeatures {
    let (eax, ebx, ecx, edx) = cpuid(1, 0);
    
    // Get extended features from leaf 7
    let (_, ext_ebx, ext_ecx, _) = cpuid(7, 0);
    
    CpuFeatures {
        // EDX features (leaf 1)
        fpu: (edx & (1 << 0)) != 0,
        vme: (edx & (1 << 1)) != 0,
        de: (edx & (1 << 2)) != 0,
        pse: (edx & (1 << 3)) != 0,
        tsc: (edx & (1 << 4)) != 0,
        msr: (edx & (1 << 5)) != 0,
        pae: (edx & (1 << 6)) != 0,
        mce: (edx & (1 << 7)) != 0,
        cx8: (edx & (1 << 8)) != 0,
        apic: (edx & (1 << 9)) != 0,
        sep: (edx & (1 << 11)) != 0,
        mtrr: (edx & (1 << 12)) != 0,
        pge: (edx & (1 << 13)) != 0,
        mca: (edx & (1 << 14)) != 0,
        cmov: (edx & (1 << 15)) != 0,
        pat: (edx & (1 << 16)) != 0,
        pse36: (edx & (1 << 17)) != 0,
        psn: (edx & (1 << 18)) != 0,
        clfsh: (edx & (1 << 19)) != 0,
        ds: (edx & (1 << 21)) != 0,
        acpi: (edx & (1 << 22)) != 0,
        mmx: (edx & (1 << 23)) != 0,
        fxsr: (edx & (1 << 24)) != 0,
        sse: (edx & (1 << 25)) != 0,
        sse2: (edx & (1 << 26)) != 0,
        ss: (edx & (1 << 27)) != 0,
        htt: (edx & (1 << 28)) != 0,
        tm: (edx & (1 << 29)) != 0,
        ia64: (edx & (1 << 30)) != 0,
        pbe: (edx & (1 << 31)) != 0,
        
        // ECX features (leaf 1)
        sse3: (ecx & (1 << 0)) != 0,
        pclmulqdq: (ecx & (1 << 1)) != 0,
        dtes64: (ecx & (1 << 2)) != 0,
        monitor: (ecx & (1 << 3)) != 0,
        ds_cpl: (ecx & (1 << 4)) != 0,
        vmx: (ecx & (1 << 5)) != 0,
        smx: (ecx & (1 << 6)) != 0,
        est: (ecx & (1 << 7)) != 0,
        tm2: (ecx & (1 << 8)) != 0,
        ssse3: (ecx & (1 << 9)) != 0,
        cnxt_id: (ecx & (1 << 10)) != 0,
        sdbg: (ecx & (1 << 11)) != 0,
        fma: (ecx & (1 << 12)) != 0,
        cx16: (ecx & (1 << 13)) != 0,
        xtpr: (ecx & (1 << 14)) != 0,
        pdcm: (ecx & (1 << 15)) != 0,
        pcid: (ecx & (1 << 17)) != 0,
        dca: (ecx & (1 << 18)) != 0,
        sse4_1: (ecx & (1 << 19)) != 0,
        sse4_2: (ecx & (1 << 20)) != 0,
        x2apic: (ecx & (1 << 21)) != 0,
        movbe: (ecx & (1 << 22)) != 0,
        popcnt: (ecx & (1 << 23)) != 0,
        tsc_deadline: (ecx & (1 << 24)) != 0,
        aes: (ecx & (1 << 25)) != 0,
        xsave: (ecx & (1 << 26)) != 0,
        osxsave: (ecx & (1 << 27)) != 0,
        avx: (ecx & (1 << 28)) != 0,
        f16c: (ecx & (1 << 29)) != 0,
        rdrand: (ecx & (1 << 30)) != 0,
        hypervisor: (ecx & (1 << 31)) != 0,
        
        // Extended features (leaf 7, EBX)
        fsgsbase: (ext_ebx & (1 << 0)) != 0,
        tsc_adjust: (ext_ebx & (1 << 1)) != 0,
        sgx: (ext_ebx & (1 << 2)) != 0,
        bmi1: (ext_ebx & (1 << 3)) != 0,
        hle: (ext_ebx & (1 << 4)) != 0,
        avx2: (ext_ebx & (1 << 5)) != 0,
        fdp_excptn_only: (ext_ebx & (1 << 6)) != 0,
        smep: (ext_ebx & (1 << 7)) != 0,
        bmi2: (ext_ebx & (1 << 8)) != 0,
        erms: (ext_ebx & (1 << 9)) != 0,
        invpcid: (ext_ebx & (1 << 10)) != 0,
        rtm: (ext_ebx & (1 << 11)) != 0,
        pqm: (ext_ebx & (1 << 12)) != 0,
        fpcsds: (ext_ebx & (1 << 13)) != 0,
        mpx: (ext_ebx & (1 << 14)) != 0,
        pqe: (ext_ebx & (1 << 15)) != 0,
        avx512f: (ext_ebx & (1 << 16)) != 0,
        avx512dq: (ext_ebx & (1 << 17)) != 0,
        rdseed: (ext_ebx & (1 << 18)) != 0,
        adx: (ext_ebx & (1 << 19)) != 0,
        smap: (ext_ebx & (1 << 20)) != 0,
        avx512ifma: (ext_ebx & (1 << 21)) != 0,
        pcommit: (ext_ebx & (1 << 22)) != 0,
        clflushopt: (ext_ebx & (1 << 23)) != 0,
        clwb: (ext_ebx & (1 << 24)) != 0,
        intel_pt: (ext_ebx & (1 << 25)) != 0,
        avx512pf: (ext_ebx & (1 << 26)) != 0,
        avx512er: (ext_ebx & (1 << 27)) != 0,
        avx512cd: (ext_ebx & (1 << 28)) != 0,
        sha: (ext_ebx & (1 << 29)) != 0,
        avx512bw: (ext_ebx & (1 << 30)) != 0,
        avx512vl: (ext_ebx & (1 << 31)) != 0,
    }
}

/// Detect CPU topology (cores, threads)
fn detect_cpu_topology() -> (u32, u32) {
    let (_, ebx, _, _) = cpuid(1, 0);
    let logical_cores = (ebx >> 16) & 0xFF;
    
    // Try to get physical core count from leaf 4 (Intel) or leaf 0x8000001E (AMD)
    let (vendor, _, _) = detect_cpu_vendor();
    let physical_cores = match vendor {
        CpuVendor::Intel => {
            // Intel: Use leaf 4 to count physical cores
            let mut cores = 0;
            for i in 0..32 {
                let (eax, _, _, _) = cpuid(4, i);
                if (eax & 0x1F) == 0 {
                    break;
                }
                cores += 1;
            }
            if cores > 0 { cores } else { logical_cores }
        },
        CpuVendor::Amd => {
            // AMD: Try extended leaf 0x8000001E
            let (max_extended, _, _, _) = cpuid(0x80000000, 0);
            if max_extended >= 0x8000001E {
                let (_, ebx, _, _) = cpuid(0x8000001E, 0);
                ((ebx >> 8) & 0xFF) + 1
            } else {
                logical_cores
            }
        },
        _ => logical_cores,
    };
    
    (logical_cores, physical_cores)
}

/// Comprehensive CPU feature detection
pub fn detect_cpu_info() -> CpuInfo {
    use x86_64::instructions::interrupts;
    
    interrupts::without_interrupts(|| {
        let (vendor, vendor_string, max_leaf) = detect_cpu_vendor();
        let brand_string = get_brand_string();
        let features = parse_cpu_features();
        let (logical_cores, physical_cores) = detect_cpu_topology();
        
        // Get CPU family, model, stepping from leaf 1
        let (eax, ebx, _, _) = cpuid(1, 0);
        let stepping = eax & 0xF;
        let model = (eax >> 4) & 0xF;
        let family = (eax >> 8) & 0xF;
        let extended_model = (eax >> 16) & 0xF;
        let extended_family = (eax >> 20) & 0xFF;
        
        // Calculate actual family and model
        let actual_family = if family == 0xF {
            family + extended_family
        } else {
            family
        };
        
        let actual_model = if family == 0x6 || family == 0xF {
            (extended_model << 4) | model
        } else {
            model
        };
        
        // Get cache line size from EBX[15:8]
        let cache_line_size = (ebx >> 8) & 0xFF;
        
        // Get maximum extended leaf
        let (max_extended_leaf, _, _, _) = cpuid(0x80000000, 0);
        
        CpuInfo {
            vendor,
            vendor_string,
            brand_string,
            family: actual_family,
            model: actual_model,
            stepping,
            features,
            logical_cores,
            physical_cores,
            cache_line_size,
            max_cpuid_leaf: max_leaf,
            max_extended_leaf,
        }
    })
}

/// Get CPU core count (legacy function for compatibility)
pub fn get_cpu_count() -> u32 {
    detect_cpu_info().logical_cores
}

/// Get the current CPU ID (simplified implementation)
/// In a real SMP system, this would read from APIC ID or similar
pub fn get_current_cpu_id() -> u32 {
    // For now, return 0 (single CPU)
    // TODO: Implement proper APIC ID reading for multi-core systems
    0
}

/// Check if a specific CPU feature is supported (type-safe version)
pub fn has_cpu_feature(feature: CpuFeature) -> bool {
    let info = detect_cpu_info();
    match feature {
        CpuFeature::Fpu => info.features.fpu,
        CpuFeature::Vme => info.features.vme,
        CpuFeature::De => info.features.de,
        CpuFeature::Pse => info.features.pse,
        CpuFeature::Tsc => info.features.tsc,
        CpuFeature::Msr => info.features.msr,
        CpuFeature::Pae => info.features.pae,
        CpuFeature::Mce => info.features.mce,
        CpuFeature::Cx8 => info.features.cx8,
        CpuFeature::Apic => info.features.apic,
        CpuFeature::Sep => info.features.sep,
        CpuFeature::Mtrr => info.features.mtrr,
        CpuFeature::Pge => info.features.pge,
        CpuFeature::Mca => info.features.mca,
        CpuFeature::Cmov => info.features.cmov,
        CpuFeature::Pat => info.features.pat,
        CpuFeature::Pse36 => info.features.pse36,
        CpuFeature::Psn => info.features.psn,
        CpuFeature::Clfsh => info.features.clfsh,
        CpuFeature::Ds => info.features.ds,
        CpuFeature::Acpi => info.features.acpi,
        CpuFeature::Mmx => info.features.mmx,
        CpuFeature::Fxsr => info.features.fxsr,
        CpuFeature::Sse => info.features.sse,
        CpuFeature::Sse2 => info.features.sse2,
        CpuFeature::Ss => info.features.ss,
        CpuFeature::Htt => info.features.htt,
        CpuFeature::Tm => info.features.tm,
        CpuFeature::Ia64 => info.features.ia64,
        CpuFeature::Pbe => info.features.pbe,
        CpuFeature::Sse3 => info.features.sse3,
        CpuFeature::Pclmulqdq => info.features.pclmulqdq,
        CpuFeature::Dtes64 => info.features.dtes64,
        CpuFeature::Monitor => info.features.monitor,
        CpuFeature::DsCpl => info.features.ds_cpl,
        CpuFeature::Vmx => info.features.vmx,
        CpuFeature::Smx => info.features.smx,
        CpuFeature::Est => info.features.est,
        CpuFeature::Tm2 => info.features.tm2,
        CpuFeature::Ssse3 => info.features.ssse3,
        CpuFeature::CnxtId => info.features.cnxt_id,
        CpuFeature::Sdbg => info.features.sdbg,
        CpuFeature::Fma => info.features.fma,
        CpuFeature::Cx16 => info.features.cx16,
        CpuFeature::Xtpr => info.features.xtpr,
        CpuFeature::Pdcm => info.features.pdcm,
        CpuFeature::Pcid => info.features.pcid,
        CpuFeature::Dca => info.features.dca,
        CpuFeature::Sse4_1 => info.features.sse4_1,
        CpuFeature::Sse4_2 => info.features.sse4_2,
        CpuFeature::X2apic => info.features.x2apic,
        CpuFeature::Movbe => info.features.movbe,
        CpuFeature::Popcnt => info.features.popcnt,
        CpuFeature::TscDeadline => info.features.tsc_deadline,
        CpuFeature::Aes => info.features.aes,
        CpuFeature::Xsave => info.features.xsave,
        CpuFeature::Osxsave => info.features.osxsave,
        CpuFeature::Avx => info.features.avx,
        CpuFeature::F16c => info.features.f16c,
        CpuFeature::Rdrand => info.features.rdrand,
        CpuFeature::Hypervisor => info.features.hypervisor,
        CpuFeature::Fsgsbase => info.features.fsgsbase,
        CpuFeature::TscAdjust => info.features.tsc_adjust,
        CpuFeature::Sgx => info.features.sgx,
        CpuFeature::Bmi1 => info.features.bmi1,
        CpuFeature::Hle => info.features.hle,
        CpuFeature::Avx2 => info.features.avx2,
        CpuFeature::FdpExcptnOnly => info.features.fdp_excptn_only,
        CpuFeature::Smep => info.features.smep,
        CpuFeature::Bmi2 => info.features.bmi2,
        CpuFeature::Erms => info.features.erms,
        CpuFeature::Invpcid => info.features.invpcid,
        CpuFeature::Rtm => info.features.rtm,
        CpuFeature::Pqm => info.features.pqm,
        CpuFeature::Fpcsds => info.features.fpcsds,
        CpuFeature::Mpx => info.features.mpx,
        CpuFeature::Pqe => info.features.pqe,
        CpuFeature::Avx512f => info.features.avx512f,
        CpuFeature::Avx512dq => info.features.avx512dq,
        CpuFeature::Rdseed => info.features.rdseed,
        CpuFeature::Adx => info.features.adx,
        CpuFeature::Smap => info.features.smap,
        CpuFeature::Avx512ifma => info.features.avx512ifma,
        CpuFeature::Pcommit => info.features.pcommit,
        CpuFeature::Clflushopt => info.features.clflushopt,
        CpuFeature::Clwb => info.features.clwb,
        CpuFeature::IntelPt => info.features.intel_pt,
        CpuFeature::Avx512pf => info.features.avx512pf,
        CpuFeature::Avx512er => info.features.avx512er,
        CpuFeature::Avx512cd => info.features.avx512cd,
        CpuFeature::Sha => info.features.sha,
        CpuFeature::Avx512bw => info.features.avx512bw,
        CpuFeature::Avx512vl => info.features.avx512vl,
    }
}

/// Check if a specific CPU feature is supported (string-based version for compatibility)
pub fn has_cpu_feature_str(feature_name: &str) -> bool {
    let info = detect_cpu_info();
    match feature_name {
        "sse" => info.features.sse,
        "sse2" => info.features.sse2,
        "sse3" => info.features.sse3,
        "sse4.1" => info.features.sse4_1,
        "sse4.2" => info.features.sse4_2,
        "avx" => info.features.avx,
        "avx2" => info.features.avx2,
        "avx512f" => info.features.avx512f,
        "aes" => info.features.aes,
        "rdrand" => info.features.rdrand,
        "rdseed" => info.features.rdseed,
        "vmx" => info.features.vmx,
        "smx" => info.features.smx,
        "smep" => info.features.smep,
        "smap" => info.features.smap,
        "tsc" => info.features.tsc,
        "apic" => info.features.apic,
        "x2apic" => info.features.x2apic,
        "hypervisor" => info.features.hypervisor,
        _ => false,
    }
}

/// Get CPU vendor as string
pub fn get_cpu_vendor_string() -> &'static str {
    match detect_cpu_info().vendor {
        CpuVendor::Intel => "Intel",
        CpuVendor::Amd => "AMD",
        CpuVendor::Unknown => "Unknown",
    }
}

