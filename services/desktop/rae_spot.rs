//! RaeSpot - Spotlight-like Search
//! Universal search with commands, plugins, and intelligent suggestions
//! Features instant search, command execution, unit conversion, and extensible plugins

use crate::services::desktop::DesktopError;
use crate::kernel::filesystem::{VirtualFileSystem, FileType};
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::collections::BTreeMap;
use core::time::Duration;

/// Search result types
#[derive(Debug, Clone, PartialEq)]
pub enum SearchResultType {
    Application,
    File,
    Folder,
    Contact,
    Email,
    Calendar,
    Bookmark,
    History,
    Command,
    Calculator,
    UnitConverter,
    Dictionary,
    Translation,
    Weather,
    Stock,
    News,
    Plugin,
    System,
    Web,
}

/// Search result item
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub id: String,
    pub result_type: SearchResultType,
    pub title: String,
    pub subtitle: Option<String>,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub thumbnail: Option<Vec<u8>>,
    pub action: SearchAction,
    pub secondary_actions: Vec<SecondaryAction>,
    pub relevance_score: f32,
    pub category: String,
    pub metadata: BTreeMap<String, String>,
    pub preview_data: Option<PreviewData>,
}

/// Search actions
#[derive(Debug, Clone)]
pub enum SearchAction {
    Launch(String),
    OpenFile(String),
    OpenFolder(String),
    OpenUrl(String),
    ExecuteCommand(String),
    ShowContact(String),
    ComposeEmail(String),
    ShowCalendarEvent(String),
    Calculate(String),
    Convert(String, String, String),
    Define(String),
    Translate(String, String, String),
    ShowWeather(String),
    ShowStock(String),
    ShowNews(String),
    RunPlugin(String, BTreeMap<String, String>),
    SystemAction(String),
    WebSearch(String),
    Copy(String),
    Share(String),
}

/// Secondary actions for results
#[derive(Debug, Clone)]
pub struct SecondaryAction {
    pub id: String,
    pub name: String,
    pub icon: Option<String>,
    pub action: SearchAction,
    pub shortcut: Option<String>,
}

/// Preview data for results
#[derive(Debug, Clone)]
pub enum PreviewData {
    Text(String),
    Image(Vec<u8>),
    Html(String),
    Json(String),
    Custom(BTreeMap<String, String>),
}

/// Search plugin interface
#[derive(Debug, Clone)]
pub struct SearchPlugin {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub author: String,
    pub enabled: bool,
    pub priority: u32,
    pub triggers: Vec<String>,
    pub search_types: Vec<SearchResultType>,
    pub config: BTreeMap<String, String>,
    pub requires_network: bool,
    pub cache_results: bool,
    pub cache_duration: Duration,
}

/// Command definition
#[derive(Debug, Clone)]
pub struct Command {
    pub id: String,
    pub name: String,
    pub description: String,
    pub aliases: Vec<String>,
    pub syntax: String,
    pub examples: Vec<String>,
    pub category: CommandCategory,
    pub parameters: Vec<CommandParameter>,
    pub enabled: bool,
}

/// Command categories
#[derive(Debug, Clone, PartialEq)]
pub enum CommandCategory {
    System,
    File,
    Network,
    Development,
    Utility,
    Media,
    Custom,
}

/// Command parameter
#[derive(Debug, Clone)]
pub struct CommandParameter {
    pub name: String,
    pub parameter_type: ParameterType,
    pub required: bool,
    pub description: String,
    pub default_value: Option<String>,
    pub validation: Option<String>,
}

/// Parameter types
#[derive(Debug, Clone, PartialEq)]
pub enum ParameterType {
    String,
    Number,
    Boolean,
    File,
    Folder,
    Url,
    Email,
    Date,
    Time,
    Choice(Vec<String>),
}

/// Search suggestion
#[derive(Debug, Clone)]
pub struct SearchSuggestion {
    pub text: String,
    pub suggestion_type: SuggestionType,
    pub icon: Option<String>,
    pub description: Option<String>,
    pub completion: String,
}

/// Suggestion types
#[derive(Debug, Clone, PartialEq)]
pub enum SuggestionType {
    Recent,
    Popular,
    Completion,
    Command,
    Plugin,
    Smart,
}

/// Search configuration
#[derive(Debug, Clone)]
pub struct SpotConfig {
    pub max_results: u32,
    pub search_delay: Duration,
    pub show_suggestions: bool,
    pub show_previews: bool,
    pub enable_fuzzy_search: bool,
    pub enable_smart_suggestions: bool,
    pub cache_size: u64,
    pub cache_duration: Duration,
    pub plugins: Vec<String>,
    pub shortcuts: BTreeMap<String, String>,
    pub theme: SpotTheme,
}

/// Theme configuration
#[derive(Debug, Clone)]
pub struct SpotTheme {
    pub background_color: String,
    pub text_color: String,
    pub accent_color: String,
    pub border_color: String,
    pub transparency: f32,
    pub blur_enabled: bool,
    pub animation_speed: f32,
    pub border_radius: f32,
    pub shadow_enabled: bool,
}

/// Search state
#[derive(Debug, Clone)]
pub struct SpotState {
    pub visible: bool,
    pub query: String,
    pub results: Vec<SearchResult>,
    pub suggestions: Vec<SearchSuggestion>,
    pub selected_index: usize,
    pub preview_visible: bool,
    pub searching: bool,
    pub last_search_time: u64,
}

/// Search index for fast lookups
#[derive(Debug, Clone)]
pub struct SearchIndex {
    pub apps: BTreeMap<String, Vec<String>>,
    pub files: BTreeMap<String, Vec<String>>,
    pub contacts: BTreeMap<String, Vec<String>>,
    pub bookmarks: BTreeMap<String, Vec<String>>,
    pub commands: BTreeMap<String, Vec<String>>,
}

/// RaeSpot main service
pub struct RaeSpot {
    config: SpotConfig,
    plugins: BTreeMap<String, SearchPlugin>,
    commands: BTreeMap<String, Command>,
    state: SpotState,
    search_index: SearchIndex,
    cache: BTreeMap<String, (Vec<SearchResult>, u64)>,
    recent_searches: Vec<String>,
    popular_searches: BTreeMap<String, u32>,
}

impl RaeSpot {
    /// Create a new RaeSpot instance
    pub fn new() -> Result<Self, DesktopError> {
        let mut spot = RaeSpot {
            config: SpotConfig {
                max_results: 20,
                search_delay: Duration::from_millis(100),
                show_suggestions: true,
                show_previews: true,
                enable_fuzzy_search: true,
                enable_smart_suggestions: true,
                cache_size: 100 * 1024 * 1024, // 100MB
                cache_duration: Duration::from_secs(3600), // 1 hour
                plugins: Vec::new(),
                shortcuts: BTreeMap::new(),
                theme: SpotTheme {
                    background_color: "rgba(0, 0, 0, 0.95)".to_string(),
                    text_color: "#ffffff".to_string(),
                    accent_color: "#007acc".to_string(),
                    border_color: "#333333".to_string(),
                    transparency: 0.95,
                    blur_enabled: true,
                    animation_speed: 1.0,
                    border_radius: 12.0,
                    shadow_enabled: true,
                },
            },
            plugins: BTreeMap::new(),
            commands: BTreeMap::new(),
            state: SpotState {
                visible: false,
                query: String::new(),
                results: Vec::new(),
                suggestions: Vec::new(),
                selected_index: 0,
                preview_visible: false,
                searching: false,
                last_search_time: 0,
            },
            search_index: SearchIndex {
                apps: BTreeMap::new(),
                files: BTreeMap::new(),
                contacts: BTreeMap::new(),
                bookmarks: BTreeMap::new(),
                commands: BTreeMap::new(),
            },
            cache: BTreeMap::new(),
            recent_searches: Vec::new(),
            popular_searches: BTreeMap::new(),
        };

        spot.setup_default_plugins()?;
        spot.setup_default_commands()?;
        spot.setup_shortcuts()?;
        Ok(spot)
    }

    /// Start the RaeSpot service
    pub fn start(&mut self) -> Result<(), DesktopError> {
        self.load_configuration()?;
        self.build_search_index()?;
        self.load_plugins()?;
        Ok(())
    }

    /// Stop the RaeSpot service
    pub fn stop(&mut self) -> Result<(), DesktopError> {
        self.save_configuration()?;
        self.cleanup_cache()?;
        Ok(())
    }

    /// Setup default plugins
    fn setup_default_plugins(&mut self) -> Result<(), DesktopError> {
        let plugins = vec![
            SearchPlugin {
                id: "calculator".to_string(),
                name: "Calculator".to_string(),
                description: "Perform mathematical calculations".to_string(),
                version: "1.0.0".to_string(),
                author: "RaeenOS".to_string(),
                enabled: true,
                priority: 100,
                triggers: vec!["=".to_string(), "calc".to_string()],
                search_types: vec![SearchResultType::Calculator],
                config: BTreeMap::new(),
                requires_network: false,
                cache_results: false,
                cache_duration: Duration::from_secs(0),
            },
            SearchPlugin {
                id: "unit_converter".to_string(),
                name: "Unit Converter".to_string(),
                description: "Convert between different units".to_string(),
                version: "1.0.0".to_string(),
                author: "RaeenOS".to_string(),
                enabled: true,
                priority: 95,
                triggers: vec!["convert".to_string(), "to".to_string()],
                search_types: vec![SearchResultType::UnitConverter],
                config: BTreeMap::new(),
                requires_network: false,
                cache_results: true,
                cache_duration: Duration::from_secs(86400), // 24 hours
            },
            SearchPlugin {
                id: "dictionary".to_string(),
                name: "Dictionary".to_string(),
                description: "Look up word definitions".to_string(),
                version: "1.0.0".to_string(),
                author: "RaeenOS".to_string(),
                enabled: true,
                priority: 80,
                triggers: vec!["define".to_string(), "meaning".to_string()],
                search_types: vec![SearchResultType::Dictionary],
                config: BTreeMap::new(),
                requires_network: true,
                cache_results: true,
                cache_duration: Duration::from_secs(604800), // 1 week
            },
            SearchPlugin {
                id: "weather".to_string(),
                name: "Weather".to_string(),
                description: "Get weather information".to_string(),
                version: "1.0.0".to_string(),
                author: "RaeenOS".to_string(),
                enabled: true,
                priority: 75,
                triggers: vec!["weather".to_string(), "forecast".to_string()],
                search_types: vec![SearchResultType::Weather],
                config: BTreeMap::new(),
                requires_network: true,
                cache_results: true,
                cache_duration: Duration::from_secs(1800), // 30 minutes
            },
        ];

        for plugin in plugins {
            self.plugins.insert(plugin.id.clone(), plugin);
        }
        Ok(())
    }

    /// Setup default commands
    fn setup_default_commands(&mut self) -> Result<(), DesktopError> {
        let commands = vec![
            Command {
                id: "quit".to_string(),
                name: "Quit Application".to_string(),
                description: "Quit the specified application".to_string(),
                aliases: vec!["exit".to_string(), "close".to_string()],
                syntax: "quit <app_name>".to_string(),
                examples: vec!["quit browser".to_string(), "quit terminal".to_string()],
                category: CommandCategory::System,
                parameters: vec![
                    CommandParameter {
                        name: "app_name".to_string(),
                        parameter_type: ParameterType::String,
                        required: true,
                        description: "Name of the application to quit".to_string(),
                        default_value: None,
                        validation: None,
                    },
                ],
                enabled: true,
            },
            Command {
                id: "open".to_string(),
                name: "Open File or Application".to_string(),
                description: "Open a file or launch an application".to_string(),
                aliases: vec!["launch".to_string(), "run".to_string()],
                syntax: "open <path_or_app>".to_string(),
                examples: vec!["open ~/documents".to_string(), "open calculator".to_string()],
                category: CommandCategory::File,
                parameters: vec![
                    CommandParameter {
                        name: "path_or_app".to_string(),
                        parameter_type: ParameterType::String,
                        required: true,
                        description: "Path to file/folder or application name".to_string(),
                        default_value: None,
                        validation: None,
                    },
                ],
                enabled: true,
            },
            Command {
                id: "find".to_string(),
                name: "Find Files".to_string(),
                description: "Search for files and folders".to_string(),
                aliases: vec!["search".to_string(), "locate".to_string()],
                syntax: "find <pattern> [in <location>]".to_string(),
                examples: vec!["find *.txt".to_string(), "find project in ~/documents".to_string()],
                category: CommandCategory::File,
                parameters: vec![
                    CommandParameter {
                        name: "pattern".to_string(),
                        parameter_type: ParameterType::String,
                        required: true,
                        description: "Search pattern or filename".to_string(),
                        default_value: None,
                        validation: None,
                    },
                    CommandParameter {
                        name: "location".to_string(),
                        parameter_type: ParameterType::Folder,
                        required: false,
                        description: "Directory to search in".to_string(),
                        default_value: Some("~".to_string()),
                        validation: None,
                    },
                ],
                enabled: true,
            },
            Command {
                id: "email".to_string(),
                name: "Compose Email".to_string(),
                description: "Compose and send an email".to_string(),
                aliases: vec!["mail".to_string(), "send".to_string()],
                syntax: "email <recipient> [subject <subject>]".to_string(),
                examples: vec!["email john@example.com".to_string(), "email team@company.com subject Meeting".to_string()],
                category: CommandCategory::Utility,
                parameters: vec![
                    CommandParameter {
                        name: "recipient".to_string(),
                        parameter_type: ParameterType::Email,
                        required: true,
                        description: "Email address of recipient".to_string(),
                        default_value: None,
                        validation: Some(r"^[\w\.-]+@[\w\.-]+\.[a-zA-Z]{2,}$".to_string()),
                    },
                    CommandParameter {
                        name: "subject".to_string(),
                        parameter_type: ParameterType::String,
                        required: false,
                        description: "Email subject line".to_string(),
                        default_value: None,
                        validation: None,
                    },
                ],
                enabled: true,
            },
        ];

        for command in commands {
            self.commands.insert(command.id.clone(), command);
        }
        Ok(())
    }

    /// Setup keyboard shortcuts
    fn setup_shortcuts(&mut self) -> Result<(), DesktopError> {
        self.config.shortcuts.insert("Cmd+Space".to_string(), "toggle_spot".to_string());
        self.config.shortcuts.insert("Alt+Space".to_string(), "toggle_spot".to_string());
        self.config.shortcuts.insert("Escape".to_string(), "hide_spot".to_string());
        self.config.shortcuts.insert("Enter".to_string(), "execute_selected".to_string());
        self.config.shortcuts.insert("Tab".to_string(), "complete_suggestion".to_string());
        self.config.shortcuts.insert("Up".to_string(), "select_previous".to_string());
        self.config.shortcuts.insert("Down".to_string(), "select_next".to_string());
        Ok(())
    }

    /// Show RaeSpot
    pub fn show(&mut self) -> Result<(), DesktopError> {
        if !self.state.visible {
            self.state.visible = true;
            self.state.query.clear();
            self.state.results.clear();
            self.state.selected_index = 0;
            
            if self.config.show_suggestions {
                self.generate_suggestions()?;
            }
        }
        Ok(())
    }

    /// Hide RaeSpot
    pub fn hide(&mut self) -> Result<(), DesktopError> {
        if self.state.visible {
            self.state.visible = false;
            self.state.preview_visible = false;
            
            // Add to recent searches if query is not empty
            if !self.state.query.is_empty() {
                self.add_to_recent_searches(&self.state.query);
            }
        }
        Ok(())
    }

    /// Toggle RaeSpot visibility
    pub fn toggle(&mut self) -> Result<(), DesktopError> {
        if self.state.visible {
            self.hide()
        } else {
            self.show()
        }
    }

    /// Perform search
    pub fn search(&mut self, query: &str) -> Result<Vec<SearchResult>, DesktopError> {
        self.state.query = query.to_string();
        self.state.searching = true;
        self.state.last_search_time = self.get_current_time();
        
        // Check cache first
        if let Some((cached_results, timestamp)) = self.cache.get(query) {
            if self.get_current_time() - timestamp < self.config.cache_duration.as_secs() {
                self.state.results = cached_results.clone();
                self.state.searching = false;
                return Ok(cached_results.clone());
            }
        }
        
        let mut results = Vec::new();
        
        if query.is_empty() {
            self.state.results.clear();
            self.state.searching = false;
            return Ok(results);
        }
        
        // Search applications
        results.extend(self.search_applications(query)?);
        
        // Search files
        results.extend(self.search_files(query)?);
        
        // Search commands
        results.extend(self.search_commands(query)?);
        
        // Search with plugins
        results.extend(self.search_with_plugins(query)?);
        
        // Calculator
        if let Some(calc_result) = self.try_calculate(query)? {
            results.push(calc_result);
        }
        
        // Unit conversion
        if let Some(convert_result) = self.try_convert(query)? {
            results.push(convert_result);
        }
        
        // Web search fallback
        if results.is_empty() && query.len() > 2 {
            results.push(SearchResult {
                id: format!("web_{}", query),
                result_type: SearchResultType::Web,
                title: format!("Search web for '{}'", query),
                subtitle: Some("Press Enter to search".to_string()),
                description: None,
                icon: Some("/system/icons/web.svg".to_string()),
                thumbnail: None,
                action: SearchAction::WebSearch(query.to_string()),
                secondary_actions: Vec::new(),
                relevance_score: 0.1,
                category: "Web".to_string(),
                metadata: BTreeMap::new(),
                preview_data: None,
            });
        }
        
        // Sort by relevance
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap());
        
        // Limit results
        if results.len() > self.config.max_results as usize {
            results.truncate(self.config.max_results as usize);
        }
        
        // Cache results
        self.cache.insert(query.to_string(), (results.clone(), self.get_current_time()));
        
        self.state.results = results.clone();
        self.state.searching = false;
        
        // Update popularity
        *self.popular_searches.entry(query.to_string()).or_insert(0) += 1;
        
        Ok(results)
    }

    /// Search applications
    fn search_applications(&self, query: &str) -> Result<Vec<SearchResult>, DesktopError> {
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();
        
        // Search through indexed applications
        for (term, app_paths) in &self.search_index.apps {
            if term.contains(&query_lower) {
                for app_path in app_paths {
                    // Extract app name from path
                    let app_name = app_path.split('/').last().unwrap_or("Unknown App");
                    let app_name_clean = app_name.split('.').next().unwrap_or(app_name);
                    
                    // Calculate relevance score
                    let score = if term.starts_with(&query_lower) {
                        0.9
                    } else if app_name_clean.to_lowercase().starts_with(&query_lower) {
                        0.8
                    } else {
                        0.6
                    };
                    
                    // Determine icon based on file extension or app type
                    let icon = self.get_app_icon(app_path);
                    
                    results.push(SearchResult {
                        id: app_path.clone(),
                        result_type: SearchResultType::Application,
                        title: app_name_clean.to_string(),
                        subtitle: Some(app_path.clone()),
                        description: Some("Application".to_string()),
                        icon: Some(icon),
                        thumbnail: None,
                        action: SearchAction::Launch(app_path.clone()),
                        secondary_actions: vec![
                            SecondaryAction {
                                id: "show_in_finder".to_string(),
                                name: "Show in RaeFinder".to_string(),
                                icon: Some("/system/icons/folder.svg".to_string()),
                                action: SearchAction::OpenFolder(app_path.rsplit('/').skip(1).collect::<Vec<_>>().join("/")),
                                shortcut: Some("Cmd+R".to_string()),
                            },
                        ],
                        relevance_score: score,
                        category: "Applications".to_string(),
                        metadata: BTreeMap::new(),
                        preview_data: None,
                    });
                }
            }
        }
        
        // Remove duplicates and sort by relevance
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(core::cmp::Ordering::Equal));
        results.dedup_by(|a, b| a.id == b.id);
        
        // Limit results
        results.truncate(10);
        
        Ok(results)
    }

    /// Search files
    fn search_files(&self, query: &str) -> Result<Vec<SearchResult>, DesktopError> {
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();
        
        // Search through indexed files
        for (term, file_paths) in &self.search_index.files {
            if term.contains(&query_lower) {
                for file_path in file_paths {
                    // Extract filename from path
                    let filename = file_path.split('/').last().unwrap_or("Unknown File");
                    let directory = file_path.rsplit('/').skip(1).collect::<Vec<_>>().join("/");
                    
                    // Calculate relevance score
                    let score = if term.starts_with(&query_lower) {
                        0.9
                    } else if filename.to_lowercase().starts_with(&query_lower) {
                        0.8
                    } else {
                        0.6
                    };
                    
                    // Determine file type and icon
                    let (file_type, icon) = self.get_file_type_and_icon(filename);
                    
                    results.push(SearchResult {
                        id: file_path.clone(),
                        result_type: SearchResultType::File,
                        title: filename.to_string(),
                        subtitle: Some(directory),
                        description: Some(file_type),
                        icon: Some(icon),
                        thumbnail: None,
                        action: SearchAction::OpenFile(file_path.clone()),
                        secondary_actions: vec![
                            SecondaryAction {
                                id: "show_in_finder".to_string(),
                                name: "Show in RaeFinder".to_string(),
                                icon: Some("/system/icons/folder.svg".to_string()),
                                action: SearchAction::OpenFolder(file_path.rsplit('/').skip(1).collect::<Vec<_>>().join("/")),
                                shortcut: Some("Cmd+R".to_string()),
                            },
                            SecondaryAction {
                                id: "copy_path".to_string(),
                                name: "Copy Path".to_string(),
                                icon: Some("/system/icons/copy.svg".to_string()),
                                action: SearchAction::Copy(file_path.clone()),
                                shortcut: Some("Cmd+C".to_string()),
                            },
                        ],
                        relevance_score: score,
                        category: "Files".to_string(),
                        metadata: BTreeMap::new(),
                        preview_data: None,
                    });
                }
            }
        }
        
        // Remove duplicates and sort by relevance
        results.sort_by(|a, b| b.relevance_score.partial_cmp(&a.relevance_score).unwrap_or(core::cmp::Ordering::Equal));
        results.dedup_by(|a, b| a.id == b.id);
        
        // Limit results
        results.truncate(15);
        
        Ok(results)
    }

    /// Search commands
    fn search_commands(&self, query: &str) -> Result<Vec<SearchResult>, DesktopError> {
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();
        
        for command in self.commands.values() {
            if !command.enabled {
                continue;
            }
            
            let mut score = 0.0;
            
            if command.name.to_lowercase().contains(&query_lower) {
                score = 0.8;
            } else if command.aliases.iter().any(|alias| alias.to_lowercase().contains(&query_lower)) {
                score = 0.7;
            } else if command.description.to_lowercase().contains(&query_lower) {
                score = 0.5;
            }
            
            if score > 0.0 {
                results.push(SearchResult {
                    id: command.id.clone(),
                    result_type: SearchResultType::Command,
                    title: command.name.clone(),
                    subtitle: Some(command.syntax.clone()),
                    description: Some(command.description.clone()),
                    icon: Some("/system/icons/command.svg".to_string()),
                    thumbnail: None,
                    action: SearchAction::ExecuteCommand(command.id.clone()),
                    secondary_actions: Vec::new(),
                    relevance_score: score,
                    category: format!("{:?}", command.category),
                    metadata: BTreeMap::new(),
                    preview_data: Some(PreviewData::Text(format!(
                        "Syntax: {}\n\nExamples:\n{}",
                        command.syntax,
                        command.examples.join("\n")
                    ))),
                });
            }
        }
        
        Ok(results)
    }

    /// Search with plugins
    fn search_with_plugins(&self, query: &str) -> Result<Vec<SearchResult>, DesktopError> {
        let mut results = Vec::new();
        
        for plugin in self.plugins.values() {
            if !plugin.enabled {
                continue;
            }
            
            // Check if query matches plugin triggers
            let triggered = plugin.triggers.iter().any(|trigger| {
                query.to_lowercase().contains(&trigger.to_lowercase())
            });
            
            if triggered {
                // Simulate plugin search
                match plugin.id.as_str() {
                    "dictionary" => {
                        if query.to_lowercase().starts_with("define ") {
                            let word = query[7..].trim();
                            results.push(SearchResult {
                                id: format!("define_{}", word),
                                result_type: SearchResultType::Dictionary,
                                title: format!("Define '{}'", word),
                                subtitle: Some("Dictionary definition".to_string()),
                                description: Some(format!("Look up the definition of '{}'", word)),
                                icon: Some("/system/icons/dictionary.svg".to_string()),
                                thumbnail: None,
                                action: SearchAction::Define(word.to_string()),
                                secondary_actions: Vec::new(),
                                relevance_score: 0.85,
                                category: "Dictionary".to_string(),
                                metadata: BTreeMap::new(),
                                preview_data: Some(PreviewData::Text(format!(
                                    "{}\n\n[noun] A sample definition for the word '{}'",
                                    word, word
                                ))),
                            });
                        }
                    },
                    "weather" => {
                        if query.to_lowercase().contains("weather") {
                            results.push(SearchResult {
                                id: "weather_current".to_string(),
                                result_type: SearchResultType::Weather,
                                title: "Current Weather".to_string(),
                                subtitle: Some("22°C, Partly Cloudy".to_string()),
                                description: Some("Weather in your current location".to_string()),
                                icon: Some("/system/icons/weather.svg".to_string()),
                                thumbnail: None,
                                action: SearchAction::ShowWeather("current".to_string()),
                                secondary_actions: Vec::new(),
                                relevance_score: 0.9,
                                category: "Weather".to_string(),
                                metadata: BTreeMap::new(),
                                preview_data: Some(PreviewData::Html(
                                    "<div>Temperature: 22°C<br>Condition: Partly Cloudy<br>Humidity: 65%</div>".to_string()
                                )),
                            });
                        }
                    },
                    _ => {},
                }
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
                        id: format!("calc_{}", query),
                        result_type: SearchResultType::Calculator,
                        title: format!("{} = {}", expression, result_str),
                        subtitle: Some("Calculator".to_string()),
                        description: None,
                        icon: Some("/system/icons/calculator.svg".to_string()),
                        thumbnail: None,
                        action: SearchAction::Calculate(result_str.clone()),
                        secondary_actions: vec![
                            SecondaryAction {
                                id: "copy_result".to_string(),
                                name: "Copy Result".to_string(),
                                icon: Some("/system/icons/copy.svg".to_string()),
                                action: SearchAction::Copy(result_str),
                                shortcut: Some("Cmd+C".to_string()),
                            },
                        ],
                        relevance_score: 0.95,
                        category: "Calculator".to_string(),
                        metadata: BTreeMap::new(),
                        preview_data: None,
                    }));
                },
                Err(_) => {
                    // Return error result for invalid expressions
                    return Ok(Some(SearchResult {
                        id: format!("calc_error_{}", query),
                        result_type: SearchResultType::Calculator,
                        title: format!("{} = Error", expression),
                        subtitle: Some("Invalid expression".to_string()),
                        description: Some("Check your mathematical expression syntax".to_string()),
                        icon: Some("/system/icons/calculator.svg".to_string()),
                        thumbnail: None,
                        action: SearchAction::Calculate("Error".to_string()),
                        secondary_actions: vec![],
                        relevance_score: 0.8,
                        category: "Calculator".to_string(),
                        metadata: BTreeMap::new(),
                        preview_data: None,
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

    /// Try to convert units
    fn try_convert(&self, query: &str) -> Result<Option<SearchResult>, DesktopError> {
        if query.to_lowercase().contains(" to ") || query.to_lowercase().contains(" in ") {
            // Simulate unit conversion
            let parts: Vec<&str> = query.split(" to ").collect();
            if parts.len() == 2 {
                let from = parts[0].trim();
                let to = parts[1].trim();
                let result = "100"; // Placeholder
                
                return Ok(Some(SearchResult {
                    id: format!("convert_{}_{}", from, to),
                    result_type: SearchResultType::UnitConverter,
                    title: format!("{} = {} {}", from, result, to),
                    subtitle: Some("Unit Conversion".to_string()),
                    description: None,
                    icon: Some("/system/icons/convert.svg".to_string()),
                    thumbnail: None,
                    action: SearchAction::Convert(from.to_string(), to.to_string(), result.to_string()),
                    secondary_actions: vec![
                        SecondaryAction {
                            id: "copy_result".to_string(),
                            name: "Copy Result".to_string(),
                            icon: Some("/system/icons/copy.svg".to_string()),
                            action: SearchAction::Copy(format!("{} {}", result, to)),
                            shortcut: Some("Cmd+C".to_string()),
                        },
                    ],
                    relevance_score: 0.9,
                    category: "Unit Converter".to_string(),
                    metadata: BTreeMap::new(),
                    preview_data: None,
                }));
            }
        }
        
        Ok(None)
    }

    /// Generate search suggestions
    fn generate_suggestions(&mut self) -> Result<(), DesktopError> {
        let mut suggestions = Vec::new();
        
        // Recent searches
        for search in self.recent_searches.iter().take(5) {
            suggestions.push(SearchSuggestion {
                text: search.clone(),
                suggestion_type: SuggestionType::Recent,
                icon: Some("/system/icons/recent.svg".to_string()),
                description: Some("Recent search".to_string()),
                completion: search.clone(),
            });
        }
        
        // Popular searches
        let mut popular: Vec<_> = self.popular_searches.iter().collect();
        popular.sort_by(|a, b| b.1.cmp(a.1));
        
        for (search, _count) in popular.iter().take(3) {
            if !self.recent_searches.contains(search) {
                suggestions.push(SearchSuggestion {
                    text: search.to_string(),
                    suggestion_type: SuggestionType::Popular,
                    icon: Some("/system/icons/popular.svg".to_string()),
                    description: Some("Popular search".to_string()),
                    completion: search.to_string(),
                });
            }
        }
        
        // Command suggestions
        for command in self.commands.values().take(3) {
            suggestions.push(SearchSuggestion {
                text: command.name.clone(),
                suggestion_type: SuggestionType::Command,
                icon: Some("/system/icons/command.svg".to_string()),
                description: Some(command.description.clone()),
                completion: command.syntax.clone(),
            });
        }
        
        self.state.suggestions = suggestions;
        Ok(())
    }

    /// Execute selected result
    pub fn execute_selected(&mut self) -> Result<(), DesktopError> {
        if let Some(result) = self.state.results.get(self.state.selected_index) {
            self.execute_action(&result.action)?;
        }
        self.hide()?;
        Ok(())
    }

    /// Execute search action
    pub fn execute_action(&self, action: &SearchAction) -> Result<(), DesktopError> {
        match action {
            SearchAction::Launch(app_id) => {
                // Launch application
            },
            SearchAction::OpenFile(path) => {
                // Open file
            },
            SearchAction::OpenFolder(path) => {
                // Open folder
            },
            SearchAction::ExecuteCommand(command_id) => {
                // Execute command
            },
            SearchAction::Calculate(result) => {
                // Copy result to clipboard
            },
            SearchAction::WebSearch(query) => {
                // Open web search
            },
            _ => {},
        }
        Ok(())
    }

    /// Select next result
    pub fn select_next(&mut self) -> Result<(), DesktopError> {
        if !self.state.results.is_empty() {
            self.state.selected_index = (self.state.selected_index + 1) % self.state.results.len();
        }
        Ok(())
    }

    /// Select previous result
    pub fn select_previous(&mut self) -> Result<(), DesktopError> {
        if !self.state.results.is_empty() {
            if self.state.selected_index == 0 {
                self.state.selected_index = self.state.results.len() - 1;
            } else {
                self.state.selected_index -= 1;
            }
        }
        Ok(())
    }

    /// Toggle preview
    pub fn toggle_preview(&mut self) -> Result<(), DesktopError> {
        self.state.preview_visible = !self.state.preview_visible;
        Ok(())
    }

    /// Helper functions
    fn add_to_recent_searches(&mut self, query: &str) {
        self.recent_searches.retain(|q| q != query);
        self.recent_searches.insert(0, query.to_string());
        
        if self.recent_searches.len() > 20 {
            self.recent_searches.truncate(20);
        }
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
    
    fn build_search_index(&mut self) -> Result<(), DesktopError> {
        // Clear existing index
        self.search_index.files.clear();
        self.search_index.apps.clear();
        
        // Index common directories
        let directories_to_index = vec![
            "/home",
            "/usr/bin",
            "/usr/local/bin",
            "/Applications",
            "/Documents",
            "/Desktop",
            "/Downloads",
        ];
        
        for dir_path in directories_to_index {
            if let Err(_) = self.index_directory(dir_path, 3) {
                // Continue indexing other directories even if one fails
                continue;
            }
        }
        
        Ok(())
    }
    
    fn index_directory(&mut self, path: &str, max_depth: usize) -> Result<(), DesktopError> {
        if max_depth == 0 {
            return Ok(());
        }
        
        // Get VFS instance
        let vfs = VirtualFileSystem::new();
        
        // List directory contents
        match vfs.list_directory(path) {
            Ok(entries) => {
                for entry in entries {
                    let full_path = if path.ends_with('/') {
                        format!("{}{}", path, entry.name)
                    } else {
                        format!("{}/{}", path, entry.name)
                    };
                    
                    match entry.file_type {
                        FileType::Directory => {
                            // Recursively index subdirectories
                            let _ = self.index_directory(&full_path, max_depth - 1);
                        },
                        FileType::RegularFile => {
                            self.index_file(&full_path, &entry.name);
                        },
                    }
                }
            },
            Err(_) => {
                // Directory doesn't exist or can't be read
                return Err(DesktopError::IoError);
            }
        }
        
        Ok(())
    }
    
    fn index_file(&mut self, full_path: &str, filename: &str) {
        // Extract file extension
        let extension = filename.split('.').last().unwrap_or("").to_lowercase();
        
        // Determine if it's an application
        let is_app = matches!(extension.as_str(), "exe" | "app" | "deb" | "rpm" | "dmg" | "msi");
        
        // Create search terms from filename
        let search_terms = self.create_search_terms(filename);
        
        if is_app {
            // Index as application
            for term in &search_terms {
                self.search_index.apps
                    .entry(term.clone())
                    .or_insert_with(Vec::new)
                    .push(full_path.to_string());
            }
        } else {
            // Index as file
            for term in &search_terms {
                self.search_index.files
                    .entry(term.clone())
                    .or_insert_with(Vec::new)
                    .push(full_path.to_string());
            }
        }
    }
    
    fn create_search_terms(&self, filename: &str) -> Vec<String> {
        let mut terms = Vec::new();
        
        // Add full filename (without extension)
        let name_without_ext = filename.split('.').next().unwrap_or(filename);
        terms.push(name_without_ext.to_lowercase());
        
        // Add individual words
        for word in name_without_ext.split(|c: char| !c.is_alphanumeric()) {
            if !word.is_empty() && word.len() > 1 {
                terms.push(word.to_lowercase());
            }
        }
        
        // Add prefixes for partial matching
        for term in terms.clone() {
            for i in 2..=term.len() {
                let prefix = &term[..i];
                if !terms.contains(&prefix.to_string()) {
                    terms.push(prefix.to_string());
                }
            }
        }
        
        terms
    }
    
    fn get_app_icon(&self, app_path: &str) -> String {
        let extension = app_path.split('.').last().unwrap_or("").to_lowercase();
        match extension.as_str() {
            "exe" => "/system/icons/app-windows.svg".to_string(),
            "app" => "/system/icons/app-macos.svg".to_string(),
            "deb" | "rpm" => "/system/icons/app-linux.svg".to_string(),
            "dmg" => "/system/icons/app-installer.svg".to_string(),
            "msi" => "/system/icons/app-installer.svg".to_string(),
            _ => "/system/icons/application.svg".to_string(),
        }
    }
    
    fn get_file_type_and_icon(&self, filename: &str) -> (String, String) {
        let extension = filename.split('.').last().unwrap_or("").to_lowercase();
        match extension.as_str() {
            "txt" | "md" | "readme" => ("Text Document".to_string(), "/system/icons/file-text.svg".to_string()),
            "pdf" => ("PDF Document".to_string(), "/system/icons/file-pdf.svg".to_string()),
            "doc" | "docx" => ("Word Document".to_string(), "/system/icons/file-word.svg".to_string()),
            "xls" | "xlsx" => ("Excel Spreadsheet".to_string(), "/system/icons/file-excel.svg".to_string()),
            "ppt" | "pptx" => ("PowerPoint Presentation".to_string(), "/system/icons/file-powerpoint.svg".to_string()),
            "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg" => ("Image".to_string(), "/system/icons/file-image.svg".to_string()),
            "mp3" | "wav" | "flac" | "aac" | "ogg" => ("Audio File".to_string(), "/system/icons/file-audio.svg".to_string()),
            "mp4" | "avi" | "mkv" | "mov" | "wmv" => ("Video File".to_string(), "/system/icons/file-video.svg".to_string()),
            "zip" | "rar" | "7z" | "tar" | "gz" => ("Archive".to_string(), "/system/icons/file-archive.svg".to_string()),
            "rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "hpp" => ("Source Code".to_string(), "/system/icons/file-code.svg".to_string()),
            "html" | "css" | "xml" | "json" | "yaml" | "yml" => ("Web File".to_string(), "/system/icons/file-web.svg".to_string()),
            _ => ("File".to_string(), "/system/icons/file.svg".to_string()),
        }
    }
    
    fn load_plugins(&mut self) -> Result<(), DesktopError> {
        // In real implementation, would load external plugins
        Ok(())
    }
    
    fn cleanup_cache(&mut self) -> Result<(), DesktopError> {
        self.cache.clear();
        Ok(())
    }

    /// Get current state
    pub fn get_state(&self) -> &SpotState {
        &self.state
    }
    
    /// Get configuration
    pub fn get_config(&self) -> &SpotConfig {
        &self.config
    }
    
    /// Get available plugins
    pub fn get_plugins(&self) -> &BTreeMap<String, SearchPlugin> {
        &self.plugins
    }
    
    /// Get available commands
    pub fn get_commands(&self) -> &BTreeMap<String, Command> {
        &self.commands
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: SpotConfig) -> Result<(), DesktopError> {
        self.config = config;
        Ok(())
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