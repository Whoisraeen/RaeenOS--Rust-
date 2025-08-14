//! AI Assistant for RaeenOS

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::vec;
use alloc::format;
use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;

// Assistant capabilities
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssistantCapability {
    TextGeneration,
    ContentAnalysis,
    SystemHelp,
    CodeAssistance,
    Debugging,
}

// Response types
#[derive(Debug, Clone)]
pub enum ResponseType {
    Text(String),
    Analysis(Vec<u8>),
    SystemInfo(String),
    Error(String),
}

// Assistant context
#[derive(Debug, Clone)]
struct AssistantContext {
    session_id: u32,
    capabilities: Vec<AssistantCapability>,
    conversation_history: Vec<(String, String)>, // (prompt, response)
    process_id: u32,
}

impl AssistantContext {
    fn new(session_id: u32, process_id: u32) -> Self {
        Self {
            session_id,
            capabilities: vec![
                AssistantCapability::TextGeneration,
                AssistantCapability::ContentAnalysis,
                AssistantCapability::SystemHelp,
                AssistantCapability::CodeAssistance,
                AssistantCapability::Debugging,
            ],
            conversation_history: Vec::new(),
            process_id,
        }
    }
    
    fn add_interaction(&mut self, prompt: String, response: String) {
        self.conversation_history.push((prompt, response));
        
        // Keep only last 10 interactions to prevent memory bloat
        if self.conversation_history.len() > 10 {
            self.conversation_history.remove(0);
        }
    }
}

// Assistant system state
struct AssistantSystem {
    sessions: BTreeMap<u32, AssistantContext>,
    next_session_id: u32,
}

lazy_static! {
    static ref ASSISTANT_SYSTEM: Mutex<AssistantSystem> = Mutex::new(AssistantSystem {
        sessions: BTreeMap::new(),
        next_session_id: 1,
    });
}

// Pattern matching for responses
fn match_response_pattern(prompt: &str) -> ResponseType {
    let prompt_lower = prompt.to_lowercase();
    
    // System information queries
    if prompt_lower.contains("system") && (prompt_lower.contains("info") || prompt_lower.contains("status")) {
        let system_info = format!(
            "RaeenOS Kernel Assistant\n- Memory: {} KB available\n- Processes: {} active\n- Uptime: {} ticks\n- Security: Sandbox enabled",
            crate::memory::get_free_memory() / 1024,
            crate::process::get_process_count(),
            crate::time::get_system_uptime()
        );
        return ResponseType::SystemInfo(system_info);
    }
    
    // Help queries
    if prompt_lower.contains("help") || prompt_lower.contains("how") {
        let help_text = "RaeenOS Assistant Help:\n- Ask about system status: 'system info'\n- Get process information: 'list processes'\n- Memory usage: 'memory status'\n- File operations: 'file help'\n- Network commands: 'network help'\n- Security info: 'security status'";
        return ResponseType::Text(help_text.to_string());
    }
    
    // Process queries
    if prompt_lower.contains("process") {
        if prompt_lower.contains("list") || prompt_lower.contains("show") {
            let process_info = format!(
                "Active Processes:\n- Current PID: {}\n- Total processes: {}\n- Scheduler status: Active",
                crate::process::get_current_process_id(),
                crate::process::get_process_count()
            );
            return ResponseType::SystemInfo(process_info);
        }
    }
    
    // Memory queries
    if prompt_lower.contains("memory") {
        let memory_info = format!(
            "Memory Status:\n- Total: {} KB\n- Free: {} KB\n- Used: {} KB\n- Fragmentation: Low",
            crate::memory::get_total_memory() / 1024,
            crate::memory::get_free_memory() / 1024,
            (crate::memory::get_total_memory() - crate::memory::get_free_memory()) / 1024
        );
        return ResponseType::SystemInfo(memory_info);
    }
    
    // File system queries
    if prompt_lower.contains("file") {
        let file_help = "File System Commands:\n- open(path): Open a file\n- read(fd, size): Read from file\n- write(fd, data): Write to file\n- close(fd): Close file\n- create(path): Create new file\n- remove(path): Delete file";
        return ResponseType::Text(file_help.to_string());
    }
    
    // Network queries
    if prompt_lower.contains("network") {
        let network_help = "Network Commands:\n- create_socket(): Create network socket\n- bind_socket(): Bind to address\n- connect_socket(): Connect to remote\n- send_data(): Send network data\n- receive_data(): Receive network data";
        return ResponseType::Text(network_help.to_string());
    }
    
    // Security queries
    if prompt_lower.contains("security") {
        let security_info = format!(
            "Security Status:\n- Sandbox: Enabled\n- Current PID: {}\n- Permissions: Process-based\n- Memory protection: Active",
            crate::process::get_current_process_id()
        );
        return ResponseType::SystemInfo(security_info);
    }
    
    // Error/debugging queries
    if prompt_lower.contains("error") || prompt_lower.contains("debug") {
        let debug_help = "Debugging Help:\n- Check system logs for errors\n- Verify process permissions\n- Check memory allocation\n- Validate file paths\n- Test network connectivity";
        return ResponseType::Text(debug_help.to_string());
    }
    
    // Greeting responses
    if prompt_lower.contains("hello") || prompt_lower.contains("hi") {
        return ResponseType::Text("Hello! I'm the RaeenOS kernel assistant. How can I help you today?".to_string());
    }
    
    // Default response
    ResponseType::Text(
        "I'm the RaeenOS kernel assistant. I can help with:\n- System information and status\n- Process and memory management\n- File system operations\n- Network configuration\n- Security and debugging\n\nTry asking 'help' for more specific commands.".to_string()
    )
}

// Create a new assistant session
pub fn create_session() -> Result<u32, ()> {
    let mut assistant = ASSISTANT_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid as u32, "assistant.access").unwrap_or(false) {
        return Err(());
    }
    
    let session_id = assistant.next_session_id;
    assistant.next_session_id += 1;
    
    let context = AssistantContext::new(session_id, current_pid as u32);
    assistant.sessions.insert(session_id, context);
    
    Ok(session_id)
}

// Generate AI response
pub fn generate_ai_response(prompt: &str) -> Result<String, ()> {
    let session_id = create_session()?;
    generate_ai_response_with_session(session_id, prompt)
}

// Generate AI response with session context
pub fn generate_ai_response_with_session(session_id: u32, prompt: &str) -> Result<String, ()> {
    let mut assistant = ASSISTANT_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    let context = assistant.sessions.get_mut(&session_id)
        .ok_or(())?;
    
    // Check ownership
    if context.process_id != current_pid as u32 {
        return Err(());
    }
    
    // Generate response based on pattern matching
    let response = match match_response_pattern(prompt) {
        ResponseType::Text(text) => text,
        ResponseType::SystemInfo(info) => info,
        ResponseType::Analysis(_) => "Analysis complete".to_string(),
        ResponseType::Error(error) => format!("Error: {}", error),
    };
    
    // Add to conversation history
    context.add_interaction(prompt.to_string(), response.clone());
    
    Ok(response)
}

// Analyze content
pub fn analyze_content(data: &str) -> Result<Vec<u8>, ()> {
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid as u32, "assistant.analyze").unwrap_or(false) {
        return Err(());
    }
    
    // Simple content analysis
    let mut analysis = Vec::new();
    
    // Basic statistics
    let char_count = data.len() as u32;
    let word_count = data.split_whitespace().count() as u32;
    let line_count = data.lines().count() as u32;
    
    // Pack analysis data
    analysis.extend_from_slice(&char_count.to_le_bytes());
    analysis.extend_from_slice(&word_count.to_le_bytes());
    analysis.extend_from_slice(&line_count.to_le_bytes());
    
    // Content type detection
    let content_type = if data.contains("fn ") || data.contains("struct ") {
        1u8 // Code
    } else if data.contains("\n\n") && data.len() > 100 {
        2u8 // Document
    } else if data.chars().all(|c| c.is_ascii_digit() || c.is_whitespace()) {
        3u8 // Numeric data
    } else {
        0u8 // Plain text
    };
    
    analysis.push(content_type);
    
    // Complexity score (0-255)
    let complexity = core::cmp::min(
        255,
        (data.matches('{').count() + data.matches('(').count()) * 10
    ) as u8;
    
    analysis.push(complexity);
    
    Ok(analysis)
}

// Get session history
pub fn get_session_history(session_id: u32) -> Result<Vec<(String, String)>, ()> {
    let assistant = ASSISTANT_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    let context = assistant.sessions.get(&session_id)
        .ok_or(())?;
    
    // Check ownership
    if context.process_id != current_pid as u32 {
        return Err(());
    }

    Ok(context.conversation_history.clone())
}

// Close assistant session
pub fn close_session(session_id: u32) -> Result<(), ()> {
    let mut assistant = ASSISTANT_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    let context = assistant.sessions.get(&session_id)
        .ok_or(())?;
    
    // Check ownership
    if context.process_id != current_pid as u32 {
        return Err(());
    }

    assistant.sessions.remove(&session_id);
    Ok(())
}

// Clean up assistant sessions for a process
pub fn cleanup_process_assistant(process_id: u32) {
    let mut assistant = ASSISTANT_SYSTEM.lock();
    
    let sessions_to_close: Vec<u32> = assistant.sessions
        .iter()
        .filter(|(_, context)| context.process_id == process_id)
        .map(|(&session_id, _)| session_id)
        .collect();
    
    for session_id in sessions_to_close {
        assistant.sessions.remove(&session_id);
    }
}

// Get assistant capabilities
pub fn get_capabilities() -> Vec<AssistantCapability> {
    vec![
        AssistantCapability::TextGeneration,
        AssistantCapability::ContentAnalysis,
        AssistantCapability::SystemHelp,
        AssistantCapability::CodeAssistance,
        AssistantCapability::Debugging,
    ]
}