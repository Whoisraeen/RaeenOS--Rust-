//! AI service contract for rae-assistantd
//! Defines IPC interface for user-space AI assistant operations

use alloc::vec::Vec;
use alloc::string::String;
use serde::{Serialize, Deserialize};
use super::{ServiceResponse, error_codes};

/// AI service requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiRequest {
    // Session management
    CreateSession { capabilities: Vec<AiCapability> },
    CloseSession { session_id: u32 },
    GetSessionInfo { session_id: u32 },
    ListSessions,
    
    // Text generation and analysis
    GenerateText { session_id: u32, prompt: String, options: GenerationOptions },
    AnalyzeContent { session_id: u32, content: Vec<u8>, analysis_type: AnalysisType },
    SummarizeText { session_id: u32, text: String, max_length: u32 },
    TranslateText { session_id: u32, text: String, target_language: String },
    
    // System assistance
    GetSystemHelp { session_id: u32, query: String },
    DiagnoseIssue { session_id: u32, symptoms: Vec<String>, context: SystemContext },
    SuggestOptimizations { session_id: u32, performance_data: PerformanceData },
    
    // Code assistance
    AnalyzeCode { session_id: u32, code: String, language: String },
    GenerateCode { session_id: u32, specification: String, language: String },
    ExplainCode { session_id: u32, code: String, language: String },
    FindBugs { session_id: u32, code: String, language: String },
    SuggestRefactoring { session_id: u32, code: String, language: String },
    
    // Conversation and context
    SendMessage { session_id: u32, message: String, context: Option<ConversationContext> },
    GetConversationHistory { session_id: u32, limit: Option<u32> },
    ClearConversationHistory { session_id: u32 },
    
    // Model management
    ListAvailableModels,
    GetModelInfo { model_id: String },
    LoadModel { model_id: String, config: ModelConfig },
    UnloadModel { model_id: String },
    
    // Privacy and data management
    SetPrivacySettings { session_id: u32, settings: PrivacySettings },
    GetPrivacySettings { session_id: u32 },
    ExportData { session_id: u32, format: DataFormat },
    DeleteData { session_id: u32, data_type: DataType },
    
    // Performance and monitoring
    GetPerformanceMetrics,
    SetResourceLimits { limits: ResourceLimits },
    GetResourceUsage,
}

/// AI service responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AiResponse {
    SessionCreated { session_id: u32, info: SessionInfo },
    SessionClosed,
    SessionInfo { info: SessionInfo },
    SessionList { sessions: Vec<SessionInfo> },
    
    TextGenerated { text: String, metadata: GenerationMetadata },
    ContentAnalyzed { analysis: AnalysisResult },
    TextSummarized { summary: String },
    TextTranslated { translated_text: String, confidence: f32 },
    
    SystemHelp { help_text: String, suggestions: Vec<String> },
    IssueDiagnosed { diagnosis: DiagnosisResult },
    OptimizationsSuggested { suggestions: Vec<OptimizationSuggestion> },
    
    CodeAnalyzed { analysis: CodeAnalysisResult },
    CodeGenerated { code: String, explanation: String },
    CodeExplained { explanation: String, key_concepts: Vec<String> },
    BugsFound { bugs: Vec<BugReport> },
    RefactoringSuggested { suggestions: Vec<RefactoringSuggestion> },
    
    MessageProcessed { response: String, context: ConversationContext },
    ConversationHistory { messages: Vec<ConversationMessage> },
    ConversationHistoryCleared,
    
    ModelList { models: Vec<ModelInfo> },
    ModelInfo { info: ModelInfo },
    ModelLoaded { model_id: String },
    ModelUnloaded { model_id: String },
    
    PrivacySettingsSet,
    PrivacySettings { settings: PrivacySettings },
    DataExported { data: Vec<u8>, format: DataFormat },
    DataDeleted { data_type: DataType },
    
    PerformanceMetrics { metrics: AiMetrics },
    ResourceLimitsSet,
    ResourceUsage { usage: ResourceUsage },
}

/// AI capabilities
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AiCapability {
    TextGeneration,
    ContentAnalysis,
    SystemHelp,
    CodeAssistance,
    Debugging,
    Translation,
    Summarization,
    ConversationalAi,
    ImageAnalysis,
    AudioProcessing,
}

/// Session information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub session_id: u32,
    pub process_id: u32,
    pub capabilities: Vec<AiCapability>,
    pub created_at: u64,
    pub last_activity: u64,
    pub message_count: u32,
    pub model_id: Option<String>,
}

/// Text generation options
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationOptions {
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub frequency_penalty: Option<f32>,
    pub presence_penalty: Option<f32>,
    pub stop_sequences: Vec<String>,
    pub stream: bool,
}

/// Content analysis types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AnalysisType {
    Sentiment,
    Topics,
    KeyPhrases,
    Language,
    Toxicity,
    Readability,
    Structure,
    Metadata,
}

/// Analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub analysis_type: AnalysisType,
    pub confidence: f32,
    pub results: Vec<AnalysisItem>,
    pub metadata: AnalysisMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisItem {
    pub label: String,
    pub score: f32,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisMetadata {
    pub processing_time_ms: u32,
    pub model_used: String,
    pub input_size: usize,
}

/// System context for diagnostics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemContext {
    pub cpu_usage: f32,
    pub memory_usage: f32,
    pub disk_usage: f32,
    pub network_activity: bool,
    pub running_processes: Vec<String>,
    pub recent_errors: Vec<String>,
    pub system_logs: Vec<String>,
}

/// Performance data for optimization suggestions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceData {
    pub cpu_metrics: CpuMetrics,
    pub memory_metrics: MemoryMetrics,
    pub io_metrics: IoMetrics,
    pub network_metrics: NetworkMetrics,
    pub application_metrics: Vec<ApplicationMetric>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuMetrics {
    pub usage_percent: f32,
    pub load_average: [f32; 3],
    pub context_switches: u64,
    pub interrupts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryMetrics {
    pub total_mb: u32,
    pub used_mb: u32,
    pub cached_mb: u32,
    pub swap_used_mb: u32,
    pub page_faults: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IoMetrics {
    pub read_bytes_per_sec: u64,
    pub write_bytes_per_sec: u64,
    pub read_iops: u32,
    pub write_iops: u32,
    pub average_latency_us: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub rx_bytes_per_sec: u64,
    pub tx_bytes_per_sec: u64,
    pub rx_packets_per_sec: u32,
    pub tx_packets_per_sec: u32,
    pub latency_ms: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplicationMetric {
    pub process_name: String,
    pub cpu_usage: f32,
    pub memory_usage_mb: u32,
    pub io_usage: f32,
}

/// Diagnosis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosisResult {
    pub issue_type: IssueType,
    pub severity: IssueSeverity,
    pub description: String,
    pub possible_causes: Vec<String>,
    pub recommended_actions: Vec<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum IssueType {
    Performance,
    Memory,
    Disk,
    Network,
    Security,
    Configuration,
    Hardware,
    Software,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum IssueSeverity {
    Low,
    Medium,
    High,
    Critical,
}

/// Optimization suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationSuggestion {
    pub category: OptimizationCategory,
    pub title: String,
    pub description: String,
    pub expected_impact: ImpactLevel,
    pub implementation_difficulty: DifficultyLevel,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum OptimizationCategory {
    Cpu,
    Memory,
    Disk,
    Network,
    Power,
    Security,
    UserExperience,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ImpactLevel {
    Low,
    Medium,
    High,
    VeryHigh,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DifficultyLevel {
    Easy,
    Medium,
    Hard,
    Expert,
}

/// Code analysis result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAnalysisResult {
    pub language: String,
    pub complexity_score: f32,
    pub maintainability_score: f32,
    pub security_issues: Vec<SecurityIssue>,
    pub performance_issues: Vec<PerformanceIssue>,
    pub style_issues: Vec<StyleIssue>,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityIssue {
    pub severity: IssueSeverity,
    pub description: String,
    pub line_number: Option<u32>,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceIssue {
    pub impact: ImpactLevel,
    pub description: String,
    pub line_number: Option<u32>,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StyleIssue {
    pub description: String,
    pub line_number: Option<u32>,
    pub fix: Option<String>,
}

/// Bug report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugReport {
    pub bug_type: BugType,
    pub severity: IssueSeverity,
    pub description: String,
    pub line_number: Option<u32>,
    pub code_snippet: Option<String>,
    pub fix_suggestion: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BugType {
    LogicError,
    MemoryLeak,
    NullPointer,
    BufferOverflow,
    RaceCondition,
    DeadLock,
    ResourceLeak,
    TypeMismatch,
    UnhandledException,
}

/// Refactoring suggestion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefactoringSuggestion {
    pub refactoring_type: RefactoringType,
    pub description: String,
    pub benefits: Vec<String>,
    pub effort_estimate: DifficultyLevel,
    pub code_changes: Vec<CodeChange>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum RefactoringType {
    ExtractMethod,
    ExtractClass,
    RenameVariable,
    SimplifyExpression,
    RemoveDuplication,
    ImproveNaming,
    OptimizePerformance,
    ReduceComplexity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChange {
    pub file_path: String,
    pub line_number: u32,
    pub old_code: String,
    pub new_code: String,
    pub explanation: String,
}

/// Conversation context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationContext {
    pub topic: Option<String>,
    pub intent: Option<String>,
    pub entities: Vec<Entity>,
    pub sentiment: Option<f32>,
    pub previous_messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity {
    pub name: String,
    pub entity_type: String,
    pub value: String,
    pub confidence: f32,
}

/// Conversation message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConversationMessage {
    pub timestamp: u64,
    pub role: MessageRole,
    pub content: String,
    pub metadata: Option<MessageMetadata>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum MessageRole {
    User,
    Assistant,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageMetadata {
    pub processing_time_ms: u32,
    pub model_used: String,
    pub confidence: f32,
    pub tokens_used: u32,
}

/// Model information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub capabilities: Vec<AiCapability>,
    pub size_mb: u32,
    pub loaded: bool,
    pub performance_tier: PerformanceTier,
    pub privacy_level: PrivacyLevel,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PerformanceTier {
    Fast,
    Balanced,
    Accurate,
    Premium,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PrivacyLevel {
    Local,      // Fully local processing
    Hybrid,     // Some cloud features with consent
    Cloud,      // Cloud-based processing
}

/// Model configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub max_memory_mb: Option<u32>,
    pub cpu_threads: Option<u32>,
    pub gpu_acceleration: bool,
    pub quantization: Option<QuantizationType>,
    pub cache_size_mb: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum QuantizationType {
    Int8,
    Int4,
    Float16,
}

/// Privacy settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrivacySettings {
    pub data_retention_days: Option<u32>,
    pub allow_cloud_processing: bool,
    pub anonymize_data: bool,
    pub share_analytics: bool,
    pub local_only_mode: bool,
    pub encryption_enabled: bool,
}

/// Data formats for export
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DataFormat {
    Json,
    Csv,
    Xml,
    Binary,
}

/// Data types for deletion
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum DataType {
    ConversationHistory,
    UserPreferences,
    ModelCache,
    Analytics,
    All,
}

/// Resource limits
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub max_memory_mb: Option<u32>,
    pub max_cpu_percent: Option<f32>,
    pub max_disk_mb: Option<u32>,
    pub max_sessions: Option<u32>,
    pub max_requests_per_minute: Option<u32>,
}

/// Resource usage
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceUsage {
    pub memory_used_mb: u32,
    pub cpu_usage_percent: f32,
    pub disk_used_mb: u32,
    pub active_sessions: u32,
    pub requests_per_minute: u32,
}

/// Generation metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationMetadata {
    pub tokens_generated: u32,
    pub processing_time_ms: u32,
    pub model_used: String,
    pub finish_reason: FinishReason,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum FinishReason {
    Completed,
    MaxTokens,
    StopSequence,
    ContentFilter,
    Error,
}

/// AI service performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiMetrics {
    pub requests_processed: u64,
    pub average_response_time_ms: u32,
    pub p99_response_time_ms: u32,
    pub active_sessions: u32,
    pub memory_usage_mb: u32,
    pub cpu_usage_percent: f32,
    pub model_load_time_ms: u32,
    pub cache_hit_rate: f32,
    pub error_rate: f32,
}

/// Convenience type alias for AI service responses
pub type AiServiceResponse<T> = ServiceResponse<T>;

/// AI service error codes (extending common error codes)
pub mod ai_errors {
    use super::error_codes;
    
    pub const SESSION_NOT_FOUND: u32 = error_codes::INTERNAL_ERROR + 1;
    pub const MODEL_NOT_LOADED: u32 = error_codes::INTERNAL_ERROR + 2;
    pub const INSUFFICIENT_MEMORY: u32 = error_codes::INTERNAL_ERROR + 3;
    pub const MODEL_LOADING_FAILED: u32 = error_codes::INTERNAL_ERROR + 4;
    pub const GENERATION_FAILED: u32 = error_codes::INTERNAL_ERROR + 5;
    pub const ANALYSIS_FAILED: u32 = error_codes::INTERNAL_ERROR + 6;
    pub const UNSUPPORTED_CAPABILITY: u32 = error_codes::INTERNAL_ERROR + 7;
    pub const RATE_LIMIT_EXCEEDED: u32 = error_codes::INTERNAL_ERROR + 8;
    pub const CONTENT_FILTERED: u32 = error_codes::INTERNAL_ERROR + 9;
    pub const PRIVACY_VIOLATION: u32 = error_codes::INTERNAL_ERROR + 10;
}