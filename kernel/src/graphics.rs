//! Graphics subsystem for RaeenOS
//! Provides GPU-accelerated rendering, window management, and RaeUI framework

use alloc::vec;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::collections::BTreeMap;
use x86_64::VirtAddr;
use spin::Mutex;
use lazy_static::lazy_static;
use bitflags::bitflags;
use core::sync::atomic::{AtomicU64, Ordering};

// Key modifier flags
bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct KeyModifiers: u8 {
        const SHIFT = 1 << 0;
        const CTRL  = 1 << 1;
        const ALT   = 1 << 2;
        const META  = 1 << 3;
    }
}

// Global timestamp counter for events
static EVENT_TIMESTAMP: AtomicU64 = AtomicU64::new(0);

// Global modifier key state
static MODIFIER_STATE: Mutex<KeyModifiers> = Mutex::new(KeyModifiers::empty());

// Get current timestamp for events
fn get_timestamp() -> u64 {
    EVENT_TIMESTAMP.fetch_add(1, Ordering::SeqCst)
}

// Called by timer interrupt to update timestamp
pub fn tick_timestamp() {
    EVENT_TIMESTAMP.fetch_add(1000, Ordering::SeqCst); // Increment by 1000 per timer tick
}

// Update modifier key state based on key presses
fn update_modifiers(key_code: u32, pressed: bool) -> KeyModifiers {
    let mut modifiers = MODIFIER_STATE.lock();
    
    match key_code {
        42 | 54 => { // Left Shift (42) or Right Shift (54)
            if pressed {
                modifiers.insert(KeyModifiers::SHIFT);
            } else {
                modifiers.remove(KeyModifiers::SHIFT);
            }
        }
        29 | 97 => { // Left Ctrl (29) or Right Ctrl (97)
            if pressed {
                modifiers.insert(KeyModifiers::CTRL);
            } else {
                modifiers.remove(KeyModifiers::CTRL);
            }
        }
        56 | 100 => { // Left Alt (56) or Right Alt (100)
            if pressed {
                modifiers.insert(KeyModifiers::ALT);
            } else {
                modifiers.remove(KeyModifiers::ALT);
            }
        }
        125 | 126 => { // Left Meta (125) or Right Meta (126)
            if pressed {
                modifiers.insert(KeyModifiers::META);
            } else {
                modifiers.remove(KeyModifiers::META);
            }
        }
        _ => {}
    }
    
    *modifiers
}

// Event system structures
#[derive(Debug, Clone, Copy)]
pub struct MouseEvent {
    pub x: i32,
    pub y: i32,
    pub button: MouseButton,
    pub pressed: bool,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

#[derive(Debug, Clone, Copy)]
pub struct KeyboardEvent {
    pub key_code: u32,
    pub pressed: bool,
    pub modifiers: KeyModifiers,
    pub timestamp: u64,
}



#[derive(Debug, Clone)]
pub enum WindowEvent {
    Mouse(MouseEvent),
    Keyboard(KeyboardEvent),
    Resize { width: u32, height: u32 },
    Close,
}

#[derive(Debug, Clone, Copy)]
pub struct Point {
    pub x: i32,
    pub y: i32,
}

impl Point {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
}

/// Color representation in RGBA format
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Color = Color { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Color = Color { r: 255, g: 255, b: 255, a: 255 };
    pub const RED: Color = Color { r: 255, g: 0, b: 0, a: 255 };
    pub const GREEN: Color = Color { r: 0, g: 255, b: 0, a: 255 };
    pub const BLUE: Color = Color { r: 0, g: 0, b: 255, a: 255 };
    pub const TRANSPARENT: Color = Color { r: 0, g: 0, b: 0, a: 0 };
    
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Color { r, g, b, a }
    }
    
    pub fn rgb(r: u8, g: u8, b: u8) -> Self {
        Color { r, g, b, a: 255 }
    }
    
    pub fn blend(&self, other: &Color) -> Color {
        let alpha = other.a as f32 / 255.0;
        let inv_alpha = 1.0 - alpha;
        
        Color {
            r: ((self.r as f32 * inv_alpha) + (other.r as f32 * alpha)) as u8,
            g: ((self.g as f32 * inv_alpha) + (other.g as f32 * alpha)) as u8,
            b: ((self.b as f32 * inv_alpha) + (other.b as f32 * alpha)) as u8,
            a: 255,
        }
    }
}

// (removed duplicate Point definition; see earlier Point)

/// Rectangle
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Rect { x, y, width, height }
    }
    
    pub fn contains(&self, point: Point) -> bool {
        point.x >= self.x && point.x < self.x + self.width as i32 &&
        point.y >= self.y && point.y < self.y + self.height as i32
    }
    
    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.x + other.width as i32 &&
        self.x + self.width as i32 > other.x &&
        self.y < other.y + other.height as i32 &&
        self.y + self.height as i32 > other.y
    }
}

/// Graphics buffer for rendering
#[derive(Clone, Debug)]
pub struct GraphicsBuffer {
    pub width: u32,
    pub height: u32,
    pub pixels: Vec<u32>,
}

impl GraphicsBuffer {
    pub fn new(width: u32, height: u32) -> Self {
        GraphicsBuffer {
            width,
            height,
            pixels: vec![0; (width * height) as usize],
        }
    }
    
    pub fn get_pixel(&self, x: u32, y: u32) -> Color {
        if x < self.width && y < self.height {
            let idx = (y * self.width + x) as usize;
            let p = self.pixels[idx];
            Color { r: ((p >> 16) & 0xFF) as u8, g: ((p >> 8) & 0xFF) as u8, b: (p & 0xFF) as u8, a: ((p >> 24) & 0xFF) as u8 }
        } else {
            Color::TRANSPARENT
        }
    }

    pub fn clear(&mut self, color: Color) {
        let pixel_value = ((color.a as u32) << 24) | ((color.r as u32) << 16) | 
                         ((color.g as u32) << 8) | (color.b as u32);
        self.pixels.fill(pixel_value);
    }
    
    pub fn set_pixel(&mut self, x: u32, y: u32, color: Color) {
        if x < self.width && y < self.height {
            let index = (y * self.width + x) as usize;
            let pixel_value = ((color.a as u32) << 24) | ((color.r as u32) << 16) | 
                             ((color.g as u32) << 8) | (color.b as u32);
            self.pixels[index] = pixel_value;
        }
    }
    
    pub fn draw_rect(&mut self, rect: Rect, color: Color) {
        for y in rect.y..rect.y + rect.height as i32 {
            for x in rect.x..rect.x + rect.width as i32 {
                if x >= 0 && y >= 0 {
                    self.set_pixel(x as u32, y as u32, color);
                }
            }
        }
    }
    
    pub fn draw_line(&mut self, start: Point, end: Point, color: Color) {
        let dx = (end.x - start.x).abs();
        let dy = (end.y - start.y).abs();
        let sx = if start.x < end.x { 1 } else { -1 };
        let sy = if start.y < end.y { 1 } else { -1 };
        let mut err = dx - dy;
        
        let mut x = start.x;
        let mut y = start.y;
        
        loop {
            if x >= 0 && y >= 0 {
                self.set_pixel(x as u32, y as u32, color);
            }
            
            if x == end.x && y == end.y {
                break;
            }
            
            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }
    }
}

/// Window ID type
pub type WindowId = u32;

/// Window state
#[derive(Debug, Clone, PartialEq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
}

/// Window flags
#[derive(Debug, Clone)]
pub struct WindowFlags {
    pub resizable: bool,
    pub decorated: bool,
    pub always_on_top: bool,
    pub transparent: bool,
    pub blur_behind: bool,
}

impl Default for WindowFlags {
    fn default() -> Self {
        WindowFlags {
            resizable: true,
            decorated: true,
            always_on_top: false,
            transparent: false,
            blur_behind: false,
        }
    }
}

/// Window structure
#[derive(Debug, Clone)]
pub struct Window {
    pub id: WindowId,
    pub title: String,
    pub rect: Rect,
    pub state: WindowState,
    pub flags: WindowFlags,
    pub z_order: i32,
    pub process_id: u32,
    pub buffer: Option<GraphicsBuffer>,
    pub visible: bool,
    pub focused: bool,
    pub pending_events: Vec<WindowEvent>,
    pub widgets: Vec<Widget>,
    pub focused_widget: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Widget {
    pub id: u32,
    pub bounds: Rect,
    pub widget_type: WidgetType,
    pub focused: bool,
}

#[derive(Debug, Clone)]
pub enum WidgetType {
    Button { text: String },
    TextInput { text: String, cursor_pos: usize },
    Label { text: String },
    Panel,
}

impl Widget {
    pub fn handle_mouse_event(&mut self, _event: MouseEvent) {
        // Basic widget mouse handling
        match &mut self.widget_type {
            WidgetType::Button { .. } => {
                // Handle button click
            }
            WidgetType::TextInput { .. } => {
                // Handle text input focus
                self.focused = true;
            }
            _ => {}
        }
    }
    
    pub fn handle_keyboard_event(&mut self, event: KeyboardEvent) {
        if !self.focused {
            return;
        }
        
        match &mut self.widget_type {
            WidgetType::TextInput { text, cursor_pos } => {
                if event.pressed {
                    match event.key_code {
                        8 => { // Backspace
                            if *cursor_pos > 0 {
                                text.remove(*cursor_pos - 1);
                                *cursor_pos -= 1;
                            }
                        }
                        32..=126 => { // Printable ASCII
                            let ch = event.key_code as u8 as char;
                            text.insert(*cursor_pos, ch);
                            *cursor_pos += 1;
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
}

impl Rect {
    pub fn contains_point(&self, point: Point) -> bool {
        point.x >= self.x && point.x < self.x + self.width as i32 &&
        point.y >= self.y && point.y < self.y + self.height as i32
    }
}

impl Window {
    pub fn new(id: WindowId, title: String, rect: Rect, process_id: u32) -> Self {
        Window {
            id,
            title,
            rect,
            state: WindowState::Normal,
            flags: WindowFlags::default(),
            z_order: 0,
            process_id,
            buffer: Some(GraphicsBuffer::new(rect.width, rect.height)),
            visible: true,
            focused: false,
            pending_events: Vec::new(),
            widgets: Vec::new(),
            focused_widget: None,
        }
    }
    
    pub fn resize(&mut self, width: u32, height: u32) {
        self.rect.width = width;
        self.rect.height = height;
        self.buffer = Some(GraphicsBuffer::new(width, height));
    }
    
    pub fn move_to(&mut self, x: i32, y: i32) {
        self.rect.x = x;
        self.rect.y = y;
    }
    
    pub fn focus_next_widget(&mut self) {
        if self.widgets.is_empty() {
            return;
        }
        
        // Clear current focus
        if let Some(current) = self.focused_widget {
            if current < self.widgets.len() {
                self.widgets[current].focused = false;
            }
        }
        
        // Move to next widget
        let next = match self.focused_widget {
            Some(current) => (current + 1) % self.widgets.len(),
            None => 0,
        };
        
        self.focused_widget = Some(next);
        self.widgets[next].focused = true;
    }
    
    pub fn handle_escape(&mut self) {
        // Clear widget focus or close window
        if let Some(focused) = self.focused_widget {
            self.widgets[focused].focused = false;
            self.focused_widget = None;
        } else {
            // Add close event
            self.pending_events.push(WindowEvent::Close);
        }
    }
    
    pub fn get_focused_widget_mut(&mut self) -> Option<&mut Widget> {
        if let Some(index) = self.focused_widget {
            self.widgets.get_mut(index)
        } else {
            None
        }
    }
}

/// RaeUI Theme for glassmorphism effects
#[derive(Debug, Clone)]
pub struct RaeTheme {
    pub primary_color: Color,
    pub secondary_color: Color,
    pub accent_color: Color,
    pub background_color: Color,
    pub text_color: Color,
    pub glass_opacity: u8,
    pub blur_radius: u32,
    pub corner_radius: u32,
}

impl Default for RaeTheme {
    fn default() -> Self {
        RaeTheme {
            primary_color: Color::new(100, 150, 255, 200),
            secondary_color: Color::new(150, 100, 255, 180),
            accent_color: Color::new(255, 100, 150, 220),
            background_color: Color::new(20, 20, 30, 240),
            text_color: Color::WHITE,
            glass_opacity: 180,
            blur_radius: 10,
            corner_radius: 8,
        }
    }
}

// (removed duplicate RaeUI Widget types; using earlier Widget/WidgetType with bounds)

/// Window Manager
pub struct WindowManager {
    windows: BTreeMap<WindowId, Window>,
    next_window_id: WindowId,
    focused_window: Option<WindowId>,
    window_order: Vec<WindowId>,
    _screen_width: u32,
    _screen_height: u32,
    theme: RaeTheme,
    widgets: BTreeMap<u32, Widget>,
    next_widget_id: u32,
}

impl WindowManager {
    pub fn new(screen_width: u32, screen_height: u32) -> Self {
        WindowManager {
            windows: BTreeMap::new(),
            next_window_id: 1,
            focused_window: None,
            window_order: Vec::new(),
            _screen_width: screen_width,
            _screen_height: screen_height,
            theme: RaeTheme::default(),
            widgets: BTreeMap::new(),
            next_widget_id: 1,
        }
    }
    
    pub fn create_window(&mut self, title: String, rect: Rect, process_id: u32) -> WindowId {
        let id = self.next_window_id;
        self.next_window_id += 1;
        
        let window = Window::new(id, title, rect, process_id);
        self.windows.insert(id, window);
        self.window_order.push(id);
        
        if self.focused_window.is_none() {
            self.focused_window = Some(id);
            if let Some(window) = self.windows.get_mut(&id) {
                window.focused = true;
            }
        }
        
        id
    }
    
    pub fn destroy_window(&mut self, window_id: WindowId) -> bool {
        if self.windows.remove(&window_id).is_some() {
            self.window_order.retain(|&id| id != window_id);
            
            if self.focused_window == Some(window_id) {
                self.focused_window = self.window_order.last().copied();
                if let Some(new_focused) = self.focused_window {
                    if let Some(window) = self.windows.get_mut(&new_focused) {
                        window.focused = true;
                    }
                }
            }
            true
        } else {
            false
        }
    }
    
    pub fn focus_window(&mut self, window_id: WindowId) -> bool {
        if self.windows.contains_key(&window_id) {
            // Unfocus current window
            if let Some(current_focused) = self.focused_window {
                if let Some(window) = self.windows.get_mut(&current_focused) {
                    window.focused = false;
                }
            }
            
            // Focus new window
            self.focused_window = Some(window_id);
            if let Some(window) = self.windows.get_mut(&window_id) {
                window.focused = true;
            }
            
            // Move to front of z-order
            self.window_order.retain(|&id| id != window_id);
            self.window_order.push(window_id);
            
            true
        } else {
            false
        }
    }
    
    pub fn get_window(&self, window_id: WindowId) -> Option<&Window> {
        self.windows.get(&window_id)
    }
    
    pub fn get_window_mut(&mut self, window_id: WindowId) -> Option<&mut Window> {
        self.windows.get_mut(&window_id)
    }
    
    pub fn get_window_at_point(&self, point: Point) -> Option<WindowId> {
        for &window_id in self.window_order.iter().rev() {
            if let Some(window) = self.windows.get(&window_id) {
                if window.visible && window.rect.contains(point) {
                    return Some(window_id);
                }
            }
        }
        None
    }
    
    pub fn create_widget(&mut self, widget_type: WidgetType, rect: Rect) -> u32 {
        let id = self.next_widget_id;
        self.next_widget_id += 1;
        
        let widget = Widget { id, bounds: rect, widget_type, focused: false };
        self.widgets.insert(id, widget);
        
        id
    }
    
    pub fn get_widget(&self, widget_id: u32) -> Option<&Widget> {
        self.widgets.get(&widget_id)
    }
    
    pub fn get_widget_mut(&mut self, widget_id: u32) -> Option<&mut Widget> {
        self.widgets.get_mut(&widget_id)
    }
    
    pub fn set_theme(&mut self, theme: RaeTheme) {
        self.theme = theme;
    }
    
    pub fn render_glassmorphism_effect(&self, buffer: &mut GraphicsBuffer, rect: Rect) {
        // Apply glassmorphism effect with blur and transparency
        let glass_color = Color::new(
            self.theme.primary_color.r,
            self.theme.primary_color.g,
            self.theme.primary_color.b,
            self.theme.glass_opacity,
        );
        
        // Implement basic blur effect (software fallback)
        // In a real implementation, this would use GPU shaders for performance
        let blur_radius = 3;
        let mut blurred_pixels = Vec::new();
        
        for y in rect.y..(rect.y + rect.height as i32) {
            for x in rect.x..(rect.x + rect.width as i32) {
                if x >= 0 && y >= 0 && (x as u32) < buffer.width && (y as u32) < buffer.height {
                    let mut r_sum = 0u32;
                    let mut g_sum = 0u32;
                    let mut b_sum = 0u32;
                    let mut count = 0u32;
                    
                    // Sample surrounding pixels for blur effect
                    for dy in -blur_radius..=blur_radius {
                        for dx in -blur_radius..=blur_radius {
                            let sample_x = x + dx;
                            let sample_y = y + dy;
                            
                            if sample_x >= 0 && sample_y >= 0 && 
                               (sample_x as u32) < buffer.width && (sample_y as u32) < buffer.height {
                                let pixel = buffer.get_pixel(sample_x as u32, sample_y as u32);
                                r_sum += pixel.r as u32;
                                g_sum += pixel.g as u32;
                                b_sum += pixel.b as u32;
                                count += 1;
                            }
                        }
                    }
                    
                    if count > 0 {
                        let blurred_color = Color::new(
                            (r_sum / count) as u8,
                            (g_sum / count) as u8,
                            (b_sum / count) as u8,
                            glass_color.a,
                        );
                        blurred_pixels.push((x as u32, y as u32, blurred_color));
                    }
                }
            }
        }
        
        // Apply blurred pixels with glass effect
        for (x, y, blurred_color) in blurred_pixels {
            let final_color = Color::new(
                ((blurred_color.r as u16 * glass_color.a as u16 + glass_color.r as u16 * (255 - glass_color.a) as u16) / 255) as u8,
                ((blurred_color.g as u16 * glass_color.a as u16 + glass_color.g as u16 * (255 - glass_color.a) as u16) / 255) as u8,
                ((blurred_color.b as u16 * glass_color.a as u16 + glass_color.b as u16 * (255 - glass_color.a) as u16) / 255) as u8,
                255,
            );
            buffer.set_pixel(x, y, final_color);
        }
    }
    
    pub fn render(&self, main_buffer: &mut GraphicsBuffer) {
        // Clear screen
        main_buffer.clear(self.theme.background_color);
        
        // Render windows in z-order
        for &window_id in &self.window_order {
            if let Some(window) = self.windows.get(&window_id) {
                if window.visible {
                    // Render window decorations with glassmorphism
                    if window.flags.decorated {
                        let title_bar = Rect::new(
                            window.rect.x,
                            window.rect.y - 30,
                            window.rect.width,
                            30,
                        );
                        self.render_glassmorphism_effect(main_buffer, title_bar);
                    }
                    
                    // Render window content
                    if let Some(window_buffer) = &window.buffer {
                        // Blit window buffer to main buffer
                        for y in 0..window.rect.height {
                            for x in 0..window.rect.width {
                                let src_index = (y * window_buffer.width + x) as usize;
                                if src_index < window_buffer.pixels.len() {
                                    let pixel = window_buffer.pixels[src_index];
                                    let color = Color::new(
                                        ((pixel >> 16) & 0xFF) as u8,
                                        ((pixel >> 8) & 0xFF) as u8,
                                        (pixel & 0xFF) as u8,
                                        ((pixel >> 24) & 0xFF) as u8,
                                    );
                                    
                                    let screen_x = window.rect.x + x as i32;
                                    let screen_y = window.rect.y + y as i32;
                                    
                                    if screen_x >= 0 && screen_y >= 0 {
                                        main_buffer.set_pixel(screen_x as u32, screen_y as u32, color);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Additional window management methods
    pub fn set_focus(&mut self, window_id: u32) -> Result<(), &'static str> {
        if self.focus_window(window_id) {
            Ok(())
        } else {
            Err("Window not found")
        }
    }
    
    pub fn get_window_list(&self) -> alloc::vec::Vec<u32> {
        self.windows.keys().copied().collect()
    }
    
    pub fn resize_window(&mut self, window_id: u32, width: u32, height: u32) -> Result<(), &'static str> {
        if let Some(window) = self.windows.get_mut(&window_id) {
            window.rect.width = width;
            window.rect.height = height;
            
            // Recreate window buffer with new size
            window.buffer = Some(GraphicsBuffer::new(width, height));
            
            Ok(())
        } else {
            Err("Window not found")
        }
    }
    
    pub fn move_window(&mut self, window_id: u32, x: i32, y: i32) -> Result<(), &'static str> {
        if let Some(window) = self.windows.get_mut(&window_id) {
            window.rect.x = x;
            window.rect.y = y;
            Ok(())
        } else {
            Err("Window not found")
        }
    }
}

/// GPU acceleration interface
pub struct GpuAccelerator {
    initialized: bool,
    vendor: String,
    memory_size: u64,
    command_buffers: Vec<u32>,
    shader_cache: BTreeMap<u32, (usize, usize)>,
    texture_cache: BTreeMap<u32, (u32, u32, u64)>,
    next_shader_id: u32,
    next_texture_id: u32,
}

impl GpuAccelerator {
    pub fn new() -> Self {
        GpuAccelerator {
            initialized: false,
            vendor: String::new(),
            memory_size: 0,
            command_buffers: Vec::new(),
            shader_cache: BTreeMap::new(),
            texture_cache: BTreeMap::new(),
            next_shader_id: 1,
            next_texture_id: 1,
        }
    }
    
    pub fn initialize(&mut self) -> Result<(), &'static str> {
        // Initialize GPU hardware detection and setup
        
        // Detect GPU vendor through PCI device enumeration (simplified)
        let vendor_id = self.detect_gpu_vendor();
        match vendor_id {
            0x10DE => self.vendor = "NVIDIA".into(),
            0x1002 => self.vendor = "AMD".into(),
            0x8086 => self.vendor = "Intel".into(),
            _ => self.vendor = "Generic GPU".into(),
        }
        
        // Set up basic GPU memory management
        self.memory_size = self.detect_gpu_memory();
        
        // Initialize command buffer system
        self.command_buffers = alloc::vec![0u32; 1024]; // Basic command buffer
        
        // Set up basic shader compiler infrastructure
        self.shader_cache = alloc::collections::BTreeMap::new();
        
        self.initialized = true;
        Ok(())
    }
    
    fn detect_gpu_vendor(&self) -> u16 {
        // Simplified GPU detection - in real implementation would read PCI config
        // For now, return Intel as most common integrated GPU
        0x8086
    }
    
    fn detect_gpu_memory(&self) -> u64 {
        // Simplified memory detection - in real implementation would query GPU
        // Return reasonable default based on system memory
        512 * 1024 * 1024 // 512MB
    }
    
    pub fn create_shader(&mut self, vertex_source: &str, fragment_source: &str) -> Result<u32, &'static str> {
        if !self.initialized {
            return Err("GPU not initialized");
        }
        
        // Basic shader compilation simulation
        let shader_id = self.next_shader_id;
        
        // Validate shader source (basic checks)
        if vertex_source.is_empty() || fragment_source.is_empty() {
            return Err("Invalid shader source");
        }
        
        // In a real implementation, this would:
        // 1. Parse GLSL/HLSL source code
        // 2. Compile to GPU bytecode
        // 3. Link vertex and fragment shaders
        // 4. Store compiled shader in GPU memory
        
        // For now, just validate basic structure
        if !vertex_source.contains("main") || !fragment_source.contains("main") {
            return Err("Shader must contain main function");
        }
        
        // Store shader metadata
        self.shader_cache.insert(shader_id, (vertex_source.len(), fragment_source.len()));
        
        Ok(shader_id)
    }
    
    pub fn create_texture(&mut self, width: u32, height: u32, data: &[u8]) -> Result<u32, &'static str> {
        if !self.initialized {
            return Err("GPU not initialized");
        }
        
        // Validate texture parameters
        if width == 0 || height == 0 {
            return Err("Invalid texture dimensions");
        }
        
        let expected_size = (width * height * 4) as usize; // RGBA format
        if data.len() != expected_size {
            return Err("Texture data size mismatch");
        }
        
        // Check if we have enough GPU memory
        let texture_size = expected_size as u64;
        if texture_size > self.memory_size / 4 { // Use max 25% of GPU memory per texture
            return Err("Texture too large for GPU memory");
        }
        
        let texture_id = self.next_texture_id;
        
        // In a real implementation, this would:
        // 1. Allocate GPU memory for texture
        // 2. Upload texture data to GPU
        // 3. Set up texture sampling parameters
        // 4. Generate mipmaps if needed
        
        // For now, simulate texture creation
        self.texture_cache.insert(texture_id, (width, height, texture_size));
        
        Ok(texture_id)
    }
    
    pub fn render_quad(&self, shader_id: u32, texture_id: u32, rect: Rect) -> Result<(), &'static str> {
        if !self.initialized {
            return Err("GPU not initialized");
        }
        
        // Validate shader and texture exist
        if !self.shader_cache.contains_key(&shader_id) {
            return Err("Invalid shader ID");
        }
        
        if !self.texture_cache.contains_key(&texture_id) {
            return Err("Invalid texture ID");
        }
        
        // Validate rectangle bounds
        if rect.width == 0 || rect.height == 0 {
            return Err("Invalid quad dimensions");
        }
        
        // In a real implementation, this would:
        // 1. Set up vertex buffer with quad vertices
        // 2. Bind shader program
        // 3. Bind texture to shader
        // 4. Set up transformation matrices
        // 5. Issue draw call to GPU
        
        // For now, simulate the render command
        let _vertices = [
            (rect.x as f32, rect.y as f32, 0.0, 0.0), // Top-left
            ((rect.x + rect.width as i32) as f32, rect.y as f32, 1.0, 0.0), // Top-right
            ((rect.x + rect.width as i32) as f32, (rect.y + rect.height as i32) as f32, 1.0, 1.0), // Bottom-right
            (rect.x as f32, (rect.y + rect.height as i32) as f32, 0.0, 1.0), // Bottom-left
        ];
        
        // Add render command to command buffer (simplified)
        if self.command_buffers.len() > 0 {
            // In real implementation, would encode GPU commands here
        }
        
        Ok(())
    }
}

/// Framebuffer compositor for hardware framebuffer access
pub struct FramebufferCompositor {
    framebuffer_addr: VirtAddr,
    width: u32,
    height: u32,
    pitch: u32,
    _bpp: u32,
    back_buffer: GraphicsBuffer,
    front_buffer: GraphicsBuffer,
    dirty_regions: Vec<Rect>,
    vsync_enabled: bool,
    frame_count: u64,
    last_present_time: u64,
}

impl FramebufferCompositor {
    pub fn new(framebuffer_addr: VirtAddr, width: u32, height: u32, pitch: u32, bpp: u32) -> Self {
        Self {
            framebuffer_addr,
            width,
            height,
            pitch,
            _bpp: bpp,
            back_buffer: GraphicsBuffer::new(width, height),
            front_buffer: GraphicsBuffer::new(width, height),
            dirty_regions: Vec::new(),
            vsync_enabled: true,
            frame_count: 0,
            last_present_time: 0,
        }
    }
    
    /// Mark a region as dirty for partial updates
    pub fn mark_dirty(&mut self, rect: Rect) {
        self.dirty_regions.push(rect);
    }
    
    /// Composite all windows to the back buffer
    pub fn composite(&mut self, window_manager: &WindowManager) {
        // Clear back buffer
        self.back_buffer.clear(window_manager.theme.background_color);
        
        // Render windows in z-order
        for &window_id in &window_manager.window_order {
            if let Some(window) = window_manager.windows.get(&window_id) {
                if window.visible {
                    self.composite_window(window);
                }
            }
        }
    }
    
    /// Composite a single window to the back buffer
    fn composite_window(&mut self, window: &Window) {
        if let Some(window_buffer) = &window.buffer {
            // Blit window buffer to back buffer with clipping
            let src_rect = Rect::new(0, 0, window_buffer.width, window_buffer.height);
            let dst_rect = window.rect;
            
            self.blit_with_clipping(window_buffer, src_rect, dst_rect);
        }
    }
    
    /// Blit with clipping support
    fn blit_with_clipping(&mut self, src: &GraphicsBuffer, _src_rect: Rect, dst_rect: Rect) {
        let clip_rect = Rect::new(0, 0, self.width, self.height);
        
        // Calculate intersection
        let x_start = core::cmp::max(dst_rect.x, clip_rect.x);
        let y_start = core::cmp::max(dst_rect.y, clip_rect.y);
        let x_end = core::cmp::min(dst_rect.x + dst_rect.width as i32, clip_rect.x + clip_rect.width as i32);
        let y_end = core::cmp::min(dst_rect.y + dst_rect.height as i32, clip_rect.y + clip_rect.height as i32);
        
        if x_start >= x_end || y_start >= y_end {
            return; // No intersection
        }
        
        // Copy pixels
        for y in y_start..y_end {
            for x in x_start..x_end {
                let src_x = (x - dst_rect.x) as u32;
                let src_y = (y - dst_rect.y) as u32;
                
                if src_x < src.width && src_y < src.height {
                    let src_index = (src_y * src.width + src_x) as usize;
                    let dst_index = (y as u32 * self.width + x as u32) as usize;
                    
                    if src_index < src.pixels.len() && dst_index < self.back_buffer.pixels.len() {
                        self.back_buffer.pixels[dst_index] = src.pixels[src_index];
                    }
                }
            }
        }
    }
    
    /// Present the back buffer to the hardware framebuffer
    pub fn present(&mut self) {
        let current_time = get_timestamp();
        
        // Implement frame rate limiting for vsync
        if self.vsync_enabled {
            let target_frame_time = 16667; // ~60 FPS in microseconds
            let elapsed = current_time.saturating_sub(self.last_present_time);
            if elapsed < target_frame_time {
                // Wait for vsync or target frame time
                let wait_time = target_frame_time - elapsed;
                self.wait_microseconds(wait_time);
            }
        }
        
        // Swap buffers - copy back buffer to front buffer
        self.front_buffer.pixels.copy_from_slice(&self.back_buffer.pixels);
        
        if self.dirty_regions.is_empty() {
            // Full screen update
            self.present_full();
        } else {
            // Partial updates for better performance
            self.present_partial();
        }
        
        self.dirty_regions.clear();
        self.frame_count += 1;
        self.last_present_time = get_timestamp();
    }
    
    /// Present the entire front buffer to hardware framebuffer
    fn present_full(&self) {
        unsafe {
            // SAFETY: This is unsafe because:
            // - framebuffer_addr must be a valid, mapped framebuffer memory address
            // - The framebuffer must be mapped with WRITABLE permissions
            // - dst_index calculations must not exceed framebuffer bounds
            // - No other code should be writing to the framebuffer concurrently
            // - The framebuffer mapping must remain valid during the entire operation
            let fb_ptr = self.framebuffer_addr.as_mut_ptr::<u32>();
            
            for y in 0..self.height {
                for x in 0..self.width {
                    let src_index = (y * self.width + x) as usize;
                    let dst_index = (y * (self.pitch / 4) + x) as isize;
                    
                    if src_index < self.front_buffer.pixels.len() {
                        *fb_ptr.offset(dst_index) = self.front_buffer.pixels[src_index];
                    }
                }
            }
        }
    }
    
    /// Present only dirty regions
    fn present_partial(&self) {
        unsafe {
            // SAFETY: This is unsafe because:
            // - framebuffer_addr must be a valid, mapped framebuffer memory address
            // - The framebuffer must be mapped with WRITABLE permissions
            // - dst_index calculations must not exceed framebuffer bounds
            // - dirty_regions must contain valid rectangle coordinates
            // - No other code should be writing to the framebuffer concurrently
            let fb_ptr = self.framebuffer_addr.as_mut_ptr::<u32>();
            
            for dirty_rect in &self.dirty_regions {
                let x_start = core::cmp::max(0, dirty_rect.x) as u32;
                let y_start = core::cmp::max(0, dirty_rect.y) as u32;
                let x_end = core::cmp::min(self.width, (dirty_rect.x + dirty_rect.width as i32) as u32);
                let y_end = core::cmp::min(self.height, (dirty_rect.y + dirty_rect.height as i32) as u32);
                
                for y in y_start..y_end {
                    for x in x_start..x_end {
                        let src_index = (y * self.width + x) as usize;
                        let dst_index = (y * (self.pitch / 4) + x) as isize;
                        
                        if src_index < self.front_buffer.pixels.len() {
                            *fb_ptr.offset(dst_index) = self.front_buffer.pixels[src_index];
                        }
                    }
                }
            }
        }
    }
    
    /// Get the back buffer for direct rendering
    pub fn get_back_buffer(&mut self) -> &mut GraphicsBuffer {
        &mut self.back_buffer
    }
    
    /// Wait for a specified number of microseconds (simple busy wait)
    fn wait_microseconds(&self, microseconds: u64) {
        let start = get_timestamp();
        while get_timestamp().saturating_sub(start) < microseconds {
            core::hint::spin_loop();
        }
    }
    
    /// Enable or disable vsync
    pub fn set_vsync(&mut self, enabled: bool) {
        self.vsync_enabled = enabled;
    }
    
    /// Get frame statistics
    pub fn get_frame_stats(&self) -> (u64, u64) {
        (self.frame_count, self.last_present_time)
    }
    
    /// Clear the back buffer with a specific color
    pub fn clear_back_buffer(&mut self, color: Color) {
        self.back_buffer.clear(color);
    }
}

lazy_static! {
    static ref WINDOW_MANAGER: Mutex<WindowManager> = Mutex::new(WindowManager::new(1920, 1080));
    static ref GPU_ACCELERATOR: Mutex<GpuAccelerator> = Mutex::new(GpuAccelerator::new());
    static ref MAIN_BUFFER: Mutex<GraphicsBuffer> = Mutex::new(GraphicsBuffer::new(1920, 1080));
    static ref FRAMEBUFFER_COMPOSITOR: Mutex<Option<FramebufferCompositor>> = Mutex::new(None);
}

// Public API functions

pub fn init_graphics(screen_width: u32, screen_height: u32) -> Result<(), &'static str> {
    let mut wm = WINDOW_MANAGER.lock();
    *wm = WindowManager::new(screen_width, screen_height);
    
    let mut gpu = GPU_ACCELERATOR.lock();
    gpu.initialize()?;
    
    let mut buffer = MAIN_BUFFER.lock();
    *buffer = GraphicsBuffer::new(screen_width, screen_height);
    
    Ok(())
}

/// Initialize framebuffer compositor with hardware framebuffer
pub fn init_framebuffer_compositor(
    framebuffer_addr: VirtAddr, 
    width: u32, 
    height: u32, 
    pitch: u32, 
    bpp: u32
) -> Result<(), &'static str> {
    let mut compositor = FRAMEBUFFER_COMPOSITOR.lock();
    *compositor = Some(FramebufferCompositor::new(framebuffer_addr, width, height, pitch, bpp));
    
    // Also update window manager and main buffer to match framebuffer
    let mut wm = WINDOW_MANAGER.lock();
    *wm = WindowManager::new(width, height);
    
    let mut buffer = MAIN_BUFFER.lock();
    *buffer = GraphicsBuffer::new(width, height);
    
    Ok(())
}

pub fn create_window(title: &str, x: i32, y: i32, width: u32, height: u32, process_id: u32) -> WindowId {
    let mut wm = WINDOW_MANAGER.lock();
    let rect = Rect::new(x, y, width, height);
    wm.create_window(title.into(), rect, process_id)
}

pub fn destroy_window(window_id: WindowId) -> bool {
    let mut wm = WINDOW_MANAGER.lock();
    wm.destroy_window(window_id)
}

pub fn focus_window(window_id: WindowId) -> bool {
    let mut wm = WINDOW_MANAGER.lock();
    wm.focus_window(window_id)
}

pub fn draw_pixel(window_id: WindowId, x: u32, y: u32, color: Color) -> Result<(), &'static str> {
    let mut wm = WINDOW_MANAGER.lock();
    if let Some(window) = wm.get_window_mut(window_id) {
        if let Some(buffer) = &mut window.buffer {
            buffer.set_pixel(x, y, color);
            Ok(())
        } else {
            Err("Window has no buffer")
        }
    } else {
        Err("Window not found")
    }
}

pub fn draw_rect(window_id: WindowId, rect: Rect, color: Color) -> Result<(), &'static str> {
    let mut wm = WINDOW_MANAGER.lock();
    if let Some(window) = wm.get_window_mut(window_id) {
        if let Some(buffer) = &mut window.buffer {
            buffer.draw_rect(rect, color);
            Ok(())
        } else {
            Err("Window has no buffer")
        }
    } else {
        Err("Window not found")
    }
}

pub fn draw_line(window_id: WindowId, start: Point, end: Point, color: Color) -> Result<(), &'static str> {
    let mut wm = WINDOW_MANAGER.lock();
    if let Some(window) = wm.get_window_mut(window_id) {
        if let Some(buffer) = &mut window.buffer {
            buffer.draw_line(start, end, color);
            Ok(())
        } else {
            Err("Window has no buffer")
        }
    } else {
        Err("Window not found")
    }
}

pub fn create_widget(widget_type: WidgetType, x: i32, y: i32, width: u32, height: u32) -> u32 {
    let mut wm = WINDOW_MANAGER.lock();
    let rect = Rect::new(x, y, width, height);
    wm.create_widget(widget_type, rect)
}

pub fn set_theme(theme: RaeTheme) {
    let mut wm = WINDOW_MANAGER.lock();
    wm.set_theme(theme);
}

pub fn render_frame() {
    let wm = WINDOW_MANAGER.lock();
    
    // Check if we have a framebuffer compositor
    let mut compositor_opt = FRAMEBUFFER_COMPOSITOR.lock();
    if let Some(compositor) = compositor_opt.as_mut() {
        // Use hardware framebuffer compositor
        compositor.composite(&wm);
        compositor.present();
    } else {
        // Fallback to software rendering
        let mut buffer = MAIN_BUFFER.lock();
        wm.render(&mut buffer);
        
        // Present buffer to VGA text buffer region as a coarse preview
        // Map RGBA to ASCII shade for now (very rough fallback display)
        unsafe {
            // SAFETY: This is unsafe because:
            // - 0xb8000 is the standard VGA text buffer address in x86 systems
            // - The VGA text buffer must be mapped and writable
            // - offset calculations must not exceed the 80x25 VGA text buffer bounds
            // - volatile writes are required for memory-mapped I/O to VGA hardware
            // - No other code should be writing to VGA text buffer concurrently
            let vga_ptr = 0xb8000 as *mut u8;
            let mut offset = 0usize;
            let width = core::cmp::min(buffer.width, 80);
            let height = core::cmp::min(buffer.height, 25);
            for y in 0..height {
                for x in 0..width {
                    let idx = (y * buffer.width + x) as usize;
                    let pix = buffer.pixels[idx];
                    let r = ((pix >> 16) & 0xFF) as u8;
                    let g = ((pix >> 8) & 0xFF) as u8;
                    let b = (pix & 0xFF) as u8;
                    let lum = (r as u16 + g as u16 + b as u16) / 3;
                    let ch = match lum {
                        0..=25 => b' ', 26..=50 => b'.', 51..=90 => b'*', 91..=140 => b'o', 141..=200 => b'0', _ => b'@'
                    };
                    core::ptr::write_volatile(vga_ptr.add(offset), ch);
                    core::ptr::write_volatile(vga_ptr.add(offset + 1), 0x0f);
                    offset += 2;
                }
                // pad rest of line if any
                while offset % (80 * 2) != 0 { 
                    core::ptr::write_volatile(vga_ptr.add(offset), b' ');
                    core::ptr::write_volatile(vga_ptr.add(offset + 1), 0x0f);
                    offset += 2;
                }
            }
        }
    }
}

/// Create a demo window to test the framebuffer compositor
pub fn create_demo_window() -> Result<u32, &'static str> {
    let mut wm = WINDOW_MANAGER.lock();
    
    // Create a demo window using the correct parameter order
    let rect = Rect::new(100, 100, 400, 300);
    let window_id = wm.create_window("Demo Window".to_string(), rect, 1);
    
    // Focus the window
    wm.focus_window(window_id);
    
    // Get the window and draw some test content
    if let Some(window) = wm.get_window_mut(window_id) {
        if let Some(buffer) = &mut window.buffer {
            // Fill window with a gradient background
            for y in 0..buffer.height {
                for x in 0..buffer.width {
                    let r = (x * 255 / buffer.width) as u8;
                    let g = (y * 255 / buffer.height) as u8;
                    let b = 128u8;
                    let color = Color::new(r, g, b, 255);
                    buffer.set_pixel(x, y, color);
                }
            }
            
            // Draw a border
            let border_color = Color::new(255, 255, 255, 255);
            for x in 0..buffer.width {
                buffer.set_pixel(x, 0, border_color);
                buffer.set_pixel(x, buffer.height - 1, border_color);
            }
            for y in 0..buffer.height {
                buffer.set_pixel(0, y, border_color);
                buffer.set_pixel(buffer.width - 1, y, border_color);
            }
            
            // Draw some test rectangles
            let red_rect = Rect::new(50, 50, 100, 80);
            let green_rect = Rect::new(200, 100, 120, 60);
            
            // Fill red rectangle
            for y in red_rect.y..(red_rect.y + red_rect.height as i32) {
                for x in red_rect.x..(red_rect.x + red_rect.width as i32) {
                    if x >= 0 && y >= 0 && (x as u32) < buffer.width && (y as u32) < buffer.height {
                        buffer.set_pixel(x as u32, y as u32, Color::new(255, 0, 0, 255));
                    }
                }
            }
            
            // Fill green rectangle
            for y in green_rect.y..(green_rect.y + green_rect.height as i32) {
                for x in green_rect.x..(green_rect.x + green_rect.width as i32) {
                    if x >= 0 && y >= 0 && (x as u32) < buffer.width && (y as u32) < buffer.height {
                        buffer.set_pixel(x as u32, y as u32, Color::new(0, 255, 0, 255));
                    }
                }
            }
        }
    }
    
    wm.focus_window(window_id);
    Ok(window_id)
}

/// Create a boot animation window
pub fn create_boot_window() -> Result<u32, &'static str> {
    let mut wm = WINDOW_MANAGER.lock();
    
    let rect = Rect::new(200, 200, 600, 400);
    let window_id = wm.create_window("RaeenOS Boot".to_string(), rect, 0);
    
    // Get the window and draw boot content
    if let Some(window) = wm.get_window_mut(window_id) {
        if let Some(buffer) = &mut window.buffer {
            // Fill with dark blue background
            let bg_color = Color::new(0, 17, 34, 255);
            buffer.clear(bg_color);
            
            // Draw RaeenOS logo text (simplified)
            let logo_color = Color::new(0, 102, 204, 255);
            let logo_rect = Rect::new(200, 150, 200, 50);
            
            for y in logo_rect.y..(logo_rect.y + logo_rect.height as i32) {
                for x in logo_rect.x..(logo_rect.x + logo_rect.width as i32) {
                    if x >= 0 && y >= 0 && (x as u32) < buffer.width && (y as u32) < buffer.height {
                        buffer.set_pixel(x as u32, y as u32, logo_color);
                    }
                }
            }
            
            // Draw loading text
            let loading_color = Color::new(255, 255, 255, 255);
            let loading_rect = Rect::new(250, 220, 100, 20);
            
            for y in loading_rect.y..(loading_rect.y + loading_rect.height as i32) {
                for x in loading_rect.x..(loading_rect.x + loading_rect.width as i32) {
                    if x >= 0 && y >= 0 && (x as u32) < buffer.width && (y as u32) < buffer.height {
                        buffer.set_pixel(x as u32, y as u32, loading_color);
                    }
                }
            }
        }
    }
    
    wm.focus_window(window_id);
    Ok(window_id)
}

/// Create a shell terminal window
pub fn create_shell_window() -> Result<u32, &'static str> {
    let mut wm = WINDOW_MANAGER.lock();
    
    let rect = Rect::new(50, 50, 800, 600);
    let window_id = wm.create_window("RaeShell Terminal".to_string(), rect, 1);
    
    // Get the window and draw terminal content
    if let Some(window) = wm.get_window_mut(window_id) {
        if let Some(buffer) = &mut window.buffer {
            // Fill with dark terminal background
            let bg_color = Color::new(30, 30, 30, 255);
            buffer.clear(bg_color);
            
            // Draw terminal prompt area
            let prompt_color = Color::new(0, 122, 204, 255);
            let prompt_rect = Rect::new(10, 550, 780, 30);
            
            for y in prompt_rect.y..(prompt_rect.y + prompt_rect.height as i32) {
                for x in prompt_rect.x..(prompt_rect.x + prompt_rect.width as i32) {
                    if x >= 0 && y >= 0 && (x as u32) < buffer.width && (y as u32) < buffer.height {
                        buffer.set_pixel(x as u32, y as u32, prompt_color);
                    }
                }
            }
            
            // Draw welcome text area
            let text_color = Color::new(255, 255, 255, 255);
            let text_rect = Rect::new(10, 10, 300, 30);
            
            for y in text_rect.y..(text_rect.y + text_rect.height as i32) {
                for x in text_rect.x..(text_rect.x + text_rect.width as i32) {
                    if x >= 0 && y >= 0 && (x as u32) < buffer.width && (y as u32) < buffer.height {
                        buffer.set_pixel(x as u32, y as u32, text_color);
                    }
                }
            }
        }
    }
    
    wm.focus_window(window_id);
    Ok(window_id)
}

/// Update window manager state
pub fn update_window_manager() {
    let mut wm = WINDOW_MANAGER.lock();
    
    // Process pending events for all windows
    for (_, window) in wm.windows.iter_mut() {
        // Process window events (simplified)
        window.pending_events.clear();
    }
    
    // Update window animations, layouts, etc.
    // This is where we would handle window transitions, animations, etc.
}

pub fn get_screen_buffer() -> &'static Mutex<GraphicsBuffer> {
    &MAIN_BUFFER
}

pub fn handle_mouse_event(x: i32, y: i32, button: u8, pressed: bool) {
    let mut wm = WINDOW_MANAGER.lock();
    let point = Point::new(x, y);
    
    if let Some(window_id) = wm.get_window_at_point(point) {
        if pressed {
            wm.focus_window(window_id);
        }
        // Send mouse event to window/widget
        if let Some(window) = wm.get_window_mut(window_id) {
            // Create mouse event structure
            let mouse_event = MouseEvent {
                x: x as i32 - window.rect.x,
                y: y as i32 - window.rect.y,
                button: if button == 0 { MouseButton::Left } else if button == 1 { MouseButton::Right } else { MouseButton::Middle },
                pressed,
                timestamp: get_timestamp(),
            };
            
            // Add to window's event queue
            window.pending_events.push(WindowEvent::Mouse(mouse_event));
            
            // Check if click is on any widgets
            let local_point = Point::new(mouse_event.x, mouse_event.y);
            for widget in &mut window.widgets {
                if widget.bounds.contains_point(local_point) {
                    widget.handle_mouse_event(mouse_event);
                }
            }
        }
    }
}

pub fn handle_keyboard_event(key_code: u32, pressed: bool) {
    let mut wm = WINDOW_MANAGER.lock();
    
    if let Some(focused_id) = wm.focused_window {
        if let Some(window) = wm.get_window_mut(focused_id) {
            // Update modifier state and get current modifiers
            let current_modifiers = update_modifiers(key_code, pressed);
            
            // Create keyboard event structure
            let keyboard_event = KeyboardEvent {
                key_code,
                pressed,
                modifiers: current_modifiers,
                timestamp: get_timestamp(),
            };
            
            // Add to window's event queue
            window.pending_events.push(WindowEvent::Keyboard(keyboard_event));
            
            // Handle special keys
            match key_code {
                9 => { // Tab key
                    if pressed {
                        // Focus next widget in window
                        window.focus_next_widget();
                    }
                }
                27 => { // Escape key
                    if pressed {
                        // Close window or cancel operation
                        window.handle_escape();
                    }
                }
                _ => {
                    // Send to focused widget if any
                    if let Some(focused_widget) = window.get_focused_widget_mut() {
                        focused_widget.handle_keyboard_event(keyboard_event);
                    }
                }
            }
        }
    }
}

pub fn resize_window(window_id: WindowId, width: u32, height: u32) -> Result<(), &'static str> {
    let mut wm = WINDOW_MANAGER.lock();
    if let Some(window) = wm.get_window_mut(window_id) {
        window.resize(width, height);
        Ok(())
    } else {
        Err("Window not found")
    }
}

pub fn move_window(window_id: WindowId, x: i32, y: i32) -> Result<(), &'static str> {
    let mut wm = WINDOW_MANAGER.lock();
    if let Some(window) = wm.get_window_mut(window_id) {
        window.move_to(x, y);
        Ok(())
    } else {
        Err("Window not found")
    }
}

pub fn set_window_state(window_id: WindowId, state: WindowState) -> Result<(), &'static str> {
    let mut wm = WINDOW_MANAGER.lock();
    if let Some(window) = wm.get_window_mut(window_id) {
        window.state = state;
        Ok(())
    } else {
        Err("Window not found")
    }
}

pub fn get_window_list() -> Vec<WindowId> {
    let wm = WINDOW_MANAGER.lock();
    wm.window_order.clone()
}

pub fn get_focused_window() -> Option<WindowId> {
    let wm = WINDOW_MANAGER.lock();
    wm.focused_window
}

// Font data structure
#[derive(Debug, Clone)]
struct FontGlyph {
    width: u8,
    height: u8,
    bitmap: Vec<u8>,
}

// Basic 8x16 bitmap font
struct BitmapFont {
    _glyph_width: u8,
    glyph_height: u8,
    glyphs: BTreeMap<char, FontGlyph>,
}

impl BitmapFont {
    fn new() -> Self {
        let mut font = BitmapFont {
            _glyph_width: 8,
            glyph_height: 16,
            glyphs: BTreeMap::new(),
        };
        
        // Add basic ASCII characters (simplified bitmap data)
        // In a real implementation, this would be loaded from font files
        font.add_basic_glyphs();
        font
    }
    
    fn add_basic_glyphs(&mut self) {
        // Add space character
        self.glyphs.insert(' ', FontGlyph {
            width: 8,
            height: 16,
            bitmap: vec![0; 16], // Empty bitmap for space
        });
        
        // Add basic letters and numbers with simple patterns
        // This is a simplified implementation - real fonts would have proper bitmaps
        for ch in 'A'..='Z' {
            self.glyphs.insert(ch, self.create_letter_glyph(ch));
        }
        
        for ch in 'a'..='z' {
            self.glyphs.insert(ch, self.create_letter_glyph(ch));
        }
        
        for ch in '0'..='9' {
            self.glyphs.insert(ch, self.create_digit_glyph(ch));
        }
        
        // Add common punctuation
        let punctuation = ['.', ',', '!', '?', ':', ';', '(', ')', '[', ']', '{', '}', '-', '_', '+', '='];
        for &ch in &punctuation {
            self.glyphs.insert(ch, self.create_punctuation_glyph(ch));
        }
    }
    
    fn create_letter_glyph(&self, ch: char) -> FontGlyph {
        // Create a simple pattern for letters
        let mut bitmap = vec![0u8; 16];
        
        // Simple pattern based on character
        let pattern = match ch.to_ascii_uppercase() {
            'A' => [
                0b00111000,
                0b01000100,
                0b10000010,
                0b10000010,
                0b11111110,
                0b10000010,
                0b10000010,
                0b10000010,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
            ],
            'B' => [
                0b11111100,
                0b10000010,
                0b10000010,
                0b11111100,
                0b11111100,
                0b10000010,
                0b10000010,
                0b11111100,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
            ],
            _ => {
                // Default pattern for other letters
                let base = (ch as u8) % 8;
                [
                    0b11111110,
                    0b10000010 | base,
                    0b10000010,
                    0b10000010,
                    0b11111110,
                    0b10000010,
                    0b10000010,
                    0b10000010,
                    0b00000000,
                    0b00000000,
                    0b00000000,
                    0b00000000,
                    0b00000000,
                    0b00000000,
                    0b00000000,
                    0b00000000,
                ]
            }
        };
        
        bitmap.copy_from_slice(&pattern);
        
        FontGlyph {
            width: 8,
            height: 16,
            bitmap,
        }
    }
    
    fn create_digit_glyph(&self, ch: char) -> FontGlyph {
        let mut bitmap = vec![0u8; 16];
        
        let pattern = match ch {
            '0' => [
                0b01111100,
                0b10000010,
                0b10000110,
                0b10001010,
                0b10010010,
                0b10100010,
                0b11000010,
                0b01111100,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
            ],
            '1' => [
                0b00011000,
                0b00111000,
                0b00011000,
                0b00011000,
                0b00011000,
                0b00011000,
                0b00011000,
                0b01111110,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
            ],
            _ => {
                // Default pattern for other digits
                let base = (ch as u8 - b'0') * 16;
                [
                    0b01111100 | (base & 0x0F),
                    0b10000010,
                    0b10000010,
                    0b10000010,
                    0b10000010,
                    0b10000010,
                    0b10000010,
                    0b01111100,
                    0b00000000,
                    0b00000000,
                    0b00000000,
                    0b00000000,
                    0b00000000,
                    0b00000000,
                    0b00000000,
                    0b00000000,
                ]
            }
        };
        
        bitmap.copy_from_slice(&pattern);
        
        FontGlyph {
            width: 8,
            height: 16,
            bitmap,
        }
    }
    
    fn create_punctuation_glyph(&self, ch: char) -> FontGlyph {
        let mut bitmap = vec![0u8; 16];
        
        let pattern = match ch {
            '.' => [
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00011000,
                0b00011000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
            ],
            '!' => [
                0b00011000,
                0b00011000,
                0b00011000,
                0b00011000,
                0b00011000,
                0b00000000,
                0b00011000,
                0b00011000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
                0b00000000,
            ],
            _ => {
                // Default pattern for other punctuation
                [
                    0b00000000,
                    0b00000000,
                    0b00111100,
                    0b00111100,
                    0b00111100,
                    0b00111100,
                    0b00000000,
                    0b00000000,
                    0b00000000,
                    0b00000000,
                    0b00000000,
                    0b00000000,
                    0b00000000,
                    0b00000000,
                    0b00000000,
                    0b00000000,
                ]
            }
        };
        
        bitmap.copy_from_slice(&pattern);
        
        FontGlyph {
            width: 8,
            height: 16,
            bitmap,
        }
    }
    
    fn get_glyph(&self, ch: char) -> Option<&FontGlyph> {
        self.glyphs.get(&ch)
    }
}

lazy_static! {
    static ref DEFAULT_FONT: BitmapFont = BitmapFont::new();
}

pub fn draw_text(window_id: WindowId, x: i32, y: i32, text: &str, color: Color) -> Result<(), &'static str> {
    let mut wm = WINDOW_MANAGER.lock();
    if let Some(window) = wm.get_window_mut(window_id) {
        if let Some(buffer) = &mut window.buffer {
            let mut current_x = x;
            
            for ch in text.chars() {
                if let Some(glyph) = DEFAULT_FONT.get_glyph(ch) {
                    // Draw the glyph bitmap
                    for row in 0..glyph.height {
                        let bitmap_row = glyph.bitmap[row as usize];
                        for col in 0..glyph.width {
                            if (bitmap_row >> (7 - col)) & 1 != 0 {
                                let pixel_x = current_x + col as i32;
                                let pixel_y = y + row as i32;
                                
                                // Check bounds
                                if pixel_x >= 0 && pixel_y >= 0 && 
                                   pixel_x < buffer.width as i32 && pixel_y < buffer.height as i32 {
                                    buffer.set_pixel(pixel_x as u32, pixel_y as u32, color);
                                }
                            }
                        }
                    }
                    current_x += glyph.width as i32;
                } else {
                    // Unknown character - draw a placeholder rectangle
                    let char_rect = Rect::new(current_x, y, 8, 16);
                    buffer.draw_rect(char_rect, Color::new(128, 128, 128, 255));
                    current_x += 8;
                }
            }
            Ok(())
        } else {
            Err("Window has no buffer")
        }
    } else {
        Err("Window not found")
    }
}

// Additional text rendering functions
pub fn get_text_width(text: &str) -> u32 {
    let mut width = 0;
    for ch in text.chars() {
        if let Some(glyph) = DEFAULT_FONT.get_glyph(ch) {
            width += glyph.width as u32;
        } else {
            width += 8; // Default character width
        }
    }
    width
}

pub fn get_text_height() -> u32 {
    DEFAULT_FONT.glyph_height as u32
}

pub fn draw_text_centered(window_id: WindowId, rect: Rect, text: &str, color: Color) -> Result<(), &'static str> {
    let text_width = get_text_width(text);
    let text_height = get_text_height();
    
    let center_x = rect.x + (rect.width as i32 - text_width as i32) / 2;
    let center_y = rect.y + (rect.height as i32 - text_height as i32) / 2;
    
    draw_text(window_id, center_x, center_y, text, color)
}

pub fn draw_text_multiline(window_id: WindowId, x: i32, y: i32, text: &str, color: Color, line_height: u32) -> Result<(), &'static str> {
    let mut current_y = y;
    
    for line in text.lines() {
        draw_text(window_id, x, current_y, line, color)?;
        current_y += line_height as i32;
    }
    
    Ok(())
}

// Invalidate all windows (called when theme changes)
pub fn invalidate_all_windows() {
    let mut wm = WINDOW_MANAGER.lock();
    
    // Mark all windows as needing redraw
    for (_, window) in wm.windows.iter_mut() {
        // In a real implementation, windows would have a needs_redraw flag
        // For now, we'll just clear and redraw all window buffers
        if let Some(buffer) = &mut window.buffer {
            buffer.clear(Color::TRANSPARENT);
        }
    }
    
    // Trigger a full screen refresh
    let mut buffer = MAIN_BUFFER.lock();
    buffer.clear(wm.theme.background_color);
}

// Enhanced graphics API functions
pub fn set_vsync(enabled: bool) -> Result<(), &'static str> {
    let mut compositor = FRAMEBUFFER_COMPOSITOR.lock();
    if let Some(ref mut comp) = compositor.as_mut() {
        comp.set_vsync(enabled);
        return Ok(());
    }
    Err("Failed to access framebuffer compositor")
}

pub fn get_frame_stats() -> Result<(u64, u64), &'static str> {
    let compositor = FRAMEBUFFER_COMPOSITOR.lock();
    if let Some(ref comp) = compositor.as_ref() {
        return Ok(comp.get_frame_stats());
    }
    Err("Failed to access framebuffer compositor")
}

pub fn clear_framebuffer(color: Color) -> Result<(), &'static str> {
    let mut compositor = FRAMEBUFFER_COMPOSITOR.lock();
    if let Some(ref mut comp) = compositor.as_mut() {
        comp.clear_back_buffer(color);
        return Ok(());
    }
    Err("Failed to access framebuffer compositor")
}

/// Process compositor frame for real-time compositor thread
/// This function is called by the compositor RT thread to handle frame rendering with vsync timing
pub fn process_compositor_frame() {
    let frame_start = crate::time::get_timestamp_ns();
    
    // Update window manager state
    update_window_manager();
    
    // Render the current frame
    render_frame();
    
    // Handle any pending compositor operations
    // In a full implementation, this would:
    // 1. Process damage regions for efficient rendering
    // 2. Handle window composition and effects
    // 3. Manage GPU command submission
    // 4. Synchronize with display refresh rate
    // 5. Handle triple buffering and vsync
    
    // For now, we just ensure the frame is rendered
    // The render_frame() function already handles the compositor logic
    
    // Record compositor frame timing for jitter measurement
    record_compositor_frame_timing(frame_start);
}

/// Compositor frame timing tracking for jitter measurement
static COMPOSITOR_FRAME_TIMES: spin::Mutex<alloc::collections::VecDeque<u64>> = spin::Mutex::new(alloc::collections::VecDeque::new());
static COMPOSITOR_LAST_FRAME_TIME: spin::Mutex<Option<u64>> = spin::Mutex::new(None);

/// Record compositor frame timing and calculate jitter
fn record_compositor_frame_timing(frame_start: u64) {
    let mut last_frame_time = COMPOSITOR_LAST_FRAME_TIME.lock();
    
    if let Some(last_time) = *last_frame_time {
        // Calculate frame interval (time between frame starts)
        let frame_interval = frame_start - last_time;
        
        // Store frame intervals for jitter calculation
        let mut frame_times = COMPOSITOR_FRAME_TIMES.lock();
        frame_times.push_back(frame_interval);
        
        // Keep only the last 100 frame intervals for measurement
        if frame_times.len() > 100 {
            frame_times.pop_front();
        }
        
        // Calculate jitter when we have enough samples
        if frame_times.len() >= 100 {
            let intervals: alloc::vec::Vec<u64> = frame_times.iter().copied().collect();
            
            // Jitter is the deviation from expected frame interval
            // For 120Hz, expected interval is ~8.33ms (8333333 ns)
            // For 60Hz, expected interval is ~16.67ms (16666667 ns)
            let expected_interval_120hz = 8333333u64; // 120Hz target
            let jitter_values: alloc::vec::Vec<u64> = intervals.iter()
                .map(|&interval| {
                    if interval > expected_interval_120hz {
                        interval - expected_interval_120hz
                    } else {
                        expected_interval_120hz - interval
                    }
                })
                .collect();
            
            // Record compositor jitter measurement using SLO system
            let jitter_values_f64: alloc::vec::Vec<f64> = jitter_values.iter()
                .map(|&jitter| (jitter / 1000) as f64) // Convert to microseconds
                .collect();
            
            crate::slo::with_slo_harness(|harness| {
                crate::slo_measure!(harness, 
                    crate::slo::SloCategory::CompositorJitter, 
                    "compositor_frame_jitter", 
                    "microseconds", 
                    jitter_values_f64.len() as u64, 
                    jitter_values_f64
                );
            });
            
            // Clear the buffer to start fresh measurement
            frame_times.clear();
        }
    }
    
    *last_frame_time = Some(frame_start);
}

pub fn blit_buffer(src_data: &[u8], dst_x: u32, dst_y: u32, width: u32, height: u32, stride: u32) -> Result<(), &'static str> {
    let mut compositor = FRAMEBUFFER_COMPOSITOR.lock();
    if let Some(ref mut comp) = compositor.as_mut() {
        let back_buffer = comp.get_back_buffer();
        
        // Perform bounds checking
        if dst_x + width > back_buffer.width || dst_y + height > back_buffer.height {
            return Err("Blit operation out of bounds");
        }
        
        // Copy pixel data
        for y in 0..height {
            let src_offset = (y * stride) as usize;
            
            if src_offset + (width as usize * 4) <= src_data.len() {
                for x in 0..width {
                    let pixel_src_offset = src_offset + (x as usize * 4);
                    if pixel_src_offset + 3 < src_data.len() {
                        let b = src_data[pixel_src_offset];
                        let g = src_data[pixel_src_offset + 1];
                        let r = src_data[pixel_src_offset + 2];
                        let a = src_data[pixel_src_offset + 3];
                        let color = Color::new(r, g, b, a);
                        back_buffer.set_pixel(dst_x + x, dst_y + y, color);
                    }
                }
            }
        }
        
        return Ok(());
    }
    Err("Failed to access framebuffer compositor")
}

pub fn set_input_focus(window_id: WindowId) -> Result<(), &'static str> {
    let mut wm = WINDOW_MANAGER.lock();
    wm.set_focus(window_id)
}



// Enhanced input handling functions
pub fn show_help_overlay() {
    // Create help overlay window
    let mut wm = WINDOW_MANAGER.lock();
    let rect = Rect::new(200, 150, 400, 300);
    let window_id = wm.create_window("Help".to_string(), rect, 0);
    
    if let Some(window) = wm.get_window_mut(window_id) {
        if let Some(buffer) = &mut window.buffer {
            buffer.clear(Color::new(40, 40, 60, 240)); // Semi-transparent dark blue
            
            // Draw help text (simplified)
            let _help_text = "RaeenOS Help\n\nF1 - Show this help\nF2 - Performance overlay\nF3 - Task manager\nESC - Cancel";
            // In a real implementation, this would render the text properly
        }
    }
}

pub fn toggle_performance_overlay() {
    // Toggle performance metrics overlay
    static mut OVERLAY_VISIBLE: bool = false;
    unsafe {
        OVERLAY_VISIBLE = !OVERLAY_VISIBLE;
        if OVERLAY_VISIBLE {
            // Show performance overlay
            let mut wm = WINDOW_MANAGER.lock();
            let rect = Rect::new(10, 10, 300, 150);
            let window_id = wm.create_window("Performance".to_string(), rect, 0);
            
            if let Some(window) = wm.get_window_mut(window_id) {
                if let Some(buffer) = &mut window.buffer {
                    buffer.clear(Color::new(0, 0, 0, 180)); // Semi-transparent black
                    // Performance metrics would be rendered here
                }
            }
        }
    }
}

pub fn create_task_manager_window() -> Result<u32, &'static str> {
    let mut wm = WINDOW_MANAGER.lock();
    let rect = Rect::new(150, 100, 500, 400);
    let window_id = wm.create_window("Task Manager".to_string(), rect, 0);
    
    if let Some(window) = wm.get_window_mut(window_id) {
        if let Some(buffer) = &mut window.buffer {
            buffer.clear(Color::new(240, 240, 240, 255)); // Light gray background
            // Task list would be rendered here
        }
    }
    
    Ok(window_id)
}

pub fn cancel_current_operation() {
    // Cancel any ongoing operations (drag, resize, etc.)
    static mut DRAG_STATE: Option<(u32, i32, i32)> = None;
    unsafe {
        DRAG_STATE = None;
    }
}

pub fn update_cursor_position(x: i32, y: i32) {
    // Update global cursor position
    static mut CURSOR_POS: (i32, i32) = (0, 0);
    unsafe {
        CURSOR_POS = (x, y);
    }
}

pub fn handle_window_drag(x: i32, y: i32, _delta_x: i32, _delta_y: i32) {
    static mut DRAG_STATE: Option<(u32, i32, i32)> = None;
    
    unsafe {
        if let Some((window_id, start_x, start_y)) = DRAG_STATE {
            // Move window by delta
            let _ = move_window(window_id, x - start_x, y - start_y);
        }
    }
}

pub fn handle_mouse_hover(x: i32, y: i32) {
    // Handle mouse hover effects
    let wm = WINDOW_MANAGER.lock();
    if let Some(_window_id) = wm.get_window_at_point(Point::new(x, y)) {
        // Update hover state for window
        // In a real implementation, this would update visual feedback
    }
}

pub fn get_window_at_point(x: i32, y: i32) -> Option<u32> {
    let wm = WINDOW_MANAGER.lock();
    wm.get_window_at_point(Point::new(x, y))
}

pub fn start_window_drag_if_title_bar(x: i32, y: i32) {
    let wm = WINDOW_MANAGER.lock();
    if let Some(window_id) = wm.get_window_at_point(Point::new(x, y)) {
        if let Some(window) = wm.get_window(window_id) {
            // Check if click is in title bar area
            let title_bar_rect = Rect::new(
                window.rect.x,
                window.rect.y - 30,
                window.rect.width,
                30
            );
            
            if title_bar_rect.contains(Point::new(x, y)) {
                static mut DRAG_STATE: Option<(u32, i32, i32)> = None;
                unsafe {
                    DRAG_STATE = Some((window_id, x - window.rect.x, y - window.rect.y));
                }
            }
        }
    }
}

pub fn end_window_drag() {
    static mut DRAG_STATE: Option<(u32, i32, i32)> = None;
    unsafe {
        DRAG_STATE = None;
    }
}