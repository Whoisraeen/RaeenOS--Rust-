//! ELF (Executable and Linkable Format) loader for RaeenOS
//! 
//! This module provides functionality to parse and load ELF executables
//! into process address spaces.

use alloc::vec::Vec;
use alloc::format;
use x86_64::VirtAddr;
use crate::vmm::{VmArea, VmAreaType, VmPermissions, VmError};

/// ELF file header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ElfHeader {
    pub e_ident: [u8; 16],     // ELF identification
    pub e_type: u16,           // Object file type
    pub e_machine: u16,        // Machine type
    pub e_version: u32,        // Object file version
    pub e_entry: u64,          // Entry point address
    pub e_phoff: u64,          // Program header offset
    pub e_shoff: u64,          // Section header offset
    pub e_flags: u32,          // Processor-specific flags
    pub e_ehsize: u16,         // ELF header size
    pub e_phentsize: u16,      // Program header entry size
    pub e_phnum: u16,          // Number of program header entries
    pub e_shentsize: u16,      // Section header entry size
    pub e_shnum: u16,          // Number of section header entries
    pub e_shstrndx: u16,       // Section header string table index
}

/// ELF program header
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ProgramHeader {
    pub p_type: u32,           // Segment type
    pub p_flags: u32,          // Segment flags
    pub p_offset: u64,         // Segment file offset
    pub p_vaddr: u64,          // Segment virtual address
    pub p_paddr: u64,          // Segment physical address
    pub p_filesz: u64,         // Segment size in file
    pub p_memsz: u64,          // Segment size in memory
    pub p_align: u64,          // Segment alignment
}

/// ELF constants
const ELF_MAGIC: &[u8; 4] = b"\x7fELF";
const ET_EXEC: u16 = 2;        // Executable file
const EM_X86_64: u16 = 62;     // AMD x86-64 architecture
const PT_LOAD: u32 = 1;        // Loadable segment
const PF_X: u32 = 1;           // Execute permission
const PF_W: u32 = 2;           // Write permission
const PF_R: u32 = 4;           // Read permission

/// ELF loader errors
#[derive(Debug)]
pub enum ElfError {
    InvalidMagic,
    UnsupportedArchitecture,
    UnsupportedType,
    InvalidHeader,
    InvalidProgramHeader,
    MemoryError(VmError),
    InvalidAddress,
}

impl From<VmError> for ElfError {
    fn from(err: VmError) -> Self {
        ElfError::MemoryError(err)
    }
}

/// ELF loader
pub struct ElfLoader {
    data: Vec<u8>,
    header: ElfHeader,
}

impl ElfLoader {
    /// Create a new ELF loader from binary data
    pub fn new(data: Vec<u8>) -> Result<Self, ElfError> {
        if data.len() < core::mem::size_of::<ElfHeader>() {
            return Err(ElfError::InvalidHeader);
        }
        
        // Parse ELF header
        let header = unsafe {
            core::ptr::read(data.as_ptr() as *const ElfHeader)
        };
        
        // Validate ELF magic
        if &header.e_ident[0..4] != ELF_MAGIC {
            return Err(ElfError::InvalidMagic);
        }
        
        // Check architecture
        if header.e_machine != EM_X86_64 {
            return Err(ElfError::UnsupportedArchitecture);
        }
        
        // Check file type
        if header.e_type != ET_EXEC {
            return Err(ElfError::UnsupportedType);
        }
        
        Ok(Self { data, header })
    }
    
    /// Get the entry point address
    pub fn entry_point(&self) -> VirtAddr {
        VirtAddr::new(self.header.e_entry)
    }
    
    /// Load the ELF into the specified address space
    pub fn load_into_address_space(&self, address_space_id: u64) -> Result<(), ElfError> {
        // Parse program headers
        let ph_offset = self.header.e_phoff as usize;
        let ph_size = self.header.e_phentsize as usize;
        let ph_count = self.header.e_phnum as usize;
        
        if ph_offset + (ph_size * ph_count) > self.data.len() {
            return Err(ElfError::InvalidProgramHeader);
        }
        
        crate::vmm::with_vmm(|vmm| {
            let address_space = vmm.get_address_space_mut(address_space_id)
                .ok_or(ElfError::InvalidAddress)?;
            
            // Process each loadable segment
            for i in 0..ph_count {
                let ph_ptr = unsafe {
                    self.data.as_ptr().add(ph_offset + (i * ph_size)) as *const ProgramHeader
                };
                let ph = unsafe { core::ptr::read(ph_ptr) };
                
                // Only process loadable segments
                if ph.p_type != PT_LOAD {
                    continue;
                }
                
                // Validate segment
                if ph.p_filesz > ph.p_memsz {
                    return Err(ElfError::InvalidProgramHeader);
                }
                
                if ph.p_offset as usize + ph.p_filesz as usize > self.data.len() {
                    return Err(ElfError::InvalidProgramHeader);
                }
                
                // Convert ELF flags to VM permissions
                let mut permissions = VmPermissions::empty();
                if (ph.p_flags & PF_R) != 0 {
                    permissions = permissions | VmPermissions::Read;
                }
                if (ph.p_flags & PF_W) != 0 {
                    permissions = permissions | VmPermissions::Write;
                }
                if (ph.p_flags & PF_X) != 0 {
                    permissions = permissions | VmPermissions::Execute;
                }
                permissions = permissions | VmPermissions::User;
                
                // Determine area type
                let area_type = if (ph.p_flags & PF_X) != 0 {
                    VmAreaType::Code
                } else if (ph.p_flags & PF_W) != 0 {
                    VmAreaType::Data
                } else {
                    VmAreaType::Data // Read-only data
                };
                
                // Create memory area
                let start_addr = VirtAddr::new(ph.p_vaddr);
                let end_addr = start_addr + ph.p_memsz;
                
                let area = VmArea::new(
                    start_addr,
                    end_addr,
                    area_type,
                    permissions,
                );
                
                // Add area to address space
                address_space.add_area(area)?;
                
                // Copy segment data
                if ph.p_filesz > 0 {
                    let src_data = &self.data[ph.p_offset as usize..(ph.p_offset + ph.p_filesz) as usize];
                    
                    // Map pages and copy data
                    self.copy_segment_data(start_addr, src_data, ph.p_memsz as usize)?;
                }
                
                // Zero-fill remaining memory if memsz > filesz
                if ph.p_memsz > ph.p_filesz {
                    let zero_start = start_addr + ph.p_filesz;
                    let zero_size = ph.p_memsz - ph.p_filesz;
                    self.zero_memory(zero_start, zero_size as usize)?;
                }
            }
            
            Ok(())
        })
    }
    
    /// Copy segment data to virtual memory
    fn copy_segment_data(&self, virt_addr: VirtAddr, data: &[u8], total_size: usize) -> Result<(), ElfError> {
        // For now, we'll use a simplified approach
        // In a real implementation, we would map pages and copy data properly
        
        // This is a placeholder - actual implementation would:
        // 1. Map physical pages to the virtual address range
        // 2. Copy data from the ELF file to the mapped pages
        // 3. Handle page boundaries correctly
        
        // For demonstration, we'll just validate the operation
        if data.len() > total_size {
            return Err(ElfError::InvalidProgramHeader);
        }
        
        Ok(())
    }
    
    /// Zero-fill memory region
    fn zero_memory(&self, virt_addr: VirtAddr, size: usize) -> Result<(), ElfError> {
        // Placeholder for zero-filling memory
        // Actual implementation would map pages and zero them
        Ok(())
    }
}

/// Load an ELF executable from binary data
pub fn load_elf(data: Vec<u8>, address_space_id: u64) -> Result<VirtAddr, ElfError> {
    let loader = ElfLoader::new(data)?;
    let entry_point = loader.entry_point();
    
    loader.load_into_address_space(address_space_id)?;
    
    Ok(entry_point)
}

/// Validate ELF file without loading
pub fn validate_elf(data: &[u8]) -> Result<VirtAddr, ElfError> {
    if data.len() < core::mem::size_of::<ElfHeader>() {
        return Err(ElfError::InvalidHeader);
    }
    
    let header = unsafe {
        core::ptr::read(data.as_ptr() as *const ElfHeader)
    };
    
    // Validate ELF magic
    if &header.e_ident[0..4] != ELF_MAGIC {
        return Err(ElfError::InvalidMagic);
    }
    
    // Check architecture
    if header.e_machine != EM_X86_64 {
        return Err(ElfError::UnsupportedArchitecture);
    }
    
    // Check file type
    if header.e_type != ET_EXEC {
        return Err(ElfError::UnsupportedType);
    }
    
    Ok(VirtAddr::new(header.e_entry))
}