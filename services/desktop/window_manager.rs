//! Window Manager - Advanced Window Management
//! Handles Always-On-Top, Picture-in-Picture, and advanced window operations
//! Features window layering, transparency, animations, and special window modes

use crate::services::desktop::DesktopError;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::collections::BTreeMap;
use core::time::Duration;

/// Window handle type
pub type WindowHandle = u64;

/// Window states
#[derive(Debug, Clone, PartialEq)]
pub enum WindowState {
    Normal,
    Minimized,
    Maximized,
    Fullscreen,
    AlwaysOnTop,
    PictureInPicture,
    Hidden,
    Snapped,
}

/// Window layer levels
#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum WindowLayer {
    Background = 0,
    Normal = 100,
    AboveNormal = 200,
    AlwaysOnTop = 300,
    PictureInPicture = 400,
    SystemOverlay = 500,
    Critical = 600,
}

/// Window information
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub handle: WindowHandle,
    pub title: String,
    pub app_id: String,
    pub process_id: u32,
    pub rect: WindowRect,
    pub state: WindowState,
    pub layer: WindowLayer,
    pub opacity: f32,
    pub visible: bool,
    pub resizable: bool,
    pub movable: bool,
    pub closable: bool,
    pub minimizable: bool,
    pub maximizable: bool,
    pub has_shadow: bool,
    pub border_width: f32,
    pub corner_radius: f32,
    pub created_at: u64,
    pub last_focused: u64,
    pub metadata: BTreeMap<String, String>,
}

/// Window rectangle
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WindowRect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// Picture-in-Picture configuration
#[derive(Debug, Clone)]
pub struct PipConfig {
    pub default_size: (u32, u32),
    pub min_size: (u32, u32),
    pub max_size: (u32, u32),
    pub default_position: PipPosition,
    pub opacity: f32,
    pub always_on_top: bool,
    pub show_controls: bool,
    pub auto_hide_controls: bool,
    pub control_timeout: Duration,
    pub snap_to_edges: bool,
    pub snap_threshold: u32,
    pub resize_handles: bool,
    pub corner_radius: f32,
    pub shadow_enabled: bool,
}

/// Picture-in-Picture position
#[derive(Debug, Clone, PartialEq)]
pub enum PipPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    Custom(i32, i32),
}

/// Window animation types
#[derive(Debug, Clone, PartialEq)]
pub enum WindowAnimation {
    None,
    Fade,
    Scale,
    Slide(SlideDirection),
    Bounce,
    Flip,
    Custom(String),
}

/// Slide directions
#[derive(Debug, Clone, PartialEq)]
pub enum SlideDirection {
    Up,
    Down,
    Left,
    Right,
}

/// Window operation
#[derive(Debug, Clone)]
pub struct WindowOperation {
    pub operation_type: OperationType,
    pub target_rect: Option<WindowRect>,
    pub target_state: Option<WindowState>,
    pub target_layer: Option<WindowLayer>,
    pub target_opacity: Option<f32>,
    pub animation: WindowAnimation,
    pub duration: Duration,
    pub delay: Duration,
}

/// Operation types
#[derive(Debug, Clone, PartialEq)]
pub enum OperationType {
    Move,
    Resize,
    ChangeState,
    ChangeLayer,
    ChangeOpacity,
    Show,
    Hide,
    Close,
    Focus,
    Minimize,
    Maximize,
    Restore,
    EnterPip,
    ExitPip,
    SetAlwaysOnTop,
    RemoveAlwaysOnTop,
}

/// Window manager configuration
#[derive(Debug, Clone)]
pub struct WindowManagerConfig {
    pub enable_animations: bool,
    pub animation_duration: Duration,
    pub pip_config: PipConfig,
    pub always_on_top_opacity: f32,
    pub window_shadows: bool,
    pub snap_threshold: u32,
    pub focus_follows_mouse: bool,
    pub auto_raise_delay: Duration,
    pub double_click_titlebar_action: TitlebarAction,
    pub middle_click_titlebar_action: TitlebarAction,
    pub window_placement_strategy: PlacementStrategy,
}

/// Titlebar actions
#[derive(Debug, Clone, PartialEq)]
pub enum TitlebarAction {
    None,
    Minimize,
    Maximize,
    Shade,
    Close,
    AlwaysOnTop,
    PictureInPicture,
}

/// Window placement strategies
#[derive(Debug, Clone, PartialEq)]
pub enum PlacementStrategy {
    Cascade,
    Center,
    Smart,
    Random,
    RememberPosition,
}

/// Window focus information
#[derive(Debug, Clone)]
pub struct FocusInfo {
    pub focused_window: Option<WindowHandle>,
    pub focus_stack: Vec<WindowHandle>,
    pub last_focus_change: u64,
    pub focus_reason: FocusReason,
}

/// Focus change reasons
#[derive(Debug, Clone, PartialEq)]
pub enum FocusReason {
    UserClick,
    UserKeyboard,
    ApplicationRequest,
    SystemEvent,
    WindowCreated,
    WindowClosed,
    WindowMinimized,
    WindowRestored,
}

/// Window Manager main service
pub struct WindowManager {
    config: WindowManagerConfig,
    windows: BTreeMap<WindowHandle, WindowInfo>,
    focus_info: FocusInfo,
    pip_windows: Vec<WindowHandle>,
    always_on_top_windows: Vec<WindowHandle>,
    next_handle: WindowHandle,
    active_operations: BTreeMap<WindowHandle, Vec<WindowOperation>>,
}

impl WindowManager {
    /// Create a new WindowManager instance
    pub fn new() -> Result<Self, DesktopError> {
        Ok(WindowManager {
            config: WindowManagerConfig {
                enable_animations: true,
                animation_duration: Duration::from_millis(250),
                pip_config: PipConfig {
                    default_size: (320, 240),
                    min_size: (160, 120),
                    max_size: (640, 480),
                    default_position: PipPosition::BottomRight,
                    opacity: 0.95,
                    always_on_top: true,
                    show_controls: true,
                    auto_hide_controls: true,
                    control_timeout: Duration::from_secs(3),
                    snap_to_edges: true,
                    snap_threshold: 20,
                    resize_handles: true,
                    corner_radius: 8.0,
                    shadow_enabled: true,
                },
                always_on_top_opacity: 0.9,
                window_shadows: true,
                snap_threshold: 10,
                focus_follows_mouse: false,
                auto_raise_delay: Duration::from_millis(500),
                double_click_titlebar_action: TitlebarAction::Maximize,
                middle_click_titlebar_action: TitlebarAction::Shade,
                window_placement_strategy: PlacementStrategy::Smart,
            },
            windows: BTreeMap::new(),
            focus_info: FocusInfo {
                focused_window: None,
                focus_stack: Vec::new(),
                last_focus_change: 0,
                focus_reason: FocusReason::SystemEvent,
            },
            pip_windows: Vec::new(),
            always_on_top_windows: Vec::new(),
            next_handle: 1,
            active_operations: BTreeMap::new(),
        })
    }

    /// Start the window manager
    pub fn start(&mut self) -> Result<(), DesktopError> {
        self.load_configuration()?;
        self.setup_event_handlers()?;
        Ok(())
    }

    /// Stop the window manager
    pub fn stop(&mut self) -> Result<(), DesktopError> {
        self.save_configuration()?;
        self.cleanup_resources()?;
        Ok(())
    }

    /// Register a new window
    pub fn register_window(&mut self, title: &str, app_id: &str, process_id: u32, rect: WindowRect) -> Result<WindowHandle, DesktopError> {
        use crate::kernel::graphics::{create_window};
        
        // Create window in graphics system first
        let graphics_window_id = create_window(
            title,
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            process_id
        );
        
        let handle = graphics_window_id as WindowHandle;
        
        let window = WindowInfo {
            handle,
            title: title.to_string(),
            app_id: app_id.to_string(),
            process_id,
            rect,
            state: WindowState::Normal,
            layer: WindowLayer::Normal,
            opacity: 1.0,
            visible: true,
            resizable: true,
            movable: true,
            closable: true,
            minimizable: true,
            maximizable: true,
            has_shadow: self.config.window_shadows,
            border_width: 1.0,
            corner_radius: 4.0,
            created_at: self.get_current_time(),
            last_focused: 0,
            metadata: BTreeMap::new(),
        };
        
        self.windows.insert(handle, window);
        self.focus_window(handle, FocusReason::WindowCreated)?;
        
        // Update next_handle to avoid conflicts
        if handle >= self.next_handle {
            self.next_handle = handle + 1;
        }
        
        Ok(handle)
    }

    /// Unregister a window
    pub fn unregister_window(&mut self, handle: WindowHandle) -> Result<(), DesktopError> {
        use crate::kernel::graphics::{destroy_window};
        
        if let Some(_window) = self.windows.remove(&handle) {
            // Destroy window in graphics system
            destroy_window(handle as u32);
            
            // Remove from special lists
            self.pip_windows.retain(|&h| h != handle);
            self.always_on_top_windows.retain(|&h| h != handle);
            
            // Update focus
            if self.focus_info.focused_window == Some(handle) {
                self.focus_next_window(FocusReason::WindowClosed)?;
            }
            
            // Remove from focus stack
            self.focus_info.focus_stack.retain(|&h| h != handle);
            
            // Cancel any active operations
            self.active_operations.remove(&handle);
        }
        
        Ok(())
    }

    /// Set window to always-on-top
    pub fn set_always_on_top(&mut self, handle: WindowHandle, enabled: bool) -> Result<(), DesktopError> {
        if let Some(window) = self.windows.get_mut(&handle) {
            if enabled {
                if !self.always_on_top_windows.contains(&handle) {
                    self.always_on_top_windows.push(handle);
                    window.layer = WindowLayer::AlwaysOnTop;
                    window.state = WindowState::AlwaysOnTop;
                    window.opacity = self.config.always_on_top_opacity;
                    
                    // Apply visual changes
                    self.apply_window_changes(handle)?;
                }
            } else {
                self.always_on_top_windows.retain(|&h| h != handle);
                window.layer = WindowLayer::Normal;
                if window.state == WindowState::AlwaysOnTop {
                    window.state = WindowState::Normal;
                }
                window.opacity = 1.0;
                
                // Apply visual changes
                self.apply_window_changes(handle)?;
            }
        }
        
        Ok(())
    }

    /// Enter Picture-in-Picture mode
    pub fn enter_picture_in_picture(&mut self, handle: WindowHandle) -> Result<(), DesktopError> {
        if let Some(window) = self.windows.get_mut(&handle) {
            // Store original state
            window.metadata.insert("pip_original_rect".to_string(), 
                format!("{},{},{},{}", window.rect.x, window.rect.y, window.rect.width, window.rect.height));
            window.metadata.insert("pip_original_state".to_string(), 
                format!("{:?}", window.state));
            
            // Calculate PiP position and size
            let pip_rect = self.calculate_pip_position(&self.config.pip_config.default_position, 
                self.config.pip_config.default_size)?;
            
            // Update window properties
            window.rect = pip_rect;
            window.state = WindowState::PictureInPicture;
            window.layer = WindowLayer::PictureInPicture;
            window.opacity = self.config.pip_config.opacity;
            window.resizable = self.config.pip_config.resize_handles;
            window.corner_radius = self.config.pip_config.corner_radius;
            window.has_shadow = self.config.pip_config.shadow_enabled;
            
            // Add to PiP list
            if !self.pip_windows.contains(&handle) {
                self.pip_windows.push(handle);
            }
            
            // Apply changes with animation
            if self.config.enable_animations {
                self.animate_window_operation(handle, WindowOperation {
                    operation_type: OperationType::EnterPip,
                    target_rect: Some(pip_rect),
                    target_state: Some(WindowState::PictureInPicture),
                    target_layer: Some(WindowLayer::PictureInPicture),
                    target_opacity: Some(self.config.pip_config.opacity),
                    animation: WindowAnimation::Scale,
                    duration: self.config.animation_duration,
                    delay: Duration::from_millis(0),
                })?;
            } else {
                self.apply_window_changes(handle)?;
            }
        }
        
        Ok(())
    }

    /// Exit Picture-in-Picture mode
    pub fn exit_picture_in_picture(&mut self, handle: WindowHandle) -> Result<(), DesktopError> {
        if let Some(window) = self.windows.get_mut(&handle) {
            if window.state == WindowState::PictureInPicture {
                // Restore original state
                if let Some(rect_str) = window.metadata.get("pip_original_rect") {
                    let parts: Vec<&str> = rect_str.split(',').collect();
                    if parts.len() == 4 {
                        if let (Ok(x), Ok(y), Ok(w), Ok(h)) = (
                            parts[0].parse::<i32>(),
                            parts[1].parse::<i32>(),
                            parts[2].parse::<u32>(),
                            parts[3].parse::<u32>(),
                        ) {
                            window.rect = WindowRect { x, y, width: w, height: h };
                        }
                    }
                }
                
                if let Some(state_str) = window.metadata.get("pip_original_state") {
                    // Parse and restore original state (simplified)
                    window.state = WindowState::Normal;
                }
                
                window.layer = WindowLayer::Normal;
                window.opacity = 1.0;
                window.resizable = true;
                window.corner_radius = 4.0;
                window.has_shadow = self.config.window_shadows;
                
                // Remove from PiP list
                self.pip_windows.retain(|&h| h != handle);
                
                // Clean up metadata
                window.metadata.remove("pip_original_rect");
                window.metadata.remove("pip_original_state");
                
                // Apply changes with animation
                if self.config.enable_animations {
                    self.animate_window_operation(handle, WindowOperation {
                        operation_type: OperationType::ExitPip,
                        target_rect: Some(window.rect),
                        target_state: Some(window.state),
                        target_layer: Some(window.layer),
                        target_opacity: Some(1.0),
                        animation: WindowAnimation::Scale,
                        duration: self.config.animation_duration,
                        delay: Duration::from_millis(0),
                    })?;
                } else {
                    self.apply_window_changes(handle)?;
                }
            }
        }
        
        Ok(())
    }

    /// Toggle Picture-in-Picture mode
    pub fn toggle_picture_in_picture(&mut self, handle: WindowHandle) -> Result<(), DesktopError> {
        if let Some(window) = self.windows.get(&handle) {
            if window.state == WindowState::PictureInPicture {
                self.exit_picture_in_picture(handle)
            } else {
                self.enter_picture_in_picture(handle)
            }
        } else {
            Err(DesktopError::WindowNotFound)
        }
    }

    /// Move PiP window
    pub fn move_pip_window(&mut self, handle: WindowHandle, x: i32, y: i32) -> Result<(), DesktopError> {
        if let Some(window) = self.windows.get_mut(&handle) {
            if window.state == WindowState::PictureInPicture {
                let mut new_rect = window.rect;
                new_rect.x = x;
                new_rect.y = y;
                
                // Snap to edges if enabled
                if self.config.pip_config.snap_to_edges {
                    new_rect = self.snap_to_screen_edges(new_rect, self.config.pip_config.snap_threshold)?;
                }
                
                window.rect = new_rect;
                self.apply_window_changes(handle)?;
            }
        }
        
        Ok(())
    }

    /// Resize PiP window
    pub fn resize_pip_window(&mut self, handle: WindowHandle, width: u32, height: u32) -> Result<(), DesktopError> {
        if let Some(window) = self.windows.get_mut(&handle) {
            if window.state == WindowState::PictureInPicture {
                let min_size = self.config.pip_config.min_size;
                let max_size = self.config.pip_config.max_size;
                
                let new_width = width.max(min_size.0).min(max_size.0);
                let new_height = height.max(min_size.1).min(max_size.1);
                
                window.rect.width = new_width;
                window.rect.height = new_height;
                
                self.apply_window_changes(handle)?;
            }
        }
        
        Ok(())
    }

    /// Focus a window
    pub fn focus_window(&mut self, handle: WindowHandle, reason: FocusReason) -> Result<(), DesktopError> {
        if self.windows.contains_key(&handle) {
            // Update focus info
            self.focus_info.focused_window = Some(handle);
            self.focus_info.last_focus_change = self.get_current_time();
            self.focus_info.focus_reason = reason;
            
            // Update focus stack
            self.focus_info.focus_stack.retain(|&h| h != handle);
            self.focus_info.focus_stack.insert(0, handle);
            
            // Limit focus stack size
            if self.focus_info.focus_stack.len() > 20 {
                self.focus_info.focus_stack.truncate(20);
            }
            
            // Update window last focused time
            if let Some(window) = self.windows.get_mut(&handle) {
                window.last_focused = self.get_current_time();
            }
            
            // Bring window to front of its layer
            self.bring_to_front(handle)?;
        }
        
        Ok(())
    }

    /// Focus next window
    pub fn focus_next_window(&mut self, reason: FocusReason) -> Result<(), DesktopError> {
        if let Some(next_handle) = self.focus_info.focus_stack.get(1).copied() {
            self.focus_window(next_handle, reason)?;
        } else if let Some(&first_handle) = self.windows.keys().next() {
            self.focus_window(first_handle, reason)?;
        } else {
            self.focus_info.focused_window = None;
        }
        
        Ok(())
    }

    /// Minimize window
    pub fn minimize_window(&mut self, handle: WindowHandle) -> Result<(), DesktopError> {
        if let Some(window) = self.windows.get_mut(&handle) {
            if window.minimizable && window.state != WindowState::Minimized {
                window.metadata.insert("minimized_from_state".to_string(), format!("{:?}", window.state));
                window.state = WindowState::Minimized;
                window.visible = false;
                
                if self.config.enable_animations {
                    self.animate_window_operation(handle, WindowOperation {
                        operation_type: OperationType::Minimize,
                        target_rect: None,
                        target_state: Some(WindowState::Minimized),
                        target_layer: None,
                        target_opacity: Some(0.0),
                        animation: WindowAnimation::Scale,
                        duration: self.config.animation_duration,
                        delay: Duration::from_millis(0),
                    })?;
                } else {
                    self.apply_window_changes(handle)?;
                }
                
                // Focus next window
                if self.focus_info.focused_window == Some(handle) {
                    self.focus_next_window(FocusReason::WindowMinimized)?;
                }
            }
        }
        
        Ok(())
    }

    /// Restore window
    pub fn restore_window(&mut self, handle: WindowHandle) -> Result<(), DesktopError> {
        if let Some(window) = self.windows.get_mut(&handle) {
            if window.state == WindowState::Minimized {
                // Restore previous state
                if let Some(prev_state_str) = window.metadata.get("minimized_from_state") {
                    // Parse and restore (simplified)
                    window.state = WindowState::Normal;
                } else {
                    window.state = WindowState::Normal;
                }
                
                window.visible = true;
                window.opacity = if self.always_on_top_windows.contains(&handle) {
                    self.config.always_on_top_opacity
                } else {
                    1.0
                };
                
                if self.config.enable_animations {
                    self.animate_window_operation(handle, WindowOperation {
                        operation_type: OperationType::Restore,
                        target_rect: None,
                        target_state: Some(window.state),
                        target_layer: None,
                        target_opacity: Some(window.opacity),
                        animation: WindowAnimation::Scale,
                        duration: self.config.animation_duration,
                        delay: Duration::from_millis(0),
                    })?;
                } else {
                    self.apply_window_changes(handle)?;
                }
                
                // Focus the restored window
                self.focus_window(handle, FocusReason::WindowRestored)?;
                
                // Clean up metadata
                window.metadata.remove("minimized_from_state");
            }
        }
        
        Ok(())
    }

    /// Maximize window
    pub fn maximize_window(&mut self, handle: WindowHandle) -> Result<(), DesktopError> {
        if let Some(window) = self.windows.get_mut(&handle) {
            if window.maximizable && window.state != WindowState::Maximized {
                // Store original rect
                window.metadata.insert("maximized_from_rect".to_string(),
                    format!("{},{},{},{}", window.rect.x, window.rect.y, window.rect.width, window.rect.height));
                window.metadata.insert("maximized_from_state".to_string(), format!("{:?}", window.state));
                
                // Get screen dimensions (simplified)
                let screen_rect = WindowRect { x: 0, y: 0, width: 1920, height: 1080 };
                
                window.rect = screen_rect;
                window.state = WindowState::Maximized;
                
                if self.config.enable_animations {
                    self.animate_window_operation(handle, WindowOperation {
                        operation_type: OperationType::Maximize,
                        target_rect: Some(screen_rect),
                        target_state: Some(WindowState::Maximized),
                        target_layer: None,
                        target_opacity: None,
                        animation: WindowAnimation::Scale,
                        duration: self.config.animation_duration,
                        delay: Duration::from_millis(0),
                    })?;
                } else {
                    self.apply_window_changes(handle)?;
                }
            }
        }
        
        Ok(())
    }

    /// Set window opacity
    pub fn set_window_opacity(&mut self, handle: WindowHandle, opacity: f32) -> Result<(), DesktopError> {
        if let Some(window) = self.windows.get_mut(&handle) {
            let clamped_opacity = opacity.max(0.0).min(1.0);
            window.opacity = clamped_opacity;
            self.apply_window_changes(handle)?;
        }
        
        Ok(())
    }

    /// Move window
    pub fn move_window(&mut self, handle: WindowHandle, x: i32, y: i32) -> Result<(), DesktopError> {
        if let Some(window) = self.windows.get_mut(&handle) {
            if window.movable {
                window.rect.x = x;
                window.rect.y = y;
                self.apply_window_changes(handle)?;
            }
        }
        
        Ok(())
    }

    /// Resize window
    pub fn resize_window(&mut self, handle: WindowHandle, width: u32, height: u32) -> Result<(), DesktopError> {
        if let Some(window) = self.windows.get_mut(&handle) {
            if window.resizable {
                window.rect.width = width;
                window.rect.height = height;
                self.apply_window_changes(handle)?;
            }
        }
        
        Ok(())
    }

    /// Helper functions
    fn calculate_pip_position(&self, position: &PipPosition, size: (u32, u32)) -> Result<WindowRect, DesktopError> {
        // Get screen dimensions (simplified)
        let screen_width = 1920;
        let screen_height = 1080;
        let margin = 20;
        
        let (x, y) = match position {
            PipPosition::TopLeft => (margin, margin),
            PipPosition::TopRight => (screen_width - size.0 as i32 - margin, margin),
            PipPosition::BottomLeft => (margin, screen_height - size.1 as i32 - margin),
            PipPosition::BottomRight => (
                screen_width - size.0 as i32 - margin,
                screen_height - size.1 as i32 - margin
            ),
            PipPosition::Custom(cx, cy) => (*cx, *cy),
        };
        
        Ok(WindowRect {
            x,
            y,
            width: size.0,
            height: size.1,
        })
    }
    
    fn snap_to_screen_edges(&self, rect: WindowRect, threshold: u32) -> Result<WindowRect, DesktopError> {
        let screen_width = 1920;
        let screen_height = 1080;
        let threshold = threshold as i32;
        
        let mut new_rect = rect;
        
        // Snap to left edge
        if rect.x.abs() <= threshold {
            new_rect.x = 0;
        }
        
        // Snap to right edge
        if (screen_width - (rect.x + rect.width as i32)).abs() <= threshold {
            new_rect.x = screen_width - rect.width as i32;
        }
        
        // Snap to top edge
        if rect.y.abs() <= threshold {
            new_rect.y = 0;
        }
        
        // Snap to bottom edge
        if (screen_height - (rect.y + rect.height as i32)).abs() <= threshold {
            new_rect.y = screen_height - rect.height as i32;
        }
        
        Ok(new_rect)
    }
    
    fn bring_to_front(&mut self, handle: WindowHandle) -> Result<(), DesktopError> {
        // Update window z-order by focusing the window in the graphics system
        use crate::kernel::graphics::{focus_window};
        
        if focus_window(handle as u32) {
            Ok(())
        } else {
            Err(DesktopError::WindowNotFound)
        }
    }
    
    fn animate_window_operation(&mut self, handle: WindowHandle, operation: WindowOperation) -> Result<(), DesktopError> {
        // Store operation for processing
        self.active_operations.entry(handle).or_insert_with(Vec::new).push(operation);
        
        // In real implementation, would start animation
        // For now, just apply the changes immediately
        self.apply_window_changes(handle)?;
        
        Ok(())
    }
    
    fn apply_window_changes(&self, handle: WindowHandle) -> Result<(), DesktopError> {
        use crate::kernel::graphics::{move_window, resize_window, set_window_state};
        
        if let Some(window) = self.windows.get(&handle) {
            // Apply position and size changes
            move_window(handle as u32, window.rect.x, window.rect.y)
                .map_err(|_| DesktopError::WindowNotFound)?;
            
            resize_window(handle as u32, window.rect.width, window.rect.height)
                .map_err(|_| DesktopError::WindowNotFound)?;
            
            // Apply state changes
            let graphics_state = match window.state {
                WindowState::Normal => crate::kernel::graphics::WindowState::Normal,
                WindowState::Minimized => crate::kernel::graphics::WindowState::Minimized,
                WindowState::Maximized => crate::kernel::graphics::WindowState::Maximized,
                WindowState::Fullscreen => crate::kernel::graphics::WindowState::Fullscreen,
                WindowState::PictureInPicture => crate::kernel::graphics::WindowState::Normal, // Map to normal for now
                WindowState::AlwaysOnTop => crate::kernel::graphics::WindowState::Normal,
            };
            
            set_window_state(handle as u32, graphics_state)
                .map_err(|_| DesktopError::WindowNotFound)?;
            
            Ok(())
        } else {
            Err(DesktopError::WindowNotFound)
        }
    }
    
    fn setup_event_handlers(&mut self) -> Result<(), DesktopError> {
        // In real implementation, would set up window event handlers
        Ok(())
    }
    
    fn load_configuration(&mut self) -> Result<(), DesktopError> {
        // In real implementation, would load config from disk
        Ok(())
    }
    
    fn save_configuration(&self) -> Result<(), DesktopError> {
        // In real implementation, would save config to disk
        Ok(())
    }
    
    fn cleanup_resources(&mut self) -> Result<(), DesktopError> {
        self.active_operations.clear();
        Ok(())
    }
    
    fn get_current_time(&self) -> u64 {
        use crate::kernel::time::get_timestamp;
        get_timestamp()
    }

    /// Get window information
    pub fn get_window(&self, handle: WindowHandle) -> Option<&WindowInfo> {
        self.windows.get(&handle)
    }
    
    /// Get all windows
    pub fn get_all_windows(&self) -> &BTreeMap<WindowHandle, WindowInfo> {
        &self.windows
    }
    
    /// Get Picture-in-Picture windows
    pub fn get_pip_windows(&self) -> &Vec<WindowHandle> {
        &self.pip_windows
    }
    
    /// Get always-on-top windows
    pub fn get_always_on_top_windows(&self) -> &Vec<WindowHandle> {
        &self.always_on_top_windows
    }
    
    /// Get focused window
    pub fn get_focused_window(&self) -> Option<WindowHandle> {
        self.focus_info.focused_window
    }
    
    /// Get focus information
    pub fn get_focus_info(&self) -> &FocusInfo {
        &self.focus_info
    }
    
    /// Get configuration
    pub fn get_config(&self) -> &WindowManagerConfig {
        &self.config
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: WindowManagerConfig) -> Result<(), DesktopError> {
        self.config = config;
        Ok(())
    }
}

impl WindowRect {
    /// Create a new WindowRect
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        WindowRect { x, y, width, height }
    }
    
    /// Check if point is inside rectangle
    pub fn contains_point(&self, x: i32, y: i32) -> bool {
        x >= self.x && x < self.x + self.width as i32 &&
        y >= self.y && y < self.y + self.height as i32
    }
    
    /// Check if rectangles intersect
    pub fn intersects(&self, other: &WindowRect) -> bool {
        self.x < other.x + other.width as i32 &&
        self.x + self.width as i32 > other.x &&
        self.y < other.y + other.height as i32 &&
        self.y + self.height as i32 > other.y
    }
    
    /// Get center point
    pub fn center(&self) -> (i32, i32) {
        (self.x + self.width as i32 / 2, self.y + self.height as i32 / 2)
    }
    
    /// Get area
    pub fn area(&self) -> u64 {
        self.width as u64 * self.height as u64
    }
}