//! PCI (Peripheral Component Interconnect) subsystem for RaeenOS
//! Implements PCI device enumeration, configuration space access, and MSI-X interrupt support

use lazy_static::lazy_static;
use spin::Mutex;
use alloc::vec::Vec;
use x86_64::instructions::port::Port;
use x86_64::{PhysAddr, VirtAddr};

use crate::apic;

/// PCI Configuration Space Access Ports
const PCI_CONFIG_ADDRESS: u16 = 0xCF8;
const PCI_CONFIG_DATA: u16 = 0xCFC;

/// PCI Configuration Space Registers
const PCI_VENDOR_ID: u8 = 0x00;
const PCI_DEVICE_ID: u8 = 0x02;
const PCI_COMMAND: u8 = 0x04;
const PCI_STATUS: u8 = 0x06;
const PCI_REVISION_ID: u8 = 0x08;
const PCI_CLASS_CODE: u8 = 0x09;
const PCI_HEADER_TYPE: u8 = 0x0E;
const PCI_BAR0: u8 = 0x10;
const PCI_CAPABILITIES_PTR: u8 = 0x34;
const PCI_INTERRUPT_LINE: u8 = 0x3C;
const PCI_INTERRUPT_PIN: u8 = 0x3D;

/// MSI-X Capability Structure
const MSIX_CAPABILITY_ID: u8 = 0x11;
const MSIX_MESSAGE_CONTROL: u8 = 0x02;
const MSIX_TABLE_OFFSET: u8 = 0x04;
const MSIX_PBA_OFFSET: u8 = 0x08;

/// MSI-X Message Control Register Bits
const MSIX_ENABLE: u16 = 1 << 15;
#[allow(dead_code)]
const MSIX_FUNCTION_MASK: u16 = 1 << 14;
const MSIX_TABLE_SIZE_MASK: u16 = 0x7FF;

/// MSI-X Table Entry Structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
struct MsixTableEntry {
    message_addr_low: u32,
    message_addr_high: u32,
    message_data: u32,
    vector_control: u32,
}

/// PCI Device Information
#[derive(Debug, Clone)]
pub struct PciDevice {
    pub bus: u8,
    pub device: u8,
    pub function: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_code: u8,
    pub subclass: u8,
    pub prog_if: u8,
    pub revision_id: u8,
    pub header_type: u8,
    pub bars: [u32; 6],
    pub interrupt_line: u8,
    pub interrupt_pin: u8,
    pub capabilities: Vec<PciCapability>,
    pub msix_info: Option<MsixInfo>,
}

/// PCI Capability Information
#[derive(Debug, Clone)]
pub struct PciCapability {
    pub id: u8,
    pub offset: u8,
    pub data: Vec<u8>,
}

/// MSI-X Capability Information
#[derive(Debug, Clone)]
pub struct MsixInfo {
    pub capability_offset: u8,
    pub table_size: u16,
    pub table_offset: u32,
    pub table_bar: u8,
    pub pba_offset: u32,
    pub pba_bar: u8,
    pub table_base: Option<VirtAddr>,
    pub pba_base: Option<VirtAddr>,
}

/// Interrupt Vector Allocation
#[derive(Debug, Copy, Clone)]
struct InterruptVector {
    vector: u8,
    allocated: bool,
    device_info: Option<(u8, u8, u8)>, // bus, device, function
}

/// PCI Manager
#[derive(Debug)]
pub struct PciManager {
    devices: Vec<PciDevice>,
    interrupt_vectors: [InterruptVector; 256],
    next_vector: u8,
}

lazy_static! {
    static ref PCI_MANAGER: Mutex<PciManager> = Mutex::new(PciManager::new());
}

impl PciManager {
    fn new() -> Self {
        let mut interrupt_vectors = [InterruptVector {
            vector: 0,
            allocated: false,
            device_info: None,
        }; 256];
        
        // Initialize vector numbers
        for (i, vector) in interrupt_vectors.iter_mut().enumerate() {
            vector.vector = i as u8;
        }
        
        Self {
            devices: Vec::new(),
            interrupt_vectors,
            next_vector: 32, // Start after legacy interrupts
        }
    }
    
    /// Enumerate all PCI devices
    pub fn enumerate_devices(&mut self) {
        crate::serial::_print(format_args!("[PCI] Starting device enumeration...\n"));
        
        for bus in 0..=255 {
            for device in 0..32 {
                for function in 0..8 {
                    if let Some(pci_device) = self.probe_device(bus, device, function) {
                        crate::serial::_print(format_args!(
                            "[PCI] Found device {:02X}:{:02X}.{} - {:04X}:{:04X} (Class: {:02X})\n",
                            bus, device, function,
                            pci_device.vendor_id, pci_device.device_id,
                            pci_device.class_code
                        ));
                        
                        self.devices.push(pci_device);
                    }
                    
                    // Only check function 0 for single-function devices
                    if function == 0 {
                        let header_type = read_config_byte(bus, device, 0, PCI_HEADER_TYPE);
                        if (header_type & 0x80) == 0 {
                            break; // Single-function device
                        }
                    }
                }
            }
        }
        
        crate::serial::_print(format_args!("[PCI] Enumeration complete. Found {} devices\n", self.devices.len()));
    }
    
    /// Probe a specific PCI device
    fn probe_device(&mut self, bus: u8, device: u8, function: u8) -> Option<PciDevice> {
        let vendor_id = read_config_word(bus, device, function, PCI_VENDOR_ID);
        
        // Check if device exists
        if vendor_id == 0xFFFF {
            return None;
        }
        
        let device_id = read_config_word(bus, device, function, PCI_DEVICE_ID);
        let class_code = read_config_byte(bus, device, function, PCI_CLASS_CODE);
        let subclass = read_config_byte(bus, device, function, PCI_CLASS_CODE + 1);
        let prog_if = read_config_byte(bus, device, function, PCI_CLASS_CODE + 2);
        let revision_id = read_config_byte(bus, device, function, PCI_REVISION_ID);
        let header_type = read_config_byte(bus, device, function, PCI_HEADER_TYPE) & 0x7F;
        let interrupt_line = read_config_byte(bus, device, function, PCI_INTERRUPT_LINE);
        let interrupt_pin = read_config_byte(bus, device, function, PCI_INTERRUPT_PIN);
        
        // Read BARs
        let mut bars = [0u32; 6];
        for i in 0..6 {
            bars[i] = read_config_dword(bus, device, function, PCI_BAR0 + (i as u8 * 4));
        }
        
        // Parse capabilities
        let capabilities = self.parse_capabilities(bus, device, function);
        
        // Check for MSI-X capability
        let msix_info = self.parse_msix_capability(bus, device, function, &capabilities);
        
        Some(PciDevice {
            bus,
            device,
            function,
            vendor_id,
            device_id,
            class_code,
            subclass,
            prog_if,
            revision_id,
            header_type,
            bars,
            interrupt_line,
            interrupt_pin,
            capabilities,
            msix_info,
        })
    }
    
    /// Parse PCI capabilities list
    fn parse_capabilities(&self, bus: u8, device: u8, function: u8) -> Vec<PciCapability> {
        let mut capabilities = Vec::new();
        
        // Check if capabilities are supported
        let status = read_config_word(bus, device, function, PCI_STATUS);
        if (status & 0x10) == 0 {
            return capabilities; // No capabilities
        }
        
        let mut cap_ptr = read_config_byte(bus, device, function, PCI_CAPABILITIES_PTR) & 0xFC;
        
        while cap_ptr != 0 {
            let cap_id = read_config_byte(bus, device, function, cap_ptr);
            let next_ptr = read_config_byte(bus, device, function, cap_ptr + 1) & 0xFC;
            
            // Read capability data (varies by capability type)
            let mut data = Vec::new();
            for i in 0..16 { // Read up to 16 bytes of capability data
                data.push(read_config_byte(bus, device, function, cap_ptr + i));
            }
            
            capabilities.push(PciCapability {
                id: cap_id,
                offset: cap_ptr,
                data,
            });
            
            cap_ptr = next_ptr;
        }
        
        capabilities
    }
    
    /// Parse MSI-X capability
    fn parse_msix_capability(&self, bus: u8, device: u8, function: u8, capabilities: &[PciCapability]) -> Option<MsixInfo> {
        for cap in capabilities {
            if cap.id == MSIX_CAPABILITY_ID {
                let message_control = read_config_word(bus, device, function, cap.offset + MSIX_MESSAGE_CONTROL);
                let table_size = (message_control & MSIX_TABLE_SIZE_MASK) + 1;
                
                let table_offset_bar = read_config_dword(bus, device, function, cap.offset + MSIX_TABLE_OFFSET);
                let table_bar = (table_offset_bar & 0x7) as u8;
                let table_offset = table_offset_bar & !0x7;
                
                let pba_offset_bar = read_config_dword(bus, device, function, cap.offset + MSIX_PBA_OFFSET);
                let pba_bar = (pba_offset_bar & 0x7) as u8;
                let pba_offset = pba_offset_bar & !0x7;
                
                return Some(MsixInfo {
                    capability_offset: cap.offset,
                    table_size,
                    table_offset,
                    table_bar,
                    pba_offset,
                    pba_bar,
                    table_base: None,
                    pba_base: None,
                });
            }
        }
        
        None
    }
    
    /// Allocate an interrupt vector
    pub fn allocate_interrupt_vector(&mut self, bus: u8, device: u8, function: u8) -> Option<u8> {
        for vector in self.next_vector..=255 {
            if !self.interrupt_vectors[vector as usize].allocated {
                self.interrupt_vectors[vector as usize].allocated = true;
                self.interrupt_vectors[vector as usize].device_info = Some((bus, device, function));
                
                if vector == self.next_vector {
                    self.next_vector = vector + 1;
                }
                
                return Some(vector);
            }
        }
        
        None
    }
    
    /// Configure MSI-X for a device
    pub fn configure_msix(&mut self, bus: u8, device: u8, function: u8, vectors_needed: u16) -> Result<Vec<u8>, &'static str> {
        let device_index = self.devices.iter().position(|d| {
            d.bus == bus && d.device == device && d.function == function
        }).ok_or("Device not found")?;
        
        // Extract needed information first to avoid borrowing conflicts
        let (table_bar_addr, pba_bar_addr, table_offset, pba_offset, _table_size, capability_offset) = {
            let pci_device = &self.devices[device_index];
            let msix_info = pci_device.msix_info.as_ref().ok_or("Device does not support MSI-X")?;
            
            if vectors_needed > msix_info.table_size {
                return Err("Requested more vectors than supported");
            }
            
            let table_bar_addr = pci_device.bars[msix_info.table_bar as usize] & !0xF;
            let pba_bar_addr = pci_device.bars[msix_info.pba_bar as usize] & !0xF;
            
            if table_bar_addr == 0 || pba_bar_addr == 0 {
                return Err("Invalid BAR addresses for MSI-X");
            }
            
            (table_bar_addr, pba_bar_addr, msix_info.table_offset, msix_info.pba_offset, 
              msix_info.table_size, msix_info.capability_offset)
        };
        
        // Map the MSI-X table (simplified - would need proper memory mapping)
        let table_phys = PhysAddr::new(table_bar_addr as u64 + table_offset as u64);
        let pba_phys = PhysAddr::new(pba_bar_addr as u64 + pba_offset as u64);
        
        // Update the device's MSI-X info
        {
            let pci_device = &mut self.devices[device_index];
            let msix_info = pci_device.msix_info.as_mut().unwrap();
            msix_info.table_base = Some(VirtAddr::new(table_phys.as_u64()));
            msix_info.pba_base = Some(VirtAddr::new(pba_phys.as_u64()));
        }
        
        // Allocate interrupt vectors
        let mut allocated_vectors = Vec::new();
        for _ in 0..vectors_needed {
            if let Some(vector) = self.allocate_interrupt_vector(bus, device, function) {
                allocated_vectors.push(vector);
            } else {
                return Err("Failed to allocate interrupt vector");
            }
        }
        
        // Configure MSI-X table entries
        for (i, &vector) in allocated_vectors.iter().enumerate() {
            let msix_info = &self.devices[device_index].msix_info.as_ref().unwrap();
            self.configure_msix_entry(msix_info, i as u16, vector)?;
        }
        
        // Enable MSI-X
        let mut message_control = read_config_word(bus, device, function, capability_offset + MSIX_MESSAGE_CONTROL);
        message_control |= MSIX_ENABLE;
        write_config_word(bus, device, function, capability_offset + MSIX_MESSAGE_CONTROL, message_control);
        
        crate::serial::_print(format_args!(
            "[PCI] Configured MSI-X for device {:02X}:{:02X}.{} with {} vectors\n",
            bus, device, function, allocated_vectors.len()
        ));
        
        Ok(allocated_vectors)
    }
    
    /// Configure a single MSI-X table entry
    fn configure_msix_entry(&self, msix_info: &MsixInfo, entry_index: u16, vector: u8) -> Result<(), &'static str> {
        if entry_index >= msix_info.table_size {
            return Err("Entry index out of bounds");
        }
        
        // Calculate Local APIC address for MSI-X
        let apic_id = apic::get_apic_id();
        let message_addr = 0xFEE00000u32 | ((apic_id & 0xFF) << 12);
        
        // Message data contains the interrupt vector
        let message_data = vector as u32;
        
        // In a real implementation, we would write to the mapped MSI-X table
        // For now, we just log the configuration
        crate::serial::_print(format_args!(
            "[PCI] MSI-X Entry {}: addr=0x{:08X}, data=0x{:08X}, vector={}\n",
            entry_index, message_addr, message_data, vector
        ));
        
        Ok(())
    }
    
    /// Get all PCI devices
    pub fn get_devices(&self) -> &[PciDevice] {
        &self.devices
    }
    
    /// Find devices by class code
    pub fn find_devices_by_class(&self, class_code: u8) -> Vec<&PciDevice> {
        self.devices.iter().filter(|d| d.class_code == class_code).collect()
    }
    
    /// Find device by vendor and device ID
    pub fn find_device_by_id(&self, vendor_id: u16, device_id: u16) -> Option<&PciDevice> {
        self.devices.iter().find(|d| d.vendor_id == vendor_id && d.device_id == device_id)
    }
}

/// Read a byte from PCI configuration space
fn read_config_byte(bus: u8, device: u8, function: u8, offset: u8) -> u8 {
    let address = 0x80000000u32
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | ((offset & 0xFC) as u32);
    
    let mut addr_port = Port::new(PCI_CONFIG_ADDRESS);
    let mut data_port = Port::new(PCI_CONFIG_DATA);
    
    unsafe {
        addr_port.write(address);
        let data: u32 = data_port.read();
        ((data >> ((offset & 3) * 8)) & 0xFF) as u8
    }
}

/// Read a word from PCI configuration space
fn read_config_word(bus: u8, device: u8, function: u8, offset: u8) -> u16 {
    let address = 0x80000000u32
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | ((offset & 0xFC) as u32);
    
    let mut addr_port = Port::new(PCI_CONFIG_ADDRESS);
    let mut data_port = Port::new(PCI_CONFIG_DATA);
    
    unsafe {
        addr_port.write(address);
        let data: u32 = data_port.read();
        ((data >> ((offset & 2) * 8)) & 0xFFFF) as u16
    }
}

/// Read a dword from PCI configuration space
fn read_config_dword(bus: u8, device: u8, function: u8, offset: u8) -> u32 {
    let address = 0x80000000u32
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | ((offset & 0xFC) as u32);
    
    let mut addr_port = Port::new(PCI_CONFIG_ADDRESS);
    let mut data_port = Port::new(PCI_CONFIG_DATA);
    
    unsafe {
        addr_port.write(address);
        data_port.read()
    }
}

/// Write a word to PCI configuration space
fn write_config_word(bus: u8, device: u8, function: u8, offset: u8, value: u16) {
    let address = 0x80000000u32
        | ((bus as u32) << 16)
        | ((device as u32) << 11)
        | ((function as u32) << 8)
        | ((offset & 0xFC) as u32);
    
    let mut addr_port = Port::new(PCI_CONFIG_ADDRESS);
    let mut data_port = Port::new(PCI_CONFIG_DATA);
    
    unsafe {
        addr_port.write(address);
        let mut data: u32 = data_port.read();
        let shift = (offset & 2) * 8;
        data = (data & !(0xFFFF << shift)) | ((value as u32) << shift);
        data_port.write(data);
    }
}

/// Initialize PCI subsystem
pub fn init() -> Result<(), &'static str> {
    crate::serial::_print(format_args!("[PCI] Initializing PCI subsystem...\n"));
    
    let mut manager = PCI_MANAGER.lock();
    manager.enumerate_devices();
    
    // Configure MSI-X for devices that support it
    let devices_with_msix: Vec<(u8, u8, u8)> = manager.devices.iter()
        .filter(|d| d.msix_info.is_some())
        .map(|d| (d.bus, d.device, d.function))
        .collect();
    
    for (bus, device, function) in devices_with_msix {
        if let Err(e) = manager.configure_msix(bus, device, function, 1) {
            crate::serial::_print(format_args!(
                "[PCI] Failed to configure MSI-X for {:02X}:{:02X}.{}: {}\n",
                bus, device, function, e
            ));
        }
    }
    
    crate::serial::_print(format_args!("[PCI] PCI subsystem initialized\n"));
    Ok(())
}

/// Get PCI manager instance
pub fn get_manager() -> &'static Mutex<PciManager> {
    &PCI_MANAGER
}

/// Enable bus mastering for a PCI device
pub fn enable_bus_mastering(bus: u8, device: u8, function: u8) {
    let mut command = read_config_word(bus, device, function, PCI_COMMAND);
    command |= 0x04; // Bus Master Enable
    write_config_word(bus, device, function, PCI_COMMAND, command);
}

/// Enable memory space for a PCI device
pub fn enable_memory_space(bus: u8, device: u8, function: u8) {
    let mut command = read_config_word(bus, device, function, PCI_COMMAND);
    command |= 0x02; // Memory Space Enable
    write_config_word(bus, device, function, PCI_COMMAND, command);
}

/// Enable I/O space for a PCI device
pub fn enable_io_space(bus: u8, device: u8, function: u8) {
    let mut command = read_config_word(bus, device, function, PCI_COMMAND);
    command |= 0x01; // I/O Space Enable
    write_config_word(bus, device, function, PCI_COMMAND, command);
}