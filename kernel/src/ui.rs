//! User interface subsystem for RaeenOS
//! Provides theming, styling, and UI component management

use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;

// Color definitions
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }
    
    pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
        Self::new(r, g, b, 255)
    }
    
    pub fn to_u32(&self) -> u32 {
        ((self.a as u32) << 24) | ((self.r as u32) << 16) | ((self.g as u32) << 8) | (self.b as u32)
    }
}

// Theme definitions
#[derive(Debug, Clone)]
pub struct Theme {
    pub name: &'static str,
    pub background: Color,
    pub foreground: Color,
    pub accent: Color,
    pub secondary: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub border: Color,
    pub shadow: Color,
}

// Predefined themes
const DARK_THEME: Theme = Theme {
    name: "Dark",
    background: Color::rgb(32, 32, 32),
    foreground: Color::rgb(255, 255, 255),
    accent: Color::rgb(0, 120, 215),
    secondary: Color::rgb(64, 64, 64),
    success: Color::rgb(16, 124, 16),
    warning: Color::rgb(255, 185, 0),
    error: Color::rgb(196, 43, 28),
    border: Color::rgb(96, 96, 96),
    shadow: Color::rgb(0, 0, 0),
};

const LIGHT_THEME: Theme = Theme {
    name: "Light",
    background: Color::rgb(255, 255, 255),
    foreground: Color::rgb(0, 0, 0),
    accent: Color::rgb(0, 120, 215),
    secondary: Color::rgb(240, 240, 240),
    success: Color::rgb(16, 124, 16),
    warning: Color::rgb(255, 185, 0),
    error: Color::rgb(196, 43, 28),
    border: Color::rgb(128, 128, 128),
    shadow: Color::rgb(128, 128, 128),
};

const HIGH_CONTRAST_THEME: Theme = Theme {
    name: "High Contrast",
    background: Color::rgb(0, 0, 0),
    foreground: Color::rgb(255, 255, 255),
    accent: Color::rgb(255, 255, 0),
    secondary: Color::rgb(128, 128, 128),
    success: Color::rgb(0, 255, 0),
    warning: Color::rgb(255, 255, 0),
    error: Color::rgb(255, 0, 0),
    border: Color::rgb(255, 255, 255),
    shadow: Color::rgb(255, 255, 255),
};

// Theme system state
struct ThemeSystem {
    current_theme: Theme,
    custom_themes: BTreeMap<u32, Theme>,
    options: u32,
}

lazy_static! {
    static ref THEME_SYSTEM: Mutex<ThemeSystem> = Mutex::new(ThemeSystem {
        current_theme: DARK_THEME,
        custom_themes: BTreeMap::new(),
        options: 0,
    });
}

// Theme options flags
pub const THEME_OPTION_ANIMATIONS: u32 = 1 << 0;
pub const THEME_OPTION_TRANSPARENCY: u32 = 1 << 1;
pub const THEME_OPTION_SHADOWS: u32 = 1 << 2;
pub const THEME_OPTION_ROUNDED_CORNERS: u32 = 1 << 3;

// Set the system theme
pub fn set_system_theme(theme_id: u32, options: u32) -> Result<(), ()> {
    let mut theme_system = THEME_SYSTEM.lock();
    
    let new_theme = match theme_id {
        0 => DARK_THEME,
        1 => LIGHT_THEME,
        2 => HIGH_CONTRAST_THEME,
        id if id >= 1000 => {
            // Custom theme IDs start at 1000
            theme_system.custom_themes.get(&id).cloned().ok_or(())?
        }
        _ => return Err(()), // Invalid theme ID
    };
    
    theme_system.current_theme = new_theme;
    theme_system.options = options;
    
    // Notify graphics system of theme change
    crate::graphics::invalidate_all_windows();
    
    Ok(())
}

// Get the current theme
pub fn get_current_theme() -> Theme {
    THEME_SYSTEM.lock().current_theme.clone()
}

// Get theme options
pub fn get_theme_options() -> u32 {
    THEME_SYSTEM.lock().options
}

// Register a custom theme
pub fn register_custom_theme(theme_id: u32, theme: Theme) -> Result<(), ()> {
    if theme_id < 1000 {
        return Err(()); // Reserved for system themes
    }
    
    let mut theme_system = THEME_SYSTEM.lock();
    theme_system.custom_themes.insert(theme_id, theme);
    Ok(())
}

// Get a color from the current theme
pub fn get_theme_color(color_type: &str) -> Color {
    let theme = THEME_SYSTEM.lock().current_theme.clone();
    
    match color_type {
        "background" => theme.background,
        "foreground" => theme.foreground,
        "accent" => theme.accent,
        "secondary" => theme.secondary,
        "success" => theme.success,
        "warning" => theme.warning,
        "error" => theme.error,
        "border" => theme.border,
        "shadow" => theme.shadow,
        _ => theme.foreground, // Default fallback
    }
}

// Check if a theme option is enabled
pub fn is_theme_option_enabled(option: u32) -> bool {
    let options = THEME_SYSTEM.lock().options;
    (options & option) != 0
}