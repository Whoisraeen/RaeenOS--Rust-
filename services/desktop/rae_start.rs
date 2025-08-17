//! RaeStart - Start Menu + Launchpad Hybrid
//! Combines Windows Start Menu functionality with macOS Launchpad design
//! Features intelligent search, app categories, recent items, and quick actions

use crate::services::desktop::DesktopError;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::collections::BTreeMap;
use core::time::Duration;

/// Start menu layout modes
#[derive(Debug, Clone, PartialEq)]
pub enum LayoutMode {
    Grid,        // Launchpad-style grid
    List,        // Traditional start menu list
    Tiles,       // Windows-style live tiles
    Compact,     // Minimal compact view
    Custom,      // User-defined layout
}

/// App category for organization
#[derive(Debug, Clone, PartialEq)]
pub enum AppCategory {
    Productivity,
    Development,
    Graphics,
    Entertainment,
    Utilities,
    Games,
    Internet,
    System,
    Office,
    Education,
    Custom(String),
}

/// Application entry in start menu
#[derive(Debug, Clone)]
pub struct AppEntry {
    pub app_id: String,
    pub name: String,
    pub description: String,
    pub icon_path: String,
    pub executable_path: String,
    pub category: AppCategory,
    pub keywords: Vec<String>,
    pub launch_count: u32,
    pub last_launched: u64,
    pub is_favorite: bool,
    pub is_recently_installed: bool,
    pub size: AppSize,
    pub live_tile: Option<LiveTileData>,
}

/// App tile size
#[derive(Debug, Clone, PartialEq)]
pub enum AppSize {
    Small,
    Medium,
    Wide,
    Large,
}

/// Live tile data for apps
#[derive(Debug, Clone)]
pub struct LiveTileData {
    pub content: TileContent,
    pub update_frequency: Duration,
    pub last_updated: u64,
    pub enabled: bool,
}

/// Live tile content types
#[derive(Debug, Clone)]
pub enum TileContent {
    Static {
        title: String,
        subtitle: Option<String>,
        background_color: String,
    },
    Dynamic {
        data: Vec<TileDataPoint>,
        template: TileTemplate,
    },
    Image {
        image_data: Vec<u8>,
        overlay_text: Option<String>,
    },
    Badge {
        count: u32,
        text: Option<String>,
    },
}

/// Tile data point for dynamic content
#[derive(Debug, Clone)]
pub struct TileDataPoint {
    pub key: String,
    pub value: String,
    pub timestamp: u64,
}

/// Tile template for dynamic content
#[derive(Debug, Clone)]
pub enum TileTemplate {
    Text,
    ImageAndText,
    PeekImageAndText,
    PeekImageCollection,
}

/// Search result item
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub result_type: SearchResultType,
    pub title: String,
    pub subtitle: Option<String>,
    pub icon: Option<String>,
    pub action: SearchAction,
    pub relevance_score: f32,
    pub category: String,
}

/// Search result types
#[derive(Debug, Clone, PartialEq)]
pub enum SearchResultType {
    Application,
    File,
    Folder,
    Setting,
    Command,
    WebSearch,
    Calculator,
    Unit,
    Contact,
    Email,
}

/// Search actions
#[derive(Debug, Clone)]
pub enum SearchAction {
    Launch(String),
    OpenFile(String),
    OpenFolder(String),
    OpenSetting(String),
    ExecuteCommand(String),
    WebSearch(String),
    Calculate(String),
    Convert(String, String),
    Custom(String),
}

/// Quick action item
#[derive(Debug, Clone)]
pub struct QuickAction {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub action: QuickActionType,
    pub shortcut: Option<String>,
    pub enabled: bool,
}

/// Quick action types
#[derive(Debug, Clone)]
pub enum QuickActionType {
    PowerOff,
    Restart,
    Sleep,
    Lock,
    SignOut,
    Settings,
    FileManager,
    TaskManager,
    ControlPanel,
    NetworkSettings,
    DisplaySettings,
    Custom(String),
}

/// Recent item for quick access
#[derive(Debug, Clone)]
pub struct RecentItem {
    pub item_type: RecentItemType,
    pub name: String,
    pub path: String,
    pub icon: Option<String>,
    pub last_accessed: u64,
    pub access_count: u32,
}

/// Recent item types
#[derive(Debug, Clone, PartialEq)]
pub enum RecentItemType {
    Document,
    Image,
    Video,
    Audio,
    Application,
    Folder,
    Project,
}

/// Start menu configuration
#[derive(Debug, Clone)]
pub struct StartConfig {
    pub layout_mode: LayoutMode,
    pub show_recent_apps: bool,
    pub show_recent_files: bool,
    pub show_quick_actions: bool,
    pub show_live_tiles: bool,
    pub max_recent_items: u32,
    pub search_providers: Vec<SearchProvider>,
    pub theme: StartTheme,
    pub shortcuts: BTreeMap<String, String>,
    pub auto_categorize: bool,
}

/// Search provider configuration
#[derive(Debug, Clone)]
pub struct SearchProvider {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub priority: u32,
    pub search_types: Vec<SearchResultType>,
}

/// Start menu theme
#[derive(Debug, Clone)]
pub struct StartTheme {
    pub background_color: String,
    pub accent_color: String,
    pub text_color: String,
    pub transparency: f32,
    pub blur_enabled: bool,
    pub animation_speed: f32,
    pub tile_spacing: u32,
    pub border_radius: f32,
}

/// Start menu state
#[derive(Debug, Clone)]
pub struct StartState {
    pub visible: bool,
    pub search_query: String,
    pub selected_category: Option<AppCategory>,
    pub current_page: u32,
    pub search_results: Vec<SearchResult>,
    pub animation_state: AnimationState,
}

/// Animation state
#[derive(Debug, Clone, PartialEq)]
pub enum AnimationState {
    Hidden,
    Opening,
    Visible,
    Closing,
    Searching,
}

/// RaeStart main service
pub struct RaeStart {
    config: StartConfig,
    apps: Vec<AppEntry>,
    categories: BTreeMap<AppCategory, Vec<String>>,
    recent_items: Vec<RecentItem>,
    quick_actions: Vec<QuickAction>,
    favorites: Vec<String>,
    state: StartState,
    search_index: BTreeMap<String, Vec<String>>,
}

impl RaeStart {
    /// Create a new RaeStart instance
    pub fn new() -> Result<Self, DesktopError> {
        let mut start = RaeStart {
            config: StartConfig {
                layout_mode: LayoutMode::Grid,
                show_recent_apps: true,
                show_recent_files: true,
                show_quick_actions: true,
                show_live_tiles: true,
                max_recent_items: 20,
                search_providers: Vec::new(),
                theme: StartTheme {
                    background_color: "rgba(0, 0, 0, 0.9)".to_string(),
                    accent_color: "#0078d4".to_string(),
                    text_color: "#ffffff".to_string(),
                    transparency: 0.9,
                    blur_enabled: true,
                    animation_speed: 1.0,
                    tile_spacing: 8,
                    border_radius: 8.0,
                },
                shortcuts: BTreeMap::new(),
                auto_categorize: true,
            },
            apps: Vec::new(),
            categories: BTreeMap::new(),
            recent_items: Vec::new(),
            quick_actions: Vec::new(),
            favorites: Vec::new(),
            state: StartState {
                visible: false,
                search_query: String::new(),
                selected_category: None,
                current_page: 0,
                search_results: Vec::new(),
                animation_state: AnimationState::Hidden,
            },
            search_index: BTreeMap::new(),
        };

        start.setup_default_apps()?;
        start.setup_quick_actions()?;
        start.setup_search_providers()?;
        start.setup_shortcuts()?;
        Ok(start)
    }

    /// Start the RaeStart service
    pub fn start(&mut self) -> Result<(), DesktopError> {
        self.load_configuration()?;
        self.scan_installed_apps()?;
        self.build_search_index()?;
        self.load_recent_items()?;
        Ok(())
    }

    /// Stop the RaeStart service
    pub fn stop(&mut self) -> Result<(), DesktopError> {
        self.save_configuration()?;
        self.save_recent_items()?;
        Ok(())
    }

    /// Setup default applications
    fn setup_default_apps(&mut self) -> Result<(), DesktopError> {
        let default_apps = vec![
            AppEntry {
                app_id: "rae_finder".to_string(),
                name: "RaeFinder".to_string(),
                description: "File manager and explorer".to_string(),
                icon_path: "/system/icons/rae_finder.svg".to_string(),
                executable_path: "/system/apps/rae_finder".to_string(),
                category: AppCategory::Utilities,
                keywords: vec!["file".to_string(), "explorer".to_string(), "manager".to_string()],
                launch_count: 0,
                last_launched: 0,
                is_favorite: true,
                is_recently_installed: false,
                size: AppSize::Medium,
                live_tile: None,
            },
            AppEntry {
                app_id: "rae_terminal".to_string(),
                name: "RaeTerminal".to_string(),
                description: "Command line interface".to_string(),
                icon_path: "/system/icons/terminal.svg".to_string(),
                executable_path: "/system/apps/terminal".to_string(),
                category: AppCategory::Development,
                keywords: vec!["terminal".to_string(), "command".to_string(), "shell".to_string()],
                launch_count: 0,
                last_launched: 0,
                is_favorite: true,
                is_recently_installed: false,
                size: AppSize::Medium,
                live_tile: None,
            },
            AppEntry {
                app_id: "rae_browser".to_string(),
                name: "RaeBrowser".to_string(),
                description: "Web browser".to_string(),
                icon_path: "/system/icons/browser.svg".to_string(),
                executable_path: "/system/apps/browser".to_string(),
                category: AppCategory::Internet,
                keywords: vec!["web".to_string(), "browser".to_string(), "internet".to_string()],
                launch_count: 0,
                last_launched: 0,
                is_favorite: true,
                is_recently_installed: false,
                size: AppSize::Wide,
                live_tile: Some(LiveTileData {
                    content: TileContent::Dynamic {
                        data: vec![
                            TileDataPoint {
                                key: "bookmarks".to_string(),
                                value: "15".to_string(),
                                timestamp: 1640995200,
                            },
                        ],
                        template: TileTemplate::Text,
                    },
                    update_frequency: Duration::from_secs(300),
                    last_updated: 1640995200,
                    enabled: true,
                }),
            },
            AppEntry {
                app_id: "rae_settings".to_string(),
                name: "Settings".to_string(),
                description: "System settings and preferences".to_string(),
                icon_path: "/system/icons/settings.svg".to_string(),
                executable_path: "/system/apps/settings".to_string(),
                category: AppCategory::System,
                keywords: vec!["settings".to_string(), "preferences".to_string(), "config".to_string()],
                launch_count: 0,
                last_launched: 0,
                is_favorite: false,
                is_recently_installed: false,
                size: AppSize::Small,
                live_tile: None,
            },
        ];

        self.apps = default_apps;
        self.categorize_apps()?;
        Ok(())
    }

    /// Setup quick actions
    fn setup_quick_actions(&mut self) -> Result<(), DesktopError> {
        self.quick_actions = vec![
            QuickAction {
                id: "power_off".to_string(),
                name: "Power Off".to_string(),
                icon: "/system/icons/power.svg".to_string(),
                action: QuickActionType::PowerOff,
                shortcut: Some("Alt+F4".to_string()),
                enabled: true,
            },
            QuickAction {
                id: "restart".to_string(),
                name: "Restart".to_string(),
                icon: "/system/icons/restart.svg".to_string(),
                action: QuickActionType::Restart,
                shortcut: None,
                enabled: true,
            },
            QuickAction {
                id: "sleep".to_string(),
                name: "Sleep".to_string(),
                icon: "/system/icons/sleep.svg".to_string(),
                action: QuickActionType::Sleep,
                shortcut: None,
                enabled: true,
            },
            QuickAction {
                id: "lock".to_string(),
                name: "Lock".to_string(),
                icon: "/system/icons/lock.svg".to_string(),
                action: QuickActionType::Lock,
                shortcut: Some("Win+L".to_string()),
                enabled: true,
            },
            QuickAction {
                id: "settings".to_string(),
                name: "Settings".to_string(),
                icon: "/system/icons/settings.svg".to_string(),
                action: QuickActionType::Settings,
                shortcut: Some("Win+I".to_string()),
                enabled: true,
            },
            QuickAction {
                id: "file_manager".to_string(),
                name: "File Manager".to_string(),
                icon: "/system/icons/folder.svg".to_string(),
                action: QuickActionType::FileManager,
                shortcut: Some("Win+E".to_string()),
                enabled: true,
            },
        ];
        Ok(())
    }

    /// Setup search providers
    fn setup_search_providers(&mut self) -> Result<(), DesktopError> {
        self.config.search_providers = vec![
            SearchProvider {
                id: "apps".to_string(),
                name: "Applications".to_string(),
                enabled: true,
                priority: 100,
                search_types: vec![SearchResultType::Application],
            },
            SearchProvider {
                id: "files".to_string(),
                name: "Files and Folders".to_string(),
                enabled: true,
                priority: 90,
                search_types: vec![SearchResultType::File, SearchResultType::Folder],
            },
            SearchProvider {
                id: "settings".to_string(),
                name: "System Settings".to_string(),
                enabled: true,
                priority: 80,
                search_types: vec![SearchResultType::Setting],
            },
            SearchProvider {
                id: "calculator".to_string(),
                name: "Calculator".to_string(),
                enabled: true,
                priority: 70,
                search_types: vec![SearchResultType::Calculator],
            },
            SearchProvider {
                id: "web".to_string(),
                name: "Web Search".to_string(),
                enabled: true,
                priority: 60,
                search_types: vec![SearchResultType::WebSearch],
            },
        ];
        Ok(())
    }

    /// Setup keyboard shortcuts
    fn setup_shortcuts(&mut self) -> Result<(), DesktopError> {
        self.config.shortcuts.insert("Win".to_string(), "toggle_start".to_string());
        self.config.shortcuts.insert("Ctrl+Esc".to_string(), "toggle_start".to_string());
        self.config.shortcuts.insert("Win+S".to_string(), "search_focus".to_string());
        self.config.shortcuts.insert("Win+X".to_string(), "quick_actions".to_string());
        Ok(())
    }

    /// Show start menu
    pub fn show(&mut self) -> Result<(), DesktopError> {
        if !self.state.visible {
            self.state.visible = true;
            self.state.animation_state = AnimationState::Opening;
            self.refresh_recent_items()?;
        }
        Ok(())
    }

    /// Hide start menu
    pub fn hide(&mut self) -> Result<(), DesktopError> {
        if self.state.visible {
            self.state.visible = false;
            self.state.animation_state = AnimationState::Closing;
            self.clear_search()?;
        }
        Ok(())
    }

    /// Toggle start menu visibility
    pub fn toggle(&mut self) -> Result<(), DesktopError> {
        if self.state.visible {
            self.hide()
        } else {
            self.show()
        }
    }

    /// Perform search
    pub fn search(&mut self, query: &str) -> Result<Vec<SearchResult>, DesktopError> {
        self.state.search_query = query.to_string();
        self.state.animation_state = AnimationState::Searching;
        
        let mut results = Vec::new();
        
        if query.is_empty() {
            self.state.search_results.clear();
            return Ok(results);
        }
        
        // Search applications
        results.extend(self.search_applications(query)?);
        
        // Search files (simulated)
        results.extend(self.search_files(query)?);
        
        // Search settings
        results.extend(self.search_settings(query)?);
        
        // Calculator
        if let Some(calc_result) = self.try_calculate(query)? {
            results.push(calc_result);
        }
        
        // Web search fallback
        if results.is_empty() {
            results.push(SearchResult {
                result_type: SearchResultType::WebSearch,
                title: format!("Search web for '{}'", query),
                subtitle: Some("Press Enter to search".to_string()),
                icon: Some("/system/icons/web.svg".to_string()),
                action: SearchAction::WebSearch(query.to_string()),
                relevance_score: 0.1,
                category: "Web".to_string(),
            });
        }
        
        // Sort by relevance
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        
        self.state.search_results = results.clone();
        Ok(results)
    }

    /// Search applications
    fn search_applications(&self, query: &str) -> Result<Vec<SearchResult>, DesktopError> {
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();
        
        for app in &self.apps {
            let mut score = 0.0;
            
            // Exact name match
            if app.name.to_lowercase() == query_lower {
                score = 1.0;
            }
            // Name starts with query
            else if app.name.to_lowercase().starts_with(&query_lower) {
                score = 0.9;
            }
            // Name contains query
            else if app.name.to_lowercase().contains(&query_lower) {
                score = 0.7;
            }
            // Keywords match
            else if app.keywords.iter().any(|k| k.to_lowercase().contains(&query_lower)) {
                score = 0.5;
            }
            // Description contains query
            else if app.description.to_lowercase().contains(&query_lower) {
                score = 0.3;
            }
            
            // Boost score for favorites and frequently used apps
            if app.is_favorite {
                score += 0.1;
            }
            if app.launch_count > 10 {
                score += 0.05;
            }
            
            if score > 0.0 {
                results.push(SearchResult {
                    result_type: SearchResultType::Application,
                    title: app.name.clone(),
                    subtitle: Some(app.description.clone()),
                    icon: Some(app.icon_path.clone()),
                    action: SearchAction::Launch(app.app_id.clone()),
                    relevance_score: score,
                    category: format!("{:?}", app.category),
                });
            }
        }
        
        Ok(results)
    }

    /// Search files (simulated)
    fn search_files(&self, query: &str) -> Result<Vec<SearchResult>, DesktopError> {
        let mut results = Vec::new();
        
        // Simulate file search results
        if query.len() > 2 {
            results.push(SearchResult {
                result_type: SearchResultType::File,
                title: format!("{}.txt", query),
                subtitle: Some("/home/user/documents/".to_string()),
                icon: Some("/system/icons/file.svg".to_string()),
                action: SearchAction::OpenFile(format!("/home/user/documents/{}.txt", query)),
                relevance_score: 0.6,
                category: "Files".to_string(),
            });
        }
        
        Ok(results)
    }

    /// Search settings
    fn search_settings(&self, query: &str) -> Result<Vec<SearchResult>, DesktopError> {
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();
        
        let settings = vec![
            ("Display", "display", "Change screen resolution and display settings"),
            ("Network", "network", "Configure network and internet settings"),
            ("Sound", "audio", "Adjust volume and sound settings"),
            ("Privacy", "privacy", "Manage privacy and security settings"),
            ("Updates", "update", "Check for and install system updates"),
            ("Accounts", "user", "Manage user accounts and sign-in options"),
        ];
        
        for (name, keyword, description) in settings {
            if name.to_lowercase().contains(&query_lower) || 
               keyword.to_lowercase().contains(&query_lower) {
                results.push(SearchResult {
                    result_type: SearchResultType::Setting,
                    title: format!("{} Settings", name),
                    subtitle: Some(description.to_string()),
                    icon: Some("/system/icons/settings.svg".to_string()),
                    action: SearchAction::OpenSetting(keyword.to_string()),
                    relevance_score: 0.8,
                    category: "Settings".to_string(),
                });
            }
        }
        
        Ok(results)
    }

    /// Try to calculate mathematical expression
    fn try_calculate(&self, query: &str) -> Result<Option<SearchResult>, DesktopError> {
        if query.chars().any(|c| "+-*/()=".contains(c)) && query.chars().any(|c| c.is_ascii_digit()) {
            // Clean the query by removing '=' if present
            let expression = query.trim().trim_end_matches('=').trim();
            
            match self.evaluate_expression(expression) {
                Ok(result) => {
                    let result_str = if result.fract() == 0.0 {
                        format!("{}", result as i64)
                    } else {
                        format!("{:.6}", result).trim_end_matches('0').trim_end_matches('.').to_string()
                    };
                    
                    return Ok(Some(SearchResult {
                        result_type: SearchResultType::Calculator,
                        title: format!("{} = {}", expression, result_str),
                        subtitle: Some("Calculator".to_string()),
                        icon: Some("/system/icons/calculator.svg".to_string()),
                        action: SearchAction::Calculate(result_str),
                        relevance_score: 0.95,
                        category: "Calculator".to_string(),
                    }));
                },
                Err(_) => {
                    // Return error result for invalid expressions
                    return Ok(Some(SearchResult {
                        result_type: SearchResultType::Calculator,
                        title: format!("{} = Error", expression),
                        subtitle: Some("Invalid expression".to_string()),
                        icon: Some("/system/icons/calculator.svg".to_string()),
                        action: SearchAction::Calculate("Error".to_string()),
                        relevance_score: 0.8,
                        category: "Calculator".to_string(),
                    }));
                }
            }
        }
        
        Ok(None)
    }

    /// Evaluate mathematical expression using recursive descent parser
    fn evaluate_expression(&self, expr: &str) -> Result<f64, &'static str> {
        let mut parser = ExpressionParser::new(expr);
        parser.parse_expression()
    }

    /// Launch application
    pub fn launch_app(&mut self, app_id: &str) -> Result<(), DesktopError> {
        if let Some(app) = self.apps.iter_mut().find(|a| a.app_id == app_id) {
            app.launch_count += 1;
            app.last_launched = self.get_current_time();
            
            // Add to recent items
            self.add_recent_item(RecentItem {
                item_type: RecentItemType::Application,
                name: app.name.clone(),
                path: app.executable_path.clone(),
                icon: Some(app.icon_path.clone()),
                last_accessed: app.last_launched,
                access_count: app.launch_count,
            });
        }
        
        self.hide()?;
        Ok(())
    }

    /// Execute quick action
    pub fn execute_quick_action(&mut self, action_id: &str) -> Result<(), DesktopError> {
        if let Some(action) = self.quick_actions.iter().find(|a| a.id == action_id) {
            match &action.action {
                QuickActionType::PowerOff => {
                    // Simulate power off
                },
                QuickActionType::Restart => {
                    // Simulate restart
                },
                QuickActionType::Sleep => {
                    // Simulate sleep
                },
                QuickActionType::Lock => {
                    // Simulate lock screen
                },
                QuickActionType::Settings => {
                    self.launch_app("rae_settings")?;
                },
                QuickActionType::FileManager => {
                    self.launch_app("rae_finder")?;
                },
                _ => {},
            }
        }
        
        self.hide()?;
        Ok(())
    }

    /// Clear search
    pub fn clear_search(&mut self) -> Result<(), DesktopError> {
        self.state.search_query.clear();
        self.state.search_results.clear();
        Ok(())
    }

    /// Add to favorites
    pub fn add_to_favorites(&mut self, app_id: &str) -> Result<(), DesktopError> {
        if !self.favorites.contains(&app_id.to_string()) {
            self.favorites.push(app_id.to_string());
            
            if let Some(app) = self.apps.iter_mut().find(|a| a.app_id == app_id) {
                app.is_favorite = true;
            }
        }
        Ok(())
    }

    /// Remove from favorites
    pub fn remove_from_favorites(&mut self, app_id: &str) -> Result<(), DesktopError> {
        self.favorites.retain(|id| id != app_id);
        
        if let Some(app) = self.apps.iter_mut().find(|a| a.app_id == app_id) {
            app.is_favorite = false;
        }
        Ok(())
    }

    /// Helper functions
    fn categorize_apps(&mut self) -> Result<(), DesktopError> {
        self.categories.clear();
        
        for app in &self.apps {
            self.categories.entry(app.category.clone())
                .or_insert_with(Vec::new)
                .push(app.app_id.clone());
        }
        
        Ok(())
    }
    
    fn scan_installed_apps(&mut self) -> Result<(), DesktopError> {
        // In real implementation, would scan system for installed apps
        Ok(())
    }
    
    fn build_search_index(&mut self) -> Result<(), DesktopError> {
        // In real implementation, would build search index for fast lookups
        for app in &self.apps {
            let mut terms = vec![app.name.to_lowercase()];
            terms.extend(app.keywords.iter().map(|k| k.to_lowercase()));
            terms.push(app.description.to_lowercase());
            
            self.search_index.insert(app.app_id.clone(), terms);
        }
        Ok(())
    }
    
    fn add_recent_item(&mut self, item: RecentItem) {
        // Remove existing entry if present
        self.recent_items.retain(|i| i.path != item.path);
        
        // Add to front
        self.recent_items.insert(0, item);
        
        // Limit size
        if self.recent_items.len() > self.config.max_recent_items as usize {
            self.recent_items.truncate(self.config.max_recent_items as usize);
        }
    }
    
    fn refresh_recent_items(&mut self) -> Result<(), DesktopError> {
        // In real implementation, would refresh recent items from system
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
    
    fn load_recent_items(&mut self) -> Result<(), DesktopError> {
        // In real implementation, would load recent items from disk
        Ok(())
    }
    
    fn save_recent_items(&self) -> Result<(), DesktopError> {
        // In real implementation, would save recent items to disk
        Ok(())
    }

    /// Get all applications
    pub fn get_apps(&self) -> &[AppEntry] {
        &self.apps
    }
    
    /// Get apps by category
    pub fn get_apps_by_category(&self, category: &AppCategory) -> Vec<&AppEntry> {
        self.apps.iter().filter(|app| app.category == *category).collect()
    }
    
    /// Get favorite apps
    pub fn get_favorite_apps(&self) -> Vec<&AppEntry> {
        self.apps.iter().filter(|app| app.is_favorite).collect()
    }
}

/// Simple expression parser for calculator functionality
struct ExpressionParser {
    input: Vec<char>,
    position: usize,
}

impl ExpressionParser {
    fn new(expr: &str) -> Self {
        Self {
            input: expr.chars().filter(|c| !c.is_whitespace()).collect(),
            position: 0,
        }
    }

    fn parse_expression(&mut self) -> Result<f64, &'static str> {
        let result = self.parse_addition()?;
        if self.position < self.input.len() {
            return Err("Unexpected character");
        }
        Ok(result)
    }

    fn parse_addition(&mut self) -> Result<f64, &'static str> {
        let mut result = self.parse_multiplication()?;
        
        while self.position < self.input.len() {
            match self.current_char() {
                '+' => {
                    self.advance();
                    result += self.parse_multiplication()?;
                },
                '-' => {
                    self.advance();
                    result -= self.parse_multiplication()?;
                },
                _ => break,
            }
        }
        
        Ok(result)
    }

    fn parse_multiplication(&mut self) -> Result<f64, &'static str> {
        let mut result = self.parse_factor()?;
        
        while self.position < self.input.len() {
            match self.current_char() {
                '*' => {
                    self.advance();
                    result *= self.parse_factor()?;
                },
                '/' => {
                    self.advance();
                    let divisor = self.parse_factor()?;
                    if divisor == 0.0 {
                        return Err("Division by zero");
                    }
                    result /= divisor;
                },
                _ => break,
            }
        }
        
        Ok(result)
    }

    fn parse_factor(&mut self) -> Result<f64, &'static str> {
        if self.position >= self.input.len() {
            return Err("Unexpected end of expression");
        }

        match self.current_char() {
            '(' => {
                self.advance(); // consume '('
                let result = self.parse_addition()?;
                if self.position >= self.input.len() || self.current_char() != ')' {
                    return Err("Missing closing parenthesis");
                }
                self.advance(); // consume ')'
                Ok(result)
            },
            '-' => {
                self.advance();
                Ok(-self.parse_factor()?)
            },
            '+' => {
                self.advance();
                self.parse_factor()
            },
            _ => self.parse_number(),
        }
    }

    fn parse_number(&mut self) -> Result<f64, &'static str> {
        let start = self.position;
        let mut has_dot = false;
        
        while self.position < self.input.len() {
            let ch = self.current_char();
            if ch.is_ascii_digit() {
                self.advance();
            } else if ch == '.' && !has_dot {
                has_dot = true;
                self.advance();
            } else {
                break;
            }
        }
        
        if start == self.position {
            return Err("Expected number");
        }
        
        let number_str: String = self.input[start..self.position].iter().collect();
        number_str.parse::<f64>().map_err(|_| "Invalid number")
    }

    fn current_char(&self) -> char {
        self.input[self.position]
    }

    fn advance(&mut self) {
        self.position += 1;
    }
}    
    /// Get recent items
    pub fn get_recent_items(&self) -> &[RecentItem] {
        &self.recent_items
    }
    
    /// Get quick actions
    pub fn get_quick_actions(&self) -> &[QuickAction] {
        &self.quick_actions
    }
    
    /// Get configuration
    pub fn get_config(&self) -> &StartConfig {
        &self.config
    }
    
    /// Get current state
    pub fn get_state(&self) -> &StartState {
        &self.state
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: StartConfig) -> Result<(), DesktopError> {
        self.config = config;
        Ok(())
    }
}