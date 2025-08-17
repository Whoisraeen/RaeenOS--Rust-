//! RaeStacks - Intelligent Desktop File Organization
//! Auto-groups files on desktop by type/date/project with AI assistance
//! Provides customizable grouping rules and project mode integration

use crate::services::contracts::ai::AiContract;
use crate::services::desktop::DesktopError;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::collections::BTreeMap;
use core::time::Duration;

/// Directory entry for filesystem scanning
#[derive(Debug, Clone)]
struct DirectoryEntry {
    name: String,
    path: String,
    is_directory: bool,
}

/// File grouping strategies
#[derive(Debug, Clone, PartialEq)]
pub enum GroupingStrategy {
    ByType,
    ByDate,
    ByProject,
    BySize,
    Custom(String),
}

/// File stack configuration
#[derive(Debug, Clone)]
pub struct StackConfig {
    pub strategy: GroupingStrategy,
    pub auto_group: bool,
    pub animation_style: AnimationStyle,
    pub grid_density: GridDensity,
    pub exclude_patterns: Vec<String>,
    pub project_mode_enabled: bool,
}

/// Animation styles for stack operations
#[derive(Debug, Clone)]
pub enum AnimationStyle {
    Smooth,
    Bounce,
    Fade,
    Instant,
}

/// Grid density options
#[derive(Debug, Clone)]
pub enum GridDensity {
    Compact,
    Normal,
    Spacious,
    Custom { rows: u32, cols: u32 },
}

/// File metadata for grouping decisions
#[derive(Debug, Clone)]
pub struct FileMetadata {
    pub path: String,
    pub name: String,
    pub extension: String,
    pub size: u64,
    pub created: u64,
    pub modified: u64,
    pub mime_type: String,
    pub project_tags: Vec<String>,
}

/// A stack of grouped files
#[derive(Debug, Clone)]
pub struct FileStack {
    pub id: String,
    pub name: String,
    pub files: Vec<FileMetadata>,
    pub position: (i32, i32),
    pub expanded: bool,
    pub auto_created: bool,
    pub project_context: Option<ProjectContext>,
}

/// Project context for enhanced grouping
#[derive(Debug, Clone)]
pub struct ProjectContext {
    pub name: String,
    pub related_emails: Vec<String>,
    pub related_notes: Vec<String>,
    pub related_apps: Vec<String>,
    pub last_accessed: u64,
}

/// AI-powered grouping suggestion
#[derive(Debug, Clone)]
pub struct GroupingSuggestion {
    pub confidence: f32,
    pub strategy: GroupingStrategy,
    pub reason: String,
    pub suggested_name: String,
}

/// RaeStacks main service
pub struct RaeStacks {
    stacks: Vec<FileStack>,
    config: StackConfig,
    desktop_files: Vec<FileMetadata>,
    grouping_rules: BTreeMap<String, GroupingStrategy>,
    ai_enabled: bool,
    monitoring_active: bool,
}

impl RaeStacks {
    /// Create a new RaeStacks instance
    pub fn new() -> Result<Self, DesktopError> {
        Ok(RaeStacks {
            stacks: Vec::new(),
            config: StackConfig {
                strategy: GroupingStrategy::ByType,
                auto_group: true,
                animation_style: AnimationStyle::Smooth,
                grid_density: GridDensity::Normal,
                exclude_patterns: vec![
                    "*.tmp".to_string(),
                    "*.log".to_string(),
                    ".*".to_string(), // Hidden files
                ],
                project_mode_enabled: true,
            },
            desktop_files: Vec::new(),
            grouping_rules: BTreeMap::new(),
            ai_enabled: true,
            monitoring_active: false,
        })
    }

    /// Start the RaeStacks service
    pub fn start(&mut self) -> Result<(), DesktopError> {
        self.scan_desktop_files()?;
        self.apply_initial_grouping()?;
        self.start_file_monitoring()?;
        self.monitoring_active = true;
        Ok(())
    }

    /// Stop the RaeStacks service
    pub fn stop(&mut self) -> Result<(), DesktopError> {
        self.monitoring_active = false;
        self.save_configuration()?;
        Ok(())
    }

    /// Scan desktop for files and populate metadata
    pub fn scan_desktop_files(&mut self) -> Result<(), DesktopError> {
        self.desktop_files.clear();
        
        // Get desktop path from environment or use default
        let desktop_path = self.get_desktop_path()?;
        
        // Scan the desktop directory
        self.scan_directory(&desktop_path)?;
        
        Ok(())
    }
    
    /// Get the desktop directory path
    fn get_desktop_path(&self) -> Result<String, DesktopError> {
        // In a real implementation, this would query the environment
        // For now, use a reasonable default based on the OS
        #[cfg(target_os = "windows")]
        let desktop_path = "C:\\Users\\Public\\Desktop".to_string();
        
        #[cfg(target_os = "linux")]
        let desktop_path = "/home/user/Desktop".to_string();
        
        #[cfg(target_os = "macos")]
        let desktop_path = "/Users/user/Desktop".to_string();
        
        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        let desktop_path = "/Desktop".to_string();
        
        Ok(desktop_path)
    }
    
    /// Recursively scan a directory for files
    fn scan_directory(&mut self, path: &str) -> Result<(), DesktopError> {
        // Use filesystem syscalls to read directory contents
        match self.read_directory_entries(path) {
            Ok(entries) => {
                for entry in entries {
                    if self.is_file(&entry.path)? {
                        if let Ok(metadata) = self.extract_file_metadata(&entry) {
                            if !self.should_exclude_file(&metadata) {
                                self.desktop_files.push(metadata);
                            }
                        }
                    } else if self.is_directory(&entry.path)? && entry.name != "." && entry.name != ".." {
                        // Optionally scan subdirectories (limited depth)
                        if self.should_scan_subdirectory(&entry.path) {
                            self.scan_directory(&entry.path)?;
                        }
                    }
                }
            },
            Err(_) => {
                // If we can't read the directory, fall back to simulated data for now
                self.populate_fallback_data();
            }
        }
        
        Ok(())
    }
    
    /// Read directory entries using filesystem syscalls
    fn read_directory_entries(&self, path: &str) -> Result<Vec<DirectoryEntry>, DesktopError> {
        // This would use actual filesystem syscalls in a real implementation
        // For now, simulate some entries to demonstrate the structure
        Ok(vec![
            DirectoryEntry {
                name: "document.pdf".to_string(),
                path: alloc::format!("{}/document.pdf", path),
                is_directory: false,
            },
            DirectoryEntry {
                name: "photo.jpg".to_string(),
                path: alloc::format!("{}/photo.jpg", path),
                is_directory: false,
            },
            DirectoryEntry {
                name: "code.rs".to_string(),
                path: alloc::format!("{}/code.rs", path),
                is_directory: false,
            },
            DirectoryEntry {
                name: "Projects".to_string(),
                path: alloc::format!("{}/Projects", path),
                is_directory: true,
            },
        ])
    }
    
    /// Check if path is a file
    fn is_file(&self, path: &str) -> Result<bool, DesktopError> {
        // This would use filesystem syscalls to check file type
        // For now, check if it has an extension
        Ok(path.contains('.') && !path.ends_with('/'))
    }
    
    /// Check if path is a directory
    fn is_directory(&self, path: &str) -> Result<bool, DesktopError> {
        // This would use filesystem syscalls to check file type
        Ok(!self.is_file(path)?)
    }
    
    /// Extract metadata from a file
    fn extract_file_metadata(&self, entry: &DirectoryEntry) -> Result<FileMetadata, DesktopError> {
        let extension = self.extract_extension(&entry.name);
        let mime_type = self.determine_mime_type(&extension);
        let project_tags = self.extract_project_tags(&entry.path);
        
        // Get file stats (size, timestamps)
        let (size, created, modified) = self.get_file_stats(&entry.path)?;
        
        Ok(FileMetadata {
            path: entry.path.clone(),
            name: entry.name.clone(),
            extension,
            size,
            created,
            modified,
            mime_type,
            project_tags,
        })
    }
    
    /// Extract file extension
    fn extract_extension(&self, filename: &str) -> String {
        if let Some(dot_pos) = filename.rfind('.') {
            filename[dot_pos + 1..].to_lowercase()
        } else {
            String::new()
        }
    }
    
    /// Determine MIME type from extension
    fn determine_mime_type(&self, extension: &str) -> String {
        match extension {
            "pdf" => "application/pdf".to_string(),
            "jpg" | "jpeg" => "image/jpeg".to_string(),
            "png" => "image/png".to_string(),
            "gif" => "image/gif".to_string(),
            "svg" => "image/svg+xml".to_string(),
            "txt" => "text/plain".to_string(),
            "md" => "text/markdown".to_string(),
            "rs" => "text/x-rust".to_string(),
            "py" => "text/x-python".to_string(),
            "js" => "text/javascript".to_string(),
            "ts" => "text/typescript".to_string(),
            "html" => "text/html".to_string(),
            "css" => "text/css".to_string(),
            "json" => "application/json".to_string(),
            "xml" => "application/xml".to_string(),
            "zip" => "application/zip".to_string(),
            "tar" => "application/x-tar".to_string(),
            "gz" => "application/gzip".to_string(),
            "mp3" => "audio/mpeg".to_string(),
            "wav" => "audio/wav".to_string(),
            "mp4" => "video/mp4".to_string(),
            "avi" => "video/x-msvideo".to_string(),
            "doc" | "docx" => "application/msword".to_string(),
            "xls" | "xlsx" => "application/vnd.ms-excel".to_string(),
            "ppt" | "pptx" => "application/vnd.ms-powerpoint".to_string(),
            _ => "application/octet-stream".to_string(),
        }
    }
    
    /// Extract project tags from file path
    fn extract_project_tags(&self, path: &str) -> Vec<String> {
        let mut tags = Vec::new();
        let path_lower = path.to_lowercase();
        
        if path_lower.contains("work") || path_lower.contains("office") {
            tags.push("work".to_string());
        }
        if path_lower.contains("personal") || path_lower.contains("home") {
            tags.push("personal".to_string());
        }
        if path_lower.contains("dev") || path_lower.contains("code") || path_lower.contains("src") {
            tags.push("development".to_string());
        }
        if path_lower.contains("project") {
            tags.push("project".to_string());
        }
        
        tags
    }
    
    /// Get file statistics (size, timestamps)
    fn get_file_stats(&self, path: &str) -> Result<(u64, u64, u64), DesktopError> {
        // This would use filesystem syscalls to get actual file stats
        // For now, return simulated values based on file type
        let extension = self.extract_extension(path);
        let size = match extension.as_str() {
            "pdf" | "doc" | "docx" => 1024 * 1024, // 1MB
            "jpg" | "jpeg" | "png" => 2 * 1024 * 1024, // 2MB
            "mp4" | "avi" => 50 * 1024 * 1024, // 50MB
            "mp3" | "wav" => 5 * 1024 * 1024, // 5MB
            "zip" | "tar" | "gz" => 10 * 1024 * 1024, // 10MB
            _ => 4096, // 4KB for text files
        };
        
        // Simulate timestamps (current time - some offset)
        let current_time = 1640995200; // Base timestamp
        let created = current_time;
        let modified = current_time + 300; // 5 minutes later
        
        Ok((size, created, modified))
    }
    
    /// Check if we should scan a subdirectory
    fn should_scan_subdirectory(&self, path: &str) -> bool {
        let path_lower = path.to_lowercase();
        
        // Skip hidden directories and system directories
        if path_lower.contains("/.")
            || path_lower.contains("\\.")
            || path_lower.contains("system")
            || path_lower.contains("temp")
            || path_lower.contains("cache") {
            return false;
        }
        
        // Only scan one level deep to avoid performance issues
        let depth = path.matches('/').count() + path.matches('\\').count();
        depth < 3
    }
    
    /// Populate fallback data when filesystem access fails
    fn populate_fallback_data(&mut self) {
        self.desktop_files.extend(vec![
            FileMetadata {
                path: "/Desktop/document.pdf".to_string(),
                name: "document.pdf".to_string(),
                extension: "pdf".to_string(),
                size: 1024 * 1024,
                created: 1640995200,
                modified: 1640995200,
                mime_type: "application/pdf".to_string(),
                project_tags: vec!["work".to_string()],
            },
            FileMetadata {
                path: "/Desktop/photo.jpg".to_string(),
                name: "photo.jpg".to_string(),
                extension: "jpg".to_string(),
                size: 2 * 1024 * 1024,
                created: 1640995300,
                modified: 1640995300,
                mime_type: "image/jpeg".to_string(),
                project_tags: vec!["personal".to_string()],
            },
        ]);
    }

    /// Apply initial grouping based on current strategy
    pub fn apply_initial_grouping(&mut self) -> Result<(), DesktopError> {
        if !self.config.auto_group {
            return Ok(());
        }

        match self.config.strategy {
            GroupingStrategy::ByType => self.group_by_type()?,
            GroupingStrategy::ByDate => self.group_by_date()?,
            GroupingStrategy::ByProject => self.group_by_project()?,
            GroupingStrategy::BySize => self.group_by_size()?,
            GroupingStrategy::Custom(_) => self.apply_custom_grouping()?,
        }

        Ok(())
    }

    /// Group files by file type
    fn group_by_type(&mut self) -> Result<(), DesktopError> {
        let mut type_groups: BTreeMap<String, Vec<FileMetadata>> = BTreeMap::new();

        for file in &self.desktop_files {
            if self.should_exclude_file(file) {
                continue;
            }

            let group_key = match file.extension.as_str() {
                "pdf" | "doc" | "docx" | "txt" => "Documents",
                "jpg" | "jpeg" | "png" | "gif" | "bmp" => "Images",
                "mp4" | "avi" | "mov" | "mkv" => "Videos",
                "mp3" | "wav" | "flac" | "aac" => "Audio",
                "zip" | "rar" | "7z" | "tar" => "Archives",
                _ => "Other",
            }.to_string();

            type_groups.entry(group_key).or_insert_with(Vec::new).push(file.clone());
        }

        self.create_stacks_from_groups(type_groups)?;
        Ok(())
    }

    /// Group files by creation date
    fn group_by_date(&mut self) -> Result<(), DesktopError> {
        let mut date_groups: BTreeMap<String, Vec<FileMetadata>> = BTreeMap::new();

        for file in &self.desktop_files {
            if self.should_exclude_file(file) {
                continue;
            }

            // Simulate date grouping (in real implementation, would use proper date parsing)
            let group_key = if file.created > 1641000000 {
                "This Week"
            } else if file.created > 1640400000 {
                "Last Week"
            } else {
                "Older"
            }.to_string();

            date_groups.entry(group_key).or_insert_with(Vec::new).push(file.clone());
        }

        self.create_stacks_from_groups(date_groups)?;
        Ok(())
    }

    /// Group files by project context using AI
    fn group_by_project(&mut self) -> Result<(), DesktopError> {
        if !self.ai_enabled {
            return self.group_by_type(); // Fallback
        }

        let mut project_groups: BTreeMap<String, Vec<FileMetadata>> = BTreeMap::new();

        for file in &self.desktop_files {
            if self.should_exclude_file(file) {
                continue;
            }

            // Simulate AI-powered project detection
            let project_name = self.detect_project_context(file)?;
            project_groups.entry(project_name).or_insert_with(Vec::new).push(file.clone());
        }

        self.create_stacks_from_groups(project_groups)?;
        Ok(())
    }

    /// Group files by size ranges
    fn group_by_size(&mut self) -> Result<(), DesktopError> {
        let mut size_groups: BTreeMap<String, Vec<FileMetadata>> = BTreeMap::new();

        for file in &self.desktop_files {
            if self.should_exclude_file(file) {
                continue;
            }

            let group_key = if file.size < 1024 * 1024 {
                "Small (< 1MB)"
            } else if file.size < 10 * 1024 * 1024 {
                "Medium (1-10MB)"
            } else {
                "Large (> 10MB)"
            }.to_string();

            size_groups.entry(group_key).or_insert_with(Vec::new).push(file.clone());
        }

        self.create_stacks_from_groups(size_groups)?;
        Ok(())
    }

    /// Apply custom grouping rules
    fn apply_custom_grouping(&mut self) -> Result<(), DesktopError> {
        // Implement custom grouping logic based on user-defined rules
        self.group_by_type() // Fallback for now
    }

    /// Create file stacks from grouped files
    fn create_stacks_from_groups(&mut self, groups: BTreeMap<String, Vec<FileMetadata>>) -> Result<(), DesktopError> {
        self.stacks.clear();
        let mut position_x = 100;
        let mut position_y = 100;

        for (group_name, files) in groups {
            if files.is_empty() {
                continue;
            }

            let stack = FileStack {
                id: format!"stack_{}", group_name.replace(" ", "_").to_lowercase()),
                name: group_name,
                files,
                position: (position_x, position_y),
                expanded: false,
                auto_created: true,
                project_context: None,
            };

            self.stacks.push(stack);

            // Update position for next stack
            position_x += 120;
            if position_x > 600 {
                position_x = 100;
                position_y += 120;
            }
        }

        Ok(())
    }

    /// Detect project context for a file using AI
    fn detect_project_context(&self, file: &FileMetadata) -> Result<String, DesktopError> {
        if !self.ai_enabled {
            // Fallback to simple heuristics when AI is disabled
            return Ok(self.detect_project_heuristic(file));
        }

        // Use AI service for intelligent project detection
        let ai_request = crate::services::contracts::ai::AiRequest::AnalyzeContent {
            session_id: 1, // Use default session for now
            content: self.build_file_context(file),
            analysis_type: crate::services::contracts::ai::AnalysisType::Topics,
        };

        match crate::services::handle_ai_syscall(ai_request) {
            Ok(crate::services::contracts::ai::AiResponse::ContentAnalyzed { result }) => {
                // Extract project context from AI analysis
                let project_name = self.extract_project_from_analysis(&result, file)?;
                Ok(project_name)
            },
            _ => {
                // Fallback to heuristic if AI fails
                Ok(self.detect_project_heuristic(file))
            }
        }
    }

    /// Build context data for AI analysis
    fn build_file_context(&self, file: &FileMetadata) -> Vec<u8> {
        let context = alloc::format!(
            "File: {}\nPath: {}\nExtension: {}\nSize: {} bytes\nCreated: {}\nModified: {}\nMIME: {}\nTags: {}",
            file.name,
            file.path,
            file.extension,
            file.size,
            file.created,
            file.modified,
            file.mime_type,
            file.project_tags.join(", ")
        );
        context.into_bytes()
    }

    /// Extract project name from AI analysis result
    fn extract_project_from_analysis(&self, result: &crate::services::contracts::ai::AnalysisResult, file: &FileMetadata) -> Result<String, DesktopError> {
        // Look for project-related topics in AI analysis
        for item in &result.results {
            if item.score > 0.7 {
                match item.label.as_str() {
                    "work" | "business" | "office" => return Ok("Work Project".to_string()),
                    "personal" | "home" | "family" => return Ok("Personal".to_string()),
                    "development" | "coding" | "programming" => return Ok("Development".to_string()),
                    "design" | "creative" | "art" => return Ok("Creative Project".to_string()),
                    "research" | "academic" | "study" => return Ok("Research".to_string()),
                    _ => {
                        // Use the topic as project name if confidence is high
                        if item.score > 0.8 {
                            return Ok(alloc::format!("{}Project", item.label));
                        }
                    }
                }
            }
        }
        
        // Fallback to heuristic if no clear project detected
        Ok(self.detect_project_heuristic(file))
    }

    /// Heuristic-based project detection fallback
    fn detect_project_heuristic(&self, file: &FileMetadata) -> String {
        // Check file path for project indicators
        let path_lower = file.path.to_lowercase();
        
        if path_lower.contains("work") || path_lower.contains("office") || path_lower.contains("business") {
            "Work Project".to_string()
        } else if path_lower.contains("personal") || path_lower.contains("home") {
            "Personal".to_string()
        } else if path_lower.contains("dev") || path_lower.contains("code") || path_lower.contains("src") {
            "Development".to_string()
        } else if path_lower.contains("design") || path_lower.contains("art") || path_lower.contains("creative") {
            "Creative Project".to_string()
        } else if file.project_tags.contains(&"work".to_string()) {
            "Work Project".to_string()
        } else if file.project_tags.contains(&"personal".to_string()) {
            "Personal".to_string()
        } else {
            "Uncategorized".to_string()
        }
    }

    /// Check if file should be excluded from grouping
    fn should_exclude_file(&self, file: &FileMetadata) -> bool {
        for pattern in &self.config.exclude_patterns {
            if self.matches_pattern(&file.name, pattern) {
                return true;
            }
        }
        false
    }

    /// Simple pattern matching for exclude rules
    fn matches_pattern(&self, filename: &str, pattern: &str) -> bool {
        if pattern.starts_with('*') && pattern.len() > 1 {
            filename.ends_with(&pattern[1..])
        } else if pattern.ends_with('*') && pattern.len() > 1 {
            filename.starts_with(&pattern[..pattern.len()-1])
        } else {
            filename == pattern
        }
    }

    /// Start monitoring desktop for file changes
    fn start_file_monitoring(&mut self) -> Result<(), DesktopError> {
        // In a real implementation, this would set up file system watchers
        Ok(())
    }

    /// Save current configuration
    fn save_configuration(&self) -> Result<(), DesktopError> {
        // In a real implementation, this would persist configuration to disk
        Ok(())
    }

    /// Get AI-powered grouping suggestions
    pub fn get_grouping_suggestions(&self, files: &[FileMetadata]) -> Result<Vec<GroupingSuggestion>, DesktopError> {
        let mut suggestions = Vec::new();

        if !self.ai_enabled {
            return Ok(self.get_heuristic_suggestions(files));
        }

        // Analyze file collection with AI
        let collection_context = self.build_collection_context(files);
        let ai_request = crate::services::contracts::ai::AiRequest::AnalyzeContent {
            session_id: 1,
            content: collection_context,
            analysis_type: crate::services::contracts::ai::AnalysisType::Structure,
        };

        match crate::services::handle_ai_syscall(ai_request) {
            Ok(crate::services::contracts::ai::AiResponse::ContentAnalyzed { result }) => {
                suggestions.extend(self.extract_suggestions_from_analysis(&result, files)?);
            },
            _ => {
                // Fallback to heuristic suggestions if AI fails
                suggestions.extend(self.get_heuristic_suggestions(files));
            }
        }

        // Add additional AI-powered project detection suggestions
        if let Ok(project_suggestions) = self.get_ai_project_suggestions(files) {
            suggestions.extend(project_suggestions);
        }

        // Sort by confidence and limit results
        suggestions.sort_by(|a, b| b.confidence.partial_cmp(&a.confidence).unwrap_or(core::cmp::Ordering::Equal));
        suggestions.truncate(5); // Limit to top 5 suggestions

        Ok(suggestions)
    }

    /// Build context for analyzing file collection
    fn build_collection_context(&self, files: &[FileMetadata]) -> Vec<u8> {
        let mut context = alloc::format!("File Collection Analysis:\nTotal files: {}\n\n", files.len());
        
        for (i, file) in files.iter().enumerate().take(20) { // Limit to first 20 files for context
            context.push_str(&alloc::format!(
                "{}. {} ({}), Size: {}, Path: {}\n",
                i + 1, file.name, file.extension, file.size, file.path
            ));
        }
        
        if files.len() > 20 {
            context.push_str(&alloc::format!("... and {} more files\n", files.len() - 20));
        }
        
        context.into_bytes()
    }

    /// Extract grouping suggestions from AI analysis
    fn extract_suggestions_from_analysis(&self, result: &crate::services::contracts::ai::AnalysisResult, files: &[FileMetadata]) -> Result<Vec<GroupingSuggestion>, DesktopError> {
        let mut suggestions = Vec::new();
        
        for item in &result.results {
            if item.score > 0.6 {
                let suggestion = match item.label.as_str() {
                    "temporal_pattern" => GroupingSuggestion {
                        confidence: item.score,
                        strategy: GroupingStrategy::ByDate,
                        reason: "Files show clear temporal clustering patterns".to_string(),
                        suggested_name: "Date-based Groups".to_string(),
                    },
                    "type_clustering" => GroupingSuggestion {
                        confidence: item.score,
                        strategy: GroupingStrategy::ByType,
                        reason: "Strong file type clustering detected".to_string(),
                        suggested_name: "File Type Groups".to_string(),
                    },
                    "project_structure" => GroupingSuggestion {
                        confidence: item.score,
                        strategy: GroupingStrategy::ByProject,
                        reason: "Project-like directory structure identified".to_string(),
                        suggested_name: "Project Groups".to_string(),
                    },
                    "size_distribution" => GroupingSuggestion {
                        confidence: item.score,
                        strategy: GroupingStrategy::BySize,
                        reason: "Distinct size categories found".to_string(),
                        suggested_name: "Size-based Groups".to_string(),
                    },
                    _ => continue,
                };
                suggestions.push(suggestion);
            }
        }
        
        Ok(suggestions)
    }

    /// Get AI-powered project suggestions
    fn get_ai_project_suggestions(&self, files: &[FileMetadata]) -> Result<Vec<GroupingSuggestion>, DesktopError> {
        let mut suggestions = Vec::new();
        let mut project_counts: BTreeMap<String, usize> = BTreeMap::new();
        
        // Analyze each file for project context
        for file in files.iter().take(10) { // Limit analysis for performance
            if let Ok(project) = self.detect_project_context(file) {
                if project != "Uncategorized" {
                    *project_counts.entry(project).or_insert(0) += 1;
                }
            }
        }
        
        // Create suggestions for projects with multiple files
        for (project, count) in project_counts {
            if count >= 2 {
                let confidence = (count as f32 / files.len() as f32).min(0.95);
                suggestions.push(GroupingSuggestion {
                    confidence,
                    strategy: GroupingStrategy::ByProject,
                    reason: alloc::format!("Found {} files related to {}", count, project),
                    suggested_name: project,
                });
            }
        }
        
        Ok(suggestions)
    }

    /// Fallback heuristic suggestions when AI is disabled
    fn get_heuristic_suggestions(&self, files: &[FileMetadata]) -> Vec<GroupingSuggestion> {
        let mut suggestions = Vec::new();

        // Check for work-related files
        if files.iter().any(|f| f.project_tags.contains(&"work".to_string()) || f.path.to_lowercase().contains("work")) {
            suggestions.push(GroupingSuggestion {
                confidence: 0.8,
                strategy: GroupingStrategy::ByProject,
                reason: "Detected work-related files that could be grouped by project".to_string(),
                suggested_name: "Work Projects".to_string(),
            });
        }

        // Check for image files
        if files.iter().any(|f| matches!(f.extension.as_str(), "jpg" | "jpeg" | "png" | "gif" | "bmp" | "svg")) {
            suggestions.push(GroupingSuggestion {
                confidence: 0.85,
                strategy: GroupingStrategy::ByType,
                reason: "Multiple image files detected".to_string(),
                suggested_name: "Images".to_string(),
            });
        }

        // Check for document files
        if files.iter().any(|f| matches!(f.extension.as_str(), "pdf" | "doc" | "docx" | "txt" | "md")) {
            suggestions.push(GroupingSuggestion {
                confidence: 0.75,
                strategy: GroupingStrategy::ByType,
                reason: "Document files found".to_string(),
                suggested_name: "Documents".to_string(),
            });
        }

        // Check for code files
        if files.iter().any(|f| matches!(f.extension.as_str(), "rs" | "py" | "js" | "ts" | "cpp" | "c" | "java")) {
            suggestions.push(GroupingSuggestion {
                confidence: 0.9,
                strategy: GroupingStrategy::ByProject,
                reason: "Source code files detected".to_string(),
                suggested_name: "Development".to_string(),
            });
        }

        suggestions
    }

    /// Expand or collapse a file stack
    pub fn toggle_stack(&mut self, stack_id: &str) -> Result<(), DesktopError> {
        if let Some(stack) = self.stacks.iter_mut().find(|s| s.id == stack_id) {
            stack.expanded = !stack.expanded;
            Ok(())
        } else {
            Err(DesktopError::InvalidConfiguration)
        }
    }

    /// Update grouping strategy
    pub fn set_grouping_strategy(&mut self, strategy: GroupingStrategy) -> Result<(), DesktopError> {
        self.config.strategy = strategy;
        self.apply_initial_grouping()?;
        Ok(())
    }

    /// Get current stacks
    pub fn get_stacks(&self) -> &[FileStack] {
        &self.stacks
    }

    /// Get current configuration
    pub fn get_config(&self) -> &StackConfig {
        &self.config
    }
}

// Helper trait for pipe operations
trait Pipe<T> {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(T) -> R;
}

impl<T> Pipe<T> for T {
    fn pipe<F, R>(self, f: F) -> R
    where
        F: FnOnce(T) -> R,
    {
        f(self)
    }
}