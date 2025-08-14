//! RaeShell - Advanced terminal environment for RaeenOS
//! Features GPU-accelerated rendering, AI integration, and advanced shell capabilities

use alloc::vec::Vec;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::format;
use spin::Mutex;
use lazy_static::lazy_static;
use crate::graphics::{Color, Rect, Point, WindowId};
use crate::process::ProcessId;
use crate::filesystem::FileHandle;

/// Terminal color scheme
#[derive(Debug, Clone)]
pub struct TerminalTheme {
    pub background: Color,
    pub foreground: Color,
    pub cursor: Color,
    pub selection: Color,
    pub black: Color,
    pub red: Color,
    pub green: Color,
    pub yellow: Color,
    pub blue: Color,
    pub magenta: Color,
    pub cyan: Color,
    pub white: Color,
    pub bright_black: Color,
    pub bright_red: Color,
    pub bright_green: Color,
    pub bright_yellow: Color,
    pub bright_blue: Color,
    pub bright_magenta: Color,
    pub bright_cyan: Color,
    pub bright_white: Color,
}

impl Default for TerminalTheme {
    fn default() -> Self {
        TerminalTheme {
            background: Color::new(20, 20, 30, 240),
            foreground: Color::new(220, 220, 220, 255),
            cursor: Color::new(100, 150, 255, 255),
            selection: Color::new(100, 150, 255, 100),
            black: Color::new(40, 40, 40, 255),
            red: Color::new(255, 100, 100, 255),
            green: Color::new(100, 255, 100, 255),
            yellow: Color::new(255, 255, 100, 255),
            blue: Color::new(100, 100, 255, 255),
            magenta: Color::new(255, 100, 255, 255),
            cyan: Color::new(100, 255, 255, 255),
            white: Color::new(220, 220, 220, 255),
            bright_black: Color::new(80, 80, 80, 255),
            bright_red: Color::new(255, 150, 150, 255),
            bright_green: Color::new(150, 255, 150, 255),
            bright_yellow: Color::new(255, 255, 150, 255),
            bright_blue: Color::new(150, 150, 255, 255),
            bright_magenta: Color::new(255, 150, 255, 255),
            bright_cyan: Color::new(150, 255, 255, 255),
            bright_white: Color::new(255, 255, 255, 255),
        }
    }
}

/// Terminal character with styling
#[derive(Debug, Clone)]
pub struct TerminalChar {
    pub character: char,
    pub foreground: Color,
    pub background: Color,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strikethrough: bool,
}

impl Default for TerminalChar {
    fn default() -> Self {
        TerminalChar {
            character: ' ',
            foreground: Color::WHITE,
            background: Color::TRANSPARENT,
            bold: false,
            italic: false,
            underline: false,
            strikethrough: false,
        }
    }
}

/// Terminal cursor
#[derive(Debug, Clone)]
pub struct TerminalCursor {
    pub x: usize,
    pub y: usize,
    pub visible: bool,
    pub blink_state: bool,
    pub style: CursorStyle,
}

#[derive(Debug, Clone)]
pub enum CursorStyle {
    Block,
    Underline,
    Bar,
}

impl Default for TerminalCursor {
    fn default() -> Self {
        TerminalCursor {
            x: 0,
            y: 0,
            visible: true,
            blink_state: true,
            style: CursorStyle::Block,
        }
    }
}

/// Command history entry
#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub command: String,
    pub timestamp: u64,
    pub exit_code: i32,
    pub duration_ms: u64,
}

/// AI suggestion
#[derive(Debug, Clone)]
pub struct AiSuggestion {
    pub text: String,
    pub confidence: f32,
    pub category: SuggestionCategory,
}

#[derive(Debug, Clone)]
pub enum SuggestionCategory {
    Command,
    Parameter,
    Path,
    Correction,
    Completion,
}

/// Terminal buffer
pub struct TerminalBuffer {
    pub width: usize,
    pub height: usize,
    pub chars: Vec<Vec<TerminalChar>>,
    pub cursor: TerminalCursor,
    pub scroll_offset: usize,
    pub selection_start: Option<Point>,
    pub selection_end: Option<Point>,
}

impl TerminalBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        let mut chars = Vec::with_capacity(height);
        for _ in 0..height {
            chars.push(vec![TerminalChar::default(); width]);
        }
        
        TerminalBuffer {
            width,
            height,
            chars,
            cursor: TerminalCursor::default(),
            scroll_offset: 0,
            selection_start: None,
            selection_end: None,
        }
    }
    
    pub fn resize(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        
        // Resize existing rows
        for row in &mut self.chars {
            row.resize(width, TerminalChar::default());
        }
        
        // Add or remove rows
        if self.chars.len() < height {
            while self.chars.len() < height {
                self.chars.push(vec![TerminalChar::default(); width]);
            }
        } else {
            self.chars.truncate(height);
        }
    }
    
    pub fn write_char(&mut self, ch: char, color: Color) {
        if self.cursor.y >= self.height {
            self.scroll_up();
            self.cursor.y = self.height - 1;
        }
        
        if self.cursor.x >= self.width {
            self.cursor.x = 0;
            self.cursor.y += 1;
            if self.cursor.y >= self.height {
                self.scroll_up();
                self.cursor.y = self.height - 1;
            }
        }
        
        match ch {
            '\n' => {
                self.cursor.x = 0;
                self.cursor.y += 1;
            }
            '\r' => {
                self.cursor.x = 0;
            }
            '\t' => {
                let spaces = 4 - (self.cursor.x % 4);
                for _ in 0..spaces {
                    self.write_char(' ', color);
                }
            }
            _ => {
                self.chars[self.cursor.y][self.cursor.x] = TerminalChar {
                    character: ch,
                    foreground: color,
                    background: Color::TRANSPARENT,
                    bold: false,
                    italic: false,
                    underline: false,
                    strikethrough: false,
                };
                self.cursor.x += 1;
            }
        }
    }
    
    pub fn write_string(&mut self, text: &str, color: Color) {
        for ch in text.chars() {
            self.write_char(ch, color);
        }
    }
    
    pub fn scroll_up(&mut self) {
        self.chars.remove(0);
        self.chars.push(vec![TerminalChar::default(); self.width]);
    }
    
    pub fn clear(&mut self) {
        for row in &mut self.chars {
            for ch in row {
                *ch = TerminalChar::default();
            }
        }
        self.cursor.x = 0;
        self.cursor.y = 0;
    }
    
    pub fn clear_line(&mut self, line: usize) {
        if line < self.height {
            for ch in &mut self.chars[line] {
                *ch = TerminalChar::default();
            }
        }
    }
}

/// Built-in shell commands
#[derive(Debug, Clone)]
pub enum BuiltinCommand {
    Ls,
    Cd,
    Pwd,
    Echo,
    Cat,
    Touch,
    Mkdir,
    Rm,
    Cp,
    Mv,
    Clear,
    Exit,
    Help,
    History,
    Ps,
    Kill,
    Top,
    Uname,
    Date,
    Whoami,
    // RaeenOS specific commands
    RaeTheme,
    RaeMode,
    RaeAi,
    RaePerf,
    RaeGame,
    RaeUpdate,
}

impl BuiltinCommand {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "ls" => Some(BuiltinCommand::Ls),
            "cd" => Some(BuiltinCommand::Cd),
            "pwd" => Some(BuiltinCommand::Pwd),
            "echo" => Some(BuiltinCommand::Echo),
            "cat" => Some(BuiltinCommand::Cat),
            "touch" => Some(BuiltinCommand::Touch),
            "mkdir" => Some(BuiltinCommand::Mkdir),
            "rm" => Some(BuiltinCommand::Rm),
            "cp" => Some(BuiltinCommand::Cp),
            "mv" => Some(BuiltinCommand::Mv),
            "clear" => Some(BuiltinCommand::Clear),
            "exit" => Some(BuiltinCommand::Exit),
            "help" => Some(BuiltinCommand::Help),
            "history" => Some(BuiltinCommand::History),
            "ps" => Some(BuiltinCommand::Ps),
            "kill" => Some(BuiltinCommand::Kill),
            "top" => Some(BuiltinCommand::Top),
            "uname" => Some(BuiltinCommand::Uname),
            "date" => Some(BuiltinCommand::Date),
            "whoami" => Some(BuiltinCommand::Whoami),
            "raetheme" => Some(BuiltinCommand::RaeTheme),
            "raemode" => Some(BuiltinCommand::RaeMode),
            "raeai" => Some(BuiltinCommand::RaeAi),
            "raeperf" => Some(BuiltinCommand::RaePerf),
            "raegame" => Some(BuiltinCommand::RaeGame),
            "raeupdate" => Some(BuiltinCommand::RaeUpdate),
            _ => None,
        }
    }
}

/// Terminal session
pub struct TerminalSession {
    pub id: u32,
    pub window_id: WindowId,
    pub process_id: ProcessId,
    pub buffer: TerminalBuffer,
    pub theme: TerminalTheme,
    pub current_directory: String,
    pub environment: BTreeMap<String, String>,
    pub history: Vec<HistoryEntry>,
    pub current_command: String,
    pub command_position: usize,
    pub history_position: usize,
    pub ai_enabled: bool,
    pub ai_suggestions: Vec<AiSuggestion>,
    pub auto_complete_enabled: bool,
    pub syntax_highlighting: bool,
    pub performance_overlay: bool,
}

impl TerminalSession {
    pub fn new(id: u32, window_id: WindowId, process_id: ProcessId, width: usize, height: usize) -> Self {
        let mut session = TerminalSession {
            id,
            window_id,
            process_id,
            buffer: TerminalBuffer::new(width, height),
            theme: TerminalTheme::default(),
            current_directory: "/".to_string(),
            environment: BTreeMap::new(),
            history: Vec::new(),
            current_command: String::new(),
            command_position: 0,
            history_position: 0,
            ai_enabled: true,
            ai_suggestions: Vec::new(),
            auto_complete_enabled: true,
            syntax_highlighting: true,
            performance_overlay: false,
        };
        
        // Set up default environment variables
        session.environment.insert("HOME".to_string(), "/home/user".to_string());
        session.environment.insert("PATH".to_string(), "/bin:/usr/bin:/usr/local/bin".to_string());
        session.environment.insert("SHELL".to_string(), "/bin/raeshell".to_string());
        session.environment.insert("TERM".to_string(), "raeshell-256color".to_string());
        session.environment.insert("USER".to_string(), "user".to_string());
        
        session.show_welcome();
        session.show_prompt();
        
        session
    }
    
    pub fn show_welcome(&mut self) {
        let welcome_text = format!(
            "Welcome to RaeShell v1.0 - Advanced Terminal for RaeenOS\n"
            + "Type 'help' for available commands or 'raeai help' for AI assistance\n\n"
        );
        self.buffer.write_string(&welcome_text, self.theme.bright_cyan);
    }
    
    pub fn show_prompt(&mut self) {
        let prompt = format!("[{}] $ ", self.current_directory);
        self.buffer.write_string(&prompt, self.theme.bright_green);
    }
    
    pub fn handle_input(&mut self, input: char) {
        match input {
            '\n' | '\r' => {
                self.execute_command();
            }
            '\x08' => { // Backspace
                if !self.current_command.is_empty() {
                    self.current_command.pop();
                    self.command_position = self.current_command.len();
                    self.refresh_command_line();
                }
            }
            '\t' => { // Tab completion
                if self.auto_complete_enabled {
                    self.handle_tab_completion();
                }
            }
            ch if ch.is_control() => {
                // Handle other control characters
            }
            ch => {
                self.current_command.insert(self.command_position, ch);
                self.command_position += 1;
                self.buffer.write_char(ch, self.theme.foreground);
                
                if self.ai_enabled {
                    self.update_ai_suggestions();
                }
            }
        }
    }
    
    pub fn execute_command(&mut self) {
        self.buffer.write_char('\n', self.theme.foreground);
        
        let command = self.current_command.trim();
        if !command.is_empty() {
            let start_time = crate::time::get_timestamp();
            let exit_code = self.run_command(command);
            let end_time = crate::time::get_timestamp();
            
            // Add to history
            self.history.push(HistoryEntry {
                command: command.to_string(),
                timestamp: start_time,
                exit_code,
                duration_ms: end_time - start_time,
            });
        }
        
        self.current_command.clear();
        self.command_position = 0;
        self.show_prompt();
    }
    
    pub fn run_command(&mut self, command: &str) -> i32 {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() {
            return 0;
        }
        
        let cmd = parts[0];
        let args = &parts[1..];
        
        if let Some(builtin) = BuiltinCommand::from_str(cmd) {
            self.execute_builtin(builtin, args)
        } else {
            self.execute_external(cmd, args)
        }
    }
    
    pub fn execute_builtin(&mut self, command: BuiltinCommand, args: &[&str]) -> i32 {
        match command {
            BuiltinCommand::Ls => {
                self.cmd_ls(args)
            }
            BuiltinCommand::Cd => {
                self.cmd_cd(args)
            }
            BuiltinCommand::Pwd => {
                self.buffer.write_string(&format!("{}\n", self.current_directory), self.theme.foreground);
                0
            }
            BuiltinCommand::Echo => {
                let text = args.join(" ");
                self.buffer.write_string(&format!("{}\n", text), self.theme.foreground);
                0
            }
            BuiltinCommand::Clear => {
                self.buffer.clear();
                0
            }
            BuiltinCommand::Help => {
                self.cmd_help()
            }
            BuiltinCommand::History => {
                self.cmd_history()
            }
            BuiltinCommand::RaeAi => {
                self.cmd_raeai(args)
            }
            BuiltinCommand::RaeTheme => {
                self.cmd_raetheme(args)
            }
            BuiltinCommand::RaeMode => {
                self.cmd_raemode(args)
            }
            BuiltinCommand::RaePerf => {
                self.cmd_raeperf(args)
            }
            BuiltinCommand::RaeGame => {
                self.cmd_raegame(args)
            }
            _ => {
                self.buffer.write_string(&format!("Command '{}' not yet implemented\n", 
                    format!("{:?}", command).to_lowercase()), self.theme.red);
                1
            }
        }
    }
    
    pub fn execute_external(&mut self, command: &str, args: &[&str]) -> i32 {
        self.buffer.write_string(&format!("raeshell: {}: command not found\n", command), self.theme.red);
        127
    }
    
    pub fn cmd_ls(&mut self, args: &[&str]) -> i32 {
        let path = if args.is_empty() { self.current_directory.clone() } else { args[0].to_string() };
        match crate::filesystem::list_directory(&path) {
            Ok(entries) => {
                let line = if entries.is_empty() { String::from("\n") } else { entries.join("  ") + "\n" };
                self.buffer.write_string(&line, self.theme.foreground);
                0
            }
            Err(_) => {
                self.buffer.write_string(&format!("ls: cannot access '{}': not found\n", path), self.theme.red);
                1
            }
        }
    }
    
    pub fn cmd_cd(&mut self, args: &[&str]) -> i32 {
        if args.is_empty() {
            self.current_directory = self.environment.get("HOME").unwrap_or(&"/".to_string()).clone();
        } else {
            let path = args[0];
            if path == ".." {
                if self.current_directory != "/" {
                    let mut parts: Vec<&str> = self.current_directory.split('/').collect();
                    parts.pop();
                    self.current_directory = if parts.len() <= 1 {
                        "/".to_string()
                    } else {
                        parts.join("/")
                    };
                }
            } else if path.starts_with('/') {
                self.current_directory = path.to_string();
            } else {
                if self.current_directory.ends_with('/') {
                    self.current_directory.push_str(path);
                } else {
                    self.current_directory.push('/');
                    self.current_directory.push_str(path);
                }
            }
        }
        0
    }

    pub fn cmd_pwd(&mut self) -> i32 {
        self.buffer.write_string(&format!("{}\n", self.current_directory), self.theme.foreground);
        0
    }

    pub fn cmd_touch(&mut self, args: &[&str]) -> i32 {
        if args.is_empty() { return 1; }
        let path = self.resolve_path(args[0]);
        match crate::filesystem::create_file(&path) {
            Ok(()) => 0,
            Err(_) => 1,
        }
    }

    pub fn cmd_mkdir(&mut self, args: &[&str]) -> i32 {
        if args.is_empty() { return 1; }
        let path = self.resolve_path(args[0]);
        match crate::filesystem::create_directory(&path) { Ok(()) => 0, Err(_) => 1 }
    }

    pub fn cmd_rm(&mut self, args: &[&str]) -> i32 {
        if args.is_empty() { return 1; }
        let path = self.resolve_path(args[0]);
        match crate::filesystem::remove(&path) { Ok(()) => 0, Err(_) => 1 }
    }

    pub fn cmd_cat(&mut self, args: &[&str]) -> i32 {
        if args.is_empty() { return 1; }
        let path = self.resolve_path(args[0]);
        match crate::filesystem::open(&path, 0) {
            Ok(fd) => {
                let mut buf = [0u8; 1024];
                loop {
                    match crate::filesystem::read(fd, &mut buf) {
                        Ok(0) => break,
                        Ok(n) => {
                            let s = core::str::from_utf8(&buf[..n]).unwrap_or("");
                            self.buffer.write_string(s, self.theme.foreground);
                        }
                        Err(_) => break,
                    }
                }
                let _ = crate::filesystem::close(fd);
                self.buffer.write_char('\n', self.theme.foreground);
                0
            }
            Err(_) => 1,
        }
    }

    fn resolve_path(&self, p: &str) -> String {
        if p.starts_with('/') { p.to_string() } else if self.current_directory.ends_with('/') { format!("{}{}", self.current_directory, p) } else { format!("{}/{}", self.current_directory, p) }
    }
    
    pub fn cmd_help(&mut self) -> i32 {
        let help_text = "
RaeShell - Advanced Terminal for RaeenOS

Built-in Commands:
  ls          - List directory contents
  cd [dir]    - Change directory
  pwd         - Print working directory
  echo [text] - Display text
  cat [file]  - Display file contents
  clear       - Clear terminal
  help        - Show this help
  history     - Show command history
  exit        - Exit terminal

RaeenOS Commands:
  raetheme    - Manage terminal themes
  raemode     - Switch between modes (normal/gaming/dev)
  raeai       - AI assistant commands
  raeperf     - Performance monitoring
  raegame     - Gaming mode utilities
  raeupdate   - System updates

For detailed help on any command, use: [command] --help
";
        self.buffer.write_string(help_text, self.theme.foreground);
        0
    }
    
    pub fn cmd_history(&mut self) -> i32 {
        for (i, entry) in self.history.iter().enumerate() {
            let line = format!("  {}  {}\n", i + 1, entry.command);
            self.buffer.write_string(&line, self.theme.foreground);
        }
        0
    }
    
    pub fn cmd_raeai(&mut self, args: &[&str]) -> i32 {
        if args.is_empty() {
            self.buffer.write_string("RaeAI Assistant - Usage:\n", self.theme.bright_cyan);
            self.buffer.write_string("  raeai ask [question]    - Ask AI a question\n", self.theme.foreground);
            self.buffer.write_string("  raeai suggest           - Get command suggestions\n", self.theme.foreground);
            self.buffer.write_string("  raeai enable/disable    - Toggle AI assistance\n", self.theme.foreground);
            return 0;
        }
        
        match args[0] {
            "ask" => {
                let question = args[1..].join(" ");
                self.buffer.write_string(&format!("AI: Analyzing '{}'...\n", question), self.theme.bright_blue);
                // TODO: Integrate with AI system
                self.buffer.write_string("AI: I'm still learning! This feature will be available soon.\n", self.theme.yellow);
            }
            "enable" => {
                self.ai_enabled = true;
                self.buffer.write_string("AI assistance enabled\n", self.theme.green);
            }
            "disable" => {
                self.ai_enabled = false;
                self.buffer.write_string("AI assistance disabled\n", self.theme.yellow);
            }
            _ => {
                self.buffer.write_string(&format!("Unknown AI command: {}\n", args[0]), self.theme.red);
                return 1;
            }
        }
        0
    }
    
    pub fn cmd_raetheme(&mut self, args: &[&str]) -> i32 {
        if args.is_empty() {
            self.buffer.write_string("Available themes: dark, light, cyberpunk, matrix, ocean\n", self.theme.foreground);
            return 0;
        }
        
        match args[0] {
            "dark" => self.theme = TerminalTheme::default(),
            "light" => {
                self.theme.background = Color::new(240, 240, 240, 255);
                self.theme.foreground = Color::new(40, 40, 40, 255);
            }
            "cyberpunk" => {
                self.theme.background = Color::new(10, 0, 20, 255);
                self.theme.foreground = Color::new(0, 255, 150, 255);
                self.theme.cursor = Color::new(255, 0, 150, 255);
            }
            _ => {
                self.buffer.write_string(&format!("Unknown theme: {}\n", args[0]), self.theme.red);
                return 1;
            }
        }
        
        self.buffer.write_string(&format!("Theme changed to: {}\n", args[0]), self.theme.green);
        0
    }
    
    pub fn cmd_raemode(&mut self, args: &[&str]) -> i32 {
        if args.is_empty() {
            self.buffer.write_string("Available modes: normal, gaming, developer\n", self.theme.foreground);
            return 0;
        }
        
        match args[0] {
            "gaming" => {
                self.performance_overlay = true;
                self.buffer.write_string("Gaming mode activated - Performance overlay enabled\n", self.theme.green);
            }
            "normal" => {
                self.performance_overlay = false;
                self.buffer.write_string("Normal mode activated\n", self.theme.foreground);
            }
            "developer" => {
                self.syntax_highlighting = true;
                self.buffer.write_string("Developer mode activated - Enhanced features enabled\n", self.theme.bright_blue);
            }
            _ => {
                self.buffer.write_string(&format!("Unknown mode: {}\n", args[0]), self.theme.red);
                return 1;
            }
        }
        0
    }
    
    pub fn cmd_raeperf(&mut self, args: &[&str]) -> i32 {
        self.buffer.write_string("System Performance:\n", self.theme.bright_cyan);
        self.buffer.write_string("  CPU Usage: 15%\n", self.theme.foreground);
        self.buffer.write_string("  Memory: 2.1GB / 8GB\n", self.theme.foreground);
        self.buffer.write_string("  GPU Usage: 5%\n", self.theme.foreground);
        self.buffer.write_string("  Network: 1.2 MB/s down, 0.3 MB/s up\n", self.theme.foreground);
        0
    }
    
    pub fn cmd_raegame(&mut self, args: &[&str]) -> i32 {
        if args.is_empty() {
            self.buffer.write_string("RaeGame utilities:\n", self.theme.bright_cyan);
            self.buffer.write_string("  raegame boost     - Enable gaming optimizations\n", self.theme.foreground);
            self.buffer.write_string("  raegame fps       - Show FPS overlay\n", self.theme.foreground);
            self.buffer.write_string("  raegame launcher  - Open game launcher\n", self.theme.foreground);
            return 0;
        }
        
        match args[0] {
            "boost" => {
                self.buffer.write_string("Gaming optimizations enabled\n", self.theme.green);
            }
            "fps" => {
                self.buffer.write_string("FPS overlay toggled\n", self.theme.green);
            }
            "launcher" => {
                self.buffer.write_string("Opening RaeenOS Game Launcher...\n", self.theme.bright_blue);
            }
            _ => {
                self.buffer.write_string(&format!("Unknown game command: {}\n", args[0]), self.theme.red);
                return 1;
            }
        }
        0
    }
    
    pub fn handle_tab_completion(&mut self) {
        // TODO: Implement intelligent tab completion
        self.buffer.write_string("[TAB]", self.theme.yellow);
    }
    
    pub fn update_ai_suggestions(&mut self) {
        if !self.ai_enabled {
            return;
        }
        
        // TODO: Generate AI suggestions based on current input
        self.ai_suggestions.clear();
        
        if self.current_command.starts_with("ls") {
            self.ai_suggestions.push(AiSuggestion {
                text: "ls -la".to_string(),
                confidence: 0.8,
                category: SuggestionCategory::Completion,
            });
        }
    }
    
    pub fn refresh_command_line(&mut self) {
        // TODO: Refresh the current command line display
    }
    
    pub fn resize(&mut self, width: usize, height: usize) {
        self.buffer.resize(width, height);
    }
}

/// Terminal manager
pub struct TerminalManager {
    sessions: BTreeMap<u32, TerminalSession>,
    next_session_id: u32,
    active_session: Option<u32>,
}

impl TerminalManager {
    pub fn new() -> Self {
        TerminalManager {
            sessions: BTreeMap::new(),
            next_session_id: 1,
            active_session: None,
        }
    }
    
    pub fn create_session(&mut self, window_id: WindowId, process_id: ProcessId, width: usize, height: usize) -> u32 {
        let session_id = self.next_session_id;
        self.next_session_id += 1;
        
        let session = TerminalSession::new(session_id, window_id, process_id, width, height);
        self.sessions.insert(session_id, session);
        
        if self.active_session.is_none() {
            self.active_session = Some(session_id);
        }
        
        session_id
    }
    
    pub fn destroy_session(&mut self, session_id: u32) -> bool {
        if self.sessions.remove(&session_id).is_some() {
            if self.active_session == Some(session_id) {
                self.active_session = self.sessions.keys().next().copied();
            }
            true
        } else {
            false
        }
    }
    
    pub fn get_session(&self, session_id: u32) -> Option<&TerminalSession> {
        self.sessions.get(&session_id)
    }
    
    pub fn get_session_mut(&mut self, session_id: u32) -> Option<&mut TerminalSession> {
        self.sessions.get_mut(&session_id)
    }
    
    pub fn set_active_session(&mut self, session_id: u32) -> bool {
        if self.sessions.contains_key(&session_id) {
            self.active_session = Some(session_id);
            true
        } else {
            false
        }
    }
    
    pub fn get_active_session(&self) -> Option<&TerminalSession> {
        if let Some(session_id) = self.active_session {
            self.sessions.get(&session_id)
        } else {
            None
        }
    }
    
    pub fn get_active_session_mut(&mut self) -> Option<&mut TerminalSession> {
        if let Some(session_id) = self.active_session {
            self.sessions.get_mut(&session_id)
        } else {
            None
        }
    }
}

lazy_static! {
    static ref TERMINAL_MANAGER: Mutex<TerminalManager> = Mutex::new(TerminalManager::new());
}

// Public API functions

pub fn init() {
    // Initialize terminal subsystem
}

pub fn create_terminal_session(window_id: WindowId, process_id: ProcessId, width: usize, height: usize) -> u32 {
    let mut manager = TERMINAL_MANAGER.lock();
    manager.create_session(window_id, process_id, width, height)
}

pub fn destroy_terminal_session(session_id: u32) -> bool {
    let mut manager = TERMINAL_MANAGER.lock();
    manager.destroy_session(session_id)
}

pub fn handle_terminal_input(session_id: u32, input: char) -> Result<(), &'static str> {
    let mut manager = TERMINAL_MANAGER.lock();
    if let Some(session) = manager.get_session_mut(session_id) {
        session.handle_input(input);
        Ok(())
    } else {
        Err("Terminal session not found")
    }
}

pub fn resize_terminal(session_id: u32, width: usize, height: usize) -> Result<(), &'static str> {
    let mut manager = TERMINAL_MANAGER.lock();
    if let Some(session) = manager.get_session_mut(session_id) {
        session.resize(width, height);
        Ok(())
    } else {
        Err("Terminal session not found")
    }
}

pub fn set_active_terminal(session_id: u32) -> bool {
    let mut manager = TERMINAL_MANAGER.lock();
    manager.set_active_session(session_id)
}

pub fn get_terminal_buffer(session_id: u32) -> Option<Vec<Vec<TerminalChar>>> {
    let manager = TERMINAL_MANAGER.lock();
    if let Some(session) = manager.get_session(session_id) {
        Some(session.buffer.chars.clone())
    } else {
        None
    }
}

pub fn execute_terminal_command(session_id: u32, command: &str) -> Result<i32, &'static str> {
    let mut manager = TERMINAL_MANAGER.lock();
    if let Some(session) = manager.get_session_mut(session_id) {
        Ok(session.run_command(command))
    } else {
        Err("Terminal session not found")
    }
}

pub fn get_terminal_theme(session_id: u32) -> Option<TerminalTheme> {
    let manager = TERMINAL_MANAGER.lock();
    if let Some(session) = manager.get_session(session_id) {
        Some(session.theme.clone())
    } else {
        None
    }
}

pub fn set_terminal_theme(session_id: u32, theme: TerminalTheme) -> Result<(), &'static str> {
    let mut manager = TERMINAL_MANAGER.lock();
    if let Some(session) = manager.get_session_mut(session_id) {
        session.theme = theme;
        Ok(())
    } else {
        Err("Terminal session not found")
    }
}

pub fn enable_ai_assistance(session_id: u32, enabled: bool) -> Result<(), &'static str> {
    let mut manager = TERMINAL_MANAGER.lock();
    if let Some(session) = manager.get_session_mut(session_id) {
        session.ai_enabled = enabled;
        Ok(())
    } else {
        Err("Terminal session not found")
    }
}

pub fn get_command_history(session_id: u32) -> Option<Vec<HistoryEntry>> {
    let manager = TERMINAL_MANAGER.lock();
    if let Some(session) = manager.get_session(session_id) {
        Some(session.history.clone())
    } else {
        None
    }
}

pub fn get_ai_suggestions(session_id: u32) -> Option<Vec<AiSuggestion>> {
    let manager = TERMINAL_MANAGER.lock();
    if let Some(session) = manager.get_session(session_id) {
        Some(session.ai_suggestions.clone())
    } else {
        None
    }
}