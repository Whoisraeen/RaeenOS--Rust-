use alloc::collections::BTreeMap;
use alloc::string::String;
use alloc::vec::Vec;
use alloc::boxed::Box;
use core::fmt;
use spin::{Mutex, RwLock};
use x86_64::instructions::port::Port;
use x86_64::VirtAddr;

static DEVICE_MANAGER: RwLock<DeviceManager> = RwLock::new(DeviceManager::new());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        }
    }
}

pub type DeviceResult<T> = Result<T, DeviceError>;

// Device Manager
#[derive(Debug)]
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
    
    pub fn get_device_mut(&mut self, id: u64) -> Option<&mut dyn Device> {
        self.devices.get_mut(&id).map(|d| d.as_mut())
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
    bpp: u8, // bits per pixel
    status: DeviceStatus,
}

impl VgaDriver {
    pub fn new() -> Self {
        Self {
            framebuffer: VirtAddr::new(0xB8000), // VGA text mode buffer
            width: 80,
            height: 25,
            bpp: 4, // 4 bits per character (16 colors)
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
            misc_port.write(0x67);
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
    sectors: u64,
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
            let mut drive_port = Port::new(base + 6);
            let mut command_port = Port::new(base + 7);
            let mut status_port = Port::new(base + 7);
            
            // Select drive
            drive_port.write(if is_master { 0xA0 } else { 0xB0 });
            
            // Send IDENTIFY command
            command_port.write(0xEC);
            
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
                sectors,
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
            let mut features_port = Port::new(base + 1);
            let mut sector_count_port = Port::new(base + 2);
            let mut lba_low_port = Port::new(base + 3);
            let mut lba_mid_port = Port::new(base + 4);
            let mut lba_high_port = Port::new(base + 5);
            let mut drive_port = Port::new(base + 6);
            let mut command_port = Port::new(base + 7);
            let mut status_port = Port::new(base + 7);
            let mut data_port = Port::new(base);
            
            // Set up LBA addressing
            features_port.write(0);
            sector_count_port.write(1);
            lba_low_port.write((lba & 0xFF) as u8);
            lba_mid_port.write(((lba >> 8) & 0xFF) as u8);
            lba_high_port.write(((lba >> 16) & 0xFF) as u8);
            
            let drive_select = if drive_info.is_master { 0xE0 } else { 0xF0 };
            drive_port.write(drive_select | (((lba >> 24) & 0x0F) as u8));
            
            // Send READ SECTORS command
            command_port.write(0x20);
            
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
            let mut features_port = Port::new(base + 1);
            let mut sector_count_port = Port::new(base + 2);
            let mut lba_low_port = Port::new(base + 3);
            let mut lba_mid_port = Port::new(base + 4);
            let mut lba_high_port = Port::new(base + 5);
            let mut drive_port = Port::new(base + 6);
            let mut command_port = Port::new(base + 7);
            let mut status_port = Port::new(base + 7);
            let mut data_port = Port::new(base);
            
            // Set up LBA addressing
            features_port.write(0);
            sector_count_port.write(1);
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
            control_port.write(0x04); // Set reset bit
            crate::time::sleep_ms(5);
            control_port.write(0x00); // Clear reset bit
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
            status_port.write(0xD4);
            while (status_port.read() & 0x02) != 0 {}
            data_port.write(command);
        }
        Ok(())
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
        
        // Enable mouse
        self.send_command(0xF4)?;
        
        // Set sample rate
        self.send_command(0xF3)?;
        self.send_command(100)?; // 100 samples per second
        
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
        unsafe {
            let mut data_port = Port::new(0x60);
            
            // Read mouse packet (3 bytes)
            let byte1: u8 = data_port.read();
            let byte2: u8 = data_port.read();
            let byte3: u8 = data_port.read();
            
            // Update button state
            self.buttons = byte1 & 0x07;
            
            // Update position (relative movement)
            let dx = byte2 as i8 as i32;
            let dy = -(byte3 as i8 as i32); // Invert Y axis
            
            self.x += dx;
            self.y += dy;
            
            // Clamp to screen bounds (assuming 1024x768)
            self.x = self.x.max(0).min(1023);
            self.y = self.y.max(0).min(767);
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
    
    // Register PS/2 mouse driver
    let mouse = Box::new(Ps2MouseDriver::new());
    let _ = manager.register_device(mouse);
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
pub fn clear_screen(color: u8) {
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
pub fn read_disk_sector(drive: u8, lba: u64, buffer: &mut [u8]) -> DeviceResult<()> {
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

pub fn write_disk_sector(drive: u8, lba: u64, buffer: &[u8]) -> DeviceResult<()> {
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
    let manager = DEVICE_MANAGER.read();
    let input_devices = manager.get_devices_by_type(DeviceType::Input);
    
    for &device_id in &input_devices {
        if let Some(device) = manager.get_device(device_id) {
            if device.name() == "PS/2 Mouse" {
                // Return default position for now
                return Some((0, 0));
            }
        }
    }
    
    None
}


pub fn get_mouse_buttons() -> Option<u8> {
    let manager = DEVICE_MANAGER.read();
    let input_devices = manager.get_devices_by_type(DeviceType::Input);
    
    for &device_id in &input_devices {
        if let Some(device) = manager.get_device(device_id) {
            if device.name() == "PS/2 Mouse" {
                return Some(0);
            }
        }
    }
    
    None
}

// Keyboard and Mouse modules for syscalls
pub mod keyboard {
    use alloc::collections::VecDeque;
    use spin::Mutex;
    
    static KEY_QUEUE: Mutex<VecDeque<u8>> = Mutex::new(VecDeque::new());
    
    pub fn init() {}
    
    pub fn get_key() -> Option<u8> {
        KEY_QUEUE.lock().pop_front()
    }
}

pub mod mouse {
    pub fn get_mouse_state() -> Option<(i32, i32, u8)> {
        Some((0, 0, 0))
    }
}