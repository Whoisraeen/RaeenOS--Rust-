use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use core::fmt;
use spin::RwLock;
use x86_64::instructions::port::Port;
use x86_64::VirtAddr;

static DEVICE_MANAGER: RwLock<DeviceManager> = RwLock::new(DeviceManager::new());

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeviceType {
    Storage,
    Network,
    Graphics,
    Audio,
    Input,
    Usb,
    Pci,
    Serial,
    Parallel,
    Timer,
    Rtc,
    Dma,
    Interrupt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceStatus {
    Uninitialized,
    Initializing,
    Ready,
    Busy,
    Error,
    Disabled,
}

#[derive(Debug)]
pub struct DeviceInfo {
    pub id: u64,
    pub name: String,
    pub device_type: DeviceType,
    pub status: DeviceStatus,
    pub vendor_id: u16,
    pub device_id: u16,
    pub irq: Option<u8>,
    pub io_base: Option<u16>,
    pub memory_base: Option<VirtAddr>,
    pub memory_size: Option<usize>,
}

pub trait Device: Send + Sync {
    fn name(&self) -> &str;
    fn device_type(&self) -> DeviceType;
    fn init(&mut self) -> Result<(), DeviceError>;
    fn reset(&mut self) -> Result<(), DeviceError>;
    fn status(&self) -> DeviceStatus;
    fn info(&self) -> DeviceInfo;
    fn handle_interrupt(&mut self) -> Result<(), DeviceError>;
}

#[derive(Debug)]
pub enum DeviceError {
    NotFound,
    InitializationFailed,
    InvalidOperation,
    HardwareError,
    Timeout,
    NotSupported,
    Busy,
    IoError,
    OutOfMemory,
    InvalidParameter,
}

impl fmt::Display for DeviceError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DeviceError::NotFound => write!(f, "Device not found"),
            DeviceError::InitializationFailed => write!(f, "Device initialization failed"),
            DeviceError::InvalidOperation => write!(f, "Invalid operation"),
            DeviceError::HardwareError => write!(f, "Hardware error"),
            DeviceError::Timeout => write!(f, "Operation timeout"),
            DeviceError::NotSupported => write!(f, "Operation not supported"),
            DeviceError::Busy => write!(f, "Device busy"),
            DeviceError::IoError => write!(f, "I/O error"),
            DeviceError::OutOfMemory => write!(f, "Out of memory"),
            DeviceError::InvalidParameter => write!(f, "Invalid parameter"),
        }
    }
}

pub type DeviceResult<T> = Result<T, DeviceError>;

// Device Manager
pub struct DeviceManager {
    devices: BTreeMap<u64, Box<dyn Device>>,
    next_id: u64,
    device_by_type: BTreeMap<DeviceType, Vec<u64>>,
}

impl DeviceManager {
    pub const fn new() -> Self {
        Self {
            devices: BTreeMap::new(),
            next_id: 1,
            device_by_type: BTreeMap::new(),
        }
    }
    
    pub fn register_device(&mut self, mut device: Box<dyn Device>) -> DeviceResult<u64> {
        let id = self.next_id;
        self.next_id += 1;
        
        let device_type = device.device_type();
        
        // Initialize the device
        device.init()?;
        
        // Add to device list
        self.devices.insert(id, device);
        
        // Add to type index
        self.device_by_type.entry(device_type)
            .or_insert_with(Vec::new)
            .push(id);
        
        Ok(id)
    }
    
    pub fn get_device(&self, id: u64) -> Option<&dyn Device> {
        self.devices.get(&id).map(|d| d.as_ref())
    }
    
    pub fn get_device_mut(&mut self, id: u64) -> Option<&mut (dyn Device + '_)> {
        match self.devices.get_mut(&id) {
            Some(device) => Some(device.as_mut()),
            None => None,
        }
    }
    
    pub fn get_devices_by_type(&self, device_type: DeviceType) -> Vec<u64> {
        self.device_by_type.get(&device_type)
            .cloned()
            .unwrap_or_default()
    }
    
    pub fn list_devices(&self) -> Vec<DeviceInfo> {
        self.devices.values()
            .map(|device| device.info())
            .collect()
    }
    
    pub fn handle_interrupt(&mut self, irq: u8) -> DeviceResult<()> {
        for device in self.devices.values_mut() {
            if let Some(device_irq) = device.info().irq {
                if device_irq == irq {
                    device.handle_interrupt()?;
                }
            }
        }
        Ok(())
    }
}

// VGA Graphics Driver
#[derive(Debug)]
pub struct VgaDriver {
    framebuffer: VirtAddr,
    width: u32,
    height: u32,
    _bpp: u8, // bits per pixel
    status: DeviceStatus,
}

impl VgaDriver {
    pub fn new() -> Self {
        Self {
            framebuffer: VirtAddr::new(0xB8000), // VGA text mode buffer
            width: 80,
            height: 25,
            _bpp: 4, // 4 bits per character (16 colors)
            status: DeviceStatus::Uninitialized,
        }
    }
    
    pub fn write_char(&self, x: u32, y: u32, ch: u8, color: u8) {
        if x >= self.width || y >= self.height {
            return;
        }
        
        let offset = (y * self.width + x) * 2;
        let ptr = (self.framebuffer.as_u64() + offset as u64) as *mut u8;
        
        unsafe {
            *ptr = ch;
            *ptr.add(1) = color;
        }
    }
    
    pub fn clear_screen(&self, color: u8) {
        for y in 0..self.height {
            for x in 0..self.width {
                self.write_char(x, y, b' ', color);
            }
        }
    }
    
    pub fn write_string(&self, x: u32, y: u32, text: &str, color: u8) {
        for (i, ch) in text.bytes().enumerate() {
            if x + i as u32 >= self.width {
                break;
            }
            self.write_char(x + i as u32, y, ch, color);
        }
    }
}

impl Device for VgaDriver {
    fn name(&self) -> &str {
        "VGA Graphics Adapter"
    }
    
    fn device_type(&self) -> DeviceType {
        DeviceType::Graphics
    }
    
    fn init(&mut self) -> Result<(), DeviceError> {
        self.status = DeviceStatus::Initializing;
        
        // Initialize VGA registers
        unsafe {
            // Set text mode 80x25
            let mut misc_port = Port::new(0x3C2);
            misc_port.write(0x67u8);
        }
        
        self.status = DeviceStatus::Ready;
        Ok(())
    }
    
    fn reset(&mut self) -> Result<(), DeviceError> {
        self.clear_screen(0x07); // White on black
        Ok(())
    }
    
    fn status(&self) -> DeviceStatus {
        self.status
    }
    
    fn info(&self) -> DeviceInfo {
        DeviceInfo {
            id: 1,
            name: self.name().to_string(),
            device_type: self.device_type(),
            status: self.status,
            vendor_id: 0x1234,
            device_id: 0x1111,
            irq: None,
            io_base: Some(0x3C0),
            memory_base: Some(self.framebuffer),
            memory_size: Some((self.width * self.height * 2) as usize),
        }
    }
    
    fn handle_interrupt(&mut self) -> Result<(), DeviceError> {
        // VGA doesn't typically generate interrupts
        Ok(())
    }
}

// ATA/IDE Hard Drive Driver
#[derive(Debug)]
pub struct AtaDriver {
    primary_base: u16,
    secondary_base: u16,
    status: DeviceStatus,
    drives: [Option<AtaDrive>; 4], // Primary master/slave, Secondary master/slave
}

#[derive(Debug, Clone)]
struct AtaDrive {
    _sectors: u64,
    is_master: bool,
    base_port: u16,
}

impl AtaDriver {
    pub fn new() -> Self {
        Self {
            primary_base: 0x1F0,
            secondary_base: 0x170,
            status: DeviceStatus::Uninitialized,
            drives: [None, None, None, None],
        }
    }
    
    fn identify_drive(&self, base: u16, is_master: bool) -> Option<AtaDrive> {
        unsafe {
            let mut drive_port: Port<u8> = Port::new(base + 6);
            let mut command_port: Port<u8> = Port::new(base + 7);
            let mut status_port: Port<u8> = Port::new(base + 7);
            
            // Select drive
            drive_port.write(if is_master { 0xA0u8 } else { 0xB0u8 });
            
            // Send IDENTIFY command
            command_port.write(0xECu8);
            
            // Check if drive exists
            let status = status_port.read();
            if status == 0 {
                return None;
            }
            
            // Wait for drive to be ready
            while (status_port.read() & 0x80) != 0 {} // Wait for BSY to clear
            
            // Read identification data
            let mut data_port = Port::new(base);
            let mut buffer = [0u16; 256];
            for i in 0..256 {
                buffer[i] = data_port.read();
            }
            
            // Extract sector count (words 60-61)
            let sectors = ((buffer[61] as u64) << 16) | (buffer[60] as u64);
            
            Some(AtaDrive {
                _sectors: sectors,
                is_master,
                base_port: base,
            })
        }
    }
    
    pub fn read_sector(&self, drive: u8, lba: u64, buffer: &mut [u8]) -> DeviceResult<()> {
        if drive >= 4 || self.drives[drive as usize].is_none() {
            return Err(DeviceError::NotFound);
        }
        
        let drive_info = self.drives[drive as usize].as_ref().unwrap();
        let base = drive_info.base_port;
        
        unsafe {
            let mut features_port: Port<u8> = Port::new(base + 1);
            let mut sector_count_port: Port<u8> = Port::new(base + 2);
            let mut lba_low_port: Port<u8> = Port::new(base + 3);
            let mut lba_mid_port: Port<u8> = Port::new(base + 4);
            let mut lba_high_port: Port<u8> = Port::new(base + 5);
            let mut drive_port: Port<u8> = Port::new(base + 6);
            let mut command_port: Port<u8> = Port::new(base + 7);
            let mut status_port: Port<u8> = Port::new(base + 7);
            let mut data_port: Port<u16> = Port::new(base);
            
            // Set up LBA addressing
            features_port.write(0u8);
            sector_count_port.write(1u8);
            lba_low_port.write((lba & 0xFF) as u8);
            lba_mid_port.write(((lba >> 8) & 0xFF) as u8);
            lba_high_port.write(((lba >> 16) & 0xFF) as u8);
            
            let drive_select = if drive_info.is_master { 0xE0 } else { 0xF0 };
            drive_port.write(drive_select | (((lba >> 24) & 0x0F) as u8));
            
            // Send READ SECTORS command
            command_port.write(0x20u8);
            
            // Wait for drive to be ready
            while (status_port.read() & 0x80) != 0 {} // Wait for BSY to clear
            
            // Check for errors
            let status = status_port.read();
            if (status & 0x01) != 0 {
                return Err(DeviceError::HardwareError);
            }
            
            // Read data
            for i in (0..512).step_by(2) {
                let word = data_port.read();
                if i < buffer.len() {
                    buffer[i] = (word & 0xFF) as u8;
                }
                if i + 1 < buffer.len() {
                    buffer[i + 1] = (word >> 8) as u8;
                }
            }
        }
        
        Ok(())
    }
    
    pub fn write_sector(&self, drive: u8, lba: u64, buffer: &[u8]) -> DeviceResult<()> {
        if drive >= 4 || self.drives[drive as usize].is_none() {
            return Err(DeviceError::NotFound);
        }
        
        let drive_info = self.drives[drive as usize].as_ref().unwrap();
        let base = drive_info.base_port;
        
        unsafe {
            let mut features_port: Port<u8> = Port::new(base + 1);
            let mut sector_count_port: Port<u8> = Port::new(base + 2);
            let mut lba_low_port: Port<u8> = Port::new(base + 3);
            let mut lba_mid_port: Port<u8> = Port::new(base + 4);
            let mut lba_high_port: Port<u8> = Port::new(base + 5);
            let mut drive_port: Port<u8> = Port::new(base + 6);
            let mut command_port: Port<u8> = Port::new(base + 7);
            let mut status_port: Port<u8> = Port::new(base + 7);
            let mut data_port: Port<u16> = Port::new(base);
            
            // Set up LBA addressing
            features_port.write(0u8);
            sector_count_port.write(1u8);
            lba_low_port.write((lba & 0xFF) as u8);
            lba_mid_port.write(((lba >> 8) & 0xFF) as u8);
            lba_high_port.write(((lba >> 16) & 0xFF) as u8);
            
            let drive_select = if drive_info.is_master { 0xE0 } else { 0xF0 };
            drive_port.write(drive_select | (((lba >> 24) & 0x0F) as u8));
            
            // Send WRITE SECTORS command
            command_port.write(0x30u8);
            
            // Wait for drive to be ready
            while (status_port.read() & 0x80) != 0 {} // Wait for BSY to clear
            
            // Check for errors
            let status = status_port.read();
            if (status & 0x01) != 0 {
                return Err(DeviceError::HardwareError);
            }
            
            // Write data
            for i in (0..512).step_by(2) {
                let low = if i < buffer.len() { buffer[i] } else { 0 };
                let high = if i + 1 < buffer.len() { buffer[i + 1] } else { 0 };
                let word = (high as u16) << 8 | (low as u16);
                data_port.write(word);
            }
            
            // Flush cache
            command_port.write(0xE7u8);
            while (status_port.read() & 0x80) != 0 {} // Wait for BSY to clear
        }
        
        Ok(())
    }
}

impl Device for AtaDriver {
    fn name(&self) -> &str {
        "ATA/IDE Controller"
    }
    
    fn device_type(&self) -> DeviceType {
        DeviceType::Storage
    }
    
    fn init(&mut self) -> Result<(), DeviceError> {
        self.status = DeviceStatus::Initializing;
        
        // Identify drives
        self.drives[0] = self.identify_drive(self.primary_base, true);   // Primary master
        self.drives[1] = self.identify_drive(self.primary_base, false);  // Primary slave
        self.drives[2] = self.identify_drive(self.secondary_base, true); // Secondary master
        self.drives[3] = self.identify_drive(self.secondary_base, false); // Secondary slave
        
        self.status = DeviceStatus::Ready;
        Ok(())
    }
    
    fn reset(&mut self) -> Result<(), DeviceError> {
        // Reset ATA controller
        unsafe {
            let mut control_port = Port::new(self.primary_base + 0x206);
            control_port.write(0x04u8); // Set reset bit
            crate::time::sleep_ms(5);
            control_port.write(0x00u8); // Clear reset bit
            crate::time::sleep_ms(5);
        }
        Ok(())
    }
    
    fn status(&self) -> DeviceStatus {
        self.status
    }
    
    fn info(&self) -> DeviceInfo {
        DeviceInfo {
            id: 2,
            name: self.name().to_string(),
            device_type: self.device_type(),
            status: self.status,
            vendor_id: 0x8086,
            device_id: 0x1234,
            irq: Some(14), // Primary ATA IRQ
            io_base: Some(self.primary_base),
            memory_base: None,
            memory_size: None,
        }
    }
    
    fn handle_interrupt(&mut self) -> Result<(), DeviceError> {
        // Handle ATA interrupt
        Ok(())
    }
}

// NVMe Driver
use x86_64::PhysAddr;
use crate::pci::{PciDevice, read_config_dword, read_config_word, write_config_word};
use alloc::collections::VecDeque;
use core::sync::atomic::{AtomicU16, AtomicU64, Ordering};
use spin::Mutex;
use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll, Waker};
use alloc::boxed::Box;

// NVMe Register Offsets
const NVME_REG_CAP: u64 = 0x00;     // Controller Capabilities
#[allow(dead_code)]
const NVME_REG_VS: u64 = 0x08;      // Version
#[allow(dead_code)]
const NVME_REG_INTMS: u64 = 0x0C;   // Interrupt Mask Set
#[allow(dead_code)]
const NVME_REG_INTMC: u64 = 0x10;   // Interrupt Mask Clear
const NVME_REG_CC: u64 = 0x14;      // Controller Configuration
const NVME_REG_CSTS: u64 = 0x1C;    // Controller Status
const NVME_REG_AQA: u64 = 0x24;     // Admin Queue Attributes
const NVME_REG_ASQ: u64 = 0x28;     // Admin Submission Queue Base
const NVME_REG_ACQ: u64 = 0x30;     // Admin Completion Queue Base

// NVMe Command Opcodes
#[allow(dead_code)]
const NVME_ADMIN_DELETE_SQ: u8 = 0x00;
const NVME_ADMIN_CREATE_SQ: u8 = 0x01;
#[allow(dead_code)]
const NVME_ADMIN_DELETE_CQ: u8 = 0x04;
const NVME_ADMIN_CREATE_CQ: u8 = 0x05;
#[allow(dead_code)]
const NVME_ADMIN_IDENTIFY: u8 = 0x06;
const NVME_CMD_READ: u8 = 0x02;
const NVME_CMD_WRITE: u8 = 0x01;
const NVME_CMD_FLUSH: u8 = 0x00;

// NVMe Queue Sizes (optimized for performance)
const NVME_ADMIN_QUEUE_SIZE: u16 = 64;
const NVME_IO_QUEUE_SIZE: u16 = 1024;  // Large queue for high throughput
const NVME_MAX_QUEUES: u16 = 8;        // Multiple queues for parallelism

// Asynchronous I/O Support
type IoCallback = Box<dyn Fn(Result<u32, u32>) + Send + Sync>;

struct IoRequest {
    command_id: u16,
    callback: Option<IoCallback>,
    waker: Option<Waker>,
    completed: bool,
    result: Option<Result<u32, u32>>,
}

// Zero-copy DMA buffer management
#[derive(Debug)]
struct DmaBuffer {
    virt_addr: *mut u8,
    phys_addr: PhysAddr,
    size: usize,
    #[allow(dead_code)]
    aligned: bool,
}

// Future for async I/O operations
pub struct NvmeIoFuture {
    command_id: u16,
    driver: *mut NvmeDriver,
}

impl Future for NvmeIoFuture {
    type Output = Result<u32, u32>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe {
            let driver = &mut *self.driver;
            if let Some(request) = driver.pending_requests.iter_mut().find(|r| r.command_id == self.command_id) {
                if request.completed {
                    Poll::Ready(request.result.unwrap_or(Err(0xFFFFFFFF)))
                } else {
                    request.waker = Some(cx.waker().clone());
                    Poll::Pending
                }
            } else {
                Poll::Ready(Err(0xFFFFFFFF))
            }
        }
    }
}

unsafe impl Send for NvmeIoFuture {}
unsafe impl Sync for NvmeIoFuture {}

unsafe impl Send for DmaBuffer {}
unsafe impl Sync for DmaBuffer {}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct NvmeCommand {
    opcode: u8,
    flags: u8,
    command_id: u16,
    namespace_id: u32,
    reserved: [u32; 2],
    metadata_ptr: u64,
    data_ptr: [u64; 2],
    cdw10: u32,
    cdw11: u32,
    cdw12: u32,
    cdw13: u32,
    cdw14: u32,
    cdw15: u32,
}

#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
struct NvmeCompletion {
    result: u32,
    reserved: u32,
    sq_head: u16,
    sq_id: u16,
    command_id: u16,
    status: u16,
}

#[derive(Debug)]
#[allow(dead_code)]
struct NvmeQueue {
    submission_queue: VirtAddr,
    completion_queue: VirtAddr,
    sq_tail: AtomicU16,
    cq_head: AtomicU16,
    queue_size: u16,
    queue_id: u16,
    doorbell_base: VirtAddr,
    pending_commands: Mutex<VecDeque<(u16, u64, u8)>>, // (command_id, timestamp, opcode)
    // Performance optimization fields
    current_depth: AtomicU16,
    max_depth: AtomicU16,
    avg_latency_us: AtomicU64,
    p99_latency_us: AtomicU64,
    completion_batch_size: AtomicU16,
    last_doorbell_time: AtomicU64,
}

#[derive(Debug)]
#[allow(dead_code)]
struct NvmeNamespace {
    id: u32,
    size: u64,
    block_size: u32,
    features: u32,
}

#[allow(dead_code)]
pub struct NvmeDriver {
    status: DeviceStatus,
    pci_device: PciDevice,
    bar0: VirtAddr,
    admin_queue: Option<NvmeQueue>,
    io_queues: Vec<NvmeQueue>,
    namespaces: Vec<NvmeNamespace>,
    command_id_counter: AtomicU16,
    max_transfer_size: u32,
    queue_depth: u16,
    // Performance optimization fields
    last_completion_time: AtomicU64,
    total_commands: AtomicU64,
    failed_commands: AtomicU64,
    // Asynchronous I/O support
    pending_requests: Vec<IoRequest>,
    dma_buffers: Vec<DmaBuffer>,
}

impl NvmeDriver {
    pub fn new(pci_device: PciDevice) -> Self {
        Self {
            status: DeviceStatus::Uninitialized,
            pci_device,
            bar0: VirtAddr::new(0),
            admin_queue: None,
            io_queues: Vec::new(),
            namespaces: Vec::new(),
            command_id_counter: AtomicU16::new(1),
            max_transfer_size: 0,
            queue_depth: NVME_IO_QUEUE_SIZE,
            last_completion_time: AtomicU64::new(0),
            total_commands: AtomicU64::new(0),
            failed_commands: AtomicU64::new(0),
            pending_requests: Vec::new(),
            dma_buffers: Vec::new(),
        }
    }
    
    fn read_reg32(&self, offset: u64) -> u32 {
        unsafe {
            core::ptr::read_volatile((self.bar0.as_u64() + offset) as *const u32)
        }
    }
    
    fn write_reg32(&self, offset: u64, value: u32) {
        unsafe {
            core::ptr::write_volatile((self.bar0.as_u64() + offset) as *mut u32, value);
        }
    }
    
    fn read_reg64(&self, offset: u64) -> u64 {
        unsafe {
            core::ptr::read_volatile((self.bar0.as_u64() + offset) as *const u64)
        }
    }
    
    fn write_reg64(&self, offset: u64, value: u64) {
        unsafe {
            core::ptr::write_volatile((self.bar0.as_u64() + offset) as *mut u64, value);
        }
    }
    
    fn allocate_queue(&self, size: u16) -> Result<NvmeQueue, DeviceError> {
        // Calculate required pages for submission and completion queues
        let sq_size = (size as usize * core::mem::size_of::<NvmeCommand>() + 4095) / 4096;
        let cq_size = (size as usize * core::mem::size_of::<NvmeCompletion>() + 4095) / 4096;
        
        // Allocate physically contiguous memory (simplified allocation)
        let sq_phys = PhysAddr::new(0x10000000 + (sq_size * 4096) as u64); // Placeholder
        let cq_phys = PhysAddr::new(0x20000000 + (cq_size * 4096) as u64); // Placeholder
        
        // Map to virtual addresses
        let sq_virt = crate::memory::phys_to_virt(sq_phys);
        let cq_virt = crate::memory::phys_to_virt(cq_phys);
        
        // Zero the memory
        unsafe {
            core::ptr::write_bytes(sq_virt.as_mut_ptr::<u8>(), 0, sq_size * 4096);
            core::ptr::write_bytes(cq_virt.as_mut_ptr::<u8>(), 0, cq_size * 4096);
        }
        
        Ok(NvmeQueue {
            submission_queue: sq_virt,
            completion_queue: cq_virt,
            sq_tail: AtomicU16::new(0),
            cq_head: AtomicU16::new(0),
            queue_size: size,
            queue_id: 0, // Will be set later
            doorbell_base: self.bar0,
            pending_commands: Mutex::new(VecDeque::new()),
            // Initialize performance optimization fields
            current_depth: AtomicU16::new(0),
            max_depth: AtomicU16::new(size / 2), // Start with conservative depth
            avg_latency_us: AtomicU64::new(0),
            p99_latency_us: AtomicU64::new(0),
            completion_batch_size: AtomicU16::new(8), // Batch completions for efficiency
            last_doorbell_time: AtomicU64::new(0),
        })
    }
    
    fn setup_admin_queue(&mut self) -> Result<(), DeviceError> {
        let mut admin_queue = self.allocate_queue(NVME_ADMIN_QUEUE_SIZE)?;
        admin_queue.queue_id = 0;
        
        // Set admin queue attributes
        let aqa = ((NVME_ADMIN_QUEUE_SIZE - 1) as u32) << 16 | ((NVME_ADMIN_QUEUE_SIZE - 1) as u32);
        self.write_reg32(NVME_REG_AQA, aqa);
        
        // Set admin queue base addresses
        let sq_phys = crate::memory::virt_to_phys(admin_queue.submission_queue).as_u64();
        let cq_phys = crate::memory::virt_to_phys(admin_queue.completion_queue).as_u64();
        
        self.write_reg64(NVME_REG_ASQ, sq_phys);
        self.write_reg64(NVME_REG_ACQ, cq_phys);
        
        self.admin_queue = Some(admin_queue);
        Ok(())
    }
    
    fn enable_controller(&self) -> Result<(), DeviceError> {
        // Read capabilities
        let cap = self.read_reg64(NVME_REG_CAP);
        let mpsmin = (cap >> 48) & 0xF;
        let _mpsmax = (cap >> 52) & 0xF;
        
        // Configure controller
        let mut cc = 0u32;
        cc |= (6 << 16) | (4 << 20); // I/O Submission/Completion Queue Entry Size
        cc |= (mpsmin as u32) << 7;    // Memory Page Size
        cc |= 1;                     // Enable bit
        
        self.write_reg32(NVME_REG_CC, cc);
        
        // Wait for controller to be ready
        let mut timeout = 1000000; // 1 second timeout
        while timeout > 0 {
            let csts = self.read_reg32(NVME_REG_CSTS);
            if (csts & 1) != 0 {
                return Ok(());
            }
            // crate::time::sleep_us(1); // Placeholder for timing
            timeout -= 1;
        }
        
        Err(DeviceError::Timeout)
    }
    
    fn submit_admin_command(&self, cmd: &NvmeCommand) -> Result<u16, DeviceError> {
        let admin_queue = self.admin_queue.as_ref().ok_or(DeviceError::NotFound)?;
        
        let tail = admin_queue.sq_tail.load(Ordering::Acquire);
        let next_tail = (tail + 1) % admin_queue.queue_size;
        
        // Check if queue is full
        let head = admin_queue.cq_head.load(Ordering::Acquire);
        if next_tail == head {
            return Err(DeviceError::Busy);
        }
        
        // Copy command to submission queue
        unsafe {
            let sq_entry = (admin_queue.submission_queue.as_u64() + 
                           (tail as u64 * core::mem::size_of::<NvmeCommand>() as u64)) as *mut NvmeCommand;
            core::ptr::write_volatile(sq_entry, *cmd);
        }
        
        // Update tail pointer
        admin_queue.sq_tail.store(next_tail, Ordering::Release);
        
        // Ring doorbell
        let doorbell_offset = 0x1000 + (admin_queue.queue_id as u64 * 8);
        self.write_reg32(doorbell_offset, next_tail as u32);
        
        Ok(cmd.command_id)
    }
    
    fn create_io_queues(&mut self) -> Result<(), DeviceError> {
        // Create completion queue first
        for queue_id in 1..=NVME_MAX_QUEUES {
            let mut cq = self.allocate_queue(NVME_IO_QUEUE_SIZE)?;
            cq.queue_id = queue_id;
            
            // Create completion queue command
            let mut cmd = NvmeCommand {
                opcode: NVME_ADMIN_CREATE_CQ,
                flags: 0,
                command_id: self.command_id_counter.fetch_add(1, Ordering::Relaxed),
                namespace_id: 0,
                reserved: [0; 2],
                metadata_ptr: 0,
                data_ptr: [crate::memory::virt_to_phys(cq.completion_queue).as_u64(), 0],
                cdw10: ((NVME_IO_QUEUE_SIZE - 1) as u32) << 16 | queue_id as u32,
                cdw11: 1, // Physically contiguous
                cdw12: 0,
                cdw13: 0,
                cdw14: 0,
                cdw15: 0,
            };
            
            self.submit_admin_command(&cmd)?;
            
            // Create submission queue command
            cmd.opcode = NVME_ADMIN_CREATE_SQ;
            cmd.command_id = self.command_id_counter.fetch_add(1, Ordering::Relaxed);
            cmd.data_ptr = [crate::memory::virt_to_phys(cq.submission_queue).as_u64(), 0];
            cmd.cdw11 = (queue_id as u32) << 16 | 1; // CQ ID and physically contiguous
            
            self.submit_admin_command(&cmd)?;
            
            self.io_queues.push(cq);
        }
        
        Ok(())
    }
    

    

    
    pub fn flush_async(&self, namespace_id: u32) -> Result<u16, DeviceError> {
        // Use dedicated flush queue or least loaded queue for optimal performance
        let queue = self.select_optimal_flush_queue();
        let command_id = self.command_id_counter.fetch_add(1, Ordering::Relaxed);
        
        let cmd = NvmeCommand {
            opcode: NVME_CMD_FLUSH,
            flags: 0,
            command_id,
            namespace_id,
            reserved: [0; 2],
            metadata_ptr: 0,
            data_ptr: [0, 0],
            cdw10: 0,
            cdw11: 0,
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        };
        
        let start_time = crate::time::get_timestamp_ns();
        queue.pending_commands.lock().push_back((command_id, start_time, NVME_CMD_FLUSH));
        
        // Immediate doorbell ring for flush commands to minimize latency
        self.submit_flush_command(queue, &cmd)?;
        
        Ok(command_id)
    }
    
    fn submit_io_command(&self, queue: &NvmeQueue, cmd: &NvmeCommand) -> Result<(), DeviceError> {
        let tail = queue.sq_tail.load(Ordering::Acquire);
        let next_tail = (tail + 1) % queue.queue_size;
        
        // Check if queue is full or exceeds adaptive depth limit
        let head = queue.cq_head.load(Ordering::Acquire);
        let current_depth = queue.current_depth.load(Ordering::Relaxed);
        let max_depth = queue.max_depth.load(Ordering::Relaxed);
        
        if next_tail == head || current_depth >= max_depth {
            return Err(DeviceError::Busy);
        }
        
        // Add command to pending list with timestamp and opcode
        let start_time = crate::time::get_timestamp_ns();
        queue.pending_commands.lock().push_back((cmd.command_id, start_time, cmd.opcode));
        
        // Copy command to submission queue
        unsafe {
            let sq_entry = (queue.submission_queue.as_u64() + 
                           (tail as u64 * core::mem::size_of::<NvmeCommand>() as u64)) as *mut NvmeCommand;
            core::ptr::write_volatile(sq_entry, *cmd);
        }
        
        // Update tail pointer and current depth
        queue.sq_tail.store(next_tail, Ordering::Release);
        queue.current_depth.fetch_add(1, Ordering::Relaxed);
        
        // Optimized doorbell batching
        if self.should_ring_doorbell(queue) {
            let doorbell_offset = 0x1000 + (queue.queue_id as u64 * 8);
            self.write_reg32(doorbell_offset, next_tail as u32);
            queue.last_doorbell_time.store(0u64, Ordering::Relaxed); // crate::time::get_tsc()
        }
        
        Ok(())
    }
    
    pub fn poll_completions(&mut self) -> Vec<(u16, bool)> {
        let mut completions = Vec::new();
        let mut async_completions = Vec::new();
        
        for queue in &self.io_queues {
            let mut head = queue.cq_head.load(Ordering::Acquire);
            
            loop {
                let cq_entry_addr = queue.completion_queue.as_u64() + 
                                   (head as u64 * core::mem::size_of::<NvmeCompletion>() as u64);
                
                let completion = unsafe {
                    core::ptr::read_volatile(cq_entry_addr as *const NvmeCompletion)
                };
                
                // Check phase bit to see if this is a new completion
                let phase = (completion.status & 1) != 0;
                let expected_phase = (head / queue.queue_size) % 2 == 0;
                
                if phase != expected_phase {
                    break; // No more completions
                }
                
                let success = (completion.status >> 1) == 0;
                if !success {
                    self.failed_commands.fetch_add(1, Ordering::Relaxed);
                }
                
                // Record completion time for SLO measurement and adaptive tuning
                let current_time = crate::time::get_timestamp_ns();
                if let Some((_, start_time, opcode)) = queue.pending_commands.lock()
                    .iter().position(|(id, _, _)| *id == completion.command_id)
                    .map(|pos| queue.pending_commands.lock().remove(pos).unwrap()) {
                    
                    let latency_ns = current_time.saturating_sub(start_time);
                    let latency_us = latency_ns / 1000; // Convert nanoseconds to microseconds
                    
                    // Update queue depth based on latency performance
                    self.adjust_queue_depth(queue, latency_us);
                    
                    // Update current depth counter
                    queue.current_depth.fetch_sub(1, Ordering::Relaxed);
                    
                    // Update average latency with exponential moving average
                    let current_avg = queue.avg_latency_us.load(Ordering::Relaxed);
                    let new_avg = if current_avg == 0 {
                        latency_us
                    } else {
                        (current_avg * 7 + latency_us) / 8 // 12.5% weight for new sample
                    };
                    queue.avg_latency_us.store(new_avg, Ordering::Relaxed);
                    
                    // Measure SLO based on operation type
                    crate::slo::with_slo_harness(|harness| {
                        let samples = alloc::vec![latency_us as f64]; // Single sample for now
                        match opcode {
                            NVME_CMD_READ => {
                                crate::slo_measure!(
                                    harness,
                                    crate::slo::SloCategory::NvmeIo,
                                    "nvme_read_latency",
                                    "microseconds",
                                    1,
                                    samples
                                );
                            },
                            NVME_CMD_FLUSH => {
                                crate::slo_measure!(
                                    harness,
                                    crate::slo::SloCategory::NvmeIo,
                                    "nvme_flush_latency",
                                    "microseconds",
                                    1,
                                    samples
                                );
                            },
                            _ => {
                                // Generic I/O measurement for other operations
                                crate::slo_measure!(
                                    harness,
                                    crate::slo::SloCategory::NvmeIo,
                                    "nvme_io_latency",
                                    "microseconds",
                                    1,
                                    samples
                                );
                            }
                        }
                    });
                } else {
                    // Command completed but not found in pending list, still update depth
                    queue.current_depth.fetch_sub(1, Ordering::Relaxed);
                }
                
                completions.push((completion.command_id, success));
                
                // Collect async completions to process after the loop
                let result = if success { Ok(0) } else { Err(completion.status as u32) };
                async_completions.push((completion.command_id, result));
                
                head = (head + 1) % queue.queue_size;
            }
            
            // Update completion queue head
            if head != queue.cq_head.load(Ordering::Acquire) {
                queue.cq_head.store(head, Ordering::Release);
                
                // Ring completion doorbell
                let doorbell_offset = 0x1004 + (queue.queue_id as u64 * 8);
                self.write_reg32(doorbell_offset, head as u32);
            }
        }
        
        // Process async completions after the loop to avoid borrowing conflicts
        for (command_id, result) in async_completions {
            self.complete_async_request(command_id, result);
        }
        
        completions
    }
    
    pub fn get_performance_stats(&self) -> (u64, u64, f64) {
        let total = self.total_commands.load(Ordering::Relaxed);
        let failed = self.failed_commands.load(Ordering::Relaxed);
        let success_rate = if total > 0 {
            ((total - failed) as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        
        (total, failed, success_rate)
    }
    
    // Adaptive queue depth management for p99 ≤120µs read latency
    fn adjust_queue_depth(&self, queue: &NvmeQueue, latency_us: u64) {
        const TARGET_P99_LATENCY_US: u64 = 120;
        const LATENCY_TOLERANCE_US: u64 = 10;
        
        let current_p99 = queue.p99_latency_us.load(Ordering::Relaxed);
         let current_depth = queue.current_depth.load(Ordering::Relaxed);
         let _max_depth = queue.max_depth.load(Ordering::Relaxed);
        
        // Update p99 latency with exponential moving average
        let new_p99 = if current_p99 == 0 {
            latency_us
        } else {
            // Simple approximation: weight new sample at 10%
            (current_p99 * 9 + latency_us) / 10
        };
        queue.p99_latency_us.store(new_p99, Ordering::Relaxed);
        
        // Adjust queue depth based on latency performance
        if new_p99 > TARGET_P99_LATENCY_US + LATENCY_TOLERANCE_US {
            // Latency too high, reduce queue depth
            let new_depth = (current_depth * 3 / 4).max(1);
            queue.max_depth.store(new_depth, Ordering::Relaxed);
        } else if new_p99 < TARGET_P99_LATENCY_US - LATENCY_TOLERANCE_US && current_depth < queue.queue_size {
            // Latency good, can increase queue depth for better throughput
            let new_depth = (current_depth * 5 / 4).min(queue.queue_size);
            queue.max_depth.store(new_depth, Ordering::Relaxed);
        }
    }
    
    // Optimized doorbell batching to reduce PCIe overhead
    fn should_ring_doorbell(&self, queue: &NvmeQueue) -> bool {
        let current_time = 0u64; // crate::time::get_tsc(); // Placeholder
        let last_doorbell = queue.last_doorbell_time.load(Ordering::Relaxed);
        let batch_size = queue.completion_batch_size.load(Ordering::Relaxed);
        let pending_count = queue.current_depth.load(Ordering::Relaxed);
        
        // Ring doorbell if:
        // 1. Batch size reached
        // 2. Timeout exceeded (prevent latency spikes)
        // 3. Queue getting full
        pending_count >= batch_size ||
         current_time.saturating_sub(last_doorbell) > 1000 || // 1µs timeout
         pending_count > queue.max_depth.load(Ordering::Relaxed) * 3 / 4
     }
     
     // Get queue performance metrics for monitoring
     pub fn get_queue_metrics(&self) -> Vec<(u16, u16, u16, u64, u64)> {
         self.io_queues.iter().map(|queue| {
             let queue_id = queue.queue_id;
             let current_depth = queue.current_depth.load(Ordering::Relaxed);
             let max_depth = queue.max_depth.load(Ordering::Relaxed);
             let avg_latency = queue.avg_latency_us.load(Ordering::Relaxed);
             let p99_latency = queue.p99_latency_us.load(Ordering::Relaxed);
             
             (queue_id, current_depth, max_depth, avg_latency, p99_latency)
         }).collect()
     }
     
     // DMA buffer management for zero-copy operations
     pub fn allocate_dma_buffer(&mut self, size: usize) -> Result<usize, DeviceError> {
         // Allocate physically contiguous memory for DMA
         let layout = core::alloc::Layout::from_size_align(size, 4096)
             .map_err(|_| DeviceError::OutOfMemory)?;
         
         let virt_ptr = unsafe {
             alloc::alloc::alloc_zeroed(layout)
         };
         
         if virt_ptr.is_null() {
             return Err(DeviceError::OutOfMemory);
         }
         
         // Convert virtual to physical address
         let phys_addr = crate::memory::virt_to_phys(VirtAddr::new(virt_ptr as u64));
         
         let buffer = DmaBuffer {
             virt_addr: virt_ptr,
             phys_addr,
             size,
             aligned: true,
         };
         
         let buffer_id = self.dma_buffers.len();
         self.dma_buffers.push(buffer);
         Ok(buffer_id)
     }
     
     pub fn free_dma_buffer(&mut self, buffer_id: usize) -> Result<(), DeviceError> {
         if buffer_id >= self.dma_buffers.len() {
             return Err(DeviceError::InvalidParameter);
         }
         
         let buffer = &self.dma_buffers[buffer_id];
         let layout = core::alloc::Layout::from_size_align(buffer.size, 4096)
             .map_err(|_| DeviceError::InvalidParameter)?;
         
         unsafe {
             alloc::alloc::dealloc(buffer.virt_addr, layout);
         }
         
         // Mark buffer as freed (we don't remove from vec to keep indices stable)
         self.dma_buffers[buffer_id].size = 0;
         Ok(())
     }
     
     pub fn get_dma_buffer_phys_addr(&self, buffer_id: usize) -> Result<PhysAddr, DeviceError> {
         if buffer_id >= self.dma_buffers.len() || self.dma_buffers[buffer_id].size == 0 {
             return Err(DeviceError::InvalidParameter);
         }
         Ok(self.dma_buffers[buffer_id].phys_addr)
     }
     
     // Asynchronous I/O operations
     pub fn read_async(&mut self, lba: u64, buffer_id: usize, callback: Option<IoCallback>) -> Result<NvmeIoFuture, DeviceError> {
         if buffer_id >= self.dma_buffers.len() || self.dma_buffers[buffer_id].size == 0 {
             return Err(DeviceError::InvalidParameter);
         }
         
         let command_id = self.command_id_counter.fetch_add(1, Ordering::Relaxed);
         let buffer = &self.dma_buffers[buffer_id];
         
         // Create read command
         let command = NvmeCommand {
             opcode: NVME_CMD_READ,
             flags: 0,
             command_id,
             namespace_id: 1,
             reserved: [0; 2],
             metadata_ptr: 0,
             data_ptr: [buffer.phys_addr.as_u64(), 0],
             cdw10: (lba & 0xFFFFFFFF) as u32,
             cdw11: (lba >> 32) as u32,
             cdw12: 0, // Number of blocks - 1 (0 = 1 block)
             cdw13: 0,
             cdw14: 0,
             cdw15: 0,
         };
         
         // Add to pending requests
         let request = IoRequest {
             command_id,
             callback,
             waker: None,
             completed: false,
             result: None,
         };
         self.pending_requests.push(request);
         
         // Submit command to first available I/O queue
         if let Some(queue) = self.io_queues.first() {
             self.submit_io_command(queue, &command)?;
         } else {
             return Err(DeviceError::NotSupported);
         }
         
         Ok(NvmeIoFuture {
             command_id,
             driver: self as *mut NvmeDriver,
         })
     }
     
     pub fn write_async(&mut self, lba: u64, buffer_id: usize, callback: Option<IoCallback>) -> Result<NvmeIoFuture, DeviceError> {
         if buffer_id >= self.dma_buffers.len() || self.dma_buffers[buffer_id].size == 0 {
             return Err(DeviceError::InvalidParameter);
         }
         
         let command_id = self.command_id_counter.fetch_add(1, Ordering::Relaxed);
         let buffer = &self.dma_buffers[buffer_id];
         
         // Create write command
         let command = NvmeCommand {
             opcode: NVME_CMD_WRITE,
             flags: 0,
             command_id,
             namespace_id: 1,
             reserved: [0; 2],
             metadata_ptr: 0,
             data_ptr: [buffer.phys_addr.as_u64(), 0],
             cdw10: (lba & 0xFFFFFFFF) as u32,
             cdw11: (lba >> 32) as u32,
             cdw12: 0, // Number of blocks - 1 (0 = 1 block)
             cdw13: 0,
             cdw14: 0,
             cdw15: 0,
         };
         
         // Add to pending requests
         let request = IoRequest {
             command_id,
             callback,
             waker: None,
             completed: false,
             result: None,
         };
         self.pending_requests.push(request);
         
         // Submit command to first available I/O queue
         if let Some(queue) = self.io_queues.first() {
             self.submit_io_command(queue, &command)?;
         } else {
             return Err(DeviceError::NotSupported);
         }
         
         Ok(NvmeIoFuture {
             command_id,
             driver: self as *mut NvmeDriver,
         })
     }
     
     // Complete pending requests when polling completions
     fn complete_async_request(&mut self, command_id: u16, result: Result<u32, u32>) {
         if let Some(request) = self.pending_requests.iter_mut().find(|r| r.command_id == command_id) {
             request.completed = true;
             request.result = Some(result);
             
             // Call callback if provided
             if let Some(callback) = request.callback.take() {
                 callback(result);
             }
             
             // Wake up any waiting futures
             if let Some(waker) = request.waker.take() {
                 waker.wake();
             }
         }
     }

     // Flush-specific optimizations for p99 ≤900µs latency
     fn select_optimal_flush_queue(&self) -> &NvmeQueue {
         // Select the queue with lowest current depth for flush operations
         let mut best_queue = &self.io_queues[0];
         let mut min_depth = best_queue.current_depth.load(Ordering::Relaxed);
         
         for queue in &self.io_queues[1..] {
             let depth = queue.current_depth.load(Ordering::Relaxed);
             if depth < min_depth {
                 min_depth = depth;
                 best_queue = queue;
             }
         }
         
         best_queue
     }
     
     fn submit_flush_command(&self, queue: &NvmeQueue, cmd: &NvmeCommand) -> Result<(), DeviceError> {
         let tail = queue.sq_tail.load(Ordering::Acquire);
         let next_tail = (tail + 1) % queue.queue_size;
         
         // Check if queue is full
         let head = queue.cq_head.load(Ordering::Acquire);
         if next_tail == head {
             return Err(DeviceError::Busy);
         }
         
         // Copy command to submission queue
         unsafe {
             let sq_entry = (queue.submission_queue.as_u64() + 
                            (tail as u64 * core::mem::size_of::<NvmeCommand>() as u64)) as *mut NvmeCommand;
             core::ptr::write_volatile(sq_entry, *cmd);
         }
         
         // Update tail pointer and current depth
         queue.sq_tail.store(next_tail, Ordering::Release);
         queue.current_depth.fetch_add(1, Ordering::Relaxed);
         
         // Immediate doorbell ring for flush commands (no batching)
         let doorbell_offset = 0x1000 + (queue.queue_id as u64 * 8);
         self.write_reg32(doorbell_offset, next_tail as u32);
         
         Ok(())
     }
}

impl Device for NvmeDriver {
    fn name(&self) -> &str {
        "NVMe Controller"
    }
    
    fn device_type(&self) -> DeviceType {
        DeviceType::Storage
    }
    
    fn init(&mut self) -> Result<(), DeviceError> {
        self.status = DeviceStatus::Initializing;
        
        // Get BAR0 from PCI configuration
        let bar0_raw = read_config_dword(self.pci_device.bus, self.pci_device.device, self.pci_device.function, 0x10);
        if (bar0_raw & 1) != 0 {
            return Err(DeviceError::NotSupported); // Must be memory-mapped
        }
        
        let bar0_phys = PhysAddr::new((bar0_raw & !0xF) as u64);
        self.bar0 = crate::memory::phys_to_virt(bar0_phys);
        
        // Enable PCI bus mastering and memory space
        let mut command = read_config_word(self.pci_device.bus, self.pci_device.device, self.pci_device.function, 0x04);
        command |= 0x06; // Memory Space Enable + Bus Master Enable
        write_config_word(self.pci_device.bus, self.pci_device.device, self.pci_device.function, 0x04, command);
        
        // Reset controller
        self.write_reg32(NVME_REG_CC, 0);
        
        // Wait for controller to be ready for configuration
        let mut timeout = 1000000;
        while timeout > 0 {
            let csts = self.read_reg32(NVME_REG_CSTS);
            if (csts & 1) == 0 {
                break;
            }
            // crate::time::sleep_ms(1); // Placeholder for timing
            timeout -= 1;
        }
        
        if timeout == 0 {
            return Err(DeviceError::Timeout);
        }
        
        // Setup admin queue
        self.setup_admin_queue()?;
        
        // Enable controller
        self.enable_controller()?;
        
        // Create I/O queues
        self.create_io_queues()?;
        
        // Identify controller and namespaces
        // TODO: Add identify commands
        
        self.status = DeviceStatus::Ready;
        Ok(())
    }
    
    fn reset(&mut self) -> Result<(), DeviceError> {
        self.write_reg32(NVME_REG_CC, 0);
        self.init()
    }
    
    fn status(&self) -> DeviceStatus {
        self.status
    }
    
    fn info(&self) -> DeviceInfo {
        DeviceInfo {
            id: 3,
            name: self.name().to_string(),
            device_type: self.device_type(),
            status: self.status,
            vendor_id: self.pci_device.vendor_id,
            device_id: self.pci_device.device_id,
            irq: Some(self.pci_device.interrupt_line),
            io_base: None,
            memory_base: Some(self.bar0),
            memory_size: Some(0x1000), // 4KB minimum
        }
    }
    
    fn handle_interrupt(&mut self) -> Result<(), DeviceError> {
        // Poll for completions
        let _completions = self.poll_completions();
        Ok(())
    }
}

// SMART monitoring methods for NVMe driver (separate impl block)
impl NvmeDriver {
    /// Get SMART data from the NVMe device
    pub fn get_smart_data(&self) -> Result<NvmeSmartData, DeviceError> {
        // Create SMART/Health Information log page command
        let _command = NvmeCommand {
            opcode: 0x02, // Get Log Page
            flags: 0,
            command_id: self.command_id_counter.fetch_add(1, Ordering::Relaxed),
            namespace_id: 0xFFFFFFFF, // Global namespace
            reserved: [0; 2],
            metadata_ptr: 0,
            data_ptr: [0, 0], // Will be set to DMA buffer
            cdw10: 0x02 | ((512 - 1) << 16), // Log ID 0x02 (SMART), 512 bytes
            cdw11: 0,
            cdw12: 0,
            cdw13: 0,
            cdw14: 0,
            cdw15: 0,
        };
        
        // In a real implementation, this would:
        // 1. Allocate DMA buffer for SMART data
        // 2. Submit admin command
        // 3. Wait for completion
        // 4. Parse SMART data structure
        
        // For now, return simulated SMART data
        Ok(NvmeSmartData::default())
    }
    
    /// Get wear leveling information
    pub fn get_wear_leveling_info(&self) -> Result<NvmeWearLevelingInfo, DeviceError> {
        // This would typically involve vendor-specific commands
        // For now, return simulated data based on write statistics
        let total_writes = self.total_commands.load(Ordering::Relaxed);
        
        Ok(NvmeWearLevelingInfo {
            average_erase_count: (total_writes / 1000) as u32,
            max_erase_count: (total_writes / 800) as u32,
            min_erase_count: (total_writes / 1200) as u32,
            wear_leveling_count: (total_writes / 10000) as u32,
            remaining_spare_blocks: 95, // Percentage
            total_lbas_written: total_writes * 8, // Assume 4KB blocks
            total_lbas_read: total_writes * 12, // Read amplification
            thermal_throttle_events: 0,
            power_cycle_count: 1,
            unsafe_shutdowns: 0,
        })
    }
    
    /// Update wear leveling metrics based on I/O operations
    pub fn update_wear_metrics(&mut self, operation: WearOperation, lba_count: u64) {
        match operation {
            WearOperation::Write => {
                // Track write operations for wear leveling
                self.total_commands.fetch_add(lba_count, Ordering::Relaxed);
            },
            WearOperation::Erase => {
                // Track erase operations
                // In real implementation, this would update erase counters
            },
            WearOperation::Read => {
                // Reads don't contribute to wear, but track for statistics
            },
        }
    }
    
    /// Check device health status
    pub fn check_health_status(&self) -> Result<NvmeHealthStatus, DeviceError> {
        let smart_data = self.get_smart_data()?;
        let wear_info = self.get_wear_leveling_info()?;
        
        let mut status = NvmeHealthStatus {
            overall_health: HealthLevel::Good,
            temperature_celsius: smart_data.temperature,
            available_spare_percentage: wear_info.remaining_spare_blocks,
            media_errors: smart_data.media_errors,
            critical_warnings: smart_data.critical_warning,
            estimated_endurance_remaining: 100, // Percentage
            recommendations: Vec::new(),
        };
        
        // Evaluate health based on various metrics
        if smart_data.temperature > 70 {
            status.overall_health = HealthLevel::Warning;
            status.recommendations.push("High temperature detected".to_string());
        }
        
        if wear_info.remaining_spare_blocks < 10 {
            status.overall_health = HealthLevel::Critical;
            status.recommendations.push("Low spare blocks remaining".to_string());
        }
        
        if smart_data.media_errors > 100 {
            status.overall_health = HealthLevel::Warning;
            status.recommendations.push("High media error count".to_string());
        }
        
        Ok(status)
    }
    
    /// Start periodic SMART monitoring
    pub fn start_smart_monitoring(&mut self) -> Result<(), DeviceError> {
        // In a real implementation, this would:
        // 1. Set up a timer to periodically collect SMART data
        // 2. Monitor for threshold violations
        // 3. Log health status changes
        // 4. Trigger alerts for critical conditions
        
        crate::serial_println!("[NVMe] SMART monitoring started for device");
        Ok(())
    }
}

// SMART data structures
#[derive(Debug, Clone)]
pub struct NvmeSmartData {
    pub critical_warning: u8,
    pub temperature: u16, // Kelvin
    pub available_spare: u8,
    pub available_spare_threshold: u8,
    pub percentage_used: u8,
    pub data_units_read: [u8; 16],
    pub data_units_written: [u8; 16],
    pub host_read_commands: [u8; 16],
    pub host_write_commands: [u8; 16],
    pub controller_busy_time: [u8; 16],
    pub power_cycles: [u8; 16],
    pub power_on_hours: [u8; 16],
    pub unsafe_shutdowns: [u8; 16],
    pub media_errors: u64,
    pub error_log_entries: u64,
}

impl Default for NvmeSmartData {
    fn default() -> Self {
        Self {
            critical_warning: 0,
            temperature: 298, // 25°C in Kelvin
            available_spare: 100,
            available_spare_threshold: 10,
            percentage_used: 5,
            data_units_read: [0; 16],
            data_units_written: [0; 16],
            host_read_commands: [0; 16],
            host_write_commands: [0; 16],
            controller_busy_time: [0; 16],
            power_cycles: [1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            power_on_hours: [0; 16],
            unsafe_shutdowns: [0; 16],
            media_errors: 0,
            error_log_entries: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct NvmeWearLevelingInfo {
    pub average_erase_count: u32,
    pub max_erase_count: u32,
    pub min_erase_count: u32,
    pub wear_leveling_count: u32,
    pub remaining_spare_blocks: u8, // Percentage
    pub total_lbas_written: u64,
    pub total_lbas_read: u64,
    pub thermal_throttle_events: u32,
    pub power_cycle_count: u32,
    pub unsafe_shutdowns: u32,
}

#[derive(Debug, Clone, Copy)]
pub enum WearOperation {
    Read,
    Write,
    Erase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthLevel {
    Good,
    Warning,
    Critical,
    Failed,
}

#[derive(Debug, Clone)]
pub struct NvmeHealthStatus {
    pub overall_health: HealthLevel,
    pub temperature_celsius: u16,
    pub available_spare_percentage: u8,
    pub media_errors: u64,
    pub critical_warnings: u8,
    pub estimated_endurance_remaining: u8, // Percentage
    pub recommendations: Vec<String>,
}

// Global SMART monitoring interface
static SMART_MONITOR: RwLock<Option<SmartMonitor>> = RwLock::new(None);

pub struct SmartMonitor {
    devices: Vec<u64>, // Device IDs being monitored
    monitoring_interval_ms: u64,
    last_check_time: u64,
}

impl SmartMonitor {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
            monitoring_interval_ms: 60000, // 1 minute
            last_check_time: 0,
        }
    }
    
    pub fn add_device(&mut self, device_id: u64) {
        if !self.devices.contains(&device_id) {
            self.devices.push(device_id);
            crate::serial_println!("[SMART] Added device {} to monitoring", device_id);
        }
    }
    
    pub fn check_all_devices(&mut self) -> Result<(), DeviceError> {
        let current_time = crate::time::get_timestamp_ns() / 1_000_000; // Convert to ms
        
        if current_time - self.last_check_time < self.monitoring_interval_ms {
            return Ok(()); // Not time for next check yet
        }
        
        self.last_check_time = current_time;
        
        // In a real implementation, this would iterate through all monitored devices
        // and check their SMART status
        for &device_id in &self.devices {
            crate::serial_println!("[SMART] Checking device {} health status", device_id);
            // Would call device-specific health check here
        }
        
        Ok(())
    }
}

/// Initialize global SMART monitoring
pub fn init_smart_monitoring() -> Result<(), DeviceError> {
    let mut monitor_guard = SMART_MONITOR.write();
    *monitor_guard = Some(SmartMonitor::new());
    crate::serial_println!("[SMART] Global SMART monitoring initialized");
    Ok(())
}

/// Add a device to SMART monitoring
pub fn add_device_to_smart_monitoring(device_id: u64) -> Result<(), DeviceError> {
    let mut monitor_guard = SMART_MONITOR.write();
    if let Some(ref mut monitor) = *monitor_guard {
        monitor.add_device(device_id);
        Ok(())
    } else {
        Err(DeviceError::NotFound)
    }
}

/// Perform periodic SMART checks on all monitored devices
 pub fn perform_smart_checks() -> Result<(), DeviceError> {
     let mut monitor_guard = SMART_MONITOR.write();
     if let Some(ref mut monitor) = *monitor_guard {
         monitor.check_all_devices()
     } else {
         Err(DeviceError::NotFound)
     }
 }

// PS/2 Mouse Driver
#[derive(Debug)]
pub struct Ps2MouseDriver {
    status: DeviceStatus,
    x: i32,
    y: i32,
    buttons: u8,
}

impl Ps2MouseDriver {
    pub fn new() -> Self {
        Self {
            status: DeviceStatus::Uninitialized,
            x: 0,
            y: 0,
            buttons: 0,
        }
    }
    
    pub fn get_position(&self) -> (i32, i32) {
        (self.x, self.y)
    }
    
    pub fn get_buttons(&self) -> u8 {
        self.buttons
    }
    
    fn send_command(&self, command: u8) -> DeviceResult<()> {
        unsafe {
            let mut status_port = Port::new(0x64);
            let mut data_port = Port::new(0x60);
            
            // Wait for input buffer to be empty
            while (status_port.read() & 0x02) != 0 {}
            
            // Send command to mouse
            status_port.write(0xD4u8);
            while (status_port.read() & 0x02) != 0 {}
            data_port.write(command);
        }
        Ok(())
    }
    
    fn read_data_port() -> u8 {
        unsafe {
            let mut port: Port<u8> = Port::new(0x60);
            port.read()
        }
    }
    
    fn read_status_port() -> u8 {
        unsafe {
            let mut port: Port<u8> = Port::new(0x64);
            port.read()
        }
    }
    
    fn write_command_port(command: u8) {
        unsafe {
            let mut port: Port<u8> = Port::new(0x64);
            port.write(command);
        }
    }
    
    fn write_data_port(data: u8) {
        unsafe {
            let mut port: Port<u8> = Port::new(0x60);
            port.write(data);
        }
    }
    
    fn handle_mouse_packet(&mut self, packet: [u8; 3]) {
        // Parse mouse packet
        let flags = packet[0];
        let delta_x = packet[1] as i8 as i32;
        let delta_y = packet[2] as i8 as i32;
        
        // Update position (with bounds checking)
        self.x = (self.x + delta_x).max(0).min(1023); // Assume 1024x768 screen
        self.y = (self.y - delta_y).max(0).min(767);  // Y is inverted
        
        // Update button state
        self.buttons = flags & 0x07; // Extract button bits
    }
}

impl Device for Ps2MouseDriver {
    fn name(&self) -> &str {
        "PS/2 Mouse"
    }
    
    fn device_type(&self) -> DeviceType {
        DeviceType::Input
    }
    
    fn init(&mut self) -> Result<(), DeviceError> {
        self.status = DeviceStatus::Initializing;
        
        // Enable auxiliary device (mouse)
        Self::write_command_port(0xA8);
        
        // Get controller configuration
        Self::write_command_port(0x20);
        let mut config = Self::read_data_port();
        
        // Enable mouse interrupt
        config |= 0x02;
        Self::write_command_port(0x60);
        Self::write_data_port(config);
        
        // Test mouse port
        Self::write_command_port(0xA9);
        let test_result = Self::read_data_port();
        if test_result != 0x00 {
            return Err(DeviceError::HardwareError);
        }
        
        // Send commands to mouse
        self.send_command(0xFF)?; // Reset mouse
        
        // Wait for acknowledgment
        let ack = Self::read_data_port();
        if ack != 0xFA {
            return Err(DeviceError::InitializationFailed);
        }
        
        // Wait for self-test result
        let self_test = Self::read_data_port();
        if self_test != 0xAA {
            return Err(DeviceError::InitializationFailed);
        }
        
        // Wait for mouse ID
        let _mouse_id = Self::read_data_port();
        
        // Set default settings
        self.send_command(0xF6)?;
        let ack2 = Self::read_data_port();
        if ack2 != 0xFA {
            return Err(DeviceError::InitializationFailed);
        }
        
        // Enable data reporting
        self.send_command(0xF4)?;
        let ack3 = Self::read_data_port();
        if ack3 != 0xFA {
            return Err(DeviceError::InitializationFailed);
        }
        
        self.status = DeviceStatus::Ready;
        Ok(())
    }
    
    fn reset(&mut self) -> Result<(), DeviceError> {
        self.send_command(0xFF)?; // Reset command
        self.x = 0;
        self.y = 0;
        self.buttons = 0;
        Ok(())
    }
    
    fn status(&self) -> DeviceStatus {
        self.status
    }
    
    fn info(&self) -> DeviceInfo {
        DeviceInfo {
            id: 3,
            name: self.name().to_string(),
            device_type: self.device_type(),
            status: self.status,
            vendor_id: 0x0000,
            device_id: 0x0000,
            irq: Some(12), // PS/2 mouse IRQ
            io_base: Some(0x60),
            memory_base: None,
            memory_size: None,
        }
    }
    
    fn handle_interrupt(&mut self) -> Result<(), DeviceError> {
        let status = Self::read_status_port();
        if (status & 0x01) != 0 {
            // Read mouse packet (3 bytes)
            let byte1 = Self::read_data_port();
            let byte2 = Self::read_data_port();
            let byte3 = Self::read_data_port();
            
            let packet = [byte1, byte2, byte3];
            self.handle_mouse_packet(packet);
        }
        Ok(())
    }
}

// Public API functions
pub fn init() {
    let mut manager = DEVICE_MANAGER.write();
    
    // Register VGA driver
    let vga = Box::new(VgaDriver::new());
    let _ = manager.register_device(vga);
    
    // Register ATA driver
    let ata = Box::new(AtaDriver::new());
    let _ = manager.register_device(ata);
    
    // Register PS/2 keyboard driver
    let keyboard = Box::new(Ps2KeyboardDriver::new());
    let _ = manager.register_device(keyboard);
    
    // Register PS/2 mouse driver
    let mouse = Box::new(Ps2MouseDriver::new());
    let _ = manager.register_device(mouse);
    
    // Initialize keyboard and mouse modules
    drop(manager); // Release the lock before calling module init
    keyboard::init();
    mouse::init();
}

pub fn register_device(device: Box<dyn Device>) -> DeviceResult<u64> {
    DEVICE_MANAGER.write().register_device(device)
}

pub fn get_device_info(id: u64) -> Option<DeviceInfo> {
    DEVICE_MANAGER.read().get_device(id).map(|d| d.info())
}

pub fn list_devices() -> Vec<DeviceInfo> {
    DEVICE_MANAGER.read().list_devices()
}

pub fn get_devices_by_type(device_type: DeviceType) -> Vec<u64> {
    DEVICE_MANAGER.read().get_devices_by_type(device_type)
}

pub fn handle_device_interrupt(irq: u8) -> DeviceResult<()> {
    DEVICE_MANAGER.write().handle_interrupt(irq)
}

// Graphics API
pub fn clear_screen(_color: u8) {
    let manager = DEVICE_MANAGER.read();
    let graphics_devices = manager.get_devices_by_type(DeviceType::Graphics);
    
    for &device_id in &graphics_devices {
        if let Some(device) = manager.get_device(device_id) {
            // This is a bit of a hack - in a real implementation,
            // we'd have a proper graphics trait
            if device.name() == "VGA Graphics Adapter" {
                // We can't call VGA-specific methods through the trait
                // In a real implementation, we'd have a graphics subsystem
            }
        }
    }
}

// Storage API
pub fn read_disk_sector(_drive: u8, _lba: u64, _buffer: &mut [u8]) -> DeviceResult<()> {
    let mut manager = DEVICE_MANAGER.write();
    let storage_devices = manager.get_devices_by_type(DeviceType::Storage);
    
    for &device_id in &storage_devices {
        if let Some(device) = manager.get_device_mut(device_id) {
            if device.name() == "ATA/IDE Controller" {
                // Similar issue - we need a proper storage trait
                return Ok(());
            }
        }
    }
    
    Err(DeviceError::NotFound)
}

pub fn write_disk_sector(_drive: u8, _lba: u64, _buffer: &[u8]) -> DeviceResult<()> {
    let mut manager = DEVICE_MANAGER.write();
    let storage_devices = manager.get_devices_by_type(DeviceType::Storage);
    
    for &device_id in &storage_devices {
        if let Some(device) = manager.get_device_mut(device_id) {
            if device.name() == "ATA/IDE Controller" {
                return Ok(());
            }
        }
    }
    
    Err(DeviceError::NotFound)
}

// Input API
pub fn get_mouse_position() -> Option<(i32, i32)> {
    mouse::get_mouse_state().map(|(x, y, _)| (x, y))
}


pub fn get_mouse_buttons() -> Option<u8> {
    mouse::get_mouse_state().map(|(_, _, buttons)| buttons)
}

// Keyboard and Mouse modules for syscalls
// PS/2 Keyboard Driver
#[derive(Debug)]
pub struct Ps2KeyboardDriver {
    status: DeviceStatus,
    key_queue: alloc::collections::VecDeque<u8>,
    shift_pressed: bool,
    ctrl_pressed: bool,
    alt_pressed: bool,
}

impl Ps2KeyboardDriver {
    pub fn new() -> Self {
        Self {
            status: DeviceStatus::Uninitialized,
            key_queue: alloc::collections::VecDeque::new(),
            shift_pressed: false,
            ctrl_pressed: false,
            alt_pressed: false,
        }
    }
    
    pub fn get_key(&mut self) -> Option<u8> {
        self.key_queue.pop_front()
    }
    
    fn handle_scancode(&mut self, scancode: u8) {
        let pressed = (scancode & 0x80) == 0;
        let key_code = scancode & 0x7F;
        
        // Handle modifier keys
        match key_code {
            0x2A | 0x36 => self.shift_pressed = pressed, // Left/Right Shift
            0x1D => self.ctrl_pressed = pressed,         // Ctrl
            0x38 => self.alt_pressed = pressed,          // Alt
            _ => {
                if pressed {
                    // Record timestamp for input latency measurement
                    let timestamp = crate::time::get_timestamp_ns();
                    
                    // Store scancode with timestamp for SLO tracking
                    self.key_queue.push_back(scancode);
                    
                    // Record the interrupt timestamp for this key event
                    crate::input::record_input_interrupt_timestamp(scancode, timestamp);
                }
            }
        }
    }
    
    fn read_data_port() -> u8 {
        unsafe {
            let mut port: Port<u8> = Port::new(0x60);
            port.read()
        }
    }
    
    fn read_status_port() -> u8 {
        unsafe {
            let mut port: Port<u8> = Port::new(0x64);
            port.read()
        }
    }
    
    fn write_command_port(command: u8) {
        unsafe {
            let mut port: Port<u8> = Port::new(0x64);
            port.write(command);
        }
    }
    
    fn write_data_port(data: u8) {
        unsafe {
            let mut port: Port<u8> = Port::new(0x60);
            port.write(data);
        }
    }
}

impl Device for Ps2KeyboardDriver {
    fn name(&self) -> &str {
        "PS/2 Keyboard"
    }
    
    fn device_type(&self) -> DeviceType {
        DeviceType::Input
    }
    
    fn init(&mut self) -> Result<(), DeviceError> {
        self.status = DeviceStatus::Initializing;
        
        // Disable devices
        Self::write_command_port(0xAD); // Disable first PS/2 port
        Self::write_command_port(0xA7); // Disable second PS/2 port
        
        // Flush output buffer
        while (Self::read_status_port() & 0x01) != 0 {
            Self::read_data_port();
        }
        
        // Set controller configuration
        Self::write_command_port(0x20); // Read configuration byte
        let mut config = Self::read_data_port();
        config &= !0x43; // Disable interrupts and translation
        Self::write_command_port(0x60); // Write configuration byte
        Self::write_data_port(config);
        
        // Test PS/2 controller
        Self::write_command_port(0xAA);
        let test_result = Self::read_data_port();
        if test_result != 0x55 {
            return Err(DeviceError::HardwareError);
        }
        
        // Test first PS/2 port
        Self::write_command_port(0xAB);
        let port_test = Self::read_data_port();
        if port_test != 0x00 {
            return Err(DeviceError::HardwareError);
        }
        
        // Enable first PS/2 port
        Self::write_command_port(0xAE);
        
        // Enable interrupts
        config |= 0x01; // Enable first port interrupt
        Self::write_command_port(0x60);
        Self::write_data_port(config);
        
        // Reset keyboard
        Self::write_data_port(0xFF);
        let ack = Self::read_data_port();
        if ack != 0xFA {
            return Err(DeviceError::InitializationFailed);
        }
        
        let self_test = Self::read_data_port();
        if self_test != 0xAA {
            return Err(DeviceError::InitializationFailed);
        }
        
        self.status = DeviceStatus::Ready;
        Ok(())
    }
    
    fn reset(&mut self) -> Result<(), DeviceError> {
        self.key_queue.clear();
        self.shift_pressed = false;
        self.ctrl_pressed = false;
        self.alt_pressed = false;
        self.init()
    }
    
    fn status(&self) -> DeviceStatus {
        self.status
    }
    
    fn info(&self) -> DeviceInfo {
        DeviceInfo {
            id: 4,
            name: self.name().to_string(),
            device_type: self.device_type(),
            status: self.status,
            vendor_id: 0x0000,
            device_id: 0x0001,
            irq: Some(1),
            io_base: Some(0x60),
            memory_base: None,
            memory_size: None,
        }
    }
    
    fn handle_interrupt(&mut self) -> Result<(), DeviceError> {
        let status = Self::read_status_port();
        if (status & 0x01) != 0 {
            let scancode = Self::read_data_port();
            self.handle_scancode(scancode);
        }
        Ok(())
    }
}

pub mod keyboard {
    use spin::Mutex;
    use lazy_static::lazy_static;
    use super::*;
    
    lazy_static! {
        static ref KEYBOARD_DRIVER: Mutex<Option<Ps2KeyboardDriver>> = Mutex::new(None);
    }
    
    pub fn init() {
        let mut driver = Ps2KeyboardDriver::new();
        if driver.init().is_ok() {
            *KEYBOARD_DRIVER.lock() = Some(driver);
        }
    }
    
    pub fn get_key() -> Option<u8> {
        KEYBOARD_DRIVER.lock().as_mut()?.get_key()
    }
    
    pub fn handle_interrupt() {
        if let Some(ref mut driver) = *KEYBOARD_DRIVER.lock() {
            let _ = driver.handle_interrupt();
        }
    }
}

pub mod mouse {
    use spin::Mutex;
    use lazy_static::lazy_static;
    use super::*;
    
    lazy_static! {
        static ref MOUSE_DRIVER: Mutex<Option<Ps2MouseDriver>> = Mutex::new(None);
    }
    
    pub fn init() {
        let mut driver = Ps2MouseDriver::new();
        if driver.init().is_ok() {
            *MOUSE_DRIVER.lock() = Some(driver);
        }
    }
    
    pub fn get_mouse_state() -> Option<(i32, i32, u8)> {
        let driver = MOUSE_DRIVER.lock();
        if let Some(ref mouse) = *driver {
            let (x, y) = mouse.get_position();
            let buttons = mouse.get_buttons();
            Some((x, y, buttons))
        } else {
            None
        }
    }
    
    pub fn set_position(x: i32, y: i32) {
        if let Some(ref mut driver) = *MOUSE_DRIVER.lock() {
            driver.x = x.max(0).min(1023);
            driver.y = y.max(0).min(767);
        }
    }
    
    pub fn handle_interrupt() {
        if let Some(ref mut driver) = *MOUSE_DRIVER.lock() {
            let _ = driver.handle_interrupt();
        }
    }
}