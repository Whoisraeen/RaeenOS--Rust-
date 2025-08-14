//! RaeenDE - Desktop Environment for RaeenOS
//! Provides theming engine, flexible window management, virtual desktops, and widgets

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::format;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::graphics::{Color, Rect, Point, WindowId, Widget, WidgetType};
use crate::process::ProcessId;

/// Desktop theme configuration
#[derive(Debug, Clone)]
pub struct DesktopTheme {
    pub name: String,
    pub version: String,
    pub author: String,
    
    // Color scheme
    pub primary_color: Color,
    pub secondary_color: Color,
    pub accent_color: Color,
    pub background_color: Color,
    pub surface_color: Color,
    pub text_color: Color,
    pub text_secondary_color: Color,
    pub border_color: Color,
    pub shadow_color: Color,
    pub error_color: Color,
    pub warning_color: Color,
    pub success_color: Color,
    
    // Visual effects
    pub glassmorphism_enabled: bool,
    pub blur_radius: u32,
    pub transparency_level: u8,
    pub corner_radius: u32,
    pub shadow_enabled: bool,
    pub shadow_offset: Point,
    pub shadow_blur: u32,
    pub animations_enabled: bool,
    pub animation_duration_ms: u32,
    
    // Typography
    pub font_family: String,
    pub font_size: u32,
    pub font_weight: FontWeight,
    pub line_height: f32,
    
    // Spacing
    pub padding_small: u32,
    pub padding_medium: u32,
    pub padding_large: u32,
    pub margin_small: u32,
    pub margin_medium: u32,
    pub margin_large: u32,
    
    // Window decorations
    pub window_title_height: u32,
    pub window_border_width: u32,
    pub window_button_size: u32,
    
    // Taskbar
    pub taskbar_height: u32,
    pub taskbar_position: TaskbarPosition,
    pub taskbar_auto_hide: bool,
    
    // Desktop
    pub wallpaper_path: String,
    pub wallpaper_mode: WallpaperMode,
    pub desktop_icons_enabled: bool,
    pub desktop_grid_size: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FontWeight {
    Thin,
    Light,
    Regular,
    Medium,
    Bold,
    Black,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TaskbarPosition {
    Top,
    Bottom,
    Left,
    Right,
}

#[derive(Debug, Clone, PartialEq)]
pub enum WallpaperMode {
    Stretch,
    Fit,
    Fill,
    Center,
    Tile,
    Span,
}

impl Default for DesktopTheme {
    fn default() -> Self {
        DesktopTheme {
            name: "RaeenOS Default".to_string(),
            version: "1.0".to_string(),
            author: "RaeenOS Team".to_string(),
            
            primary_color: Color::new(100, 150, 255, 255),
            secondary_color: Color::new(150, 100, 255, 255),
            accent_color: Color::new(255, 100, 150, 255),
            background_color: Color::new(20, 20, 30, 255),
            surface_color: Color::new(40, 40, 50, 240),
            text_color: Color::new(220, 220, 220, 255),
            text_secondary_color: Color::new(160, 160, 160, 255),
            border_color: Color::new(80, 80, 90, 255),
            shadow_color: Color::new(0, 0, 0, 100),
            error_color: Color::new(255, 100, 100, 255),
            warning_color: Color::new(255, 200, 100, 255),
            success_color: Color::new(100, 255, 100, 255),
            
            glassmorphism_enabled: true,
            blur_radius: 10,
            transparency_level: 180,
            corner_radius: 8,
            shadow_enabled: true,
            shadow_offset: Point::new(2, 2),
            shadow_blur: 8,
            animations_enabled: true,
            animation_duration_ms: 250,
            
            font_family: "RaeenOS Sans".to_string(),
            font_size: 14,
            font_weight: FontWeight::Regular,
            line_height: 1.4,
            
            padding_small: 8,
            padding_medium: 16,
            padding_large: 24,
            margin_small: 4,
            margin_medium: 8,
            margin_large: 16,
            
            window_title_height: 32,
            window_border_width: 1,
            window_button_size: 24,
            
            taskbar_height: 48,
            taskbar_position: TaskbarPosition::Bottom,
            taskbar_auto_hide: false,
            
            wallpaper_path: "/usr/share/wallpapers/default.jpg".to_string(),
            wallpaper_mode: WallpaperMode::Fill,
            desktop_icons_enabled: true,
            desktop_grid_size: 64,
        }
    }
}

/// Virtual desktop
#[derive(Debug, Clone)]
pub struct VirtualDesktop {
    pub id: u32,
    pub name: String,
    pub windows: Vec<WindowId>,
    pub wallpaper: String,
    pub widgets: Vec<u32>,
    pub active: bool,
}

impl VirtualDesktop {
    pub fn new(id: u32, name: String) -> Self {
        VirtualDesktop {
            id,
            name,
            windows: Vec::new(),
            wallpaper: String::new(),
            widgets: Vec::new(),
            active: false,
        }
    }
}

/// Desktop widget
#[derive(Debug, Clone)]
pub struct DesktopWidget {
    pub id: u32,
    pub widget_type: DesktopWidgetType,
    pub position: Point,
    pub size: (u32, u32),
    pub visible: bool,
    pub locked: bool,
    pub config: WidgetConfig,
}

#[derive(Debug, Clone)]
pub enum DesktopWidgetType {
    Clock,
    Weather,
    SystemMonitor,
    Calendar,
    Notes,
    QuickLauncher,
    MediaPlayer,
    NetworkMonitor,
    Battery,
    CustomWidget(String),
}

#[derive(Debug, Clone)]
pub struct WidgetConfig {
    pub properties: BTreeMap<String, String>,
    pub refresh_interval_ms: u32,
    pub auto_update: bool,
}

/// Taskbar item
#[derive(Debug, Clone)]
pub struct TaskbarItem {
    pub window_id: WindowId,
    pub title: String,
    pub icon_path: String,
    pub active: bool,
    pub minimized: bool,
    pub urgent: bool,
}

/// System tray item
#[derive(Debug, Clone)]
pub struct SystemTrayItem {
    pub id: u32,
    pub process_id: ProcessId,
    pub icon_path: String,
    pub tooltip: String,
    pub menu_items: Vec<TrayMenuItem>,
}

#[derive(Debug, Clone)]
pub struct TrayMenuItem {
    pub id: u32,
    pub text: String,
    pub icon: Option<String>,
    pub enabled: bool,
    pub separator: bool,
    pub submenu: Vec<TrayMenuItem>,
}

/// Desktop icon
#[derive(Debug, Clone)]
pub struct DesktopIcon {
    pub id: u32,
    pub name: String,
    pub icon_path: String,
    pub position: Point,
    pub target: IconTarget,
    pub selected: bool,
}

#[derive(Debug, Clone)]
pub enum IconTarget {
    Application(String),
    File(String),
    Directory(String),
    Url(String),
    Command(String),
}

/// Window management rules
#[derive(Debug, Clone)]
pub struct WindowRule {
    pub id: u32,
    pub name: String,
    pub window_class: Option<String>,
    pub window_title: Option<String>,
    pub process_name: Option<String>,
    pub desktop: Option<u32>,
    pub position: Option<Point>,
    pub size: Option<(u32, u32)>,
    pub always_on_top: bool,
    pub skip_taskbar: bool,
    pub fullscreen: bool,
    pub maximized: bool,
    pub minimized: bool,
}

/// Desktop environment state
pub struct DesktopEnvironment {
    pub theme: DesktopTheme,
    pub virtual_desktops: Vec<VirtualDesktop>,
    pub current_desktop: u32,
    pub widgets: BTreeMap<u32, DesktopWidget>,
    pub taskbar_items: Vec<TaskbarItem>,
    pub system_tray_items: Vec<SystemTrayItem>,
    pub desktop_icons: Vec<DesktopIcon>,
    pub window_rules: Vec<WindowRule>,
    pub next_widget_id: u32,
    pub next_desktop_id: u32,
    pub next_icon_id: u32,
    pub next_rule_id: u32,
    pub screen_width: u32,
    pub screen_height: u32,
    pub taskbar_visible: bool,
    pub desktop_locked: bool,
}

impl DesktopEnvironment {
    pub fn new(screen_width: u32, screen_height: u32) -> Self {
        let mut de = DesktopEnvironment {
            theme: DesktopTheme::default(),
            virtual_desktops: Vec::new(),
            current_desktop: 1,
            widgets: BTreeMap::new(),
            taskbar_items: Vec::new(),
            system_tray_items: Vec::new(),
            desktop_icons: Vec::new(),
            window_rules: Vec::new(),
            next_widget_id: 1,
            next_desktop_id: 1,
            next_icon_id: 1,
            next_rule_id: 1,
            screen_width,
            screen_height,
            taskbar_visible: true,
            desktop_locked: false,
        };
        
        // Create default virtual desktop
        de.create_virtual_desktop("Desktop 1".to_string());
        
        // Add default widgets
        de.add_default_widgets();
        
        // Add default desktop icons
        de.add_default_icons();
        
        de
    }
    
    fn add_default_widgets(&mut self) {
        // Clock widget
        self.add_widget(
            DesktopWidgetType::Clock,
            Point::new(self.screen_width as i32 - 200, 20),
            (180, 60),
        );
        
        // System monitor widget
        self.add_widget(
            DesktopWidgetType::SystemMonitor,
            Point::new(20, 20),
            (200, 150),
        );
        
        // Weather widget
        self.add_widget(
            DesktopWidgetType::Weather,
            Point::new(self.screen_width as i32 - 220, 100),
            (200, 120),
        );
    }
    
    fn add_default_icons(&mut self) {
        // File manager icon
        self.add_desktop_icon(
            "File Manager".to_string(),
            "/usr/share/icons/file-manager.svg".to_string(),
            Point::new(50, 50),
            IconTarget::Application("file-manager".to_string()),
        );
        
        // Terminal icon
        self.add_desktop_icon(
            "Terminal".to_string(),
            "/usr/share/icons/terminal.svg".to_string(),
            Point::new(50, 130),
            IconTarget::Application("raeshell".to_string()),
        );
        
        // Settings icon
        self.add_desktop_icon(
            "Settings".to_string(),
            "/usr/share/icons/settings.svg".to_string(),
            Point::new(50, 210),
            IconTarget::Application("rae-settings".to_string()),
        );
    }
    
    pub fn create_virtual_desktop(&mut self, name: String) -> u32 {
        let desktop_id = self.next_desktop_id;
        self.next_desktop_id += 1;
        
        let mut desktop = VirtualDesktop::new(desktop_id, name);
        if self.virtual_desktops.is_empty() {
            desktop.active = true;
            self.current_desktop = desktop_id;
        }
        
        self.virtual_desktops.push(desktop);
        desktop_id
    }
    
    pub fn switch_desktop(&mut self, desktop_id: u32) -> bool {
        if let Some(current) = self.virtual_desktops.iter_mut().find(|d| d.id == self.current_desktop) {
            current.active = false;
        }
        
        if let Some(new_desktop) = self.virtual_desktops.iter_mut().find(|d| d.id == desktop_id) {
            new_desktop.active = true;
            self.current_desktop = desktop_id;
            true
        } else {
            false
        }
    }
    
    pub fn add_widget(&mut self, widget_type: DesktopWidgetType, position: Point, size: (u32, u32)) -> u32 {
        let widget_id = self.next_widget_id;
        self.next_widget_id += 1;
        
        let widget = DesktopWidget {
            id: widget_id,
            widget_type,
            position,
            size,
            visible: true,
            locked: false,
            config: WidgetConfig {
                properties: BTreeMap::new(),
                refresh_interval_ms: 1000,
                auto_update: true,
            },
        };
        
        self.widgets.insert(widget_id, widget);
        widget_id
    }
    
    pub fn remove_widget(&mut self, widget_id: u32) -> bool {
        self.widgets.remove(&widget_id).is_some()
    }
    
    pub fn move_widget(&mut self, widget_id: u32, position: Point) -> bool {
        if let Some(widget) = self.widgets.get_mut(&widget_id) {
            if !widget.locked {
                widget.position = position;
                true
            } else {
                false
            }
        } else {
            false
        }
    }
    
    pub fn resize_widget(&mut self, widget_id: u32, size: (u32, u32)) -> bool {
        if let Some(widget) = self.widgets.get_mut(&widget_id) {
            if !widget.locked {
                widget.size = size;
                true
            } else {
                false
            }
        } else {
            false
        }
    }
    
    pub fn add_desktop_icon(&mut self, name: String, icon_path: String, position: Point, target: IconTarget) -> u32 {
        let icon_id = self.next_icon_id;
        self.next_icon_id += 1;
        
        let icon = DesktopIcon {
            id: icon_id,
            name,
            icon_path,
            position,
            target,
            selected: false,
        };
        
        self.desktop_icons.push(icon);
        icon_id
    }
    
    pub fn remove_desktop_icon(&mut self, icon_id: u32) -> bool {
        if let Some(pos) = self.desktop_icons.iter().position(|i| i.id == icon_id) {
            self.desktop_icons.remove(pos);
            true
        } else {
            false
        }
    }
    
    pub fn add_taskbar_item(&mut self, window_id: WindowId, title: String, icon_path: String) {
        let item = TaskbarItem {
            window_id,
            title,
            icon_path,
            active: false,
            minimized: false,
            urgent: false,
        };
        
        self.taskbar_items.push(item);
    }
    
    pub fn remove_taskbar_item(&mut self, window_id: WindowId) -> bool {
        if let Some(pos) = self.taskbar_items.iter().position(|i| i.window_id == window_id) {
            self.taskbar_items.remove(pos);
            true
        } else {
            false
        }
    }
    
    pub fn update_taskbar_item(&mut self, window_id: WindowId, title: Option<String>, active: Option<bool>, minimized: Option<bool>) -> bool {
        if let Some(item) = self.taskbar_items.iter_mut().find(|i| i.window_id == window_id) {
            if let Some(new_title) = title {
                item.title = new_title;
            }
            if let Some(new_active) = active {
                item.active = new_active;
            }
            if let Some(new_minimized) = minimized {
                item.minimized = new_minimized;
            }
            true
        } else {
            false
        }
    }
    
    pub fn add_system_tray_item(&mut self, process_id: ProcessId, icon_path: String, tooltip: String) -> u32 {
        let item_id = self.system_tray_items.len() as u32 + 1;
        
        let item = SystemTrayItem {
            id: item_id,
            process_id,
            icon_path,
            tooltip,
            menu_items: Vec::new(),
        };
        
        self.system_tray_items.push(item);
        item_id
    }
    
    pub fn remove_system_tray_item(&mut self, item_id: u32) -> bool {
        if let Some(pos) = self.system_tray_items.iter().position(|i| i.id == item_id) {
            self.system_tray_items.remove(pos);
            true
        } else {
            false
        }
    }
    
    pub fn add_window_rule(&mut self, rule: WindowRule) -> u32 {
        let rule_id = self.next_rule_id;
        self.next_rule_id += 1;
        
        let mut new_rule = rule;
        new_rule.id = rule_id;
        
        self.window_rules.push(new_rule);
        rule_id
    }
    
    pub fn remove_window_rule(&mut self, rule_id: u32) -> bool {
        if let Some(pos) = self.window_rules.iter().position(|r| r.id == rule_id) {
            self.window_rules.remove(pos);
            true
        } else {
            false
        }
    }
    
    pub fn apply_window_rules(&self, window_id: WindowId, window_class: Option<&str>, window_title: Option<&str>, process_name: Option<&str>) -> Vec<&WindowRule> {
        let mut matching_rules = Vec::new();
        
        for rule in &self.window_rules {
            let mut matches = true;
            
            if let Some(class) = &rule.window_class {
                if let Some(window_class) = window_class {
                    if !window_class.contains(class) {
                        matches = false;
                    }
                } else {
                    matches = false;
                }
            }
            
            if let Some(title) = &rule.window_title {
                if let Some(window_title) = window_title {
                    if !window_title.contains(title) {
                        matches = false;
                    }
                } else {
                    matches = false;
                }
            }
            
            if let Some(process) = &rule.process_name {
                if let Some(process_name) = process_name {
                    if !process_name.contains(process) {
                        matches = false;
                    }
                } else {
                    matches = false;
                }
            }
            
            if matches {
                matching_rules.push(rule);
            }
        }
        
        matching_rules
    }
    
    pub fn set_theme(&mut self, theme: DesktopTheme) {
        self.theme = theme;
    }
    
    pub fn get_theme(&self) -> &DesktopTheme {
        &self.theme
    }
    
    pub fn toggle_taskbar(&mut self) {
        self.taskbar_visible = !self.taskbar_visible;
    }
    
    pub fn lock_desktop(&mut self, locked: bool) {
        self.desktop_locked = locked;
        
        // Lock all widgets
        for widget in self.widgets.values_mut() {
            widget.locked = locked;
        }
    }
    
    pub fn get_desktop_at_point(&self, point: Point) -> Option<u32> {
        // For now, just return current desktop
        // In a multi-monitor setup, this would determine which desktop/monitor
        Some(self.current_desktop)
    }
    
    pub fn get_widget_at_point(&self, point: Point) -> Option<u32> {
        for (id, widget) in &self.widgets {
            if widget.visible {
                let widget_rect = Rect::new(
                    widget.position.x,
                    widget.position.y,
                    widget.size.0,
                    widget.size.1,
                );
                
                if widget_rect.contains(point) {
                    return Some(*id);
                }
            }
        }
        None
    }
    
    pub fn get_icon_at_point(&self, point: Point) -> Option<u32> {
        for icon in &self.desktop_icons {
            let icon_rect = Rect::new(
                icon.position.x,
                icon.position.y,
                self.theme.desktop_grid_size,
                self.theme.desktop_grid_size,
            );
            
            if icon_rect.contains(point) {
                return Some(icon.id);
            }
        }
        None
    }
    
    pub fn update_widgets(&mut self) {
        let current_time = crate::time::get_timestamp();
        
        for widget in self.widgets.values_mut() {
            if widget.config.auto_update {
                // TODO: Update widget content based on type
                match widget.widget_type {
                    DesktopWidgetType::Clock => {
                        // Update clock display
                    }
                    DesktopWidgetType::SystemMonitor => {
                        // Update system stats
                    }
                    DesktopWidgetType::Weather => {
                        // Update weather data
                    }
                    _ => {}
                }
            }
        }
    }
    
    pub fn render_desktop(&self, buffer: &mut crate::graphics::GraphicsBuffer) {
        // Clear with background color
        buffer.clear(self.theme.background_color);
        
        // Render wallpaper
        self.render_wallpaper(buffer);
        
        // Render desktop icons
        if self.theme.desktop_icons_enabled {
            self.render_desktop_icons(buffer);
        }
        
        // Render widgets
        self.render_widgets(buffer);
        
        // Render taskbar
        if self.taskbar_visible {
            self.render_taskbar(buffer);
        }
    }
    
    fn render_wallpaper(&self, buffer: &mut crate::graphics::GraphicsBuffer) {
        // TODO: Load and render wallpaper image
        // For now, just fill with background gradient
        let gradient_start = self.theme.background_color;
        let gradient_end = Color::new(
            gradient_start.r.saturating_add(20),
            gradient_start.g.saturating_add(20),
            gradient_start.b.saturating_add(30),
            gradient_start.a,
        );
        
        for y in 0..buffer.height {
            let ratio = y as f32 / buffer.height as f32;
            let color = Color::new(
                (gradient_start.r as f32 * (1.0 - ratio) + gradient_end.r as f32 * ratio) as u8,
                (gradient_start.g as f32 * (1.0 - ratio) + gradient_end.g as f32 * ratio) as u8,
                (gradient_start.b as f32 * (1.0 - ratio) + gradient_end.b as f32 * ratio) as u8,
                255,
            );
            
            for x in 0..buffer.width {
                buffer.set_pixel(x, y, color);
            }
        }
    }
    
    fn render_desktop_icons(&self, buffer: &mut crate::graphics::GraphicsBuffer) {
        for icon in &self.desktop_icons {
            let icon_rect = Rect::new(
                icon.position.x,
                icon.position.y,
                self.theme.desktop_grid_size,
                self.theme.desktop_grid_size,
            );
            
            // Render icon background if selected
            if icon.selected {
                let selection_color = Color::new(
                    self.theme.accent_color.r,
                    self.theme.accent_color.g,
                    self.theme.accent_color.b,
                    100,
                );
                buffer.draw_rect(icon_rect, selection_color);
            }
            
            // TODO: Render actual icon image
            // For now, draw a placeholder rectangle
            let icon_color = if icon.selected {
                self.theme.accent_color
            } else {
                self.theme.primary_color
            };
            
            let inner_rect = Rect::new(
                icon.position.x + 8,
                icon.position.y + 8,
                self.theme.desktop_grid_size - 16,
                self.theme.desktop_grid_size - 24,
            );
            
            buffer.draw_rect(inner_rect, icon_color);
            
            // TODO: Render icon text label
        }
    }
    
    fn render_widgets(&self, buffer: &mut crate::graphics::GraphicsBuffer) {
        for widget in self.widgets.values() {
            if widget.visible {
                self.render_widget(buffer, widget);
            }
        }
    }
    
    fn render_widget(&self, buffer: &mut crate::graphics::GraphicsBuffer, widget: &DesktopWidget) {
        let widget_rect = Rect::new(
            widget.position.x,
            widget.position.y,
            widget.size.0,
            widget.size.1,
        );
        
        // Render widget background with glassmorphism effect
        if self.theme.glassmorphism_enabled {
            let glass_color = Color::new(
                self.theme.surface_color.r,
                self.theme.surface_color.g,
                self.theme.surface_color.b,
                self.theme.transparency_level,
            );
            buffer.draw_rect(widget_rect, glass_color);
        } else {
            buffer.draw_rect(widget_rect, self.theme.surface_color);
        }
        
        // Render widget border
        if self.theme.corner_radius > 0 {
            // TODO: Render rounded corners
        }
        
        // Render widget content based on type
        match widget.widget_type {
            DesktopWidgetType::Clock => {
                self.render_clock_widget(buffer, widget_rect);
            }
            DesktopWidgetType::SystemMonitor => {
                self.render_system_monitor_widget(buffer, widget_rect);
            }
            DesktopWidgetType::Weather => {
                self.render_weather_widget(buffer, widget_rect);
            }
            _ => {
                // Render placeholder content
                let text_rect = Rect::new(
                    widget_rect.x + 10,
                    widget_rect.y + 10,
                    widget_rect.width - 20,
                    20,
                );
                buffer.draw_rect(text_rect, self.theme.text_color);
            }
        }
    }
    
    fn render_clock_widget(&self, buffer: &mut crate::graphics::GraphicsBuffer, rect: Rect) {
        // TODO: Render actual time
        // For now, render placeholder
        let time_text_rect = Rect::new(
            rect.x + 10,
            rect.y + 15,
            rect.width - 20,
            30,
        );
        buffer.draw_rect(time_text_rect, self.theme.text_color);
    }
    
    fn render_system_monitor_widget(&self, buffer: &mut crate::graphics::GraphicsBuffer, rect: Rect) {
        // TODO: Render actual system stats
        // For now, render placeholder bars
        let bar_height = 8;
        let bar_spacing = 15;
        
        for i in 0..4 {
            let bar_rect = Rect::new(
                rect.x + 10,
                rect.y + 20 + i * bar_spacing,
                rect.width - 20,
                bar_height,
            );
            
            // Background bar
            buffer.draw_rect(bar_rect, self.theme.border_color);
            
            // Progress bar (placeholder values)
            let progress = match i {
                0 => 0.3, // CPU
                1 => 0.6, // Memory
                2 => 0.2, // Disk
                3 => 0.1, // Network
                _ => 0.0,
            };
            
            let progress_rect = Rect::new(
                bar_rect.x,
                bar_rect.y,
                (bar_rect.width as f32 * progress) as u32,
                bar_rect.height,
            );
            
            let color = match i {
                0 => self.theme.primary_color,
                1 => self.theme.secondary_color,
                2 => self.theme.accent_color,
                3 => self.theme.success_color,
                _ => self.theme.text_color,
            };
            
            buffer.draw_rect(progress_rect, color);
        }
    }
    
    fn render_weather_widget(&self, buffer: &mut crate::graphics::GraphicsBuffer, rect: Rect) {
        // TODO: Render actual weather data
        // For now, render placeholder
        let weather_icon_rect = Rect::new(
            rect.x + 10,
            rect.y + 10,
            40,
            40,
        );
        buffer.draw_rect(weather_icon_rect, self.theme.accent_color);
        
        let temp_rect = Rect::new(
            rect.x + 60,
            rect.y + 20,
            rect.width - 70,
            20,
        );
        buffer.draw_rect(temp_rect, self.theme.text_color);
    }
    
    fn render_taskbar(&self, buffer: &mut crate::graphics::GraphicsBuffer) {
        let taskbar_rect = match self.theme.taskbar_position {
            TaskbarPosition::Bottom => Rect::new(
                0,
                buffer.height as i32 - self.theme.taskbar_height as i32,
                buffer.width,
                self.theme.taskbar_height,
            ),
            TaskbarPosition::Top => Rect::new(
                0,
                0,
                buffer.width,
                self.theme.taskbar_height,
            ),
            TaskbarPosition::Left => Rect::new(
                0,
                0,
                self.theme.taskbar_height,
                buffer.height,
            ),
            TaskbarPosition::Right => Rect::new(
                buffer.width as i32 - self.theme.taskbar_height as i32,
                0,
                self.theme.taskbar_height,
                buffer.height,
            ),
        };
        
        // Render taskbar background
        if self.theme.glassmorphism_enabled {
            let glass_color = Color::new(
                self.theme.surface_color.r,
                self.theme.surface_color.g,
                self.theme.surface_color.b,
                self.theme.transparency_level,
            );
            buffer.draw_rect(taskbar_rect, glass_color);
        } else {
            buffer.draw_rect(taskbar_rect, self.theme.surface_color);
        }
        
        // Render taskbar items
        let item_size = self.theme.taskbar_height - 8;
        let item_spacing = 4;
        
        for (i, item) in self.taskbar_items.iter().enumerate() {
            let item_x = match self.theme.taskbar_position {
                TaskbarPosition::Bottom | TaskbarPosition::Top => {
                    taskbar_rect.x + 4 + i as i32 * (item_size as i32 + item_spacing)
                }
                _ => taskbar_rect.x + 4,
            };
            
            let item_y = match self.theme.taskbar_position {
                TaskbarPosition::Bottom | TaskbarPosition::Top => taskbar_rect.y + 4,
                _ => taskbar_rect.y + 4 + i as i32 * (item_size as i32 + item_spacing),
            };
            
            let item_rect = Rect::new(item_x, item_y, item_size, item_size);
            
            let item_color = if item.active {
                self.theme.accent_color
            } else if item.minimized {
                self.theme.text_secondary_color
            } else {
                self.theme.primary_color
            };
            
            buffer.draw_rect(item_rect, item_color);
            
            // TODO: Render actual application icon
        }
        
        // Render system tray
        self.render_system_tray(buffer, taskbar_rect);
    }
    
    fn render_system_tray(&self, buffer: &mut crate::graphics::GraphicsBuffer, taskbar_rect: Rect) {
        let tray_item_size = 24;
        let tray_spacing = 2;
        
        for (i, item) in self.system_tray_items.iter().enumerate() {
            let item_x = match self.theme.taskbar_position {
                TaskbarPosition::Bottom | TaskbarPosition::Top => {
                    taskbar_rect.x + taskbar_rect.width as i32 - (tray_item_size + tray_spacing) * (i as i32 + 1)
                }
                _ => taskbar_rect.x + (taskbar_rect.width - tray_item_size) as i32 / 2,
            };
            
            let item_y = match self.theme.taskbar_position {
                TaskbarPosition::Bottom | TaskbarPosition::Top => {
                    taskbar_rect.y + (taskbar_rect.height - tray_item_size) as i32 / 2
                }
                _ => {
                    taskbar_rect.y + taskbar_rect.height as i32 - (tray_item_size + tray_spacing) * (i as i32 + 1)
                }
            };
            
            let item_rect = Rect::new(item_x, item_y, tray_item_size, tray_item_size);
            
            // TODO: Render actual tray icon
            buffer.draw_rect(item_rect, self.theme.text_color);
        }
    }
}

lazy_static! {
    static ref DESKTOP_ENVIRONMENT: Mutex<DesktopEnvironment> = Mutex::new(DesktopEnvironment::new(1920, 1080));
}

// Public API functions

pub fn init_desktop_environment(screen_width: u32, screen_height: u32) {
    let mut de = DESKTOP_ENVIRONMENT.lock();
    *de = DesktopEnvironment::new(screen_width, screen_height);
}

pub fn set_desktop_theme(theme: DesktopTheme) {
    let mut de = DESKTOP_ENVIRONMENT.lock();
    de.set_theme(theme);
}

pub fn get_desktop_theme() -> DesktopTheme {
    let de = DESKTOP_ENVIRONMENT.lock();
    de.get_theme().clone()
}

pub fn create_virtual_desktop(name: &str) -> u32 {
    let mut de = DESKTOP_ENVIRONMENT.lock();
    de.create_virtual_desktop(name.to_string())
}

pub fn switch_virtual_desktop(desktop_id: u32) -> bool {
    let mut de = DESKTOP_ENVIRONMENT.lock();
    de.switch_desktop(desktop_id)
}

pub fn add_desktop_widget(widget_type: DesktopWidgetType, x: i32, y: i32, width: u32, height: u32) -> u32 {
    let mut de = DESKTOP_ENVIRONMENT.lock();
    de.add_widget(widget_type, Point::new(x, y), (width, height))
}

pub fn remove_desktop_widget(widget_id: u32) -> bool {
    let mut de = DESKTOP_ENVIRONMENT.lock();
    de.remove_widget(widget_id)
}

pub fn move_desktop_widget(widget_id: u32, x: i32, y: i32) -> bool {
    let mut de = DESKTOP_ENVIRONMENT.lock();
    de.move_widget(widget_id, Point::new(x, y))
}

pub fn add_desktop_icon(name: &str, icon_path: &str, x: i32, y: i32, target: IconTarget) -> u32 {
    let mut de = DESKTOP_ENVIRONMENT.lock();
    de.add_desktop_icon(name.to_string(), icon_path.to_string(), Point::new(x, y), target)
}

pub fn remove_desktop_icon(icon_id: u32) -> bool {
    let mut de = DESKTOP_ENVIRONMENT.lock();
    de.remove_desktop_icon(icon_id)
}

pub fn add_taskbar_item(window_id: WindowId, title: &str, icon_path: &str) {
    let mut de = DESKTOP_ENVIRONMENT.lock();
    de.add_taskbar_item(window_id, title.to_string(), icon_path.to_string());
}

pub fn remove_taskbar_item(window_id: WindowId) -> bool {
    let mut de = DESKTOP_ENVIRONMENT.lock();
    de.remove_taskbar_item(window_id)
}

pub fn update_taskbar_item(window_id: WindowId, title: Option<&str>, active: Option<bool>, minimized: Option<bool>) -> bool {
    let mut de = DESKTOP_ENVIRONMENT.lock();
    de.update_taskbar_item(window_id, title.map(|s| s.to_string()), active, minimized)
}

pub fn add_system_tray_item(process_id: ProcessId, icon_path: &str, tooltip: &str) -> u32 {
    let mut de = DESKTOP_ENVIRONMENT.lock();
    de.add_system_tray_item(process_id, icon_path.to_string(), tooltip.to_string())
}

pub fn remove_system_tray_item(item_id: u32) -> bool {
    let mut de = DESKTOP_ENVIRONMENT.lock();
    de.remove_system_tray_item(item_id)
}

pub fn add_window_rule(rule: WindowRule) -> u32 {
    let mut de = DESKTOP_ENVIRONMENT.lock();
    de.add_window_rule(rule)
}

pub fn remove_window_rule(rule_id: u32) -> bool {
    let mut de = DESKTOP_ENVIRONMENT.lock();
    de.remove_window_rule(rule_id)
}

pub fn toggle_taskbar() {
    let mut de = DESKTOP_ENVIRONMENT.lock();
    de.toggle_taskbar();
}

pub fn lock_desktop(locked: bool) {
    let mut de = DESKTOP_ENVIRONMENT.lock();
    de.lock_desktop(locked);
}

pub fn render_desktop_frame() {
    let de = DESKTOP_ENVIRONMENT.lock();
    let mut buffer = crate::graphics::get_screen_buffer().lock();
    de.render_desktop(&mut buffer);
}

pub fn handle_desktop_click(x: i32, y: i32, button: u8) {
    let mut de = DESKTOP_ENVIRONMENT.lock();
    let point = Point::new(x, y);
    
    // Check for widget clicks
    if let Some(widget_id) = de.get_widget_at_point(point) {
        // TODO: Handle widget interaction
        return;
    }
    
    // Check for icon clicks
    if let Some(icon_id) = de.get_icon_at_point(point) {
        // TODO: Handle icon activation
        return;
    }
    
    // Clear icon selections if clicking on empty desktop
    for icon in &mut de.desktop_icons {
        icon.selected = false;
    }
}

pub fn handle_desktop_double_click(x: i32, y: i32) {
    let de = DESKTOP_ENVIRONMENT.lock();
    let point = Point::new(x, y);
    
    if let Some(icon_id) = de.get_icon_at_point(point) {
        if let Some(icon) = de.desktop_icons.iter().find(|i| i.id == icon_id) {
            // TODO: Launch application or open file based on icon target
            match &icon.target {
                IconTarget::Application(app_name) => {
                    // Launch application
                }
                IconTarget::File(file_path) => {
                    // Open file
                }
                IconTarget::Directory(dir_path) => {
                    // Open directory in file manager
                }
                IconTarget::Url(url) => {
                    // Open URL in browser
                }
                IconTarget::Command(command) => {
                    // Execute command
                }
            }
        }
    }
}

pub fn update_desktop_widgets() {
    let mut de = DESKTOP_ENVIRONMENT.lock();
    de.update_widgets();
}

pub fn get_current_virtual_desktop() -> u32 {
    let de = DESKTOP_ENVIRONMENT.lock();
    de.current_desktop
}

pub fn get_virtual_desktop_list() -> Vec<(u32, String)> {
    let de = DESKTOP_ENVIRONMENT.lock();
    de.virtual_desktops.iter().map(|d| (d.id, d.name.clone())).collect()
}

pub fn get_desktop_widget_list() -> Vec<u32> {
    let de = DESKTOP_ENVIRONMENT.lock();
    de.widgets.keys().cloned().collect()
}

pub fn get_desktop_icon_list() -> Vec<u32> {
    let de = DESKTOP_ENVIRONMENT.lock();
    de.desktop_icons.iter().map(|i| i.id).collect()
}