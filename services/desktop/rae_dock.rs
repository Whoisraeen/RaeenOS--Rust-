//! RaeDock - Dock/Taskbar Hybrid
//! Combines the best of macOS dock and Windows taskbar with live badges and progress bars
//! Features app launching, window management, live tiles, and contextual menus

use crate::services::desktop::DesktopError;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::collections::BTreeMap;
use core::time::Duration;

/// Dock position on screen
#[derive(Debug, Clone, PartialEq)]
pub enum DockPosition {
    Bottom,
    Top,
    Left,
    Right,
    Floating,
}

/// Dock size configuration
#[derive(Debug, Clone, PartialEq)]
pub enum DockSize {
    Small,
    Medium,
    Large,
    Auto,
}

/// Dock behavior settings
#[derive(Debug, Clone)]
pub struct DockBehavior {
    pub auto_hide: bool,
    pub magnification: bool,
    pub magnification_scale: f32,
    pub show_recent_apps: bool,
    pub show_running_indicators: bool,
    pub animate_launch: bool,
    pub bounce_on_launch: bool,
    pub minimize_to_dock: bool,
}

/// App icon configuration
#[derive(Debug, Clone)]
pub struct AppIcon {
    pub app_id: String,
    pub name: String,
    pub icon_path: String,
    pub executable_path: String,
    pub is_pinned: bool,
    pub is_running: bool,
    pub window_count: u32,
    pub badge: Option<Badge>,
    pub progress: Option<ProgressInfo>,
    pub position: u32,
    pub last_used: u64,
}

/// Badge information for app icons
#[derive(Debug, Clone)]
pub struct Badge {
    pub badge_type: BadgeType,
    pub content: String,
    pub color: String,
    pub priority: BadgePriority,
    pub expires_at: Option<u64>,
}

/// Badge types
#[derive(Debug, Clone, PartialEq)]
pub enum BadgeType {
    Number,
    Text,
    Dot,
    Icon,
    Progress,
}

/// Badge priority levels
#[derive(Debug, Clone, PartialEq)]
pub enum BadgePriority {
    Low,
    Normal,
    High,
    Critical,
}

/// Progress information
#[derive(Debug, Clone)]
pub struct ProgressInfo {
    pub progress_type: ProgressType,
    pub value: f32, // 0.0 to 1.0
    pub state: ProgressState,
    pub description: Option<String>,
}

/// Progress types
#[derive(Debug, Clone, PartialEq)]
pub enum ProgressType {
    Determinate,
    Indeterminate,
    Paused,
    Error,
}

/// Progress states
#[derive(Debug, Clone, PartialEq)]
pub enum ProgressState {
    Normal,
    Paused,
    Error,
    Warning,
}

/// Live tile information
#[derive(Debug, Clone)]
pub struct LiveTile {
    pub app_id: String,
    pub content: TileContent,
    pub update_interval: Duration,
    pub last_updated: u64,
    pub enabled: bool,
}

/// Live tile content
#[derive(Debug, Clone)]
pub enum TileContent {
    Text {
        title: String,
        subtitle: Option<String>,
        body: String,
    },
    Image {
        image_data: Vec<u8>,
        caption: Option<String>,
    },
    Chart {
        data_points: Vec<f32>,
        chart_type: ChartType,
        color: String,
    },
    Custom {
        html_content: String,
    },
}

/// Chart types for live tiles
#[derive(Debug, Clone)]
pub enum ChartType {
    Line,
    Bar,
    Pie,
    Gauge,
}

/// Context menu item
#[derive(Debug, Clone)]
pub struct ContextMenuItem {
    pub id: String,
    pub label: String,
    pub icon: Option<String>,
    pub action: ContextAction,
    pub enabled: bool,
    pub separator_after: bool,
    pub submenu: Option<Vec<ContextMenuItem>>,
}

/// Context menu actions
#[derive(Debug, Clone)]
pub enum ContextAction {
    Launch,
    Quit,
    Hide,
    Show,
    Minimize,
    Maximize,
    Pin,
    Unpin,
    RemoveFromDock,
    ShowInFinder,
    GetInfo,
    Custom(String),
}

/// Window information for taskbar functionality
#[derive(Debug, Clone)]
pub struct WindowInfo {
    pub window_id: u64,
    pub app_id: String,
    pub title: String,
    pub is_minimized: bool,
    pub is_focused: bool,
    pub thumbnail: Option<Vec<u8>>,
    pub last_active: u64,
}

/// Dock configuration
#[derive(Debug, Clone)]
pub struct DockConfig {
    pub position: DockPosition,
    pub size: DockSize,
    pub behavior: DockBehavior,
    pub theme: DockTheme,
    pub shortcuts: BTreeMap<String, String>,
    pub max_recent_apps: u32,
    pub tile_update_interval: Duration,
}

/// Dock theme configuration
#[derive(Debug, Clone)]
pub struct DockTheme {
    pub background_color: String,
    pub background_opacity: f32,
    pub border_radius: f32,
    pub icon_size: u32,
    pub spacing: u32,
    pub shadow_enabled: bool,
    pub blur_enabled: bool,
}

/// Dock state
#[derive(Debug, Clone)]
pub struct DockState {
    pub visible: bool,
    pub auto_hidden: bool,
    pub mouse_over: bool,
    pub dragging_icon: Option<String>,
    pub context_menu_open: Option<String>,
}

/// RaeDock main service
pub struct RaeDock {
    config: DockConfig,
    pinned_apps: Vec<AppIcon>,
    running_apps: BTreeMap<String, AppIcon>,
    recent_apps: Vec<String>,
    live_tiles: BTreeMap<String, LiveTile>,
    windows: BTreeMap<u64, WindowInfo>,
    state: DockState,
    context_menus: BTreeMap<String, Vec<ContextMenuItem>>,
}

impl RaeDock {
    /// Create a new RaeDock instance
    pub fn new() -> Result<Self, DesktopError> {
        let mut dock = RaeDock {
            config: DockConfig {
                position: DockPosition::Bottom,
                size: DockSize::Medium,
                behavior: DockBehavior {
                    auto_hide: false,
                    magnification: true,
                    magnification_scale: 1.5,
                    show_recent_apps: true,
                    show_running_indicators: true,
                    animate_launch: true,
                    bounce_on_launch: true,
                    minimize_to_dock: true,
                },
                theme: DockTheme {
                    background_color: "rgba(0, 0, 0, 0.8)".to_string(),
                    background_opacity: 0.8,
                    border_radius: 12.0,
                    icon_size: 48,
                    spacing: 8,
                    shadow_enabled: true,
                    blur_enabled: true,
                },
                shortcuts: BTreeMap::new(),
                max_recent_apps: 10,
                tile_update_interval: Duration::from_secs(30),
            },
            pinned_apps: Vec::new(),
            running_apps: BTreeMap::new(),
            recent_apps: Vec::new(),
            live_tiles: BTreeMap::new(),
            windows: BTreeMap::new(),
            state: DockState {
                visible: true,
                auto_hidden: false,
                mouse_over: false,
                dragging_icon: None,
                context_menu_open: None,
            },
            context_menus: BTreeMap::new(),
        };

        dock.setup_default_apps()?;
        dock.setup_default_shortcuts()?;
        Ok(dock)
    }

    /// Start the RaeDock service
    pub fn start(&mut self) -> Result<(), DesktopError> {
        self.load_configuration()?;
        self.start_window_monitoring()?;
        self.start_live_tile_updates()?;
        Ok(())
    }

    /// Stop the RaeDock service
    pub fn stop(&mut self) -> Result<(), DesktopError> {
        self.save_configuration()?;
        self.stop_live_tile_updates()?;
        Ok(())
    }

    /// Setup default pinned applications
    fn setup_default_apps(&mut self) -> Result<(), DesktopError> {
        let default_apps = vec![
            AppIcon {
                app_id: "rae_finder".to_string(),
                name: "RaeFinder".to_string(),
                icon_path: "/system/icons/rae_finder.svg".to_string(),
                executable_path: "/system/apps/rae_finder".to_string(),
                is_pinned: true,
                is_running: false,
                window_count: 0,
                badge: None,
                progress: None,
                position: 0,
                last_used: 0,
            },
            AppIcon {
                app_id: "rae_terminal".to_string(),
                name: "RaeTerminal".to_string(),
                icon_path: "/system/icons/terminal.svg".to_string(),
                executable_path: "/system/apps/terminal".to_string(),
                is_pinned: true,
                is_running: false,
                window_count: 0,
                badge: None,
                progress: None,
                position: 1,
                last_used: 0,
            },
            AppIcon {
                app_id: "rae_browser".to_string(),
                name: "RaeBrowser".to_string(),
                icon_path: "/system/icons/browser.svg".to_string(),
                executable_path: "/system/apps/browser".to_string(),
                is_pinned: true,
                is_running: false,
                window_count: 0,
                badge: None,
                progress: None,
                position: 2,
                last_used: 0,
            },
            AppIcon {
                app_id: "rae_settings".to_string(),
                name: "Settings".to_string(),
                icon_path: "/system/icons/settings.svg".to_string(),
                executable_path: "/system/apps/settings".to_string(),
                is_pinned: true,
                is_running: false,
                window_count: 0,
                badge: None,
                progress: None,
                position: 3,
                last_used: 0,
            },
        ];

        self.pinned_apps = default_apps;
        Ok(())
    }

    /// Setup default keyboard shortcuts
    fn setup_default_shortcuts(&mut self) -> Result<(), DesktopError> {
        self.config.shortcuts.insert("Cmd+Space".to_string(), "toggle_dock".to_string());
        self.config.shortcuts.insert("Cmd+Tab".to_string(), "app_switcher".to_string());
        self.config.shortcuts.insert("Cmd+H".to_string(), "hide_app".to_string());
        self.config.shortcuts.insert("Cmd+M".to_string(), "minimize_window".to_string());
        self.config.shortcuts.insert("Cmd+Q".to_string(), "quit_app".to_string());
        Ok(())
    }

    /// Launch application
    pub fn launch_app(&mut self, app_id: &str) -> Result<(), DesktopError> {
        if let Some(app) = self.find_app_mut(app_id) {
            if !app.is_running {
                // Simulate app launch
                app.is_running = true;
                app.window_count = 1;
                app.last_used = self.get_current_time();
                
                // Add to running apps if not pinned
                if !app.is_pinned {
                    self.running_apps.insert(app_id.to_string(), app.clone());
                }
                
                // Add to recent apps
                self.add_to_recent_apps(app_id);
                
                // Animate launch if enabled
                if self.config.behavior.animate_launch {
                    self.animate_app_launch(app_id)?;
                }
            } else {
                // App is already running, bring to front
                self.bring_app_to_front(app_id)?;
            }
        }
        Ok(())
    }

    /// Quit application
    pub fn quit_app(&mut self, app_id: &str) -> Result<(), DesktopError> {
        if let Some(app) = self.find_app_mut(app_id) {
            app.is_running = false;
            app.window_count = 0;
            app.badge = None;
            app.progress = None;
            
            // Remove from running apps if not pinned
            if !app.is_pinned {
                self.running_apps.remove(app_id);
            }
        }
        
        // Remove associated windows
        self.windows.retain(|_, window| window.app_id != app_id);
        Ok(())
    }

    /// Pin application to dock
    pub fn pin_app(&mut self, app_id: &str) -> Result<(), DesktopError> {
        if let Some(app) = self.running_apps.get(app_id) {
            let mut pinned_app = app.clone();
            pinned_app.is_pinned = true;
            pinned_app.position = self.pinned_apps.len() as u32;
            
            self.pinned_apps.push(pinned_app);
            self.running_apps.remove(app_id);
        }
        Ok(())
    }

    /// Unpin application from dock
    pub fn unpin_app(&mut self, app_id: &str) -> Result<(), DesktopError> {
        if let Some(pos) = self.pinned_apps.iter().position(|app| app.app_id == app_id) {
            let mut app = self.pinned_apps.remove(pos);
            
            if app.is_running {
                app.is_pinned = false;
                self.running_apps.insert(app_id.to_string(), app);
            }
            
            // Update positions of remaining pinned apps
            for (i, app) in self.pinned_apps.iter_mut().enumerate() {
                app.position = i as u32;
            }
        }
        Ok(())
    }

    /// Update app badge
    pub fn update_badge(&mut self, app_id: &str, badge: Option<Badge>) -> Result<(), DesktopError> {
        if let Some(app) = self.find_app_mut(app_id) {
            app.badge = badge;
        }
        Ok(())
    }

    /// Update app progress
    pub fn update_progress(&mut self, app_id: &str, progress: Option<ProgressInfo>) -> Result<(), DesktopError> {
        if let Some(app) = self.find_app_mut(app_id) {
            app.progress = progress;
        }
        Ok(())
    }

    /// Add window to dock tracking
    pub fn add_window(&mut self, window: WindowInfo) -> Result<(), DesktopError> {
        // Update app window count
        if let Some(app) = self.find_app_mut(&window.app_id) {
            app.window_count += 1;
            app.is_running = true;
        }
        
        self.windows.insert(window.window_id, window);
        Ok(())
    }

    /// Remove window from dock tracking
    pub fn remove_window(&mut self, window_id: u64) -> Result<(), DesktopError> {
        if let Some(window) = self.windows.remove(&window_id) {
            // Update app window count
            if let Some(app) = self.find_app_mut(&window.app_id) {
                if app.window_count > 0 {
                    app.window_count -= 1;
                }
                
                if app.window_count == 0 {
                    app.is_running = false;
                    
                    // Remove from running apps if not pinned
                    if !app.is_pinned {
                        self.running_apps.remove(&window.app_id);
                    }
                }
            }
        }
        Ok(())
    }

    /// Show context menu for app
    pub fn show_context_menu(&mut self, app_id: &str) -> Result<Vec<ContextMenuItem>, DesktopError> {
        let menu = self.build_context_menu(app_id)?;
        self.state.context_menu_open = Some(app_id.to_string());
        Ok(menu)
    }

    /// Hide context menu
    pub fn hide_context_menu(&mut self) -> Result<(), DesktopError> {
        self.state.context_menu_open = None;
        Ok(())
    }

    /// Build context menu for app
    fn build_context_menu(&self, app_id: &str) -> Result<Vec<ContextMenuItem>, DesktopError> {
        let mut menu = Vec::new();
        
        if let Some(app) = self.find_app(app_id) {
            if app.is_running {
                menu.push(ContextMenuItem {
                    id: "show".to_string(),
                    label: "Show".to_string(),
                    icon: None,
                    action: ContextAction::Show,
                    enabled: true,
                    separator_after: false,
                    submenu: None,
                });
                
                menu.push(ContextMenuItem {
                    id: "hide".to_string(),
                    label: "Hide".to_string(),
                    icon: None,
                    action: ContextAction::Hide,
                    enabled: true,
                    separator_after: false,
                    submenu: None,
                });
                
                menu.push(ContextMenuItem {
                    id: "quit".to_string(),
                    label: "Quit".to_string(),
                    icon: None,
                    action: ContextAction::Quit,
                    enabled: true,
                    separator_after: true,
                    submenu: None,
                });
            } else {
                menu.push(ContextMenuItem {
                    id: "launch".to_string(),
                    label: "Open".to_string(),
                    icon: None,
                    action: ContextAction::Launch,
                    enabled: true,
                    separator_after: true,
                    submenu: None,
                });
            }
            
            if app.is_pinned {
                menu.push(ContextMenuItem {
                    id: "unpin".to_string(),
                    label: "Remove from Dock".to_string(),
                    icon: None,
                    action: ContextAction::Unpin,
                    enabled: true,
                    separator_after: false,
                    submenu: None,
                });
            } else {
                menu.push(ContextMenuItem {
                    id: "pin".to_string(),
                    label: "Keep in Dock".to_string(),
                    icon: None,
                    action: ContextAction::Pin,
                    enabled: true,
                    separator_after: false,
                    submenu: None,
                });
            }
            
            menu.push(ContextMenuItem {
                id: "show_in_finder".to_string(),
                label: "Show in RaeFinder".to_string(),
                icon: None,
                action: ContextAction::ShowInFinder,
                enabled: true,
                separator_after: false,
                submenu: None,
            });
        }
        
        Ok(menu)
    }

    /// Handle context menu action
    pub fn handle_context_action(&mut self, app_id: &str, action: ContextAction) -> Result<(), DesktopError> {
        match action {
            ContextAction::Launch => self.launch_app(app_id)?,
            ContextAction::Quit => self.quit_app(app_id)?,
            ContextAction::Pin => self.pin_app(app_id)?,
            ContextAction::Unpin => self.unpin_app(app_id)?,
            ContextAction::Show => self.bring_app_to_front(app_id)?,
            ContextAction::Hide => self.hide_app(app_id)?,
            _ => {}, // Handle other actions as needed
        }
        
        self.hide_context_menu()?;
        Ok(())
    }

    /// Create live tile for app
    pub fn create_live_tile(&mut self, app_id: &str, content: TileContent) -> Result<(), DesktopError> {
        let tile = LiveTile {
            app_id: app_id.to_string(),
            content,
            update_interval: self.config.tile_update_interval,
            last_updated: self.get_current_time(),
            enabled: true,
        };
        
        self.live_tiles.insert(app_id.to_string(), tile);
        Ok(())
    }

    /// Update live tile content
    pub fn update_live_tile(&mut self, app_id: &str, content: TileContent) -> Result<(), DesktopError> {
        if let Some(tile) = self.live_tiles.get_mut(app_id) {
            tile.content = content;
            tile.last_updated = self.get_current_time();
        }
        Ok(())
    }

    /// Toggle dock visibility
    pub fn toggle_dock(&mut self) -> Result<(), DesktopError> {
        self.state.visible = !self.state.visible;
        Ok(())
    }

    /// Set dock position
    pub fn set_position(&mut self, position: DockPosition) -> Result<(), DesktopError> {
        self.config.position = position;
        Ok(())
    }

    /// Set dock size
    pub fn set_size(&mut self, size: DockSize) -> Result<(), DesktopError> {
        self.config.size = size;
        Ok(())
    }

    /// Helper functions
    fn find_app(&self, app_id: &str) -> Option<&AppIcon> {
        self.pinned_apps.iter().find(|app| app.app_id == app_id)
            .or_else(|| self.running_apps.get(app_id))
    }
    
    fn find_app_mut(&mut self, app_id: &str) -> Option<&mut AppIcon> {
        if let Some(app) = self.pinned_apps.iter_mut().find(|app| app.app_id == app_id) {
            Some(app)
        } else {
            self.running_apps.get_mut(app_id)
        }
    }
    
    fn add_to_recent_apps(&mut self, app_id: &str) {
        self.recent_apps.retain(|id| id != app_id);
        self.recent_apps.insert(0, app_id.to_string());
        
        if self.recent_apps.len() > self.config.max_recent_apps as usize {
            self.recent_apps.truncate(self.config.max_recent_apps as usize);
        }
    }
    
    fn animate_app_launch(&self, _app_id: &str) -> Result<(), DesktopError> {
        // In real implementation, would trigger launch animation
        Ok(())
    }
    
    fn bring_app_to_front(&self, _app_id: &str) -> Result<(), DesktopError> {
        // In real implementation, would bring app windows to front
        Ok(())
    }
    
    fn hide_app(&self, _app_id: &str) -> Result<(), DesktopError> {
        // In real implementation, would hide app windows
        Ok(())
    }
    
    fn get_current_time(&self) -> u64 {
        // In real implementation, would return current timestamp
        1640995200
    }
    
    fn load_configuration(&mut self) -> Result<(), DesktopError> {
        // In real implementation, would load config from disk
        Ok(())
    }
    
    fn save_configuration(&self) -> Result<(), DesktopError> {
        // In real implementation, would save config to disk
        Ok(())
    }
    
    fn start_window_monitoring(&mut self) -> Result<(), DesktopError> {
        // In real implementation, would start monitoring window events
        Ok(())
    }
    
    fn start_live_tile_updates(&mut self) -> Result<(), DesktopError> {
        // In real implementation, would start periodic tile updates
        Ok(())
    }
    
    fn stop_live_tile_updates(&mut self) -> Result<(), DesktopError> {
        // In real implementation, would stop tile update timer
        Ok(())
    }

    /// Get all dock apps (pinned + running)
    pub fn get_all_apps(&self) -> Vec<&AppIcon> {
        let mut apps: Vec<&AppIcon> = self.pinned_apps.iter().collect();
        
        // Add running apps that aren't pinned
        for app in self.running_apps.values() {
            if !self.pinned_apps.iter().any(|pinned| pinned.app_id == app.app_id) {
                apps.push(app);
            }
        }
        
        apps
    }
    
    /// Get dock configuration
    pub fn get_config(&self) -> &DockConfig {
        &self.config
    }
    
    /// Get dock state
    pub fn get_state(&self) -> &DockState {
        &self.state
    }
    
    /// Update dock configuration
    pub fn update_config(&mut self, config: DockConfig) -> Result<(), DesktopError> {
        self.config = config;
        Ok(())
    }
}