//! RaeDE - Text/code editor for RaeenOS

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;

// Text buffer for editing
#[derive(Debug, Clone)]
struct TextBuffer {
    lines: Vec<String>,
    cursor_line: usize,
    cursor_column: usize,
    selection_start: Option<(usize, usize)>,
    selection_end: Option<(usize, usize)>,
    modified: bool,
    file_path: Option<String>,
}

impl TextBuffer {
    fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_column: 0,
            selection_start: None,
            selection_end: None,
            modified: false,
            file_path: None,
        }
    }
    
    fn from_text(text: &str) -> Self {
        let lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();
        let lines = if lines.is_empty() {
            vec![String::new()]
        } else {
            lines
        };
        
        Self {
            lines,
            cursor_line: 0,
            cursor_column: 0,
            selection_start: None,
            selection_end: None,
            modified: false,
            file_path: None,
        }
    }
    
    fn insert_char(&mut self, ch: char) {
        if self.cursor_line >= self.lines.len() {
            self.lines.push(String::new());
        }
        
        let line = &mut self.lines[self.cursor_line];
        if self.cursor_column > line.len() {
            self.cursor_column = line.len();
        }
        
        line.insert(self.cursor_column, ch);
        self.cursor_column += 1;
        self.modified = true;
    }
    
    fn insert_newline(&mut self) {
        if self.cursor_line >= self.lines.len() {
            self.lines.push(String::new());
        }
        
        let current_line = self.lines[self.cursor_line].clone();
        let split_pos = self.cursor_column.min(current_line.len());
        let (left, right) = current_line.split_at(split_pos);
        
        self.lines[self.cursor_line] = left.to_string();
        self.lines.insert(self.cursor_line + 1, right.to_string());
        
        self.cursor_line += 1;
        self.cursor_column = 0;
        self.modified = true;
    }
    
    fn delete_char(&mut self) {
        if self.cursor_line >= self.lines.len() {
            return;
        }
        
        let line = &mut self.lines[self.cursor_line];
        if self.cursor_column > 0 && self.cursor_column <= line.len() {
            line.remove(self.cursor_column - 1);
            self.cursor_column -= 1;
            self.modified = true;
        } else if self.cursor_column == 0 && self.cursor_line > 0 {
            // Join with previous line
            let current_line = self.lines.remove(self.cursor_line);
            self.cursor_line -= 1;
            self.cursor_column = self.lines[self.cursor_line].len();
            self.lines[self.cursor_line].push_str(&current_line);
            self.modified = true;
        }
    }
    
    fn move_cursor(&mut self, direction: CursorDirection) {
        match direction {
            CursorDirection::Up => {
                if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    let line_len = self.lines[self.cursor_line].len();
                    self.cursor_column = self.cursor_column.min(line_len);
                }
            }
            CursorDirection::Down => {
                if self.cursor_line + 1 < self.lines.len() {
                    self.cursor_line += 1;
                    let line_len = self.lines[self.cursor_line].len();
                    self.cursor_column = self.cursor_column.min(line_len);
                }
            }
            CursorDirection::Left => {
                if self.cursor_column > 0 {
                    self.cursor_column -= 1;
                } else if self.cursor_line > 0 {
                    self.cursor_line -= 1;
                    self.cursor_column = self.lines[self.cursor_line].len();
                }
            }
            CursorDirection::Right => {
                let line_len = if self.cursor_line < self.lines.len() {
                    self.lines[self.cursor_line].len()
                } else {
                    0
                };
                
                if self.cursor_column < line_len {
                    self.cursor_column += 1;
                } else if self.cursor_line + 1 < self.lines.len() {
                    self.cursor_line += 1;
                    self.cursor_column = 0;
                }
            }
            CursorDirection::Home => {
                self.cursor_column = 0;
            }
            CursorDirection::End => {
                if self.cursor_line < self.lines.len() {
                    self.cursor_column = self.lines[self.cursor_line].len();
                }
            }
        }
    }
    
    fn get_text(&self) -> String {
        self.lines.join("\n")
    }
    
    fn get_line(&self, line_num: usize) -> Option<&String> {
        self.lines.get(line_num)
    }
    
    fn line_count(&self) -> usize {
        self.lines.len()
    }
}

// Cursor movement directions
#[derive(Debug, Clone, Copy)]
enum CursorDirection {
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
}

// Editor modes
#[derive(Debug, Clone, Copy, PartialEq)]
enum EditorMode {
    Normal,
    Insert,
    Visual,
    Command,
}

// Syntax highlighting types
#[derive(Debug, Clone, Copy, PartialEq)]
enum SyntaxType {
    None,
    Rust,
    C,
    Python,
    JavaScript,
    Markdown,
}

// Editor configuration
#[derive(Debug, Clone)]
struct EditorConfig {
    tab_size: usize,
    use_spaces: bool,
    show_line_numbers: bool,
    syntax_highlighting: bool,
    auto_indent: bool,
    word_wrap: bool,
}

impl Default for EditorConfig {
    fn default() -> Self {
        Self {
            tab_size: 4,
            use_spaces: true,
            show_line_numbers: true,
            syntax_highlighting: true,
            auto_indent: true,
            word_wrap: false,
        }
    }
}

// Editor session
#[derive(Debug)]
struct EditorSession {
    session_id: u32,
    buffer: TextBuffer,
    mode: EditorMode,
    syntax_type: SyntaxType,
    config: EditorConfig,
    viewport_top: usize,
    viewport_height: usize,
    search_query: Option<String>,
    undo_stack: Vec<TextBuffer>,
    redo_stack: Vec<TextBuffer>,
}

impl EditorSession {
    fn new(session_id: u32) -> Self {
        Self {
            session_id,
            buffer: TextBuffer::new(),
            mode: EditorMode::Normal,
            syntax_type: SyntaxType::None,
            config: EditorConfig::default(),
            viewport_top: 0,
            viewport_height: 25,
            search_query: None,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        }
    }
    
    fn save_state(&mut self) {
        self.undo_stack.push(self.buffer.clone());
        if self.undo_stack.len() > 100 {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
    }
    
    fn undo(&mut self) -> bool {
        if let Some(previous_state) = self.undo_stack.pop() {
            self.redo_stack.push(self.buffer.clone());
            self.buffer = previous_state;
            true
        } else {
            false
        }
    }
    
    fn redo(&mut self) -> bool {
        if let Some(next_state) = self.redo_stack.pop() {
            self.undo_stack.push(self.buffer.clone());
            self.buffer = next_state;
            true
        } else {
            false
        }
    }
    
    fn detect_syntax_type(&mut self) {
        if let Some(ref path) = self.buffer.file_path {
            self.syntax_type = match path.split('.').last() {
                Some("rs") => SyntaxType::Rust,
                Some("c") | Some("h") => SyntaxType::C,
                Some("py") => SyntaxType::Python,
                Some("js") | Some("ts") => SyntaxType::JavaScript,
                Some("md") => SyntaxType::Markdown,
                _ => SyntaxType::None,
            };
        }
    }
}

// RaeDE editor system
struct RaeDeSystem {
    sessions: BTreeMap<u32, EditorSession>,
    next_session_id: u32,
    active_session: Option<u32>,
    recent_files: Vec<String>,
}

lazy_static! {
    static ref RAEDE_SYSTEM: Mutex<RaeDeSystem> = {
        Mutex::new(RaeDeSystem {
            sessions: BTreeMap::new(),
            next_session_id: 1,
            active_session: None,
            recent_files: Vec::new(),
        })
    };
}

// Initialize RaeDE editor
pub fn init_raede() -> Result<(), ()> {
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.init").unwrap_or(false) {
        return Err(());
    }
    
    // Create editor directories
    let _ = crate::fs::create_directory("/usr/share/raede");
    let _ = crate::fs::create_directory("/var/lib/raede");
    
    Ok(())
}

// Create a new editor session
pub fn create_session() -> Result<u32, ()> {
    let mut raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.session").unwrap_or(false) {
        return Err(());
    }
    
    let session_id = raede.next_session_id;
    raede.next_session_id += 1;
    
    let session = EditorSession::new(session_id);
    raede.sessions.insert(session_id, session);
    raede.active_session = Some(session_id);
    
    Ok(session_id)
}

// Open a file in the editor
pub fn open_file(session_id: u32, file_path: &str) -> Result<(), ()> {
    let mut raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.file.open").unwrap_or(false) {
        return Err(());
    }
    
    let session = raede.sessions.get_mut(&session_id).ok_or(())?;
    
    // Read file content
    match crate::fs::read_file(file_path) {
        Ok(content) => {
            let text = String::from_utf8_lossy(&content);
            session.buffer = TextBuffer::from_text(&text);
            session.buffer.file_path = Some(file_path.to_string());
            session.buffer.modified = false;
            session.detect_syntax_type();
            
            // Add to recent files
            if !raede.recent_files.contains(&file_path.to_string()) {
                raede.recent_files.push(file_path.to_string());
                if raede.recent_files.len() > 10 {
                    raede.recent_files.remove(0);
                }
            }
            
            Ok(())
        }
        Err(_) => Err(()),
    }
}

// Save the current buffer to file
pub fn save_file(session_id: u32, file_path: Option<&str>) -> Result<(), ()> {
    let mut raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.file.save").unwrap_or(false) {
        return Err(());
    }
    
    let session = raede.sessions.get_mut(&session_id).ok_or(())?;
    
    let path = if let Some(path) = file_path {
        path
    } else if let Some(ref path) = session.buffer.file_path {
        path
    } else {
        return Err(());
    };
    
    let content = session.buffer.get_text();
    match crate::fs::write_file(path, content.as_bytes()) {
        Ok(_) => {
            session.buffer.file_path = Some(path.to_string());
            session.buffer.modified = false;
            session.detect_syntax_type();
            Ok(())
        }
        Err(_) => Err(()),
    }
}

// Insert text at cursor position
pub fn insert_text(session_id: u32, text: &str) -> Result<(), ()> {
    let mut raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.edit").unwrap_or(false) {
        return Err(());
    }
    
    let session = raede.sessions.get_mut(&session_id).ok_or(())?;
    session.save_state();
    
    for ch in text.chars() {
        if ch == '\n' {
            session.buffer.insert_newline();
        } else {
            session.buffer.insert_char(ch);
        }
    }
    
    Ok(())
}

// Delete character at cursor
pub fn delete_char(session_id: u32) -> Result<(), ()> {
    let mut raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.edit").unwrap_or(false) {
        return Err(());
    }
    
    let session = raede.sessions.get_mut(&session_id).ok_or(())?;
    session.save_state();
    session.buffer.delete_char();
    
    Ok(())
}

// Move cursor
pub fn move_cursor(session_id: u32, direction: &str) -> Result<(), ()> {
    let mut raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.navigate").unwrap_or(false) {
        return Err(());
    }
    
    let session = raede.sessions.get_mut(&session_id).ok_or(())?;
    
    let cursor_direction = match direction {
        "up" => CursorDirection::Up,
        "down" => CursorDirection::Down,
        "left" => CursorDirection::Left,
        "right" => CursorDirection::Right,
        "home" => CursorDirection::Home,
        "end" => CursorDirection::End,
        _ => return Err(()),
    };
    
    session.buffer.move_cursor(cursor_direction);
    Ok(())
}

// Get cursor position
pub fn get_cursor_position(session_id: u32) -> Result<(usize, usize), ()> {
    let raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.info").unwrap_or(false) {
        return Err(());
    }
    
    let session = raede.sessions.get(&session_id).ok_or(())?;
    Ok((session.buffer.cursor_line, session.buffer.cursor_column))
}

// Get buffer content
pub fn get_buffer_content(session_id: u32) -> Result<String, ()> {
    let raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.read").unwrap_or(false) {
        return Err(());
    }
    
    let session = raede.sessions.get(&session_id).ok_or(())?;
    Ok(session.buffer.get_text())
}

// Get line content
pub fn get_line(session_id: u32, line_num: usize) -> Result<Option<String>, ()> {
    let raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.read").unwrap_or(false) {
        return Err(());
    }
    
    let session = raede.sessions.get(&session_id).ok_or(())?;
    Ok(session.buffer.get_line(line_num).cloned())
}

// Get line count
pub fn get_line_count(session_id: u32) -> Result<usize, ()> {
    let raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.info").unwrap_or(false) {
        return Err(());
    }
    
    let session = raede.sessions.get(&session_id).ok_or(())?;
    Ok(session.buffer.line_count())
}

// Undo last operation
pub fn undo(session_id: u32) -> Result<bool, ()> {
    let mut raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.edit").unwrap_or(false) {
        return Err(());
    }
    
    let session = raede.sessions.get_mut(&session_id).ok_or(())?;
    Ok(session.undo())
}

// Redo last undone operation
pub fn redo(session_id: u32) -> Result<bool, ()> {
    let mut raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.edit").unwrap_or(false) {
        return Err(());
    }
    
    let session = raede.sessions.get_mut(&session_id).ok_or(())?;
    Ok(session.redo())
}

// Search for text
pub fn search(session_id: u32, query: &str) -> Result<Vec<(usize, usize)>, ()> {
    let mut raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.search").unwrap_or(false) {
        return Err(());
    }
    
    let session = raede.sessions.get_mut(&session_id).ok_or(())?;
    session.search_query = Some(query.to_string());
    
    let mut matches = Vec::new();
    
    for (line_idx, line) in session.buffer.lines.iter().enumerate() {
        let mut start = 0;
        while let Some(pos) = line[start..].find(query) {
            matches.push((line_idx, start + pos));
            start += pos + 1;
        }
    }
    
    Ok(matches)
}

// Replace text
pub fn replace(session_id: u32, search: &str, replace: &str, replace_all: bool) -> Result<usize, ()> {
    let mut raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.edit").unwrap_or(false) {
        return Err(());
    }
    
    let session = raede.sessions.get_mut(&session_id).ok_or(())?;
    session.save_state();
    
    let mut replacements = 0;
    
    for line in &mut session.buffer.lines {
        if replace_all {
            let new_line = line.replace(search, replace);
            if new_line != *line {
                replacements += line.matches(search).count();
                *line = new_line;
                session.buffer.modified = true;
            }
        } else {
            if let Some(pos) = line.find(search) {
                line.replace_range(pos..pos + search.len(), replace);
                replacements += 1;
                session.buffer.modified = true;
                break;
            }
        }
    }
    
    Ok(replacements)
}

// Set editor mode
pub fn set_mode(session_id: u32, mode: &str) -> Result<(), ()> {
    let mut raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.mode").unwrap_or(false) {
        return Err(());
    }
    
    let session = raede.sessions.get_mut(&session_id).ok_or(())?;
    
    session.mode = match mode {
        "normal" => EditorMode::Normal,
        "insert" => EditorMode::Insert,
        "visual" => EditorMode::Visual,
        "command" => EditorMode::Command,
        _ => return Err(()),
    };
    
    Ok(())
}

// Get editor configuration
pub fn get_config(session_id: u32) -> Result<(usize, bool, bool, bool, bool, bool), ()> {
    let raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.config").unwrap_or(false) {
        return Err(());
    }
    
    let session = raede.sessions.get(&session_id).ok_or(())?;
    let config = &session.config;
    
    Ok((
        config.tab_size,
        config.use_spaces,
        config.show_line_numbers,
        config.syntax_highlighting,
        config.auto_indent,
        config.word_wrap,
    ))
}

// Set editor configuration
pub fn set_config(session_id: u32, tab_size: usize, use_spaces: bool, show_line_numbers: bool, 
                  syntax_highlighting: bool, auto_indent: bool, word_wrap: bool) -> Result<(), ()> {
    let mut raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.config").unwrap_or(false) {
        return Err(());
    }
    
    let session = raede.sessions.get_mut(&session_id).ok_or(())?;
    
    session.config.tab_size = tab_size;
    session.config.use_spaces = use_spaces;
    session.config.show_line_numbers = show_line_numbers;
    session.config.syntax_highlighting = syntax_highlighting;
    session.config.auto_indent = auto_indent;
    session.config.word_wrap = word_wrap;
    
    Ok(())
}

// Get recent files
pub fn get_recent_files() -> Result<Vec<String>, ()> {
    let raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.files").unwrap_or(false) {
        return Err(());
    }
    
    Ok(raede.recent_files.clone())
}

// Close editor session
pub fn close_session(session_id: u32) -> Result<(), ()> {
    let mut raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.session").unwrap_or(false) {
        return Err(());
    }
    
    raede.sessions.remove(&session_id);
    
    if raede.active_session == Some(session_id) {
        raede.active_session = raede.sessions.keys().next().copied();
    }
    
    Ok(())
}

// Get active session
pub fn get_active_session() -> Result<Option<u32>, ()> {
    let raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.session").unwrap_or(false) {
        return Err(());
    }
    
    Ok(raede.active_session)
}

// Set active session
pub fn set_active_session(session_id: u32) -> Result<(), ()> {
    let mut raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.session").unwrap_or(false) {
        return Err(());
    }
    
    if raede.sessions.contains_key(&session_id) {
        raede.active_session = Some(session_id);
        Ok(())
    } else {
        Err(())
    }
}

// Check if buffer is modified
pub fn is_modified(session_id: u32) -> Result<bool, ()> {
    let raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.info").unwrap_or(false) {
        return Err(());
    }
    
    let session = raede.sessions.get(&session_id).ok_or(())?;
    Ok(session.buffer.modified)
}

// Get file path
pub fn get_file_path(session_id: u32) -> Result<Option<String>, ()> {
    let raede = RAEDE_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "raede.info").unwrap_or(false) {
        return Err(());
    }
    
    let session = raede.sessions.get(&session_id).ok_or(())?;
    Ok(session.buffer.file_path.clone())
}

// Clean up RaeDE resources for a process
pub fn cleanup_process_raede(process_id: u32) {
    let mut raede = RAEDE_SYSTEM.lock();
    
    // In a real implementation, we would track which process owns which sessions
    // For now, we'll just clean up if there are no active processes
    let _ = process_id; // Suppress unused warning
    
    // Could implement session ownership tracking here
}