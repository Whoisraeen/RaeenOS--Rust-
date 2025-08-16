//! UEFI (Unified Extensible Firmware Interface) support for RaeenOS
//! Handles UEFI boot services, GOP initialization, and memory map parsing

use x86_64::PhysAddr;
use spin::Mutex;
use lazy_static::lazy_static;
use alloc::vec::Vec;
use core::slice;

/// UEFI System Table signature
const EFI_SYSTEM_TABLE_SIGNATURE: u64 = 0x5453595320494645; // "EFI SYST"

/// UEFI Boot Services signature
#[allow(dead_code)]
const EFI_BOOT_SERVICES_SIGNATURE: u64 = 0x56524553544f4f42; // "BOOTSERV"

/// UEFI Runtime Services signature
#[allow(dead_code)]
const EFI_RUNTIME_SERVICES_SIGNATURE: u64 = 0x56524553544e5552; // "RUNTSERV"

/// EFI Status codes
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(usize)]
pub enum EfiStatus {
    Success = 0,
    LoadError = 1,
    InvalidParameter = 2,
    Unsupported = 3,
    BadBufferSize = 4,
    BufferTooSmall = 5,
    NotReady = 6,
    DeviceError = 7,
    WriteProtected = 8,
    OutOfResources = 9,
    VolumeCorrupted = 10,
    VolumeFull = 11,
    NoMedia = 12,
    MediaChanged = 13,
    NotFound = 14,
    AccessDenied = 15,
    NoResponse = 16,
    NoMapping = 17,
    Timeout = 18,
    NotStarted = 19,
    AlreadyStarted = 20,
    Aborted = 21,
    IcmpError = 22,
    TftpError = 23,
    ProtocolError = 24,
    IncompatibleVersion = 25,
    SecurityViolation = 26,
    CrcError = 27,
    EndOfMedia = 28,
    EndOfFile = 31,
    InvalidLanguage = 32,
    CompromisedData = 33,
    IpAddressConflict = 34,
    HttpError = 35,
}

/// EFI Memory Types
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
pub enum EfiMemoryType {
    ReservedMemoryType = 0,
    LoaderCode = 1,
    LoaderData = 2,
    BootServicesCode = 3,
    BootServicesData = 4,
    RuntimeServicesCode = 5,
    RuntimeServicesData = 6,
    ConventionalMemory = 7,
    UnusableMemory = 8,
    AcpiReclaimMemory = 9,
    AcpiMemoryNvs = 10,
    MemoryMappedIo = 11,
    MemoryMappedIoPortSpace = 12,
    PalCode = 13,
    PersistentMemory = 14,
    MaxMemoryType = 15,
}

/// EFI Memory Descriptor
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EfiMemoryDescriptor {
    pub memory_type: EfiMemoryType,
    pub physical_start: u64,
    pub virtual_start: u64,
    pub number_of_pages: u64,
    pub attribute: u64,
}

/// Graphics Output Protocol (GOP) pixel format
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
pub enum GopPixelFormat {
    PixelRedGreenBlueReserved8BitPerColor = 0,
    PixelBlueGreenRedReserved8BitPerColor = 1,
    PixelBitMask = 2,
    PixelBltOnly = 3,
    PixelFormatMax = 4,
}

/// GOP Pixel Bitmask
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GopPixelBitmask {
    pub red_mask: u32,
    pub green_mask: u32,
    pub blue_mask: u32,
    pub reserved_mask: u32,
}

/// GOP Mode Information
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct GopModeInfo {
    pub version: u32,
    pub horizontal_resolution: u32,
    pub vertical_resolution: u32,
    pub pixel_format: GopPixelFormat,
    pub pixel_information: GopPixelBitmask,
    pub pixels_per_scan_line: u32,
}

/// GOP Mode
#[derive(Debug)]
#[repr(C)]
pub struct GopMode {
    pub max_mode: u32,
    pub mode: u32,
    pub info: *const GopModeInfo,
    pub size_of_info: usize,
    pub frame_buffer_base: u64,
    pub frame_buffer_size: usize,
}

/// Graphics Output Protocol
#[derive(Debug)]
#[repr(C)]
pub struct GraphicsOutputProtocol {
    pub query_mode: extern "efiapi" fn(
        this: *const GraphicsOutputProtocol,
        mode_number: u32,
        size_of_info: *mut usize,
        info: *mut *const GopModeInfo,
    ) -> EfiStatus,
    pub set_mode: extern "efiapi" fn(
        this: *const GraphicsOutputProtocol,
        mode_number: u32,
    ) -> EfiStatus,
    pub blt: extern "efiapi" fn(
        this: *const GraphicsOutputProtocol,
        blt_buffer: *const u32,
        blt_operation: u32,
        source_x: usize,
        source_y: usize,
        dest_x: usize,
        dest_y: usize,
        width: usize,
        height: usize,
        delta: usize,
    ) -> EfiStatus,
    pub mode: *const GopMode,
}

/// UEFI Boot Services
#[derive(Debug)]
#[repr(C)]
pub struct EfiBootServices {
    pub hdr: EfiTableHeader,
    
    // Task Priority Services
    pub raise_tpl: extern "efiapi" fn(new_tpl: usize) -> usize,
    pub restore_tpl: extern "efiapi" fn(old_tpl: usize),
    
    // Memory Services
    pub allocate_pages: extern "efiapi" fn(
        alloc_type: u32,
        memory_type: EfiMemoryType,
        pages: usize,
        memory: *mut u64,
    ) -> EfiStatus,
    pub free_pages: extern "efiapi" fn(memory: u64, pages: usize) -> EfiStatus,
    pub get_memory_map: extern "efiapi" fn(
        memory_map_size: *mut usize,
        memory_map: *mut EfiMemoryDescriptor,
        map_key: *mut usize,
        descriptor_size: *mut usize,
        descriptor_version: *mut u32,
    ) -> EfiStatus,
    pub allocate_pool: extern "efiapi" fn(
        pool_type: EfiMemoryType,
        size: usize,
        buffer: *mut *mut u8,
    ) -> EfiStatus,
    pub free_pool: extern "efiapi" fn(buffer: *mut u8) -> EfiStatus,
    
    // Event & Timer Services
    pub create_event: extern "efiapi" fn(
        event_type: u32,
        notify_tpl: usize,
        notify_function: Option<extern "efiapi" fn()>,
        notify_context: *const u8,
        event: *mut *const u8,
    ) -> EfiStatus,
    pub set_timer: extern "efiapi" fn(
        event: *const u8,
        timer_type: u32,
        trigger_time: u64,
    ) -> EfiStatus,
    pub wait_for_event: extern "efiapi" fn(
        number_of_events: usize,
        event: *const *const u8,
        index: *mut usize,
    ) -> EfiStatus,
    pub signal_event: extern "efiapi" fn(event: *const u8) -> EfiStatus,
    pub close_event: extern "efiapi" fn(event: *const u8) -> EfiStatus,
    pub check_event: extern "efiapi" fn(event: *const u8) -> EfiStatus,
    
    // Protocol Handler Services
    pub install_protocol_interface: extern "efiapi" fn(
        handle: *mut *const u8,
        protocol: *const [u8; 16], // EFI_GUID
        interface_type: u32,
        interface: *const u8,
    ) -> EfiStatus,
    pub reinstall_protocol_interface: extern "efiapi" fn(
        handle: *const u8,
        protocol: *const [u8; 16],
        old_interface: *const u8,
        new_interface: *const u8,
    ) -> EfiStatus,
    pub uninstall_protocol_interface: extern "efiapi" fn(
        handle: *const u8,
        protocol: *const [u8; 16],
        interface: *const u8,
    ) -> EfiStatus,
    pub handle_protocol: extern "efiapi" fn(
        handle: *const u8,
        protocol: *const [u8; 16],
        interface: *mut *const u8,
    ) -> EfiStatus,
    pub reserved: *const u8,
    pub register_protocol_notify: extern "efiapi" fn(
        protocol: *const [u8; 16],
        event: *const u8,
        registration: *mut *const u8,
    ) -> EfiStatus,
    pub locate_handle: extern "efiapi" fn(
        search_type: u32,
        protocol: *const [u8; 16],
        search_key: *const u8,
        buffer_size: *mut usize,
        buffer: *mut *const u8,
    ) -> EfiStatus,
    pub locate_device_path: extern "efiapi" fn(
        protocol: *const [u8; 16],
        device_path: *mut *const u8,
        device: *mut *const u8,
    ) -> EfiStatus,
    pub install_configuration_table: extern "efiapi" fn(
        guid: *const [u8; 16],
        table: *const u8,
    ) -> EfiStatus,
    
    // Image Services
    pub load_image: extern "efiapi" fn(
        boot_policy: bool,
        parent_image_handle: *const u8,
        device_path: *const u8,
        source_buffer: *const u8,
        source_size: usize,
        image_handle: *mut *const u8,
    ) -> EfiStatus,
    pub start_image: extern "efiapi" fn(
        image_handle: *const u8,
        exit_data_size: *mut usize,
        exit_data: *mut *const u16,
    ) -> EfiStatus,
    pub exit: extern "efiapi" fn(
        image_handle: *const u8,
        exit_status: EfiStatus,
        exit_data_size: usize,
        exit_data: *const u16,
    ) -> !,
    pub unload_image: extern "efiapi" fn(image_handle: *const u8) -> EfiStatus,
    pub exit_boot_services: extern "efiapi" fn(
        image_handle: *const u8,
        map_key: usize,
    ) -> EfiStatus,
    
    // Miscellaneous Services
    pub get_next_monotonic_count: extern "efiapi" fn(count: *mut u64) -> EfiStatus,
    pub stall: extern "efiapi" fn(microseconds: usize) -> EfiStatus,
    pub set_watchdog_timer: extern "efiapi" fn(
        timeout: usize,
        watchdog_code: u64,
        data_size: usize,
        watchdog_data: *const u16,
    ) -> EfiStatus,
    
    // DriverSupport Services
    pub connect_controller: extern "efiapi" fn(
        controller_handle: *const u8,
        driver_image_handle: *const *const u8,
        remaining_device_path: *const u8,
        recursive: bool,
    ) -> EfiStatus,
    pub disconnect_controller: extern "efiapi" fn(
        controller_handle: *const u8,
        driver_image_handle: *const u8,
        child_handle: *const u8,
    ) -> EfiStatus,
    
    // Open and Close Protocol Services
    pub open_protocol: extern "efiapi" fn(
        handle: *const u8,
        protocol: *const [u8; 16],
        interface: *mut *const u8,
        agent_handle: *const u8,
        controller_handle: *const u8,
        attributes: u32,
    ) -> EfiStatus,
    pub close_protocol: extern "efiapi" fn(
        handle: *const u8,
        protocol: *const [u8; 16],
        agent_handle: *const u8,
        controller_handle: *const u8,
    ) -> EfiStatus,
    pub open_protocol_information: extern "efiapi" fn(
        handle: *const u8,
        protocol: *const [u8; 16],
        entry_buffer: *mut *const u8,
        entry_count: *mut usize,
    ) -> EfiStatus,
    
    // Library Services
    pub protocols_per_handle: extern "efiapi" fn(
        handle: *const u8,
        protocol_buffer: *mut *const *const [u8; 16],
        protocol_buffer_count: *mut usize,
    ) -> EfiStatus,
    pub locate_handle_buffer: extern "efiapi" fn(
        search_type: u32,
        protocol: *const [u8; 16],
        search_key: *const u8,
        no_handles: *mut usize,
        buffer: *mut *const *const u8,
    ) -> EfiStatus,
    pub locate_protocol: extern "efiapi" fn(
        protocol: *const [u8; 16],
        registration: *const u8,
        interface: *mut *const u8,
    ) -> EfiStatus,
    pub install_multiple_protocol_interfaces: extern "efiapi" fn(
        handle: *mut *const u8,
        // Variable arguments follow
    ) -> EfiStatus,
    pub uninstall_multiple_protocol_interfaces: extern "efiapi" fn(
        handle: *const u8,
        // Variable arguments follow
    ) -> EfiStatus,
    
    // 32-bit CRC Services
    pub calculate_crc32: extern "efiapi" fn(
        data: *const u8,
        data_size: usize,
        crc32: *mut u32,
    ) -> EfiStatus,
    
    // Miscellaneous Services
    pub copy_mem: extern "efiapi" fn(
        destination: *mut u8,
        source: *const u8,
        length: usize,
    ),
    pub set_mem: extern "efiapi" fn(
        buffer: *mut u8,
        size: usize,
        value: u8,
    ),
    pub create_event_ex: extern "efiapi" fn(
        event_type: u32,
        notify_tpl: usize,
        notify_function: Option<extern "efiapi" fn()>,
        notify_context: *const u8,
        event_group: *const [u8; 16],
        event: *mut *const u8,
    ) -> EfiStatus,
}

/// UEFI Runtime Services
#[derive(Debug)]
#[repr(C)]
pub struct EfiRuntimeServices {
    pub hdr: EfiTableHeader,
    
    // Time Services
    pub get_time: extern "efiapi" fn(
        time: *mut EfiTime,
        capabilities: *mut EfiTimeCapabilities,
    ) -> EfiStatus,
    pub set_time: extern "efiapi" fn(time: *const EfiTime) -> EfiStatus,
    pub get_wakeup_time: extern "efiapi" fn(
        enabled: *mut bool,
        pending: *mut bool,
        time: *mut EfiTime,
    ) -> EfiStatus,
    pub set_wakeup_time: extern "efiapi" fn(
        enable: bool,
        time: *const EfiTime,
    ) -> EfiStatus,
    
    // Virtual Memory Services
    pub set_virtual_address_map: extern "efiapi" fn(
        memory_map_size: usize,
        descriptor_size: usize,
        descriptor_version: u32,
        virtual_map: *const EfiMemoryDescriptor,
    ) -> EfiStatus,
    pub convert_pointer: extern "efiapi" fn(
        debug_disposition: usize,
        address: *mut *const u8,
    ) -> EfiStatus,
    
    // Variable Services
    pub get_variable: extern "efiapi" fn(
        variable_name: *const u16,
        vendor_guid: *const [u8; 16],
        attributes: *mut u32,
        data_size: *mut usize,
        data: *mut u8,
    ) -> EfiStatus,
    pub get_next_variable_name: extern "efiapi" fn(
        variable_name_size: *mut usize,
        variable_name: *mut u16,
        vendor_guid: *mut [u8; 16],
    ) -> EfiStatus,
    pub set_variable: extern "efiapi" fn(
        variable_name: *const u16,
        vendor_guid: *const [u8; 16],
        attributes: u32,
        data_size: usize,
        data: *const u8,
    ) -> EfiStatus,
    
    // Miscellaneous Services
    pub get_next_high_mono_count: extern "efiapi" fn(high_count: *mut u32) -> EfiStatus,
    pub reset_system: extern "efiapi" fn(
        reset_type: u32,
        reset_status: EfiStatus,
        data_size: usize,
        reset_data: *const u8,
    ) -> !,
    
    // UEFI 2.0 Capsule Services
    pub update_capsule: extern "efiapi" fn(
        capsule_header_array: *const *const u8,
        capsule_count: usize,
        scatter_gather_list: u64,
    ) -> EfiStatus,
    pub query_capsule_capabilities: extern "efiapi" fn(
        capsule_header_array: *const *const u8,
        capsule_count: usize,
        maximum_capsule_size: *mut u64,
        reset_type: *mut u32,
    ) -> EfiStatus,
    
    // Miscellaneous UEFI 2.0 Service
    pub query_variable_info: extern "efiapi" fn(
        attributes: u32,
        maximum_variable_storage_size: *mut u64,
        remaining_variable_storage_size: *mut u64,
        maximum_variable_size: *mut u64,
    ) -> EfiStatus,
}

/// EFI Table Header
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EfiTableHeader {
    pub signature: u64,
    pub revision: u32,
    pub header_size: u32,
    pub crc32: u32,
    pub reserved: u32,
}

/// EFI Configuration Table
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EfiConfigurationTable {
    pub vendor_guid: [u8; 16],
    pub vendor_table: *const u8,
}

/// UEFI System Table
#[derive(Debug)]
#[repr(C)]
pub struct EfiSystemTable {
    pub hdr: EfiTableHeader,
    pub firmware_vendor: *const u16,
    pub firmware_revision: u32,
    pub console_in_handle: *const u8,
    pub con_in: *const u8, // EFI_SIMPLE_TEXT_INPUT_PROTOCOL
    pub console_out_handle: *const u8,
    pub con_out: *const u8, // EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL
    pub standard_error_handle: *const u8,
    pub std_err: *const u8, // EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL
    pub runtime_services: *const EfiRuntimeServices,
    pub boot_services: *const EfiBootServices,
    pub number_of_table_entries: usize,
    pub configuration_table: *const EfiConfigurationTable,
}

/// EFI Time structure
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EfiTime {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub pad1: u8,
    pub nanosecond: u32,
    pub time_zone: i16,
    pub daylight: u8,
    pub pad2: u8,
}

/// EFI Time Capabilities
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct EfiTimeCapabilities {
    pub resolution: u32,
    pub accuracy: u32,
    pub sets_to_zero: bool,
}

/// UEFI Boot Information
#[derive(Debug, Clone)]
pub struct UefiBootInfo {
    pub system_table: *const EfiSystemTable,
    pub memory_map: Vec<EfiMemoryDescriptor>,
    pub framebuffer: Option<FramebufferInfo>,
    pub rsdp_addr: Option<PhysAddr>,
    pub image_handle: *const u8,
}

/// Framebuffer Information
#[derive(Debug, Clone)]
pub struct FramebufferInfo {
    pub base_addr: PhysAddr,
    pub size: usize,
    pub width: u32,
    pub height: u32,
    pub pixels_per_scanline: u32,
    pub pixel_format: GopPixelFormat,
    pub pixel_bitmask: GopPixelBitmask,
}

/// UEFI Manager
#[derive(Debug)]
pub struct UefiManager {
    boot_info: Option<UefiBootInfo>,
    runtime_services: Option<*const EfiRuntimeServices>,
    gop: Option<*const GraphicsOutputProtocol>,
}

// SAFETY: UefiManager contains pointers to UEFI runtime services which are designed
// to be accessed from any CPU core. The UEFI specification guarantees thread safety
// for runtime services.
unsafe impl Send for UefiManager {}
unsafe impl Sync for UefiManager {}

impl UefiManager {
    /// Create a new UEFI manager
    pub fn new() -> Self {
        Self {
            boot_info: None,
            runtime_services: None,
            gop: None,
        }
    }
    
    /// Initialize UEFI services
    pub fn init(&mut self, system_table: *const EfiSystemTable, image_handle: *const u8) -> Result<(), &'static str> {
        if system_table.is_null() {
            return Err("Invalid system table");
        }
        
        let system_table_ref = unsafe { &*system_table };
        
        // Verify system table signature
        if system_table_ref.hdr.signature != EFI_SYSTEM_TABLE_SIGNATURE {
            return Err("Invalid system table signature");
        }
        
        // Store runtime services
        self.runtime_services = Some(system_table_ref.runtime_services);
        
        // Initialize Graphics Output Protocol
        self.init_gop(system_table_ref)?;
        
        // Get memory map
        let memory_map = self.get_memory_map(system_table_ref)?;
        
        // Find RSDP (Root System Description Pointer)
        let rsdp_addr = self.find_rsdp(system_table_ref);
        
        // Create framebuffer info
        let framebuffer = self.get_framebuffer_info()?;
        
        // Store boot information
        self.boot_info = Some(UefiBootInfo {
            system_table,
            memory_map,
            framebuffer,
            rsdp_addr,
            image_handle,
        });
        
        Ok(())
    }
    
    /// Initialize Graphics Output Protocol
    fn init_gop(&mut self, system_table: &EfiSystemTable) -> Result<(), &'static str> {
        let boot_services = unsafe { &*system_table.boot_services };
        
        // GOP GUID: 9042A9DE-23DC-4A38-96FB-7ADED080516A
        let gop_guid = [
            0xDE, 0xA9, 0x42, 0x90, 0xDC, 0x23, 0x38, 0x4A,
            0x96, 0xFB, 0x7A, 0xDE, 0xD0, 0x80, 0x51, 0x6A,
        ];
        
        let mut gop_interface: *const GraphicsOutputProtocol = core::ptr::null();
        
        let status = (boot_services.locate_protocol)(
            &gop_guid,
            core::ptr::null(),
            &mut gop_interface as *mut _ as *mut *const u8,
        );
        
        if status != EfiStatus::Success {
            return Err("Failed to locate Graphics Output Protocol");
        }
        
        self.gop = Some(gop_interface);
        
        // Set the best available graphics mode
        self.set_best_graphics_mode()?;
        
        Ok(())
    }
    
    /// Set the best available graphics mode
    fn set_best_graphics_mode(&self) -> Result<(), &'static str> {
        let gop = self.gop.ok_or("GOP not initialized")?;
        let gop_ref = unsafe { &*gop };
        let mode = unsafe { &*gop_ref.mode };
        
        let mut best_mode = 0;
        let mut best_resolution = 0;
        
        // Find the highest resolution mode
        for mode_num in 0..mode.max_mode {
            let mut size_of_info = 0;
            let mut mode_info: *const GopModeInfo = core::ptr::null();
            
            let status = (gop_ref.query_mode)(
                gop,
                mode_num,
                &mut size_of_info,
                &mut mode_info,
            );
            
            if status == EfiStatus::Success && !mode_info.is_null() {
                let info = unsafe { &*mode_info };
                let resolution = info.horizontal_resolution * info.vertical_resolution;
                
                if resolution > best_resolution {
                    best_resolution = resolution;
                    best_mode = mode_num;
                }
            }
        }
        
        // Set the best mode
        let status = (gop_ref.set_mode)(gop, best_mode);
        if status != EfiStatus::Success {
            return Err("Failed to set graphics mode");
        }
        
        Ok(())
    }
    
    /// Get memory map from UEFI
    fn get_memory_map(&self, system_table: &EfiSystemTable) -> Result<Vec<EfiMemoryDescriptor>, &'static str> {
        let boot_services = unsafe { &*system_table.boot_services };
        
        let mut memory_map_size = 0;
        let mut map_key = 0;
        let mut descriptor_size = 0;
        let mut descriptor_version = 0;
        
        // Get memory map size
        let status = (boot_services.get_memory_map)(
            &mut memory_map_size,
            core::ptr::null_mut(),
            &mut map_key,
            &mut descriptor_size,
            &mut descriptor_version,
        );
        
        if status != EfiStatus::BufferTooSmall {
            return Err("Failed to get memory map size");
        }
        
        // Allocate buffer for memory map
        let mut buffer: *mut u8 = core::ptr::null_mut();
        let alloc_status = (boot_services.allocate_pool)(
            EfiMemoryType::LoaderData,
            memory_map_size,
            &mut buffer,
        );
        
        if alloc_status != EfiStatus::Success {
            return Err("Failed to allocate memory map buffer");
        }
        
        // Get actual memory map
        let status = (boot_services.get_memory_map)(
            &mut memory_map_size,
            buffer as *mut EfiMemoryDescriptor,
            &mut map_key,
            &mut descriptor_size,
            &mut descriptor_version,
        );
        
        if status != EfiStatus::Success {
            (boot_services.free_pool)(buffer);
            return Err("Failed to get memory map");
        }
        
        // Convert to Vec
        let descriptor_count = memory_map_size / descriptor_size;
        let mut memory_map = Vec::with_capacity(descriptor_count);
        
        for i in 0..descriptor_count {
            let descriptor_ptr = unsafe {
                buffer.add(i * descriptor_size) as *const EfiMemoryDescriptor
            };
            let descriptor = unsafe { *descriptor_ptr };
            memory_map.push(descriptor);
        }
        
        // Free the buffer
        (boot_services.free_pool)(buffer);
        
        Ok(memory_map)
    }
    
    /// Find RSDP (Root System Description Pointer) in configuration tables
    fn find_rsdp(&self, system_table: &EfiSystemTable) -> Option<PhysAddr> {
        let config_tables = unsafe {
            slice::from_raw_parts(
                system_table.configuration_table,
                system_table.number_of_table_entries,
            )
        };
        
        // ACPI 2.0 RSDP GUID: 8868E871-E4F1-11D3-BC22-0080C73C8881
        let acpi_20_guid = [
            0x71, 0xE8, 0x68, 0x88, 0xF1, 0xE4, 0xD3, 0x11,
            0xBC, 0x22, 0x00, 0x80, 0xC7, 0x3C, 0x88, 0x81,
        ];
        
        // ACPI 1.0 RSDP GUID: EB9D2D30-2D88-11D3-9A16-0090273FC14D
        let acpi_10_guid = [
            0x30, 0x2D, 0x9D, 0xEB, 0x88, 0x2D, 0xD3, 0x11,
            0x9A, 0x16, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D,
        ];
        
        for table in config_tables {
            if table.vendor_guid == acpi_20_guid || table.vendor_guid == acpi_10_guid {
                return Some(PhysAddr::new(table.vendor_table as u64));
            }
        }
        
        None
    }
    
    /// Get framebuffer information from GOP
    fn get_framebuffer_info(&self) -> Result<Option<FramebufferInfo>, &'static str> {
        let gop = match self.gop {
            Some(gop) => gop,
            None => return Ok(None),
        };
        
        let gop_ref = unsafe { &*gop };
        let mode = unsafe { &*gop_ref.mode };
        let mode_info = unsafe { &*mode.info };
        
        Ok(Some(FramebufferInfo {
            base_addr: PhysAddr::new(mode.frame_buffer_base),
            size: mode.frame_buffer_size,
            width: mode_info.horizontal_resolution,
            height: mode_info.vertical_resolution,
            pixels_per_scanline: mode_info.pixels_per_scan_line,
            pixel_format: mode_info.pixel_format,
            pixel_bitmask: mode_info.pixel_information,
        }))
    }
    
    /// Exit boot services and transition to runtime
    pub fn exit_boot_services(&self) -> Result<(), &'static str> {
        let boot_info = self.boot_info.as_ref().ok_or("UEFI not initialized")?;
        let system_table = unsafe { &*boot_info.system_table };
        let boot_services = unsafe { &*system_table.boot_services };
        
        // Get current memory map key
        let mut memory_map_size = 0;
        let mut map_key = 0;
        let mut descriptor_size = 0;
        let mut descriptor_version = 0;
        
        let status = (boot_services.get_memory_map)(
            &mut memory_map_size,
            core::ptr::null_mut(),
            &mut map_key,
            &mut descriptor_size,
            &mut descriptor_version,
        );
        
        if status != EfiStatus::BufferTooSmall {
            return Err("Failed to get memory map for exit");
        }
        
        // Exit boot services
        let exit_status = (boot_services.exit_boot_services)(boot_info.image_handle, map_key);
        if exit_status != EfiStatus::Success {
            return Err("Failed to exit boot services");
        }
        
        Ok(())
    }
    
    /// Get boot information
    pub fn get_boot_info(&self) -> Option<&UefiBootInfo> {
        self.boot_info.as_ref()
    }
    
    /// Get runtime services
    pub fn get_runtime_services(&self) -> Option<&EfiRuntimeServices> {
        self.runtime_services.map(|rs| unsafe { &*rs })
    }
}

lazy_static! {
    /// Global UEFI manager instance
    static ref UEFI_MANAGER: Mutex<UefiManager> = Mutex::new(UefiManager::new());
}

/// Initialize UEFI services
pub fn init(system_table: *const EfiSystemTable, image_handle: *const u8) -> Result<(), &'static str> {
    let mut manager = UEFI_MANAGER.lock();
    manager.init(system_table, image_handle)
}

/// Exit UEFI boot services
pub fn exit_boot_services() -> Result<(), &'static str> {
    let manager = UEFI_MANAGER.lock();
    manager.exit_boot_services()
}

/// Get UEFI boot information
pub fn get_boot_info() -> Option<UefiBootInfo> {
    let manager = UEFI_MANAGER.lock();
    manager.get_boot_info().cloned()
}

/// Get framebuffer information
pub fn get_framebuffer_info() -> Option<FramebufferInfo> {
    let manager = UEFI_MANAGER.lock();
    manager.get_boot_info()?.framebuffer.clone()
}

/// Get memory map
pub fn get_memory_map() -> Option<Vec<EfiMemoryDescriptor>> {
    let manager = UEFI_MANAGER.lock();
    Some(manager.get_boot_info()?.memory_map.clone())
}

/// Get RSDP address
pub fn get_rsdp_addr() -> Option<PhysAddr> {
    let manager = UEFI_MANAGER.lock();
    manager.get_boot_info()?.rsdp_addr
}

/// Convert EFI memory type to kernel memory type
pub fn efi_memory_type_to_kernel(efi_type: EfiMemoryType) -> crate::memory::MemoryType {
    match efi_type {
        EfiMemoryType::ConventionalMemory => crate::memory::MemoryType::Usable,
        EfiMemoryType::LoaderCode | EfiMemoryType::LoaderData => crate::memory::MemoryType::Bootloader,
        EfiMemoryType::BootServicesCode | EfiMemoryType::BootServicesData => crate::memory::MemoryType::Usable,
        EfiMemoryType::RuntimeServicesCode | EfiMemoryType::RuntimeServicesData => crate::memory::MemoryType::Reserved,
        EfiMemoryType::AcpiReclaimMemory => crate::memory::MemoryType::AcpiReclaimable,
        EfiMemoryType::AcpiMemoryNvs => crate::memory::MemoryType::AcpiNvs,
        EfiMemoryType::MemoryMappedIo | EfiMemoryType::MemoryMappedIoPortSpace => crate::memory::MemoryType::Reserved,
        _ => crate::memory::MemoryType::Reserved,
    }
}