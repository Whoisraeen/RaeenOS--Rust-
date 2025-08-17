//! File Operations - Advanced File Management
//! Handles Power Rename, bulk operations, regex support, and file transformations
//! Features batch processing, undo/redo, progress tracking, and extensible operations

use crate::services::desktop::DesktopError;
use alloc::vec::Vec;
use alloc::string::{String, ToString};
use alloc::collections::BTreeMap;
use core::time::Duration;

/// File operation types
#[derive(Debug, Clone, PartialEq)]
pub enum FileOperationType {
    Rename,
    Move,
    Copy,
    Delete,
    CreateFolder,
    ChangeAttributes,
    ChangePermissions,
    Compress,
    Extract,
    Hash,
    Duplicate,
    Custom(String),
}

/// Rename operation types
#[derive(Debug, Clone, PartialEq)]
pub enum RenameType {
    Simple,
    Regex,
    Template,
    Case,
    NumberSequence,
    DateStamp,
    Extension,
    Prefix,
    Suffix,
    Replace,
    Custom,
}

/// Case transformation types
#[derive(Debug, Clone, PartialEq)]
pub enum CaseTransform {
    Lowercase,
    Uppercase,
    TitleCase,
    CamelCase,
    PascalCase,
    SnakeCase,
    KebabCase,
    SentenceCase,
}

/// File operation item
#[derive(Debug, Clone)]
pub struct FileOperationItem {
    pub id: String,
    pub source_path: String,
    pub target_path: Option<String>,
    pub operation_type: FileOperationType,
    pub status: OperationStatus,
    pub progress: f32,
    pub error: Option<String>,
    pub size: u64,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
    pub metadata: BTreeMap<String, String>,
}

/// Operation status
#[derive(Debug, Clone, PartialEq)]
pub enum OperationStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
    Cancelled,
    Skipped,
}

/// Rename rule
#[derive(Debug, Clone)]
pub struct RenameRule {
    pub id: String,
    pub name: String,
    pub rename_type: RenameType,
    pub pattern: String,
    pub replacement: String,
    pub case_transform: Option<CaseTransform>,
    pub apply_to_extension: bool,
    pub regex_flags: Vec<RegexFlag>,
    pub template_vars: BTreeMap<String, String>,
    pub enabled: bool,
    pub order: u32,
}

/// Regex flags
#[derive(Debug, Clone, PartialEq)]
pub enum RegexFlag {
    CaseInsensitive,
    Multiline,
    DotAll,
    Global,
    Unicode,
}

/// Batch operation
#[derive(Debug, Clone)]
pub struct BatchOperation {
    pub id: String,
    pub name: String,
    pub description: String,
    pub operation_type: FileOperationType,
    pub items: Vec<FileOperationItem>,
    pub rules: Vec<RenameRule>,
    pub filters: Vec<FileFilter>,
    pub options: OperationOptions,
    pub status: BatchStatus,
    pub progress: BatchProgress,
    pub created_at: u64,
    pub started_at: Option<u64>,
    pub completed_at: Option<u64>,
}

/// Batch status
#[derive(Debug, Clone, PartialEq)]
pub enum BatchStatus {
    Preparing,
    Ready,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

/// Batch progress
#[derive(Debug, Clone)]
pub struct BatchProgress {
    pub total_items: u32,
    pub completed_items: u32,
    pub failed_items: u32,
    pub skipped_items: u32,
    pub total_bytes: u64,
    pub processed_bytes: u64,
    pub current_item: Option<String>,
    pub estimated_time_remaining: Option<Duration>,
    pub speed_bytes_per_sec: u64,
}

/// File filter
#[derive(Debug, Clone)]
pub struct FileFilter {
    pub filter_type: FilterType,
    pub pattern: String,
    pub case_sensitive: bool,
    pub include: bool, // true for include, false for exclude
}

/// Filter types
#[derive(Debug, Clone, PartialEq)]
pub enum FilterType {
    Name,
    Extension,
    Size,
    DateCreated,
    DateModified,
    Attributes,
    Content,
    Regex,
}

/// Operation options
#[derive(Debug, Clone)]
pub struct OperationOptions {
    pub overwrite_existing: bool,
    pub create_backup: bool,
    pub preserve_timestamps: bool,
    pub preserve_attributes: bool,
    pub follow_symlinks: bool,
    pub recursive: bool,
    pub verify_operations: bool,
    pub parallel_operations: u32,
    pub chunk_size: u64,
    pub retry_count: u32,
    pub retry_delay: Duration,
    pub log_operations: bool,
}

/// Undo/Redo operation
#[derive(Debug, Clone)]
pub struct UndoOperation {
    pub id: String,
    pub batch_id: String,
    pub operation_type: FileOperationType,
    pub original_path: String,
    pub new_path: Option<String>,
    pub backup_path: Option<String>,
    pub metadata: BTreeMap<String, String>,
    pub timestamp: u64,
    pub can_undo: bool,
}

/// Template variable
#[derive(Debug, Clone)]
pub struct TemplateVariable {
    pub name: String,
    pub description: String,
    pub variable_type: VariableType,
    pub format: Option<String>,
    pub example: String,
}

/// Variable types
#[derive(Debug, Clone, PartialEq)]
pub enum VariableType {
    FileName,
    FileExtension,
    FileSize,
    DateCreated,
    DateModified,
    Counter,
    Random,
    Custom,
}

/// File Operations configuration
#[derive(Debug, Clone)]
pub struct FileOpsConfig {
    pub enable_undo: bool,
    pub max_undo_operations: u32,
    pub auto_backup: bool,
    pub backup_directory: String,
    pub default_parallel_ops: u32,
    pub max_parallel_ops: u32,
    pub chunk_size: u64,
    pub verify_operations: bool,
    pub log_operations: bool,
    pub log_directory: String,
    pub show_progress: bool,
    pub confirm_destructive: bool,
    pub template_variables: Vec<TemplateVariable>,
}

/// File Operations main service
pub struct FileOperations {
    config: FileOpsConfig,
    active_operations: BTreeMap<String, BatchOperation>,
    undo_stack: Vec<UndoOperation>,
    redo_stack: Vec<UndoOperation>,
    operation_counter: u64,
    rename_presets: BTreeMap<String, Vec<RenameRule>>,
    operation_history: Vec<BatchOperation>,
}

impl FileOperations {
    /// Create a new FileOperations instance
    pub fn new() -> Result<Self, DesktopError> {
        let mut file_ops = FileOperations {
            config: FileOpsConfig {
                enable_undo: true,
                max_undo_operations: 100,
                auto_backup: true,
                backup_directory: "/tmp/rae_backups".to_string(),
                default_parallel_ops: 4,
                max_parallel_ops: 16,
                chunk_size: 64 * 1024, // 64KB
                verify_operations: true,
                log_operations: true,
                log_directory: "/var/log/rae_file_ops".to_string(),
                show_progress: true,
                confirm_destructive: true,
                template_variables: Vec::new(),
            },
            active_operations: BTreeMap::new(),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            operation_counter: 1,
            rename_presets: BTreeMap::new(),
            operation_history: Vec::new(),
        };
        
        file_ops.setup_default_template_variables()?;
        file_ops.setup_default_rename_presets()?;
        Ok(file_ops)
    }

    /// Start the file operations service
    pub fn start(&mut self) -> Result<(), DesktopError> {
        self.load_configuration()?;
        self.load_presets()?;
        self.create_directories()?;
        Ok(())
    }

    /// Stop the file operations service
    pub fn stop(&mut self) -> Result<(), DesktopError> {
        self.save_configuration()?;
        self.save_presets()?;
        self.cleanup_temp_files()?;
        Ok(())
    }

    /// Create a new batch rename operation
    pub fn create_batch_rename(&mut self, name: &str, files: Vec<String>, rules: Vec<RenameRule>) -> Result<String, DesktopError> {
        let batch_id = format!("batch_{}", self.operation_counter);
        self.operation_counter += 1;
        
        let mut items = Vec::new();
        for (index, file_path) in files.iter().enumerate() {
            let item_id = format!("{}_{}", batch_id, index);
            items.push(FileOperationItem {
                id: item_id,
                source_path: file_path.clone(),
                target_path: None, // Will be calculated when applying rules
                operation_type: FileOperationType::Rename,
                status: OperationStatus::Pending,
                progress: 0.0,
                error: None,
                size: self.get_file_size(file_path)?,
                created_at: self.get_current_time(),
                started_at: None,
                completed_at: None,
                metadata: BTreeMap::new(),
            });
        }
        
        let batch = BatchOperation {
            id: batch_id.clone(),
            name: name.to_string(),
            description: format!("Batch rename {} files", files.len()),
            operation_type: FileOperationType::Rename,
            items,
            rules,
            filters: Vec::new(),
            options: OperationOptions {
                overwrite_existing: false,
                create_backup: self.config.auto_backup,
                preserve_timestamps: true,
                preserve_attributes: true,
                follow_symlinks: false,
                recursive: false,
                verify_operations: self.config.verify_operations,
                parallel_operations: self.config.default_parallel_ops,
                chunk_size: self.config.chunk_size,
                retry_count: 3,
                retry_delay: Duration::from_millis(500),
                log_operations: self.config.log_operations,
            },
            status: BatchStatus::Preparing,
            progress: BatchProgress {
                total_items: files.len() as u32,
                completed_items: 0,
                failed_items: 0,
                skipped_items: 0,
                total_bytes: items.iter().map(|i| i.size).sum(),
                processed_bytes: 0,
                current_item: None,
                estimated_time_remaining: None,
                speed_bytes_per_sec: 0,
            },
            created_at: self.get_current_time(),
            started_at: None,
            completed_at: None,
        };
        
        self.active_operations.insert(batch_id.clone(), batch);
        Ok(batch_id)
    }

    /// Preview rename results
    pub fn preview_rename(&mut self, batch_id: &str) -> Result<Vec<(String, String)>, DesktopError> {
        if let Some(batch) = self.active_operations.get_mut(batch_id) {
            let mut previews = Vec::new();
            
            for item in &mut batch.items {
                let new_name = self.apply_rename_rules(&item.source_path, &batch.rules)?;
                item.target_path = Some(new_name.clone());
                previews.push((item.source_path.clone(), new_name));
            }
            
            batch.status = BatchStatus::Ready;
            Ok(previews)
        } else {
            Err(DesktopError::OperationNotFound)
        }
    }

    /// Execute batch operation
    pub fn execute_batch(&mut self, batch_id: &str) -> Result<(), DesktopError> {
        if let Some(batch) = self.active_operations.get_mut(batch_id) {
            if batch.status != BatchStatus::Ready {
                return Err(DesktopError::InvalidOperation);
            }
            
            batch.status = BatchStatus::Running;
            batch.started_at = Some(self.get_current_time());
            
            // Execute operations
            for item in &mut batch.items {
                if let Some(target_path) = &item.target_path {
                    item.status = OperationStatus::InProgress;
                    item.started_at = Some(self.get_current_time());
                    
                    match self.execute_rename(&item.source_path, target_path, &batch.options) {
                        Ok(_) => {
                            item.status = OperationStatus::Completed;
                            item.progress = 100.0;
                            item.completed_at = Some(self.get_current_time());
                            batch.progress.completed_items += 1;
                            batch.progress.processed_bytes += item.size;
                            
                            // Add to undo stack
                            if self.config.enable_undo {
                                self.add_undo_operation(batch_id, item)?;
                            }
                        },
                        Err(e) => {
                            item.status = OperationStatus::Failed;
                            item.error = Some(format!("{:?}", e));
                            batch.progress.failed_items += 1;
                        }
                    }
                }
            }
            
            batch.status = if batch.progress.failed_items == 0 {
                BatchStatus::Completed
            } else {
                BatchStatus::Failed
            };
            batch.completed_at = Some(self.get_current_time());
            
            // Move to history
            let completed_batch = batch.clone();
            self.operation_history.push(completed_batch);
            
            Ok(())
        } else {
            Err(DesktopError::OperationNotFound)
        }
    }

    /// Apply rename rules to a file path
    fn apply_rename_rules(&self, file_path: &str, rules: &[RenameRule]) -> Result<String, DesktopError> {
        let file_name = self.extract_filename(file_path)?;
        let mut result = file_name;
        
        for rule in rules {
            if !rule.enabled {
                continue;
            }
            
            result = match rule.rename_type {
                RenameType::Simple => rule.replacement.clone(),
                RenameType::Regex => self.apply_regex_rule(&result, rule)?,
                RenameType::Template => self.apply_template_rule(&result, rule, file_path)?,
                RenameType::Case => self.apply_case_transform(&result, rule)?,
                RenameType::NumberSequence => self.apply_number_sequence(&result, rule)?,
                RenameType::DateStamp => self.apply_date_stamp(&result, rule, file_path)?,
                RenameType::Extension => self.apply_extension_rule(&result, rule)?,
                RenameType::Prefix => format!("{}{}", rule.replacement, result),
                RenameType::Suffix => {
                    let (name, ext) = self.split_name_extension(&result);
                    if ext.is_empty() {
                        format!("{}{}", name, rule.replacement)
                    } else {
                        format!("{}{}.{}", name, rule.replacement, ext)
                    }
                },
                RenameType::Replace => result.replace(&rule.pattern, &rule.replacement),
                RenameType::Custom => self.apply_custom_rule(&result, rule)?,
            };
        }
        
        // Reconstruct full path
        let dir_path = self.extract_directory(file_path)?;
        Ok(format!("{}/{}", dir_path, result))
    }

    /// Apply regex rule
    fn apply_regex_rule(&self, input: &str, rule: &RenameRule) -> Result<String, DesktopError> {
        // Simplified regex implementation
        // In real implementation, would use proper regex library
        if rule.regex_flags.contains(&RegexFlag::CaseInsensitive) {
            Ok(input.to_lowercase().replace(&rule.pattern.to_lowercase(), &rule.replacement))
        } else {
            Ok(input.replace(&rule.pattern, &rule.replacement))
        }
    }

    /// Apply template rule
    fn apply_template_rule(&self, input: &str, rule: &RenameRule, file_path: &str) -> Result<String, DesktopError> {
        let mut result = rule.replacement.clone();
        
        // Replace template variables
        for var in &self.config.template_variables {
            let placeholder = format!("{{{}}}", var.name);
            if result.contains(&placeholder) {
                let value = self.get_template_variable_value(var, input, file_path)?;
                result = result.replace(&placeholder, &value);
            }
        }
        
        // Replace custom variables from rule
        for (key, value) in &rule.template_vars {
            let placeholder = format!("{{{}}}", key);
            result = result.replace(&placeholder, value);
        }
        
        Ok(result)
    }

    /// Apply case transformation
    fn apply_case_transform(&self, input: &str, rule: &RenameRule) -> Result<String, DesktopError> {
        if let Some(case_transform) = &rule.case_transform {
            let (name, ext) = self.split_name_extension(input);
            let transformed_name = match case_transform {
                CaseTransform::Lowercase => name.to_lowercase(),
                CaseTransform::Uppercase => name.to_uppercase(),
                CaseTransform::TitleCase => self.to_title_case(&name),
                CaseTransform::CamelCase => self.to_camel_case(&name),
                CaseTransform::PascalCase => self.to_pascal_case(&name),
                CaseTransform::SnakeCase => self.to_snake_case(&name),
                CaseTransform::KebabCase => self.to_kebab_case(&name),
                CaseTransform::SentenceCase => self.to_sentence_case(&name),
            };
            
            let transformed_ext = if rule.apply_to_extension {
                match case_transform {
                    CaseTransform::Lowercase => ext.to_lowercase(),
                    CaseTransform::Uppercase => ext.to_uppercase(),
                    _ => ext,
                }
            } else {
                ext
            };
            
            if transformed_ext.is_empty() {
                Ok(transformed_name)
            } else {
                Ok(format!("{}.{}", transformed_name, transformed_ext))
            }
        } else {
            Ok(input.to_string())
        }
    }

    /// Apply number sequence
    fn apply_number_sequence(&self, input: &str, rule: &RenameRule) -> Result<String, DesktopError> {
        // Extract counter from rule metadata or use default
        let counter = rule.template_vars.get("counter")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(1);
        
        let (name, ext) = self.split_name_extension(input);
        let numbered_name = if rule.replacement.contains("{}") {
            rule.replacement.replace("{}", &counter.to_string())
        } else {
            format!("{} ({})", name, counter)
        };
        
        if ext.is_empty() {
            Ok(numbered_name)
        } else {
            Ok(format!("{}.{}", numbered_name, ext))
        }
    }

    /// Apply date stamp
    fn apply_date_stamp(&self, input: &str, rule: &RenameRule, file_path: &str) -> Result<String, DesktopError> {
        let date_format = rule.template_vars.get("format").unwrap_or(&"YYYY-MM-DD".to_string());
        let date_str = self.format_file_date(file_path, date_format)?;
        
        let (name, ext) = self.split_name_extension(input);
        let dated_name = if rule.replacement.contains("{}") {
            rule.replacement.replace("{}", &date_str)
        } else {
            format!("{} {}", date_str, name)
        };
        
        if ext.is_empty() {
            Ok(dated_name)
        } else {
            Ok(format!("{}.{}", dated_name, ext))
        }
    }

    /// Apply extension rule
    fn apply_extension_rule(&self, input: &str, rule: &RenameRule) -> Result<String, DesktopError> {
        let (name, _ext) = self.split_name_extension(input);
        let new_ext = rule.replacement.trim_start_matches('.');
        
        if new_ext.is_empty() {
            Ok(name)
        } else {
            Ok(format!("{}.{}", name, new_ext))
        }
    }

    /// Apply custom rule
    fn apply_custom_rule(&self, input: &str, _rule: &RenameRule) -> Result<String, DesktopError> {
        // Placeholder for custom rule implementation
        Ok(input.to_string())
    }

    /// Execute rename operation
    fn execute_rename(&self, source: &str, target: &str, options: &OperationOptions) -> Result<(), DesktopError> {
        // Check if source exists
        if !self.file_exists(source) {
            return Err(DesktopError::FileNotFound);
        }
        
        // Check if target exists
        if self.file_exists(target) && !options.overwrite_existing {
            return Err(DesktopError::FileExists);
        }
        
        // Create backup if enabled
        if options.create_backup {
            self.create_backup(source)?;
        }
        
        // Perform actual rename using filesystem syscalls
        match self.handle_filesystem_syscall("rename", source, Some(target)) {
            Ok(_) => {
                // Verify operation if enabled
                if options.verify_operations {
                    self.verify_rename(source, target)?;
                }
                Ok(())
            }
            Err(_) => Err(DesktopError::IoError)
        }
    }

    /// Undo last operation
    pub fn undo(&mut self) -> Result<(), DesktopError> {
        if let Some(undo_op) = self.undo_stack.pop() {
            if undo_op.can_undo {
                // Perform undo operation
                match undo_op.operation_type {
                    FileOperationType::Rename => {
                        if let Some(new_path) = &undo_op.new_path {
                            self.execute_rename(new_path, &undo_op.original_path, &OperationOptions {
                                overwrite_existing: true,
                                create_backup: false,
                                preserve_timestamps: true,
                                preserve_attributes: true,
                                follow_symlinks: false,
                                recursive: false,
                                verify_operations: false,
                                parallel_operations: 1,
                                chunk_size: self.config.chunk_size,
                                retry_count: 0,
                                retry_delay: Duration::from_millis(0),
                                log_operations: false,
                            })?;
                        }
                    },
                    _ => {
                        // Handle other operation types
                    }
                }
                
                // Move to redo stack
                self.redo_stack.push(undo_op);
            }
        }
        
        Ok(())
    }

    /// Redo last undone operation
    pub fn redo(&mut self) -> Result<(), DesktopError> {
        if let Some(redo_op) = self.redo_stack.pop() {
            // Perform redo operation
            match redo_op.operation_type {
                FileOperationType::Rename => {
                    if let Some(new_path) = &redo_op.new_path {
                        self.execute_rename(&redo_op.original_path, new_path, &OperationOptions {
                            overwrite_existing: true,
                            create_backup: false,
                            preserve_timestamps: true,
                            preserve_attributes: true,
                            follow_symlinks: false,
                            recursive: false,
                            verify_operations: false,
                            parallel_operations: 1,
                            chunk_size: self.config.chunk_size,
                            retry_count: 0,
                            retry_delay: Duration::from_millis(0),
                            log_operations: false,
                        })?;
                    }
                },
                _ => {
                    // Handle other operation types
                }
            }
            
            // Move back to undo stack
            self.undo_stack.push(redo_op);
        }
        
        Ok(())
    }

    /// Helper functions
    fn setup_default_template_variables(&mut self) -> Result<(), DesktopError> {
        self.config.template_variables = vec![
            TemplateVariable {
                name: "filename".to_string(),
                description: "Original filename without extension".to_string(),
                variable_type: VariableType::FileName,
                format: None,
                example: "document".to_string(),
            },
            TemplateVariable {
                name: "extension".to_string(),
                description: "File extension".to_string(),
                variable_type: VariableType::FileExtension,
                format: None,
                example: "txt".to_string(),
            },
            TemplateVariable {
                name: "size".to_string(),
                description: "File size in bytes".to_string(),
                variable_type: VariableType::FileSize,
                format: Some("bytes|kb|mb|gb".to_string()),
                example: "1024".to_string(),
            },
            TemplateVariable {
                name: "date_created".to_string(),
                description: "File creation date".to_string(),
                variable_type: VariableType::DateCreated,
                format: Some("YYYY-MM-DD|DD-MM-YYYY|MM/DD/YYYY".to_string()),
                example: "2024-01-15".to_string(),
            },
            TemplateVariable {
                name: "date_modified".to_string(),
                description: "File modification date".to_string(),
                variable_type: VariableType::DateModified,
                format: Some("YYYY-MM-DD|DD-MM-YYYY|MM/DD/YYYY".to_string()),
                example: "2024-01-15".to_string(),
            },
            TemplateVariable {
                name: "counter".to_string(),
                description: "Sequential counter".to_string(),
                variable_type: VariableType::Counter,
                format: Some("start:1|step:1|pad:3".to_string()),
                example: "001".to_string(),
            },
        ];
        Ok(())
    }
    
    fn setup_default_rename_presets(&mut self) -> Result<(), DesktopError> {
        // Add common rename presets
        let mut presets = BTreeMap::new();
        
        // Lowercase preset
        presets.insert("lowercase".to_string(), vec![
            RenameRule {
                id: "lowercase_rule".to_string(),
                name: "Convert to lowercase".to_string(),
                rename_type: RenameType::Case,
                pattern: String::new(),
                replacement: String::new(),
                case_transform: Some(CaseTransform::Lowercase),
                apply_to_extension: false,
                regex_flags: Vec::new(),
                template_vars: BTreeMap::new(),
                enabled: true,
                order: 1,
            },
        ]);
        
        // Number sequence preset
        presets.insert("numbered".to_string(), vec![
            RenameRule {
                id: "number_rule".to_string(),
                name: "Add number sequence".to_string(),
                rename_type: RenameType::NumberSequence,
                pattern: String::new(),
                replacement: "{filename} ({counter})".to_string(),
                case_transform: None,
                apply_to_extension: false,
                regex_flags: Vec::new(),
                template_vars: BTreeMap::new(),
                enabled: true,
                order: 1,
            },
        ]);
        
        self.rename_presets = presets;
        Ok(())
    }
    
    fn get_template_variable_value(&self, var: &TemplateVariable, input: &str, file_path: &str) -> Result<String, DesktopError> {
        match var.variable_type {
            VariableType::FileName => {
                let (name, _) = self.split_name_extension(input);
                Ok(name)
            },
            VariableType::FileExtension => {
                let (_, ext) = self.split_name_extension(input);
                Ok(ext)
            },
            VariableType::FileSize => {
                let size = self.get_file_size(file_path)?;
                Ok(size.to_string())
            },
            VariableType::DateCreated => {
                self.format_file_date(file_path, "YYYY-MM-DD")
            },
            VariableType::DateModified => {
                self.format_file_date(file_path, "YYYY-MM-DD")
            },
            VariableType::Counter => {
                Ok("1".to_string()) // Simplified
            },
            _ => Ok(String::new()),
        }
    }
    
    // String transformation helpers
    fn to_title_case(&self, s: &str) -> String {
        s.split_whitespace()
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
    
    fn to_camel_case(&self, s: &str) -> String {
        let words: Vec<&str> = s.split(|c: char| !c.is_alphanumeric()).collect();
        if words.is_empty() {
            return String::new();
        }
        
        let mut result = words[0].to_lowercase();
        for word in &words[1..] {
            if !word.is_empty() {
                let mut chars = word.chars();
                if let Some(first) = chars.next() {
                    result.push_str(&first.to_uppercase().collect::<String>());
                    result.push_str(&chars.as_str().to_lowercase());
                }
            }
        }
        result
    }
    
    fn to_pascal_case(&self, s: &str) -> String {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
                }
            })
            .collect()
    }
    
    fn to_snake_case(&self, s: &str) -> String {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .map(|word| word.to_lowercase())
            .collect::<Vec<_>>()
            .join("_")
    }
    
    fn to_kebab_case(&self, s: &str) -> String {
        s.split(|c: char| !c.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .map(|word| word.to_lowercase())
            .collect::<Vec<_>>()
            .join("-")
    }
    
    fn to_sentence_case(&self, s: &str) -> String {
        if s.is_empty() {
            return String::new();
        }
        
        let mut chars = s.chars();
        match chars.next() {
            None => String::new(),
            Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
        }
    }
    
    // File system helpers (simplified implementations)
    fn extract_filename(&self, path: &str) -> Result<String, DesktopError> {
        Ok(path.split('/').last().unwrap_or(path).to_string())
    }
    
    fn extract_directory(&self, path: &str) -> Result<String, DesktopError> {
        let parts: Vec<&str> = path.split('/').collect();
        if parts.len() > 1 {
            Ok(parts[..parts.len()-1].join("/"))
        } else {
            Ok(".".to_string())
        }
    }
    
    fn split_name_extension(&self, filename: &str) -> (String, String) {
        if let Some(dot_pos) = filename.rfind('.') {
            if dot_pos > 0 {
                return (filename[..dot_pos].to_string(), filename[dot_pos+1..].to_string());
            }
        }
        (filename.to_string(), String::new())
    }
    
    fn get_file_size(&self, path: &str) -> Result<u64, DesktopError> {
        match self.handle_filesystem_syscall("stat", path, None) {
            Ok(metadata) => {
                // Parse metadata to extract file size
                // In a real implementation, this would parse the stat structure
                // For now, simulate based on file extension
                let extension = path.split('.').last().unwrap_or("").to_lowercase();
                let size = match extension.as_str() {
                    "jpg" | "jpeg" | "png" | "gif" => 2 * 1024 * 1024, // 2MB for images
                    "mp4" | "avi" | "mkv" => 100 * 1024 * 1024, // 100MB for videos
                    "mp3" | "wav" | "flac" => 5 * 1024 * 1024, // 5MB for audio
                    "pdf" | "doc" | "docx" => 1024 * 1024, // 1MB for documents
                    "zip" | "tar" | "7z" => 10 * 1024 * 1024, // 10MB for archives
                    _ => 4096, // 4KB for text files
                };
                Ok(size)
            }
            Err(_) => Err(DesktopError::FileNotFound)
        }
    }
    
    fn file_exists(&self, path: &str) -> bool {
        self.handle_filesystem_syscall("stat", path, None).is_ok()
    }
    
    fn create_backup(&self, source: &str) -> Result<(), DesktopError> {
        let backup_path = format!("{}.backup", source);
        match self.handle_filesystem_syscall("copy", source, Some(&backup_path)) {
            Ok(_) => Ok(()),
            Err(_) => Err(DesktopError::IoError)
        }
    }
    
    fn verify_rename(&self, source: &str, target: &str) -> Result<(), DesktopError> {
        // Verify that source no longer exists and target exists
        if self.file_exists(source) {
            return Err(DesktopError::OperationFailed);
        }
        if !self.file_exists(target) {
            return Err(DesktopError::OperationFailed);
        }
        Ok(())
    }
    
    /// Handle filesystem syscalls for actual file operations
    fn handle_filesystem_syscall(&self, operation: &str, source: &str, target: Option<&str>) -> Result<u64, DesktopError> {
        match operation {
            "stat" => {
                // Use sys_stat syscall to get file metadata
                let result = unsafe {
                    crate::syscall::handle_syscall(
                        15, // sys_stat
                        source.as_ptr() as u64,
                        0, // statbuf will be allocated internally
                        0, 0, 0, 0
                    )
                };
                if result.success {
                    Ok(result.value as u64)
                } else {
                    Err(DesktopError::FileNotFound)
                }
            }
            "rename" => {
                if let Some(target_path) = target {
                    // Use filesystem rename function
                    match crate::filesystem::VFS.write().rename(source, target_path) {
                        Ok(_) => Ok(0),
                        Err(_) => Err(DesktopError::IoError)
                    }
                } else {
                    Err(DesktopError::InvalidArgument)
                }
            }
            "copy" => {
                if let Some(target_path) = target {
                    self.copy_file_internal(source, target_path)
                } else {
                    Err(DesktopError::InvalidArgument)
                }
            }
            "move" => {
                if let Some(target_path) = target {
                    // Move is rename + verify
                    match crate::filesystem::VFS.write().rename(source, target_path) {
                        Ok(_) => Ok(0),
                        Err(_) => Err(DesktopError::IoError)
                    }
                } else {
                    Err(DesktopError::InvalidArgument)
                }
            }
            "delete" => {
                match crate::filesystem::VFS.write().remove(source) {
                    Ok(_) => Ok(0),
                    Err(_) => Err(DesktopError::IoError)
                }
            }
            _ => Err(DesktopError::InvalidArgument)
        }
    }
    
    /// Internal file copy implementation
    fn copy_file_internal(&self, source: &str, target: &str) -> Result<u64, DesktopError> {
        // Open source file for reading
        let source_fd = match crate::filesystem::open(source, 0) {
            Ok(fd) => fd,
            Err(_) => return Err(DesktopError::FileNotFound)
        };
        
        // Create target file
        if let Err(_) = crate::filesystem::create_file(target) {
            let _ = crate::filesystem::close(source_fd);
            return Err(DesktopError::IoError);
        }
        
        // Open target file for writing
        let target_fd = match crate::filesystem::open(target, 1) {
            Ok(fd) => fd,
            Err(_) => {
                let _ = crate::filesystem::close(source_fd);
                return Err(DesktopError::IoError);
            }
        };
        
        // Copy data in chunks
        let mut buffer = [0u8; 4096];
        let mut total_copied = 0u64;
        
        loop {
            let bytes_read = match crate::filesystem::VFS.write().read(source_fd, &mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => n,
                Err(_) => {
                    let _ = crate::filesystem::close(source_fd);
                    let _ = crate::filesystem::close(target_fd);
                    return Err(DesktopError::IoError);
                }
            };
            
            match crate::filesystem::VFS.write().write(target_fd, &buffer[..bytes_read]) {
                Ok(n) if n == bytes_read => total_copied += n as u64,
                _ => {
                    let _ = crate::filesystem::close(source_fd);
                    let _ = crate::filesystem::close(target_fd);
                    return Err(DesktopError::IoError);
                }
            }
        }
        
        // Close files
        let _ = crate::filesystem::close(source_fd);
        let _ = crate::filesystem::close(target_fd);
        
        Ok(total_copied)
    }
    
    fn format_file_date(&self, _path: &str, _format: &str) -> Result<String, DesktopError> {
        // Simplified implementation
        Ok("2024-01-15".to_string())
    }
    
    fn add_undo_operation(&mut self, batch_id: &str, item: &FileOperationItem) -> Result<(), DesktopError> {
        if let Some(target_path) = &item.target_path {
            let undo_op = UndoOperation {
                id: format!("undo_{}", item.id),
                batch_id: batch_id.to_string(),
                operation_type: item.operation_type.clone(),
                original_path: item.source_path.clone(),
                new_path: Some(target_path.clone()),
                backup_path: None,
                metadata: item.metadata.clone(),
                timestamp: self.get_current_time(),
                can_undo: true,
            };
            
            self.undo_stack.push(undo_op);
            
            // Limit undo stack size
            if self.undo_stack.len() > self.config.max_undo_operations as usize {
                self.undo_stack.remove(0);
            }
            
            // Clear redo stack when new operation is added
            self.redo_stack.clear();
        }
        
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
    
    fn load_presets(&mut self) -> Result<(), DesktopError> {
        // In real implementation, would load presets from disk
        Ok(())
    }
    
    fn save_presets(&self) -> Result<(), DesktopError> {
        // In real implementation, would save presets to disk
        Ok(())
    }
    
    fn create_directories(&self) -> Result<(), DesktopError> {
        // In real implementation, would create necessary directories
        Ok(())
    }
    
    fn cleanup_temp_files(&self) -> Result<(), DesktopError> {
        // In real implementation, would clean up temporary files
        Ok(())
    }

    /// Get active operations
    pub fn get_active_operations(&self) -> &BTreeMap<String, BatchOperation> {
        &self.active_operations
    }
    
    /// Get operation history
    pub fn get_operation_history(&self) -> &Vec<BatchOperation> {
        &self.operation_history
    }
    
    /// Get rename presets
    pub fn get_rename_presets(&self) -> &BTreeMap<String, Vec<RenameRule>> {
        &self.rename_presets
    }
    
    /// Get configuration
    pub fn get_config(&self) -> &FileOpsConfig {
        &self.config
    }
    
    /// Update configuration
    pub fn update_config(&mut self, config: FileOpsConfig) -> Result<(), DesktopError> {
        self.config = config;
        Ok(())
    }
    
    /// Can undo
    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }
    
    /// Can redo
    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }
    
    /// Execute copy operation
    pub fn execute_copy(&mut self, batch_id: &str) -> Result<(), DesktopError> {
        let batch = self.active_operations.get_mut(batch_id)
            .ok_or(DesktopError::OperationNotFound)?;
        
        batch.status = BatchStatus::Running;
        batch.started_at = Some(self.get_current_time());
        
        for item in &mut batch.items {
            if item.status != OperationStatus::Pending {
                continue;
            }
            
            item.status = OperationStatus::InProgress;
            item.started_at = Some(self.get_current_time());
            
            // Perform actual copy operation
            if let Some(ref target_path) = item.target_path {
                match self.handle_filesystem_syscall("copy", &item.source_path, Some(target_path)) {
                    Ok(bytes_copied) => {
                        item.status = OperationStatus::Completed;
                        item.completed_at = Some(self.get_current_time());
                        item.progress = 100.0;
                        item.size = bytes_copied;
                        batch.progress.completed_items += 1;
                        batch.progress.processed_bytes += bytes_copied;
                    }
                    Err(e) => {
                        item.status = OperationStatus::Failed;
                        item.error = Some(format!("Copy failed: {:?}", e));
                        item.completed_at = Some(self.get_current_time());
                        batch.progress.failed_items += 1;
                    }
                }
            } else {
                item.status = OperationStatus::Failed;
                item.error = Some("No target path specified".to_string());
                item.completed_at = Some(self.get_current_time());
                batch.progress.failed_items += 1;
            }
        }
        
        // Update batch status based on item results
        let all_completed = batch.items.iter().all(|item| 
            item.status == OperationStatus::Completed || item.status == OperationStatus::Failed
        );
        
        if all_completed {
            let any_failed = batch.items.iter().any(|item| item.status == OperationStatus::Failed);
            batch.status = if any_failed { BatchStatus::Failed } else { BatchStatus::Completed };
            batch.completed_at = Some(self.get_current_time());
        }
        
        Ok(())
    }
    
    /// Execute move operation
    pub fn execute_move(&mut self, batch_id: &str) -> Result<(), DesktopError> {
        let batch = self.active_operations.get_mut(batch_id)
            .ok_or(DesktopError::OperationNotFound)?;
        
        batch.status = BatchStatus::Running;
        batch.started_at = Some(self.get_current_time());
        
        for item in &mut batch.items {
            if item.status != OperationStatus::Pending {
                continue;
            }
            
            item.status = OperationStatus::InProgress;
            item.started_at = Some(self.get_current_time());
            
            // Perform actual move operation
            if let Some(ref target_path) = item.target_path {
                match self.handle_filesystem_syscall("move", &item.source_path, Some(target_path)) {
                    Ok(_) => {
                        item.status = OperationStatus::Completed;
                        item.completed_at = Some(self.get_current_time());
                        item.progress = 100.0;
                        batch.progress.completed_items += 1;
                        batch.progress.processed_bytes += item.size;
                        
                        // Add to undo stack
                        if self.config.enable_undo {
                            let _ = self.add_undo_operation(batch_id, item);
                        }
                    }
                    Err(e) => {
                        item.status = OperationStatus::Failed;
                        item.error = Some(format!("Move failed: {:?}", e));
                        item.completed_at = Some(self.get_current_time());
                        batch.progress.failed_items += 1;
                    }
                }
            } else {
                item.status = OperationStatus::Failed;
                item.error = Some("No target path specified".to_string());
                item.completed_at = Some(self.get_current_time());
                batch.progress.failed_items += 1;
            }
        }
        
        // Update batch status based on item results
        let all_completed = batch.items.iter().all(|item| 
            item.status == OperationStatus::Completed || item.status == OperationStatus::Failed
        );
        
        if all_completed {
            let any_failed = batch.items.iter().any(|item| item.status == OperationStatus::Failed);
            batch.status = if any_failed { BatchStatus::Failed } else { BatchStatus::Completed };
            batch.completed_at = Some(self.get_current_time());
        }
        
        Ok(())
    }
    
    /// Execute delete operation
    pub fn execute_delete(&mut self, batch_id: &str) -> Result<(), DesktopError> {
        let batch = self.active_operations.get_mut(batch_id)
            .ok_or(DesktopError::OperationNotFound)?;
        
        batch.status = BatchStatus::Running;
        batch.started_at = Some(self.get_current_time());
        
        for item in &mut batch.items {
            if item.status != OperationStatus::Pending {
                continue;
            }
            
            item.status = OperationStatus::InProgress;
            item.started_at = Some(self.get_current_time());
            
            // Create backup before deletion if enabled
            if batch.options.create_backup {
                if let Err(e) = self.create_backup(&item.source_path) {
                    item.status = OperationStatus::Failed;
                    item.error = Some(format!("Backup failed: {:?}", e));
                    item.completed_at = Some(self.get_current_time());
                    batch.progress.failed_items += 1;
                    continue;
                }
            }
            
            // Perform actual delete operation
            match self.handle_filesystem_syscall("delete", &item.source_path, None) {
                Ok(_) => {
                    item.status = OperationStatus::Completed;
                    item.completed_at = Some(self.get_current_time());
                    item.progress = 100.0;
                    batch.progress.completed_items += 1;
                    batch.progress.processed_bytes += item.size;
                    
                    // Add to undo stack
                    if self.config.enable_undo {
                        let _ = self.add_undo_operation(batch_id, item);
                    }
                }
                Err(e) => {
                    item.status = OperationStatus::Failed;
                    item.error = Some(format!("Delete failed: {:?}", e));
                    item.completed_at = Some(self.get_current_time());
                    batch.progress.failed_items += 1;
                }
            }
        }
        
        // Update batch status based on item results
        let all_completed = batch.items.iter().all(|item| 
            item.status == OperationStatus::Completed || item.status == OperationStatus::Failed
        );
        
        if all_completed {
            let any_failed = batch.items.iter().any(|item| item.status == OperationStatus::Failed);
            batch.status = if any_failed { BatchStatus::Failed } else { BatchStatus::Completed };
            batch.completed_at = Some(self.get_current_time());
        }
        
        Ok(())
    }
}