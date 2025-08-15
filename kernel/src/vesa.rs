//! VESA VBE (Video BIOS Extensions) driver for RaeenOS
//! Provides linear framebuffer setup for graphics mode

use x86_64::VirtAddr;
use x86_64::instructions::interrupts;
use core::ptr;
use spin::Mutex;
use lazy_static::lazy_static;

/// VESA VBE mode information structure
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct VbeModeInfo {
    pub mode_attributes: u16,
    pub win_a_attributes: u8,
    pub win_b_attributes: u8,
    pub win_granularity: u16,
    pub win_size: u16,
    pub win_a_segment: u16,
    pub win_b_segment: u16,
    pub win_func_ptr: u32,
    pub bytes_per_scanline: u16,
    pub x_resolution: u16,
    pub y_resolution: u16,
    pub x_char_size: u8,
    pub y_char_size: u8,
    pub number_of_planes: u8,
    pub bits_per_pixel: u8,
    pub number_of_banks: u8,
    pub memory_model: u8,
    pub bank_size: u8,
    pub number_of_image_pages: u8,
    pub reserved1: u8,
    pub red_mask_size: u8,
    pub red_field_position: u8,
    pub green_mask_size: u8,
    pub green_field_position: u8,
    pub blue_mask_size: u8,
    pub blue_field_position: u8,
    pub reserved_mask_size: u8,
    pub reserved_field_position: u8,
    pub direct_color_mode_info: u8,
    pub phys_base_ptr: u32,
    pub reserved2: u32,
    pub reserved3: u16,
}

/// VESA VBE controller information
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct VbeControllerInfo {
    pub signature: [u8; 4],
    pub version: u16,
    pub oem_string_ptr: u32,
    pub capabilities: u32,
    pub video_mode_ptr: u32,
    pub total_memory: u16,
    pub oem_software_rev: u16,
    pub oem_vendor_name_ptr: u32,
    pub oem_product_name_ptr: u32,
    pub oem_product_rev_ptr: u32,
    pub reserved: [u8; 222],
    pub oem_data: [u8; 256],
}

/// VESA framebuffer information
#[derive(Debug, Clone, Copy)]
pub struct VesaFramebuffer {
    pub address: VirtAddr,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u32,
}

/// VESA VBE driver
pub struct VesaDriver {
    framebuffer: Option<VesaFramebuffer>,
    mode_info: Option<VbeModeInfo>,
}

impl VesaDriver {
    pub fn new() -> Self {
        Self {
            framebuffer: None,
            mode_info: None,
        }
    }
    
    /// Initialize VESA VBE and set up a graphics mode
    pub fn init(&mut self) -> Result<VesaFramebuffer, &'static str> {
        // Try to set up a common graphics mode (1024x768x32)
        if let Ok(fb) = self.set_mode(1024, 768, 32) {
            self.framebuffer = Some(fb);
            return Ok(fb);
        }
        
        // Fallback to 800x600x32
        if let Ok(fb) = self.set_mode(800, 600, 32) {
            self.framebuffer = Some(fb);
            return Ok(fb);
        }
        
        // Last resort: 640x480x32
        if let Ok(fb) = self.set_mode(640, 480, 32) {
            self.framebuffer = Some(fb);
            return Ok(fb);
        }
        
        Err("Failed to initialize any VESA graphics mode")
    }
    
    /// Set a specific VESA graphics mode
    fn set_mode(&mut self, width: u16, height: u16, bpp: u8) -> Result<VesaFramebuffer, &'static str> {
        // Find a suitable VESA mode
        let mode_number = self.find_mode(width, height, bpp)?;
        
        // Get mode information
        let mode_info = self.get_mode_info(mode_number)?;
        
        // Set the mode
        self.set_vbe_mode(mode_number)?;
        
        // Map the framebuffer to virtual memory
        let fb_phys = mode_info.phys_base_ptr as u64;
        let _fb_size = (mode_info.bytes_per_scanline as u32 * mode_info.y_resolution as u32) as u64;
        
        // For now, use identity mapping (this should be properly mapped through VMM)
        let fb_virt = VirtAddr::new(fb_phys);
        
        let framebuffer = VesaFramebuffer {
            address: fb_virt,
            width: mode_info.x_resolution as u32,
            height: mode_info.y_resolution as u32,
            pitch: mode_info.bytes_per_scanline as u32,
            bpp: mode_info.bits_per_pixel as u32,
        };
        
        self.mode_info = Some(mode_info);
        
        Ok(framebuffer)
    }
    
    /// Find a VESA mode matching the specified parameters
    fn find_mode(&self, width: u16, height: u16, bpp: u8) -> Result<u16, &'static str> {
        // Common VESA mode numbers for different resolutions
        let modes = [
            (640, 480, 32, 0x112),   // 640x480x32
            (800, 600, 32, 0x115),   // 800x600x32
            (1024, 768, 32, 0x118),  // 1024x768x32
            (1280, 1024, 32, 0x11B), // 1280x1024x32
        ];
        
        for &(mode_width, mode_height, mode_bpp, mode_num) in &modes {
            if mode_width == width && mode_height == height && mode_bpp == bpp {
                return Ok(mode_num);
            }
        }
        
        Err("Unsupported VESA mode")
    }
    
    /// Get mode information for a specific VESA mode
    fn get_mode_info(&self, mode: u16) -> Result<VbeModeInfo, &'static str> {
        // Allocate buffer for mode info (should be in low memory for real mode access)
        let mut mode_info = VbeModeInfo {
            mode_attributes: 0,
            win_a_attributes: 0,
            win_b_attributes: 0,
            win_granularity: 0,
            win_size: 0,
            win_a_segment: 0,
            win_b_segment: 0,
            win_func_ptr: 0,
            bytes_per_scanline: 0,
            x_resolution: 0,
            y_resolution: 0,
            x_char_size: 0,
            y_char_size: 0,
            number_of_planes: 0,
            bits_per_pixel: 0,
            number_of_banks: 0,
            memory_model: 0,
            bank_size: 0,
            number_of_image_pages: 0,
            reserved1: 0,
            red_mask_size: 0,
            red_field_position: 0,
            green_mask_size: 0,
            green_field_position: 0,
            blue_mask_size: 0,
            blue_field_position: 0,
            reserved_mask_size: 0,
            reserved_field_position: 0,
            direct_color_mode_info: 0,
            phys_base_ptr: 0,
            reserved2: 0,
            reserved3: 0,
        };
        
        // For now, provide hardcoded mode info for common modes
        // In a real implementation, this would use BIOS interrupts
        match mode {
            0x112 => { // 640x480x32
                mode_info.x_resolution = 640;
                mode_info.y_resolution = 480;
                mode_info.bits_per_pixel = 32;
                mode_info.bytes_per_scanline = 640 * 4;
                mode_info.phys_base_ptr = 0xE0000000; // Common framebuffer address
                mode_info.mode_attributes = 0x90; // Linear framebuffer supported
            },
            0x115 => { // 800x600x32
                mode_info.x_resolution = 800;
                mode_info.y_resolution = 600;
                mode_info.bits_per_pixel = 32;
                mode_info.bytes_per_scanline = 800 * 4;
                mode_info.phys_base_ptr = 0xE0000000;
                mode_info.mode_attributes = 0x90;
            },
            0x118 => { // 1024x768x32
                mode_info.x_resolution = 1024;
                mode_info.y_resolution = 768;
                mode_info.bits_per_pixel = 32;
                mode_info.bytes_per_scanline = 1024 * 4;
                mode_info.phys_base_ptr = 0xE0000000;
                mode_info.mode_attributes = 0x90;
            },
            _ => return Err("Unsupported mode"),
        }
        
        Ok(mode_info)
    }
    
    /// Set VESA VBE mode
    fn set_vbe_mode(&self, _mode: u16) -> Result<(), &'static str> {
        // In a real implementation, this would use BIOS interrupt 0x10
        // with AX=0x4F02 and BX=mode|0x4000 (linear framebuffer)
        // For now, we'll assume the mode is set successfully
        
        // Disable interrupts during mode switch
        interrupts::without_interrupts(|| {
            // Mode switch would happen here via BIOS call
            // Since we can't make BIOS calls in long mode, we'll simulate success
        });
        
        Ok(())
    }
    
    /// Get current framebuffer information
    pub fn get_framebuffer(&self) -> Option<VesaFramebuffer> {
        self.framebuffer
    }
    
    /// Clear the framebuffer with a specific color
    pub fn clear_screen(&self, color: u32) -> Result<(), &'static str> {
        let fb = self.framebuffer.ok_or("Framebuffer not initialized")?;
        
        unsafe {
            let fb_ptr = fb.address.as_ptr::<u32>() as *mut u32;
            let pixel_count = (fb.width * fb.height) as usize;
            
            for i in 0..pixel_count {
                ptr::write_volatile(fb_ptr.add(i), color);
            }
        }
        
        Ok(())
    }
    
    /// Draw a pixel at the specified coordinates
    pub fn draw_pixel(&self, x: u32, y: u32, color: u32) -> Result<(), &'static str> {
        let fb = self.framebuffer.ok_or("Framebuffer not initialized")?;
        
        if x >= fb.width || y >= fb.height {
            return Err("Pixel coordinates out of bounds");
        }
        
        unsafe {
            let fb_ptr = fb.address.as_ptr::<u32>() as *mut u32;
            let offset = (y * fb.width + x) as usize;
            ptr::write_volatile(fb_ptr.add(offset), color);
        }
        
        Ok(())
    }
}

// Global VESA driver instance
lazy_static! {
    static ref VESA_DRIVER: Mutex<Option<VesaDriver>> = Mutex::new(None);
}

/// Initialize VESA VBE graphics
pub fn init() -> Result<VesaFramebuffer, &'static str> {
    let mut driver = VesaDriver::new();
    let framebuffer = driver.init()?;
    *VESA_DRIVER.lock() = Some(driver);
    Ok(framebuffer)
}

/// Get the current VESA framebuffer
pub fn get_framebuffer() -> Option<VesaFramebuffer> {
    VESA_DRIVER.lock().as_ref()?.get_framebuffer()
}

/// Clear the screen with a specific color
pub fn clear_screen(color: u32) -> Result<(), &'static str> {
    VESA_DRIVER.lock()
        .as_ref()
        .ok_or("VESA driver not initialized")?
        .clear_screen(color)
}

/// Draw a pixel
pub fn draw_pixel(x: u32, y: u32, color: u32) -> Result<(), &'static str> {
    VESA_DRIVER.lock()
        .as_ref()
        .ok_or("VESA driver not initialized")?
        .draw_pixel(x, y, color)
}