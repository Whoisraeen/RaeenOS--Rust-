//! Graphics service contract for rae-compositord
//! Defines IPC interface for user-space compositor and graphics operations

use alloc::vec::Vec;
use alloc::string::String;
use serde::{Serialize, Deserialize};
use super::{ServiceResponse, error_codes};

/// Graphics service requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GraphicsRequest {
    // Window management
    CreateWindow { title: String, rect: WindowRect, z_order: u32 },
    DestroyWindow { window_id: u32 },
    MoveWindow { window_id: u32, x: i32, y: i32 },
    ResizeWindow { window_id: u32, width: u32, height: u32 },
    SetWindowTitle { window_id: u32, title: String },
    FocusWindow { window_id: u32 },
    GetWindowList,
    GetWindowInfo { window_id: u32 },
    
    // Drawing operations
    DrawPixel { window_id: u32, x: u32, y: u32, color: Color },
    DrawRect { window_id: u32, rect: WindowRect, color: Color, filled: bool },
    DrawText { window_id: u32, x: u32, y: u32, text: String, color: Color, font_size: u32 },
    BlitBuffer { window_id: u32, src_data: Vec<u8>, dst_rect: WindowRect, stride: u32 },
    ClearWindow { window_id: u32, color: Color },
    
    // Framebuffer operations
    ClearFramebuffer { color: Color },
    GetFramebufferInfo,
    SetVsync { enabled: bool },
    GetFrameStats,
    
    // Compositor operations
    SetCompositorMode { mode: CompositorMode },
    GetCompositorStats,
    SetEffectsEnabled { enabled: bool },
    SetAnimationQuality { quality: AnimationQuality },
    
    // Input focus and events
    SetInputFocus { window_id: u32 },
    GetInputFocus,
    RegisterInputHandler { window_id: u32, event_types: Vec<InputEventType> },
    
    // Theme and appearance
    SetTheme { theme: Theme },
    GetTheme,
    SetDarkMode { enabled: bool },
    
    // Performance and debugging
    EnableDebugOverlay { enabled: bool },
    GetPerformanceMetrics,
    SetFrameRateLimit { fps: u32 },
}

/// Graphics service responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GraphicsResponse {
    WindowCreated { window_id: u32 },
    WindowDestroyed,
    WindowMoved,
    WindowResized,
    WindowTitleSet,
    WindowFocused,
    WindowList { windows: Vec<WindowInfo> },
    WindowInfo { info: WindowInfo },
    
    PixelDrawn,
    RectDrawn,
    TextDrawn,
    BufferBlitted,
    WindowCleared,
    
    FramebufferCleared,
    FramebufferInfo { info: FramebufferInfo },
    VsyncSet,
    FrameStats { stats: FrameStatistics },
    
    CompositorModeSet,
    CompositorStats { stats: CompositorStatistics },
    EffectsToggled,
    AnimationQualitySet,
    
    InputFocusSet,
    InputFocus { window_id: Option<u32> },
    InputHandlerRegistered,
    
    ThemeSet,
    Theme { theme: Theme },
    DarkModeSet,
    
    DebugOverlayToggled,
    PerformanceMetrics { metrics: GraphicsMetrics },
    FrameRateLimitSet,
}

/// Window rectangle
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Color representation
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const BLACK: Self = Self { r: 0, g: 0, b: 0, a: 255 };
    pub const WHITE: Self = Self { r: 255, g: 255, b: 255, a: 255 };
    pub const RED: Self = Self { r: 255, g: 0, b: 0, a: 255 };
    pub const GREEN: Self = Self { r: 0, g: 255, b: 0, a: 255 };
    pub const BLUE: Self = Self { r: 0, g: 0, b: 255, a: 255 };
    pub const TRANSPARENT: Self = Self { r: 0, g: 0, b: 0, a: 0 };
}

/// Window information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowInfo {
    pub id: u32,
    pub title: String,
    pub rect: WindowRect,
    pub z_order: u32,
    pub visible: bool,
    pub focused: bool,
    pub process_id: u32,
}

/// Framebuffer information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FramebufferInfo {
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub bpp: u32,
    pub format: PixelFormat,
    pub physical_address: u64,
}

/// Pixel format
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PixelFormat {
    Rgb888,
    Rgba8888,
    Bgr888,
    Bgra8888,
    Rgb565,
    Rgb555,
}

/// Frame statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameStatistics {
    pub frame_count: u64,
    pub last_present_time: u64,
    pub average_frame_time_us: u32,
    pub frame_time_jitter_us: u32,
    pub missed_frames: u64,
    pub vsync_enabled: bool,
}

/// Compositor mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum CompositorMode {
    Normal,
    Gaming,      // Low latency, minimal effects
    PowerSaving, // Reduced refresh rate, simplified rendering
    Safe,        // Fallback mode with basic GOP scanout
}

/// Animation quality settings
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AnimationQuality {
    Disabled,
    Low,
    Medium,
    High,
    Ultra,
}

/// Compositor statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompositorStatistics {
    pub active_windows: u32,
    pub visible_windows: u32,
    pub composition_time_us: u32,
    pub gpu_utilization: f32,
    pub memory_usage_mb: u32,
    pub backlog_depth: u32,
    pub effects_enabled: bool,
    pub mode: CompositorMode,
}

/// Input event types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum InputEventType {
    KeyPress,
    KeyRelease,
    MouseMove,
    MouseButtonPress,
    MouseButtonRelease,
    MouseWheel,
    Touch,
    Gesture,
}

/// Theme configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Theme {
    pub name: String,
    pub dark_mode: bool,
    pub accent_color: Color,
    pub background_color: Color,
    pub text_color: Color,
    pub border_color: Color,
    pub window_transparency: f32,
    pub blur_enabled: bool,
    pub animation_speed: f32,
}

/// Graphics performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicsMetrics {
    pub frames_rendered: u64,
    pub average_frame_time_us: u32,
    pub p99_frame_time_us: u32,
    pub compositor_jitter_us: u32,
    pub gpu_memory_used_mb: u32,
    pub texture_cache_hit_rate: f32,
    pub draw_calls_per_frame: u32,
    pub triangles_per_frame: u64,
    pub shader_compilation_time_us: u32,
    pub present_queue_depth: u32,
}

/// Graphics service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphicsConfig {
    pub vsync_enabled: bool,
    pub triple_buffering: bool,
    pub frame_rate_limit: Option<u32>,
    pub compositor_mode: CompositorMode,
    pub animation_quality: AnimationQuality,
    pub effects_enabled: bool,
    pub debug_overlay: bool,
    pub gpu_acceleration: bool,
    pub texture_compression: bool,
    pub async_present: bool,
}

/// GPU capabilities
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuCapabilities {
    pub vendor: String,
    pub device_name: String,
    pub memory_size_mb: u32,
    pub supports_vulkan: bool,
    pub supports_opengl: bool,
    pub supports_compute: bool,
    pub max_texture_size: u32,
    pub max_render_targets: u32,
    pub supports_async_compute: bool,
}

/// Surface format for direct scanout
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SurfaceFormat {
    pub pixel_format: PixelFormat,
    pub color_space: ColorSpace,
    pub modifiers: u64, // DRM format modifiers
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ColorSpace {
    Srgb,
    Rec2020,
    DciP3,
    AdobeRgb,
}

/// Fence for explicit synchronization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncFence {
    pub fence_id: u32,
    pub signaled: bool,
    pub timeline_value: u64,
}

/// Convenience type alias for graphics service responses
pub type GraphicsServiceResponse<T> = ServiceResponse<T>;

/// Graphics service error codes (extending common error codes)
pub mod graphics_errors {
    use super::error_codes;
    
    pub const WINDOW_NOT_FOUND: u32 = error_codes::INTERNAL_ERROR + 1;
    pub const INVALID_WINDOW_SIZE: u32 = error_codes::INTERNAL_ERROR + 2;
    pub const FRAMEBUFFER_NOT_AVAILABLE: u32 = error_codes::INTERNAL_ERROR + 3;
    pub const GPU_ERROR: u32 = error_codes::INTERNAL_ERROR + 4;
    pub const SHADER_COMPILATION_FAILED: u32 = error_codes::INTERNAL_ERROR + 5;
    pub const TEXTURE_CREATION_FAILED: u32 = error_codes::INTERNAL_ERROR + 6;
    pub const SURFACE_LOST: u32 = error_codes::INTERNAL_ERROR + 7;
    pub const OUT_OF_GPU_MEMORY: u32 = error_codes::INTERNAL_ERROR + 8;
    pub const INVALID_PIXEL_FORMAT: u32 = error_codes::INTERNAL_ERROR + 9;
    pub const COMPOSITOR_BUSY: u32 = error_codes::INTERNAL_ERROR + 10;
}