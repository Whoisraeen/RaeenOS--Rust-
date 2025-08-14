//! Rae Assistant - AI Integration System for RaeenOS
//! Provides intelligent assistance, automation, and natural language interaction

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::format;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::process::ProcessId;
use crate::filesystem::FileHandle;
use crate::graphics::WindowId;

/// AI model configuration
#[derive(Debug, Clone)]
pub struct AiModelConfig {
    pub model_name: String,
    pub model_version: String,
    pub model_type: ModelType,
    pub context_window: u32,
    pub max_tokens: u32,
    pub temperature: f32,
    pub top_p: f32,
    pub frequency_penalty: f32,
    pub presence_penalty: f32,
    pub stop_sequences: Vec<String>,
    pub system_prompt: String,
    pub capabilities: Vec<AiCapability>,
    pub privacy_level: PrivacyLevel,
    pub local_processing: bool,
    pub model_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ModelType {
    TextGeneration,
    CodeGeneration,
    ImageGeneration,
    ImageAnalysis,
    AudioGeneration,
    AudioTranscription,
    Translation,
    Embedding,
    Classification,
    Multimodal,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum AiCapability {
    TextGeneration,
    CodeGeneration,
    CodeExecution,
    ImageGeneration,
    ImageAnalysis,
    AudioGeneration,
    AudioTranscription,
    VoiceSynthesis,
    Translation,
    Summarization,
    QuestionAnswering,
    TaskAutomation,
    FileManagement,
    SystemControl,
    WebSearch,
    EmailManagement,
    CalendarManagement,
    DocumentEditing,
    DataAnalysis,
    MathSolving,
    CreativeWriting,
    ProgrammingHelp,
    Debugging,
    Learning,
    PersonalAssistant,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum PrivacyLevel {
    Public,      // Data can be sent to external services
    Private,     // Data stays on device, limited external access
    Confidential, // Strict local processing only
    Encrypted,   // All data encrypted, secure processing
}

/// Conversation context and history
#[derive(Debug, Clone)]
pub struct Conversation {
    pub id: u32,
    pub title: String,
    pub messages: Vec<Message>,
    pub context: ConversationContext,
    pub created_at: u64,
    pub updated_at: u64,
    pub archived: bool,
    pub tags: Vec<String>,
    pub privacy_level: PrivacyLevel,
}

#[derive(Debug, Clone)]
pub struct Message {
    pub id: u32,
    pub role: MessageRole,
    pub content: MessageContent,
    pub timestamp: u64,
    pub metadata: MessageMetadata,
    pub attachments: Vec<Attachment>,
    pub reactions: Vec<Reaction>,
    pub edited: bool,
    pub edit_history: Vec<MessageEdit>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageRole {
    User,
    Assistant,
    System,
    Function,
    Tool,
}

#[derive(Debug, Clone)]
pub enum MessageContent {
    Text(String),
    Code { language: String, code: String },
    Image { path: String, description: Option<String> },
    Audio { path: String, transcript: Option<String> },
    File { path: String, file_type: String },
    Structured(BTreeMap<String, String>),
    Mixed(Vec<MessageContent>),
}

#[derive(Debug, Clone)]
pub struct MessageMetadata {
    pub model_used: Option<String>,
    pub processing_time_ms: u32,
    pub token_count: u32,
    pub confidence_score: f32,
    pub source: MessageSource,
    pub intent: Option<Intent>,
    pub entities: Vec<Entity>,
    pub sentiment: Option<Sentiment>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MessageSource {
    DirectInput,
    VoiceInput,
    FileUpload,
    SystemGenerated,
    ApiCall,
    Automation,
}

#[derive(Debug, Clone)]
pub struct Intent {
    pub name: String,
    pub confidence: f32,
    pub parameters: BTreeMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct Entity {
    pub entity_type: String,
    pub value: String,
    pub confidence: f32,
    pub start_pos: usize,
    pub end_pos: usize,
}

#[derive(Debug, Clone)]
pub struct Sentiment {
    pub polarity: f32, // -1.0 to 1.0
    pub confidence: f32,
    pub emotions: BTreeMap<String, f32>,
}

#[derive(Debug, Clone)]
pub struct Attachment {
    pub id: u32,
    pub file_path: String,
    pub file_type: String,
    pub file_size: u64,
    pub description: Option<String>,
    pub processed: bool,
    pub analysis_results: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Reaction {
    pub user_id: Option<u32>,
    pub reaction_type: ReactionType,
    pub timestamp: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ReactionType {
    Like,
    Dislike,
    Helpful,
    NotHelpful,
    Funny,
    Confused,
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct MessageEdit {
    pub timestamp: u64,
    pub old_content: MessageContent,
    pub edit_reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConversationContext {
    pub current_task: Option<String>,
    pub active_files: Vec<String>,
    pub working_directory: String,
    pub environment_variables: BTreeMap<String, String>,
    pub user_preferences: UserPreferences,
    pub session_data: BTreeMap<String, String>,
    pub related_conversations: Vec<u32>,
}

#[derive(Debug, Clone)]
pub struct UserPreferences {
    pub language: String,
    pub timezone: String,
    pub response_style: ResponseStyle,
    pub verbosity_level: VerbosityLevel,
    pub code_style: CodeStyle,
    pub explanation_level: ExplanationLevel,
    pub safety_level: SafetyLevel,
    pub response_length: ResponseLength,
    pub detail_level: DetailLevel,
    pub communication_style: CommunicationStyle,
    pub personalization_enabled: bool,
    pub learning_enabled: bool,
    pub suggestions_enabled: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseStyle {
    Professional,
    Casual,
    Technical,
    Creative,
    Educational,
    Concise,
    Detailed,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum VerbosityLevel {
    Minimal,
    Brief,
    Normal,
    Detailed,
    Comprehensive,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CodeStyle {
    Minimal,
    Commented,
    Documented,
    Educational,
    Production,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExplanationLevel {
    None,
    Basic,
    Intermediate,
    Advanced,
    Expert,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SafetyLevel {
    Permissive,
    Standard,
    Strict,
    Paranoid,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ResponseLength {
    Short,
    Medium,
    Long,
    Variable,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DetailLevel {
    Minimal,
    Basic,
    Detailed,
    Comprehensive,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CommunicationStyle {
    Clear,
    Technical,
    Conversational,
    Formal,
    Friendly,
}

/// AI function and tool system
#[derive(Debug, Clone)]
pub struct AiFunction {
    pub name: String,
    pub description: String,
    pub parameters: FunctionParameters,
    pub implementation: FunctionImplementation,
    pub permissions: Vec<String>,
    pub rate_limit: Option<RateLimit>,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct FunctionParameters {
    pub required: Vec<Parameter>,
    pub optional: Vec<Parameter>,
    pub schema: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Parameter {
    pub name: String,
    pub parameter_type: ParameterType,
    pub description: String,
    pub default_value: Option<String>,
    pub validation: Option<ParameterValidation>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ParameterType {
    String,
    Integer,
    Float,
    Boolean,
    Array(Box<ParameterType>),
    Object(BTreeMap<String, ParameterType>),
    File,
    Path,
    Url,
    Email,
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct ParameterValidation {
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<String>,
    pub allowed_values: Option<Vec<String>>,
    pub min_value: Option<f64>,
    pub max_value: Option<f64>,
}

#[derive(Debug, Clone)]
pub enum FunctionImplementation {
    Native(fn(&BTreeMap<String, String>) -> Result<String, String>),
    SystemCall(String),
    Script { language: String, code: String },
    External { endpoint: String, method: String },
    Plugin(String),
}

#[derive(Debug, Clone)]
pub struct RateLimit {
    pub calls_per_minute: u32,
    pub calls_per_hour: u32,
    pub calls_per_day: u32,
    pub concurrent_calls: u32,
}

/// Task automation and workflow
#[derive(Debug, Clone)]
pub struct AutomationTask {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub trigger: TaskTrigger,
    pub actions: Vec<TaskAction>,
    pub conditions: Vec<TaskCondition>,
    pub schedule: Option<TaskSchedule>,
    pub enabled: bool,
    pub created_at: u64,
    pub last_run: Option<u64>,
    pub run_count: u32,
    pub success_count: u32,
    pub failure_count: u32,
}

#[derive(Debug, Clone)]
pub enum TaskTrigger {
    Manual,
    Schedule(TaskSchedule),
    FileChange(String),
    SystemEvent(String),
    UserInput(String),
    ApiCall,
    ConversationKeyword(String),
    TimeInterval(u64),
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct TaskSchedule {
    pub schedule_type: ScheduleType,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub timezone: String,
    pub repeat_count: Option<u32>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ScheduleType {
    Once(u64),
    Daily { hour: u8, minute: u8 },
    Weekly { day: u8, hour: u8, minute: u8 },
    Monthly { day: u8, hour: u8, minute: u8 },
    Interval(u64), // milliseconds
    Cron(String),
}

#[derive(Debug, Clone)]
pub struct TaskAction {
    pub action_type: ActionType,
    pub parameters: BTreeMap<String, String>,
    pub timeout_ms: Option<u32>,
    pub retry_count: u32,
    pub on_failure: FailureAction,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActionType {
    RunCommand,
    SendMessage,
    CreateFile,
    DeleteFile,
    MoveFile,
    CopyFile,
    OpenApplication,
    CloseApplication,
    SendNotification,
    SendEmail,
    MakeHttpRequest,
    ExecuteScript,
    CallFunction,
    SetVariable,
    WaitDelay,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum FailureAction {
    Continue,
    Stop,
    Retry,
    Rollback,
    Notify,
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct TaskCondition {
    pub condition_type: ConditionType,
    pub operator: ConditionOperator,
    pub value: String,
    pub negate: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConditionType {
    FileExists,
    ProcessRunning,
    NetworkConnected,
    TimeOfDay,
    DayOfWeek,
    SystemLoad,
    DiskSpace,
    MemoryUsage,
    VariableValue,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum ConditionOperator {
    Equals,
    NotEquals,
    GreaterThan,
    LessThan,
    GreaterOrEqual,
    LessOrEqual,
    Contains,
    StartsWith,
    EndsWith,
    Matches, // regex
}

/// Learning and personalization
#[derive(Debug, Clone)]
pub struct LearningData {
    pub user_interactions: Vec<UserInteraction>,
    pub preferences: UserPreferences,
    pub usage_patterns: UsagePatterns,
    pub feedback_history: Vec<Feedback>,
    pub knowledge_base: KnowledgeBase,
    pub personalization_model: Option<PersonalizationModel>,
}

#[derive(Debug, Clone)]
pub struct UserInteraction {
    pub timestamp: u64,
    pub interaction_type: InteractionType,
    pub context: String,
    pub user_input: String,
    pub ai_response: String,
    pub user_satisfaction: Option<f32>,
    pub follow_up_actions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum InteractionType {
    Question,
    Command,
    Request,
    Conversation,
    Correction,
    Feedback,
    Task,
}

#[derive(Debug, Clone)]
pub struct UsagePatterns {
    pub most_used_features: BTreeMap<String, u32>,
    pub peak_usage_times: Vec<(u8, u8)>, // (hour, minute)
    pub common_workflows: Vec<Workflow>,
    pub preferred_response_types: BTreeMap<String, f32>,
    pub error_patterns: Vec<ErrorPattern>,
}

#[derive(Debug, Clone)]
pub struct Workflow {
    pub name: String,
    pub steps: Vec<WorkflowStep>,
    pub frequency: u32,
    pub success_rate: f32,
    pub average_duration: u64,
}

#[derive(Debug, Clone)]
pub struct WorkflowStep {
    pub action: String,
    pub parameters: BTreeMap<String, String>,
    pub duration_ms: u64,
    pub success: bool,
}

#[derive(Debug, Clone)]
pub struct ErrorPattern {
    pub error_type: String,
    pub frequency: u32,
    pub context: String,
    pub resolution: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Feedback {
    pub timestamp: u64,
    pub feedback_type: FeedbackType,
    pub rating: Option<u8>, // 1-5
    pub comment: Option<String>,
    pub context: String,
    pub improvement_suggestions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FeedbackType {
    Positive,
    Negative,
    Suggestion,
    Bug,
    Feature,
    General,
}

#[derive(Debug, Clone)]
pub struct KnowledgeBase {
    pub facts: BTreeMap<String, Fact>,
    pub relationships: Vec<Relationship>,
    pub categories: BTreeMap<String, Category>,
    pub last_updated: u64,
}

#[derive(Debug, Clone)]
pub struct Fact {
    pub id: String,
    pub content: String,
    pub confidence: f32,
    pub source: String,
    pub timestamp: u64,
    pub tags: Vec<String>,
    pub verified: bool,
}

#[derive(Debug, Clone)]
pub struct Relationship {
    pub subject: String,
    pub predicate: String,
    pub object: String,
    pub confidence: f32,
}

#[derive(Debug, Clone)]
pub struct Category {
    pub name: String,
    pub description: String,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub facts: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct PersonalizationModel {
    pub model_type: String,
    pub model_data: Vec<u8>,
    pub training_data_size: u32,
    pub accuracy: f32,
    pub last_trained: u64,
    pub version: String,
}

/// Main Rae Assistant system
#[derive(Debug)]
pub struct RaeAssistant {
    pub config: AiModelConfig,
    pub conversations: BTreeMap<u32, Conversation>,
    pub active_conversation: Option<u32>,
    pub next_conversation_id: u32,
    pub next_message_id: u32,
    pub functions: BTreeMap<String, AiFunction>,
    pub automation_tasks: BTreeMap<u32, AutomationTask>,
    pub next_task_id: u32,
    pub learning_data: LearningData,
    pub enabled: bool,
    pub processing_queue: Vec<ProcessingRequest>,
    pub rate_limiter: RateLimiter,
    pub security_manager: SecurityManager,
    pub plugin_manager: PluginManager,
}

#[derive(Debug, Clone)]
pub struct ProcessingRequest {
    pub id: u32,
    pub request_type: RequestType,
    pub content: String,
    pub context: BTreeMap<String, String>,
    pub priority: Priority,
    pub timestamp: u64,
    pub timeout_ms: u32,
    pub callback: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RequestType {
    TextGeneration,
    CodeGeneration,
    ImageAnalysis,
    AudioTranscription,
    Translation,
    Summarization,
    QuestionAnswering,
    FunctionCall,
    TaskExecution,
    Learning,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
    Emergency,
}

#[derive(Debug)]
pub struct RateLimiter {
    pub limits: BTreeMap<String, RateLimit>,
    pub usage_counters: BTreeMap<String, UsageCounter>,
}

#[derive(Debug, Clone)]
pub struct UsageCounter {
    pub minute_count: u32,
    pub hour_count: u32,
    pub day_count: u32,
    pub concurrent_count: u32,
    pub last_reset_minute: u64,
    pub last_reset_hour: u64,
    pub last_reset_day: u64,
}

#[derive(Debug)]
pub struct SecurityManager {
    pub access_policies: Vec<AccessPolicy>,
    pub audit_log: Vec<AuditEntry>,
    pub threat_detection: ThreatDetection,
    pub encryption_enabled: bool,
    pub data_retention_days: u32,
}

#[derive(Debug, Clone)]
pub struct AccessPolicy {
    pub name: String,
    pub rules: Vec<AccessRule>,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct AccessRule {
    pub resource: String,
    pub action: String,
    pub condition: Option<String>,
    pub allow: bool,
}

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub timestamp: u64,
    pub user_id: Option<u32>,
    pub action: String,
    pub resource: String,
    pub result: AuditResult,
    pub details: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AuditResult {
    Success,
    Failure,
    Blocked,
    Warning,
}

#[derive(Debug)]
pub struct ThreatDetection {
    pub enabled: bool,
    pub detection_rules: Vec<ThreatRule>,
    pub threat_scores: BTreeMap<String, f32>,
    pub blocked_patterns: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ThreatRule {
    pub name: String,
    pub pattern: String,
    pub severity: ThreatSeverity,
    pub action: ThreatAction,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ThreatSeverity {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ThreatAction {
    Log,
    Warn,
    Block,
    Quarantine,
    Alert,
}

#[derive(Debug)]
pub struct PluginManager {
    pub plugins: BTreeMap<String, AiPlugin>,
    pub plugin_registry: PluginRegistry,
    pub sandboxing_enabled: bool,
}

#[derive(Debug, Clone)]
pub struct AiPlugin {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub capabilities: Vec<AiCapability>,
    pub functions: Vec<String>,
    pub enabled: bool,
    pub trusted: bool,
    pub plugin_path: String,
    pub config: BTreeMap<String, String>,
}

#[derive(Debug)]
pub struct PluginRegistry {
    pub available_plugins: BTreeMap<String, PluginInfo>,
    pub update_sources: Vec<String>,
    pub last_update_check: u64,
}

#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
    pub download_url: String,
    pub checksum: String,
    pub rating: f32,
    pub download_count: u32,
}

// Implementation
impl RaeAssistant {
    pub fn new() -> Self {
        RaeAssistant {
            config: AiModelConfig::default(),
            conversations: BTreeMap::new(),
            active_conversation: None,
            next_conversation_id: 1,
            next_message_id: 1,
            functions: BTreeMap::new(),
            automation_tasks: BTreeMap::new(),
            next_task_id: 1,
            learning_data: LearningData::new(),
            enabled: true,
            processing_queue: Vec::new(),
            rate_limiter: RateLimiter::new(),
            security_manager: SecurityManager::new(),
            plugin_manager: PluginManager::new(),
        }
    }
    
    pub fn start_conversation(&mut self, title: Option<String>) -> u32 {
        let conversation_id = self.next_conversation_id;
        self.next_conversation_id += 1;
        
        let conversation = Conversation {
            id: conversation_id,
            title: title.unwrap_or_else(|| format!("Conversation {}", conversation_id)),
            messages: Vec::new(),
            context: ConversationContext::default(),
            created_at: 0, // TODO: Use crate::time::get_timestamp() when time module is available
            updated_at: 0, // TODO: Use crate::time::get_timestamp() when time module is available
            archived: false,
            tags: Vec::new(),
            privacy_level: PrivacyLevel::Private,
        };
        
        self.conversations.insert(conversation_id, conversation);
        self.active_conversation = Some(conversation_id);
        
        conversation_id
    }
    
    pub fn send_message(&mut self, conversation_id: u32, content: String, role: MessageRole) -> Result<u32, String> {
        if !self.enabled {
            return Err("Rae Assistant is disabled".to_string());
        }
        
        let conversation = self.conversations.get_mut(&conversation_id)
            .ok_or("Conversation not found")?;
        
        let message_id = self.next_message_id;
        self.next_message_id += 1;
        
        let message = Message {
            id: message_id,
            role,
            content: MessageContent::Text(content.clone()),
            timestamp: crate::time::get_timestamp(),
            metadata: MessageMetadata {
                model_used: None,
                processing_time_ms: 0,
                token_count: content.len() as u32,
                confidence_score: 1.0,
                source: MessageSource::DirectInput,
                intent: None,
                entities: Vec::new(),
                sentiment: None,
            },
            attachments: Vec::new(),
            reactions: Vec::new(),
            edited: false,
            edit_history: Vec::new(),
        };
        
        conversation.messages.push(message);
        conversation.updated_at = crate::time::get_timestamp();
        
        // Process user messages with AI
        if role == MessageRole::User {
            self.process_user_message(conversation_id, &content)?;
        }
        
        Ok(message_id)
    }
    
    fn process_user_message(&mut self, conversation_id: u32, content: &str) -> Result<(), String> {
        // Analyze intent and entities
        let intent = self.analyze_intent(content);
        let entities = self.extract_entities(content);
        
        // Check for function calls
        if let Some(function_call) = self.detect_function_call(content) {
            return self.execute_function(conversation_id, &function_call);
        }
        
        // Check for automation triggers
        self.check_automation_triggers(content);
        
        // Generate AI response
        let response = self.generate_response(conversation_id, content, &intent, &entities)?;
        
        // Add AI response to conversation
        self.send_message(conversation_id, response, MessageRole::Assistant)?;
        
        // Update learning data
        self.update_learning_data(content, &intent, &entities);
        
        Ok(())
    }
    
    fn analyze_intent(&self, content: &str) -> Option<Intent> {
        // Implement intent analysis using pattern matching and keyword detection
        let content_lower = content.to_lowercase();
        let mut parameters = BTreeMap::new();
        
        // File operations
        if content_lower.contains("create") || content_lower.contains("make") || content_lower.contains("new") {
            if content_lower.contains("file") {
                parameters.insert("object_type".to_string(), "file".to_string());
            } else if content_lower.contains("folder") || content_lower.contains("directory") {
                parameters.insert("object_type".to_string(), "directory".to_string());
            }
            Some(Intent {
                name: "create".to_string(),
                confidence: 0.85,
                parameters,
            })
        } else if content_lower.contains("delete") || content_lower.contains("remove") || content_lower.contains("rm") {
            Some(Intent {
                name: "delete".to_string(),
                confidence: 0.85,
                parameters,
            })
        } else if content_lower.contains("open") || content_lower.contains("launch") {
            Some(Intent {
                name: "open".to_string(),
                confidence: 0.8,
                parameters,
            })
        } else if content_lower.contains("search") || content_lower.contains("find") {
            Some(Intent {
                name: "search".to_string(),
                confidence: 0.8,
                parameters,
            })
        } else if content_lower.contains("help") || content_lower.contains("how") || content_lower.contains("?") {
            Some(Intent {
                name: "help".to_string(),
                confidence: 0.9,
                parameters,
            })
        } else if content_lower.contains("install") || content_lower.contains("download") {
            Some(Intent {
                name: "install".to_string(),
                confidence: 0.8,
                parameters,
            })
        } else if content_lower.contains("run") || content_lower.contains("execute") {
            Some(Intent {
                name: "execute".to_string(),
                confidence: 0.8,
                parameters,
            })
        } else if content_lower.contains("settings") || content_lower.contains("config") {
            Some(Intent {
                name: "settings".to_string(),
                confidence: 0.8,
                parameters,
            })
        } else {
            None
        }
    }
    
    fn extract_entities(&self, content: &str) -> Vec<Entity> {
        // Implement entity extraction using pattern matching
        let mut entities = Vec::new();
        
        // Extract file paths (simple pattern matching)
        let words: Vec<&str> = content.split_whitespace().collect();
        for word in &words {
            // File paths
            if word.contains('/') || word.contains('\\') {
                entities.push(Entity {
                    entity_type: "file_path".to_string(),
                    value: word.to_string(),
                    confidence: 0.8,
                });
            }
            // File extensions
            else if word.contains('.') && word.len() > 2 {
                let parts: Vec<&str> = word.split('.').collect();
                if parts.len() == 2 && parts[1].len() <= 4 {
                    entities.push(Entity {
                        entity_type: "file_extension".to_string(),
                        value: parts[1].to_string(),
                        confidence: 0.7,
                    });
                }
            }
            // Numbers
            else if word.parse::<i32>().is_ok() {
                entities.push(Entity {
                    entity_type: "number".to_string(),
                    value: word.to_string(),
                    confidence: 0.9,
                });
            }
        }
        
        // Extract quoted strings (file names, etc.)
        let mut in_quotes = false;
        let mut current_quote = String::new();
        for ch in content.chars() {
            if ch == '"' || ch == '\'' {
                if in_quotes {
                    if !current_quote.is_empty() {
                        entities.push(Entity {
                            entity_type: "quoted_string".to_string(),
                            value: current_quote.clone(),
                            confidence: 0.9,
                        });
                    }
                    current_quote.clear();
                    in_quotes = false;
                } else {
                    in_quotes = true;
                }
            } else if in_quotes {
                current_quote.push(ch);
            }
        }
        
        entities
    }
    
    fn detect_function_call(&self, content: &str) -> Option<String> {
        // Implement function call detection
        let content_lower = content.to_lowercase();
        
        // Command execution patterns
        if content_lower.contains("run command") || content_lower.contains("execute command") {
            return Some("execute_command".to_string());
        }
        
        // File operations
        if content_lower.contains("create file") {
            return Some("create_file".to_string());
        }
        if content_lower.contains("delete file") || content_lower.contains("remove file") {
            return Some("delete_file".to_string());
        }
        
        // System operations
        if content_lower.contains("open application") || content_lower.contains("launch app") {
            return Some("launch_application".to_string());
        }
        if content_lower.contains("install package") || content_lower.contains("install app") {
            return Some("install_package".to_string());
        }
        
        // Search operations
        if content_lower.contains("search for") || content_lower.contains("find file") {
            return Some("search_files".to_string());
        }
        
        // System info
        if content_lower.contains("system info") || content_lower.contains("system status") {
            return Some("get_system_info".to_string());
        }
        
        None
    }
    
    fn execute_function(&mut self, conversation_id: u32, function_name: &str) -> Result<(), String> {
        // Implement function execution
        match function_name {
            "execute_command" => {
                // In a real implementation, this would execute shell commands safely
                // For now, just log the action
                Ok(())
            }
            "create_file" => {
                // In a real implementation, this would create files
                // For now, just acknowledge the request
                Ok(())
            }
            "delete_file" => {
                // In a real implementation, this would delete files safely
                // For now, just acknowledge the request
                Ok(())
            }
            "launch_application" => {
                // In a real implementation, this would launch applications
                // For now, just acknowledge the request
                Ok(())
            }
            "install_package" => {
                // In a real implementation, this would install packages
                // For now, just acknowledge the request
                Ok(())
            }
            "search_files" => {
                // In a real implementation, this would search the filesystem
                // For now, just acknowledge the request
                Ok(())
            }
            "get_system_info" => {
                // In a real implementation, this would gather system information
                // For now, just acknowledge the request
                Ok(())
            }
            _ => Err(format!("Unknown function: {}", function_name))
        }
    }
    
    fn check_automation_triggers(&mut self, content: &str) {
        // Check if content matches any automation triggers
        let content_lower = content.to_lowercase();
        let mut triggered_tasks = Vec::new();
        
        for (task_id, task) in &self.automation_tasks {
            let should_trigger = match &task.trigger {
                TaskTrigger::ConversationKeyword(keyword) => {
                    content_lower.contains(&keyword.to_lowercase())
                }
                TaskTrigger::TimeSchedule(schedule) => {
                    // For time-based triggers, this would check against current time
                    // For now, skip time-based triggers in conversation context
                    false
                }
                TaskTrigger::SystemEvent(event) => {
                    // System event triggers would be handled elsewhere
                    false
                }
            };
            
            if should_trigger {
                triggered_tasks.push(*task_id);
            }
        }
        
        // Execute triggered automation tasks
        for task_id in triggered_tasks {
            if let Some(task) = self.automation_tasks.get(&task_id) {
                for action in &task.actions {
                    match action {
                        TaskAction::SendMessage(message) => {
                            // In a real implementation, this would send a message
                            // For now, just acknowledge the action
                        }
                        TaskAction::ExecuteCommand(command) => {
                            // In a real implementation, this would execute the command safely
                            // For now, just acknowledge the action
                        }
                        TaskAction::CreateFile(path, content) => {
                            // In a real implementation, this would create the file
                            // For now, just acknowledge the action
                        }
                        TaskAction::OpenApplication(app_name) => {
                            // In a real implementation, this would open the application
                            // For now, just acknowledge the action
                        }
                    }
                }
            }
        }
    }
    
    fn generate_response(&self, conversation_id: u32, content: &str, intent: &Option<Intent>, entities: &[Entity]) -> Result<String, String> {
        // Implement actual AI response generation
        let mut response = String::new();
        
        // Generate response based on intent and entities
        if let Some(intent) = intent {
            match intent.name.as_str() {
                "help" => {
                    response = "I'm here to help! I can assist you with:\n".to_string();
                    response.push_str("• Creating and managing files\n");
                    response.push_str("• Running commands and applications\n");
                    response.push_str("• Searching for files and information\n");
                    response.push_str("• Installing packages and software\n");
                    response.push_str("• System configuration and settings\n");
                    response.push_str("What would you like assistance with?");
                }
                "create" => {
                    if let Some(object_type) = intent.parameters.get("object_type") {
                        response = format!("I can help you create a {}. ", object_type);
                    } else {
                        response = "I can help you create various things. ".to_string();
                    }
                    
                    // Check for file names in entities
                    let file_entities: Vec<&Entity> = entities.iter()
                        .filter(|e| e.entity_type == "quoted_string" || e.entity_type == "file_path")
                        .collect();
                    
                    if !file_entities.is_empty() {
                        response.push_str(&format!("Would you like me to create '{}'?", file_entities[0].value));
                    } else {
                        response.push_str("Please specify what you'd like to create.");
                    }
                }
                "delete" => {
                    response = "I can help you delete files or data. ".to_string();
                    
                    // Check for file paths in entities
                    let file_entities: Vec<&Entity> = entities.iter()
                        .filter(|e| e.entity_type == "file_path" || e.entity_type == "quoted_string")
                        .collect();
                    
                    if !file_entities.is_empty() {
                        response.push_str(&format!("Are you sure you want to delete '{}'? This action cannot be undone.", file_entities[0].value));
                    } else {
                        response.push_str("Please specify what you'd like to delete.");
                    }
                }
                "open" => {
                    response = "I can help you open applications or files. ".to_string();
                    
                    let app_entities: Vec<&Entity> = entities.iter()
                        .filter(|e| e.entity_type == "quoted_string")
                        .collect();
                    
                    if !app_entities.is_empty() {
                        response.push_str(&format!("Opening '{}'...", app_entities[0].value));
                    } else {
                        response.push_str("What would you like me to open?");
                    }
                }
                "search" => {
                    response = "I can help you search for files and information. ".to_string();
                    
                    let search_entities: Vec<&Entity> = entities.iter()
                        .filter(|e| e.entity_type == "quoted_string")
                        .collect();
                    
                    if !search_entities.is_empty() {
                        response.push_str(&format!("Searching for '{}'...", search_entities[0].value));
                    } else {
                        response.push_str("What would you like me to search for?");
                    }
                }
                "install" => {
                    response = "I can help you install packages and applications. ".to_string();
                    
                    let package_entities: Vec<&Entity> = entities.iter()
                        .filter(|e| e.entity_type == "quoted_string")
                        .collect();
                    
                    if !package_entities.is_empty() {
                        response.push_str(&format!("Installing '{}'...", package_entities[0].value));
                    } else {
                        response.push_str("What would you like me to install?");
                    }
                }
                "execute" => {
                    response = "I can help you run commands and scripts. ".to_string();
                    
                    let command_entities: Vec<&Entity> = entities.iter()
                        .filter(|e| e.entity_type == "quoted_string")
                        .collect();
                    
                    if !command_entities.is_empty() {
                        response.push_str(&format!("Executing '{}'...", command_entities[0].value));
                    } else {
                        response.push_str("What command would you like me to run?");
                    }
                }
                "settings" => {
                    response = "I can help you configure system settings. What would you like to change?".to_string();
                }
                _ => {
                    response = "I understand your request. How can I assist you further?".to_string();
                }
            }
        } else {
            // No clear intent detected, provide general assistance
            response = "I'm here to help! I can assist with file management, running applications, ".to_string();
            response.push_str("searching for information, and system configuration. ");
            response.push_str("What would you like me to help you with?");
        }
        
        Ok(response)
    }
    
    fn update_learning_data(&mut self, content: &str, intent: &Option<Intent>, entities: &[Entity]) {
        // Update learning data with new interaction
        let interaction_type = if content.contains('?') {
            InteractionType::Question
        } else if let Some(intent) = intent {
            match intent.name.as_str() {
                "execute" | "create" | "delete" | "install" => InteractionType::Command,
                _ => InteractionType::Question,
            }
        } else {
            InteractionType::Question
        };
        
        let context = if let Some(intent) = intent {
            intent.name.clone()
        } else {
            "general".to_string()
        };
        
        let interaction = UserInteraction {
            timestamp: 0, // TODO: Use crate::time::get_timestamp() when time module is available
            interaction_type,
            context,
            user_input: content.to_string(),
            ai_response: "Generated response".to_string(), // This would be filled with actual response
            user_satisfaction: None,
            follow_up_actions: Vec::new(),
        };
        
        self.learning_data.user_interactions.push(interaction);
        
        // Update pattern recognition based on entities
        for entity in entities {
            // Track commonly used file types
            if entity.entity_type == "file_extension" {
                // In a real implementation, this would update file type usage statistics
            }
            
            // Track commonly accessed paths
            if entity.entity_type == "file_path" {
                // In a real implementation, this would update path usage statistics
            }
        }
        
        // Update intent frequency for better prediction
        if let Some(intent) = intent {
            // In a real implementation, this would update intent usage statistics
            // to improve future intent recognition accuracy
        }
    }
    
    pub fn create_automation_task(&mut self, name: String, trigger: TaskTrigger, actions: Vec<TaskAction>) -> u32 {
        let task_id = self.next_task_id;
        self.next_task_id += 1;
        
        let task = AutomationTask {
            id: task_id,
            name,
            description: String::new(),
            trigger,
            actions,
            conditions: Vec::new(),
            schedule: None,
            enabled: true,
            created_at: 0, // TODO: Use crate::time::get_timestamp() when time module is available
            last_run: None,
            run_count: 0,
            success_count: 0,
            failure_count: 0,
        };
        
        self.automation_tasks.insert(task_id, task);
        task_id
    }
    
    pub fn register_function(&mut self, function: AiFunction) {
        self.functions.insert(function.name.clone(), function);
    }
    
    pub fn enable(&mut self) {
        self.enabled = true;
    }
    
    pub fn disable(&mut self) {
        self.enabled = false;
    }
    
    pub fn get_conversation(&self, conversation_id: u32) -> Option<&Conversation> {
        self.conversations.get(&conversation_id)
    }
    
    pub fn get_conversations(&self) -> Vec<&Conversation> {
        self.conversations.values().collect()
    }
    
    pub fn archive_conversation(&mut self, conversation_id: u32) -> bool {
        if let Some(conversation) = self.conversations.get_mut(&conversation_id) {
            conversation.archived = true;
            true
        } else {
            false
        }
    }
    
    pub fn delete_conversation(&mut self, conversation_id: u32) -> bool {
        self.conversations.remove(&conversation_id).is_some()
    }
    
    pub fn set_active_conversation(&mut self, conversation_id: u32) -> bool {
        if self.conversations.contains_key(&conversation_id) {
            self.active_conversation = Some(conversation_id);
            true
        } else {
            false
        }
    }
    
    pub fn get_active_conversation(&self) -> Option<&Conversation> {
        if let Some(id) = self.active_conversation {
            self.conversations.get(&id)
        } else {
            None
        }
    }
    
    pub fn process_queue(&mut self) {
        // Process pending requests in the queue
        while let Some(request) = self.processing_queue.pop() {
            match request.request_type {
                RequestType::TextGeneration => {
                    // Process text generation request
                    if let Some(conversation_id) = request.conversation_id {
                        let _ = self.send_message(
                            conversation_id,
                            request.content.clone(),
                            MessageRole::User
                        );
                    }
                }
                RequestType::CodeGeneration => {
                    // Process code generation request
                    if let Some(conversation_id) = request.conversation_id {
                        let response = format!("Generated code for: {}", request.content);
                        let _ = self.add_message(
                            conversation_id,
                            response,
                            MessageRole::Assistant
                        );
                    }
                }
                RequestType::QuestionAnswering => {
                    // Process Q&A request
                    if let Some(conversation_id) = request.conversation_id {
                        let response = self.generate_response(&request.content, conversation_id);
                        let _ = self.add_message(
                            conversation_id,
                            response,
                            MessageRole::Assistant
                        );
                    }
                }
                RequestType::TaskAutomation => {
                    // Process automation task request
                    // Check if this triggers any automation tasks
                    self.check_automation_triggers(&request.content);
                }
                RequestType::FunctionCall => {
                    // Process function call request
                    if let Some(conversation_id) = request.conversation_id {
                        if let Some(function_name) = self.detect_function_call(&request.content) {
                            let _ = self.execute_function(conversation_id, &function_name);
                        }
                    }
                }
            }
        }
    }
    
    pub fn add_feedback(&mut self, conversation_id: u32, message_id: u32, feedback: Feedback) {
        self.learning_data.feedback_history.push(feedback.clone());
        
        // Update conversation with feedback
        if let Some(conversation) = self.conversations.get_mut(&conversation_id) {
            // Find the message and update it with feedback
            for message in &mut conversation.messages {
                if message.id == message_id {
                    message.feedback = Some(feedback.clone());
                    break;
                }
            }
            
            // Update conversation metadata based on feedback
            match feedback.rating {
                1..=2 => {
                    // Negative feedback - mark for improvement
                    conversation.needs_improvement = true;
                }
                4..=5 => {
                    // Positive feedback - mark as successful interaction
                    conversation.quality_score += 1;
                }
                _ => {
                    // Neutral feedback - no special action
                }
            }
        }
        
        // Update learning patterns based on feedback
        self.update_learning_patterns(&feedback);
    }
    
    fn update_learning_patterns(&mut self, feedback: &Feedback) {
        // Update learning patterns based on user feedback
        match feedback.rating {
            1..=2 => {
                // Negative feedback - adjust response patterns
                // In a real implementation, this would update ML models
                // to avoid similar responses in the future
                if let Some(ref comment) = feedback.comment {
                    // Analyze negative feedback comments to improve responses
                    if comment.to_lowercase().contains("too long") {
                        // Adjust response length preferences
                        self.learning_data.preferences.response_length = ResponseLength::Short;
                    } else if comment.to_lowercase().contains("not helpful") {
                        // Adjust helpfulness scoring
                        // This would update internal scoring mechanisms
                    }
                }
            }
            4..=5 => {
                // Positive feedback - reinforce successful patterns
                // In a real implementation, this would strengthen
                // the patterns that led to this positive response
                if let Some(ref comment) = feedback.comment {
                    if comment.to_lowercase().contains("helpful") {
                        // Reinforce helpful response patterns
                        self.learning_data.preferences.detail_level = DetailLevel::Detailed;
                    } else if comment.to_lowercase().contains("clear") {
                        // Reinforce clear communication patterns
                        self.learning_data.preferences.communication_style = CommunicationStyle::Clear;
                    }
                }
            }
            _ => {
                // Neutral feedback - maintain current patterns
            }
        }
        
        // Update overall satisfaction metrics
        let total_feedback = self.learning_data.feedback_history.len() as f32;
        let positive_feedback = self.learning_data.feedback_history
            .iter()
            .filter(|f| f.rating >= 4)
            .count() as f32;
            
        // Calculate satisfaction rate (this could be used for model adjustments)
        let _satisfaction_rate = if total_feedback > 0.0 {
            positive_feedback / total_feedback
        } else {
            0.0
        };
    }
    
    pub fn get_suggestions(&self, context: &str) -> Vec<String> {
        // Generate contextual suggestions based on context and learning data
        let mut suggestions = Vec::new();
        let context_lower = context.to_lowercase();
        
        // Context-aware suggestions
        if context_lower.contains("code") || context_lower.contains("programming") {
            suggestions.extend(vec![
                "Generate a function for...".to_string(),
                "Debug this code snippet".to_string(),
                "Explain this algorithm".to_string(),
                "Optimize this code".to_string(),
                "Add error handling".to_string(),
            ]);
        } else if context_lower.contains("file") || context_lower.contains("directory") {
            suggestions.extend(vec![
                "Create a new file".to_string(),
                "Search for files".to_string(),
                "Organize files by type".to_string(),
                "Show file permissions".to_string(),
                "Backup important files".to_string(),
            ]);
        } else if context_lower.contains("system") || context_lower.contains("process") {
            suggestions.extend(vec![
                "Show system information".to_string(),
                "List running processes".to_string(),
                "Check memory usage".to_string(),
                "Monitor system performance".to_string(),
                "Manage system services".to_string(),
            ]);
        } else if context_lower.contains("help") || context_lower.contains("how") {
            suggestions.extend(vec![
                "Explain how to...".to_string(),
                "Show me examples of...".to_string(),
                "What is the best way to...".to_string(),
                "Guide me through...".to_string(),
                "Troubleshoot this issue".to_string(),
            ]);
        } else {
            // General suggestions
            suggestions.extend(vec![
                "How can I help you today?".to_string(),
                "Ask me about coding, files, or system tasks".to_string(),
                "I can help automate repetitive tasks".to_string(),
                "Would you like me to explain something?".to_string(),
                "Let me know what you're working on".to_string(),
            ]);
        }
        
        // Add suggestions based on user's interaction history
        let recent_interactions: Vec<_> = self.learning_data.user_interactions
            .iter()
            .rev()
            .take(5)
            .collect();
            
        for interaction in recent_interactions {
            if interaction.context.contains("code") && !suggestions.iter().any(|s| s.contains("code")) {
                suggestions.push("Continue with code-related tasks".to_string());
            }
            if interaction.context.contains("file") && !suggestions.iter().any(|s| s.contains("file")) {
                suggestions.push("More file operations".to_string());
            }
        }
        
        // Limit suggestions to avoid overwhelming the user
        suggestions.truncate(8);
        suggestions
    }
    
    pub fn export_conversation(&self, conversation_id: u32, format: &str) -> Result<String, String> {
        let conversation = self.conversations.get(&conversation_id)
            .ok_or("Conversation not found")?;
        
        match format {
            "json" => {
                // TODO: Serialize to JSON
                Ok(format!("{{\"conversation_id\": {}, \"title\": \"{}\"}}", conversation.id, conversation.title))
            }
            "text" => {
                let mut output = format!("Conversation: {}\n\n", conversation.title);
                for message in &conversation.messages {
                    if let MessageContent::Text(text) = &message.content {
                        output.push_str(&format!("{:?}: {}\n", message.role, text));
                    }
                }
                Ok(output)
            }
            _ => Err("Unsupported format".to_string())
        }
    }
}

// Default implementations
impl Default for AiModelConfig {
    fn default() -> Self {
        AiModelConfig {
            model_name: "RaeenOS Assistant".to_string(),
            model_version: "1.0".to_string(),
            model_type: ModelType::Multimodal,
            context_window: 4096,
            max_tokens: 2048,
            temperature: 0.7,
            top_p: 0.9,
            frequency_penalty: 0.0,
            presence_penalty: 0.0,
            stop_sequences: Vec::new(),
            system_prompt: "You are Rae, the helpful AI assistant for RaeenOS. You are knowledgeable, friendly, and focused on helping users with their computing tasks.".to_string(),
            capabilities: vec![
                AiCapability::TextGeneration,
                AiCapability::CodeGeneration,
                AiCapability::QuestionAnswering,
                AiCapability::TaskAutomation,
                AiCapability::PersonalAssistant,
            ],
            privacy_level: PrivacyLevel::Private,
            local_processing: true,
            model_path: None,
        }
    }
}

impl Default for ConversationContext {
    fn default() -> Self {
        ConversationContext {
            current_task: None,
            active_files: Vec::new(),
            working_directory: "/home/user".to_string(),
            environment_variables: BTreeMap::new(),
            user_preferences: UserPreferences::default(),
            session_data: BTreeMap::new(),
            related_conversations: Vec::new(),
        }
    }
}

impl Default for UserPreferences {
    fn default() -> Self {
        UserPreferences {
            language: "en".to_string(),
            timezone: "UTC".to_string(),
            response_style: ResponseStyle::Professional,
            verbosity_level: VerbosityLevel::Normal,
            code_style: CodeStyle::Commented,
            explanation_level: ExplanationLevel::Intermediate,
            safety_level: SafetyLevel::Standard,
            response_length: ResponseLength::Medium,
            detail_level: DetailLevel::Detailed,
            communication_style: CommunicationStyle::Clear,
            personalization_enabled: true,
            learning_enabled: true,
            suggestions_enabled: true,
        }
    }
}

impl LearningData {
    fn new() -> Self {
        LearningData {
            user_interactions: Vec::new(),
            preferences: UserPreferences::default(),
            usage_patterns: UsagePatterns {
                most_used_features: BTreeMap::new(),
                peak_usage_times: Vec::new(),
                common_workflows: Vec::new(),
                preferred_response_types: BTreeMap::new(),
                error_patterns: Vec::new(),
            },
            feedback_history: Vec::new(),
            knowledge_base: KnowledgeBase {
                facts: BTreeMap::new(),
                relationships: Vec::new(),
                categories: BTreeMap::new(),
                last_updated: 0, // TODO: Use crate::time::get_timestamp() when time module is available
            },
            personalization_model: None,
        }
    }
}

impl RateLimiter {
    fn new() -> Self {
        RateLimiter {
            limits: BTreeMap::new(),
            usage_counters: BTreeMap::new(),
        }
    }
}

impl SecurityManager {
    fn new() -> Self {
        SecurityManager {
            access_policies: Vec::new(),
            audit_log: Vec::new(),
            threat_detection: ThreatDetection {
                enabled: true,
                detection_rules: Vec::new(),
                threat_scores: BTreeMap::new(),
                blocked_patterns: Vec::new(),
            },
            encryption_enabled: true,
            data_retention_days: 90,
        }
    }
}

impl PluginManager {
    fn new() -> Self {
        PluginManager {
            plugins: BTreeMap::new(),
            plugin_registry: PluginRegistry {
                available_plugins: BTreeMap::new(),
                update_sources: Vec::new(),
                last_update_check: 0,
            },
            sandboxing_enabled: true,
        }
    }
}

// Global Rae Assistant instance
lazy_static! {
    static ref RAE_ASSISTANT: Mutex<RaeAssistant> = Mutex::new(RaeAssistant::new());
}

// Public API functions

pub fn init_rae_assistant() {
    let mut assistant = RAE_ASSISTANT.lock();
    *assistant = RaeAssistant::new();
}

pub fn start_conversation(title: Option<&str>) -> u32 {
    let mut assistant = RAE_ASSISTANT.lock();
    assistant.start_conversation(title.map(|s| s.to_string()))
}

pub fn send_message(conversation_id: u32, content: &str) -> Result<u32, String> {
    let mut assistant = RAE_ASSISTANT.lock();
    assistant.send_message(conversation_id, content.to_string(), MessageRole::User)
}

pub fn get_conversation(conversation_id: u32) -> Option<Conversation> {
    let assistant = RAE_ASSISTANT.lock();
    assistant.get_conversation(conversation_id).cloned()
}

pub fn get_conversations() -> Vec<Conversation> {
    let assistant = RAE_ASSISTANT.lock();
    assistant.get_conversations().into_iter().cloned().collect()
}

pub fn set_active_conversation(conversation_id: u32) -> bool {
    let mut assistant = RAE_ASSISTANT.lock();
    assistant.set_active_conversation(conversation_id)
}

pub fn get_active_conversation() -> Option<Conversation> {
    let assistant = RAE_ASSISTANT.lock();
    assistant.get_active_conversation().cloned()
}

pub fn create_automation_task(name: &str, trigger: TaskTrigger, actions: Vec<TaskAction>) -> u32 {
    let mut assistant = RAE_ASSISTANT.lock();
    assistant.create_automation_task(name.to_string(), trigger, actions)
}

pub fn register_ai_function(function: AiFunction) {
    let mut assistant = RAE_ASSISTANT.lock();
    assistant.register_function(function);
}

pub fn enable_assistant() {
    let mut assistant = RAE_ASSISTANT.lock();
    assistant.enable();
}

pub fn disable_assistant() {
    let mut assistant = RAE_ASSISTANT.lock();
    assistant.disable();
}

pub fn process_assistant_queue() {
    let mut assistant = RAE_ASSISTANT.lock();
    assistant.process_queue();
}

pub fn add_conversation_feedback(conversation_id: u32, message_id: u32, feedback: Feedback) {
    let mut assistant = RAE_ASSISTANT.lock();
    assistant.add_feedback(conversation_id, message_id, feedback);
}

pub fn get_ai_suggestions(context: &str) -> Vec<String> {
    let assistant = RAE_ASSISTANT.lock();
    assistant.get_suggestions(context)
}

pub fn export_conversation(conversation_id: u32, format: &str) -> Result<String, String> {
    let assistant = RAE_ASSISTANT.lock();
    assistant.export_conversation(conversation_id, format)
}

pub fn archive_conversation(conversation_id: u32) -> bool {
    let mut assistant = RAE_ASSISTANT.lock();
    assistant.archive_conversation(conversation_id)
}

pub fn delete_conversation(conversation_id: u32) -> bool {
    let mut assistant = RAE_ASSISTANT.lock();
    assistant.delete_conversation(conversation_id)
}

pub fn is_assistant_enabled() -> bool {
    let assistant = RAE_ASSISTANT.lock();
    assistant.enabled
}

pub fn get_assistant_config() -> AiModelConfig {
    let assistant = RAE_ASSISTANT.lock();
    assistant.config.clone()
}

pub fn update_assistant_config(config: AiModelConfig) {
    let mut assistant = RAE_ASSISTANT.lock();
    assistant.config = config;
}

pub fn get_learning_data() -> LearningData {
    let assistant = RAE_ASSISTANT.lock();
    assistant.learning_data.clone()
}

pub fn update_user_preferences(preferences: UserPreferences) {
    let mut assistant = RAE_ASSISTANT.lock();
    assistant.learning_data.preferences = preferences;
}

pub fn ask_assistant(question: &str) -> Result<String, String> {
    let mut assistant = RAE_ASSISTANT.lock();
    let conversation_id = assistant.start_conversation(Some("Quick Question".to_string()));
    assistant.send_message(conversation_id, question.to_string(), MessageRole::User)?;
    
    // Get the last message (AI response)
    if let Some(conversation) = assistant.get_conversation(conversation_id) {
        if let Some(last_message) = conversation.messages.last() {
            if let MessageContent::Text(response) = &last_message.content {
                return Ok(response.clone());
            }
        }
    }
    
    Err("Failed to get AI response".to_string())
}