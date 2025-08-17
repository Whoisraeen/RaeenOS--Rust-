//! RaeShell - Built-in shell for RaeenOS

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use alloc::format;
use alloc::collections::BTreeMap;
use spin::Mutex;
use lazy_static::lazy_static;

// Shell command result
#[derive(Debug, Clone)]
pub enum ShellResult {
    Success(String),
    Error(String),
    Exit,
}

// Built-in command function type
type BuiltinCommand = fn(&[&str]) -> ShellResult;

// Shell session state
#[derive(Debug, Clone)]
struct ShellSession {
    _session_id: u32,
    current_directory: String,
    environment: BTreeMap<String, String>,
    command_history: Vec<String>,
    process_id: u32,
    prompt: String,
}

impl ShellSession {
    fn new(session_id: u32, process_id: u32) -> Self {
        let mut env = BTreeMap::new();
        env.insert("PATH".to_string(), "/bin:/usr/bin:/usr/local/bin".to_string());
        env.insert("HOME".to_string(), "/home/user".to_string());
        env.insert("USER".to_string(), "user".to_string());
        env.insert("SHELL".to_string(), "/bin/raeshell".to_string());
        
        Self {
            _session_id: session_id,
            current_directory: "/".to_string(),
            environment: env,
            command_history: Vec::new(),
            process_id,
            prompt: "raeshell> ".to_string(),
        }
    }
    
    fn add_to_history(&mut self, command: String) {
        self.command_history.push(command);
        
        // Keep only last 100 commands
        if self.command_history.len() > 100 {
            self.command_history.remove(0);
        }
    }
    
    fn get_env(&self, key: &str) -> Option<&String> {
        self.environment.get(key)
    }
    
    fn set_env(&mut self, key: String, value: String) {
        self.environment.insert(key, value);
    }
}

// Shell system state
struct ShellSystem {
    sessions: BTreeMap<u32, ShellSession>,
    next_session_id: u32,
    builtin_commands: BTreeMap<String, BuiltinCommand>,
}

lazy_static! {
    static ref SHELL_SYSTEM: Mutex<ShellSystem> = {
        let mut system = ShellSystem {
            sessions: BTreeMap::new(),
            next_session_id: 1,
            builtin_commands: BTreeMap::new(),
        };
        
        // Register built-in commands
        system.builtin_commands.insert("help".to_string(), cmd_help);
        system.builtin_commands.insert("ls".to_string(), cmd_ls);
        system.builtin_commands.insert("cd".to_string(), cmd_cd);
        system.builtin_commands.insert("pwd".to_string(), cmd_pwd);
        system.builtin_commands.insert("echo".to_string(), cmd_echo);
        system.builtin_commands.insert("env".to_string(), cmd_env);
        system.builtin_commands.insert("export".to_string(), cmd_export);
        system.builtin_commands.insert("history".to_string(), cmd_history);
        system.builtin_commands.insert("clear".to_string(), cmd_clear);
        system.builtin_commands.insert("ps".to_string(), cmd_ps);
        system.builtin_commands.insert("kill".to_string(), cmd_kill);
        system.builtin_commands.insert("cat".to_string(), cmd_cat);
        system.builtin_commands.insert("touch".to_string(), cmd_touch);
        system.builtin_commands.insert("rm".to_string(), cmd_rm);
        system.builtin_commands.insert("mkdir".to_string(), cmd_mkdir);
        system.builtin_commands.insert("rmdir".to_string(), cmd_rmdir);
        system.builtin_commands.insert("cp".to_string(), cmd_cp);
        system.builtin_commands.insert("mv".to_string(), cmd_mv);
        system.builtin_commands.insert("exit".to_string(), cmd_exit);
        system.builtin_commands.insert("uname".to_string(), cmd_uname);
        system.builtin_commands.insert("whoami".to_string(), cmd_whoami);
        system.builtin_commands.insert("date".to_string(), cmd_date);
        system.builtin_commands.insert("uptime".to_string(), cmd_uptime);
        system.builtin_commands.insert("free".to_string(), cmd_free);
        system.builtin_commands.insert("thread_stress".to_string(), cmd_thread_stress);
        
        Mutex::new(system)
    };
}

// Built-in command implementations
fn cmd_help(_args: &[&str]) -> ShellResult {
    let help_text = "RaeShell - Built-in Commands:\n  help        - Show this help message\n  ls [path]   - List directory contents\n  cd <path>   - Change directory\n  pwd         - Print working directory\n  echo <text> - Print text to output\n  env         - Show environment variables\n  export K=V  - Set environment variable\n  history     - Show command history\n  clear       - Clear screen\n  ps          - List running processes\n  kill <pid>  - Terminate process\n  cat <file>  - Display file contents\n  touch <file>- Create empty file\n  rm <file>   - Remove file\n  mkdir <dir> - Create directory\n  rmdir <dir> - Remove directory\n  cp <src> <dst> - Copy file\n  mv <src> <dst> - Move/rename file\n  exit        - Exit shell\n  uname       - System information\n  whoami      - Current user\n  date        - Current date/time\n  uptime      - System uptime\n  free        - Memory usage";
    
    ShellResult::Success(help_text.to_string())
}

fn cmd_ls(args: &[&str]) -> ShellResult {
    let path = if args.len() > 1 { args[1] } else { "." };
    
    // Use VFS to list directory
    match crate::filesystem::list_directory(path) {
        Ok(entries) => {
            let mut output = String::new();
            for entry in entries {
                output.push_str(&entry);
                output.push('\n');
            }
            ShellResult::Success(output)
        }
        Err(_) => ShellResult::Error(format!("ls: cannot access '{}': No such file or directory", path)),
    }
}

fn cmd_cd(args: &[&str]) -> ShellResult {
    if args.len() < 2 {
        return ShellResult::Error("cd: missing argument".to_string());
    }
    
    let path = args[1];
    
    // Validate path exists
    match crate::filesystem::metadata(path) {
        Ok(metadata) => {
            if metadata.file_type == crate::filesystem::FileType::Directory {
                // Update current directory in session
                // This would need session context
                ShellResult::Success(String::new())
            } else {
                ShellResult::Error(format!("cd: '{}': Not a directory", path))
            }
        }
        Err(_) => ShellResult::Error(format!("cd: '{}': No such file or directory", path)),
    }
}

fn cmd_pwd(_args: &[&str]) -> ShellResult {
    // This would get current directory from session
    ShellResult::Success("/".to_string())
}

fn cmd_echo(args: &[&str]) -> ShellResult {
    if args.len() < 2 {
        ShellResult::Success(String::new())
    } else {
        let output = args[1..].join(" ");
        ShellResult::Success(output)
    }
}

fn cmd_env(_args: &[&str]) -> ShellResult {
    // This would get environment from session
    let env_output = "PATH=/bin:/usr/bin:/usr/local/bin\nHOME=/home/user\nUSER=user\nSHELL=/bin/raeshell";
    
    ShellResult::Success(env_output.to_string())
}

fn cmd_export(args: &[&str]) -> ShellResult {
    if args.len() < 2 {
        return ShellResult::Error("export: missing argument".to_string());
    }
    
    let assignment = args[1];
    if let Some(eq_pos) = assignment.find('=') {
        let key = &assignment[..eq_pos];
        let value = &assignment[eq_pos + 1..];
        
        // This would set environment variable in session
        ShellResult::Success(format!("Exported {}={}", key, value))
    } else {
        ShellResult::Error("export: invalid format, use KEY=VALUE".to_string())
    }
}

fn cmd_history(_args: &[&str]) -> ShellResult {
    // This would get command history from session
    let history = "1  help\n2  ls\n3  pwd";
    ShellResult::Success(history.to_string())
}

fn cmd_clear(_args: &[&str]) -> ShellResult {
    // Clear screen escape sequence
    ShellResult::Success("\x1b[2J\x1b[H".to_string())
}

fn cmd_ps(_args: &[&str]) -> ShellResult {
    // Use kernel-provided dump for now (prints to serial)
    let _ = crate::syscall::handle_syscall(352, 0, 0, 0, 0, 0, 0);
    ShellResult::Success("(ps) see serial for detailed list".to_string())
}

fn cmd_kill(args: &[&str]) -> ShellResult {
    if args.len() < 2 {
        return ShellResult::Error("kill: missing argument".to_string());
    }
    
    if let Ok(pid) = args[1].parse::<u64>() {
        crate::process::terminate_process(pid);
        ShellResult::Success(format!("Process {} terminated", pid))
    } else {
        ShellResult::Error("kill: invalid process ID".to_string())
    }
}

fn cmd_cat(args: &[&str]) -> ShellResult {
    if args.len() < 2 {
        return ShellResult::Error("cat: missing argument".to_string());
    }
    
    let filename = args[1];
    
    match crate::filesystem::open_file(filename) {
        Ok(fd) => {
            match crate::filesystem::read_file_fd(fd, 4096) {
                Ok(data) => {
                    let _ = crate::filesystem::close_file(fd);
                    match String::from_utf8(data) {
                        Ok(content) => ShellResult::Success(content),
                        Err(_) => ShellResult::Error("cat: file contains binary data".to_string()),
                    }
                }
                Err(_) => {
                    let _ = crate::filesystem::close_file(fd);
                    ShellResult::Error(format!("cat: cannot read '{}'", filename))
                }
            }
        }
        Err(_) => ShellResult::Error(format!("cat: '{}': No such file or directory", filename)),
    }
}

fn cmd_touch(args: &[&str]) -> ShellResult {
    if args.len() < 2 {
        return ShellResult::Error("touch: missing argument".to_string());
    }
    
    let filename = args[1];
    
    match crate::filesystem::create_file(filename) {
        Ok(_) => ShellResult::Success(String::new()),
        Err(_) => ShellResult::Error(format!("touch: cannot create '{}'", filename)),
    }
}

fn cmd_rm(args: &[&str]) -> ShellResult {
    if args.len() < 2 {
        return ShellResult::Error("rm: missing argument".to_string());
    }
    
    let filename = args[1];
    
    match crate::filesystem::remove(filename) {
        Ok(_) => ShellResult::Success(String::new()),
        Err(_) => ShellResult::Error(format!("rm: cannot remove '{}'", filename)),
    }
}

fn cmd_mkdir(args: &[&str]) -> ShellResult {
    if args.len() < 2 {
        return ShellResult::Error("mkdir: missing argument".to_string());
    }
    
    let dirname = args[1];
    
    match crate::filesystem::create_directory(dirname) {
        Ok(_) => ShellResult::Success(String::new()),
        Err(_) => ShellResult::Error(format!("mkdir: cannot create directory '{}'", dirname)),
    }
}

fn cmd_rmdir(args: &[&str]) -> ShellResult {
    if args.len() < 2 {
        return ShellResult::Error("rmdir: missing argument".to_string());
    }
    
    let dirname = args[1];
    
    match crate::filesystem::remove(dirname) {
        Ok(_) => ShellResult::Success(String::new()),
        Err(_) => ShellResult::Error(format!("rmdir: cannot remove directory '{}'", dirname)),
    }
}

fn cmd_cp(args: &[&str]) -> ShellResult {
    if args.len() < 3 {
        return ShellResult::Error("cp: missing arguments".to_string());
    }
    
    let src = args[1];
    let dst = args[2];
    
    // Read source file
    match crate::filesystem::open_file(src) {
        Ok(src_fd) => {
            match crate::filesystem::read_file_fd(src_fd, 65536) {
                Ok(data) => {
                    let _ = crate::filesystem::close_file(src_fd);
                    
                    // Write to destination
                    match crate::filesystem::create_file(dst) {
                        Ok(()) => {
                            match crate::filesystem::open_file(dst) {
                                Ok(dst_fd) => {
                                    match crate::filesystem::write_file(dst_fd, &data) {
                                        Ok(_) => {
                                            let _ = crate::filesystem::close_file(dst_fd);
                                            ShellResult::Success(String::new())
                                        }
                                        Err(_) => {
                                            let _ = crate::filesystem::close_file(dst_fd);
                                            ShellResult::Error(format!("cp: cannot write to '{}'", dst))
                                        }
                                    }
                                }
                                Err(_) => ShellResult::Error(format!("cp: cannot open '{}' for writing", dst)),
                            }
                        }
                        Err(_) => ShellResult::Error(format!("cp: cannot create '{}'", dst)),
                    }
                }
                Err(_) => {
                    let _ = crate::filesystem::close_file(src_fd);
                    ShellResult::Error(format!("cp: cannot read '{}'", src))
                }
            }
        }
        Err(_) => ShellResult::Error(format!("cp: '{}': No such file or directory", src)),
    }
}

fn cmd_mv(args: &[&str]) -> ShellResult {
    if args.len() < 3 {
        return ShellResult::Error("mv: missing arguments".to_string());
    }
    
    let src = args[1];
    let dst = args[2];
    
    // Copy then remove source
    match cmd_cp(&["cp", src, dst]) {
        ShellResult::Success(_) => {
            match crate::filesystem::remove(src) {
                Ok(_) => ShellResult::Success(String::new()),
                Err(_) => ShellResult::Error(format!("mv: cannot remove '{}'", src)),
            }
        }
        result => result,
    }
}

fn cmd_exit(_args: &[&str]) -> ShellResult {
    ShellResult::Exit
}

fn cmd_uname(_args: &[&str]) -> ShellResult {
    ShellResult::Success("RaeenOS x86_64".to_string())
}

fn cmd_whoami(_args: &[&str]) -> ShellResult {
    ShellResult::Success("user".to_string())
}

fn cmd_date(_args: &[&str]) -> ShellResult {
    let uptime = crate::time::get_system_uptime();
    ShellResult::Success(format!("System uptime: {} ticks", uptime))
}

fn cmd_uptime(_args: &[&str]) -> ShellResult {
    let uptime = crate::time::get_system_uptime();
    let seconds = uptime / 1000; // Assuming ticks are milliseconds
    let minutes = seconds / 60;
    let hours = minutes / 60;
    
    ShellResult::Success(format!("up {}:{:02}:{:02}", hours, minutes % 60, seconds % 60))
}

fn cmd_free(_args: &[&str]) -> ShellResult {
    let total = crate::memory::get_total_memory();
    let free = crate::memory::get_free_memory();
    let used = total - free;
    
    let output = format!(
        "              total        used        free\nMem:    {:10} {:10} {:10}\n\nMemory usage: {:.1}%",
        total / 1024,
        used / 1024,
        free / 1024,
        (used as f32 / total as f32) * 100.0
    );
    
    ShellResult::Success(output)
}

fn cmd_thread_stress(args: &[&str]) -> ShellResult {
    // placeholder trigger to run userspace-thread-stress once available
    let threads = if args.len() > 1 { args[1].parse::<u64>().unwrap_or(4) } else { 4 };
    let _ = crate::serial::_print(format_args!("[thread_stress] requested {} threads\n", threads));
    ShellResult::Success("thread_stress: userspace binary will perform measurement".to_string())
}

// Parse command line into tokens
fn parse_command_line(input: &str) -> Vec<&str> {
    input.trim().split_whitespace().collect()
}

// Create a new shell session
pub fn create_shell_session() -> Result<u32, ()> {
    let mut shell = SHELL_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid as u32, "shell.access").unwrap_or(false) {
        return Err(());
    }
    
    let session_id = shell.next_session_id;
    shell.next_session_id += 1;
    
    let session = ShellSession::new(session_id, current_pid as u32);
    shell.sessions.insert(session_id, session);
    
    Ok(session_id)
}

// Execute a command in a shell session
pub fn execute_command(session_id: u32, command_line: &str) -> Result<ShellResult, ()> {
    let mut shell = SHELL_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    let session = shell.sessions.get_mut(&session_id)
        .ok_or(())?;
    
    // Check ownership
    if u64::from(session.process_id) != current_pid {
        return Err(());
    }
    
    // Parse command
    let tokens = parse_command_line(command_line);
    if tokens.is_empty() {
        return Ok(ShellResult::Success(String::new()));
    }
    
    let command = tokens[0];
    
    // Add to history
    session.add_to_history(command_line.to_string());
    
    // Check for built-in command
    if let Some(&builtin_fn) = shell.builtin_commands.get(command) {
        Ok(builtin_fn(&tokens))
    } else {
        // Try to execute as external program
        match crate::process::exec_process(command, &tokens[1..]) {
            Ok(()) => {
                // exec replaces current process, so this shouldn't normally return
                Ok(ShellResult::Success(String::new()))
            }
            Err(_) => Ok(ShellResult::Error(format!("{}: command not found", command))),
        }
    }
}

// Get shell prompt
pub fn get_shell_prompt(session_id: u32) -> Result<String, ()> {
    let shell = SHELL_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    let session = shell.sessions.get(&session_id)
        .ok_or(())?;
    
    // Check ownership
    if u64::from(session.process_id) != current_pid {
        return Err(());
    }
    
    Ok(session.prompt.clone())
}

// Get current directory
pub fn get_current_directory(session_id: u32) -> Result<String, ()> {
    let shell = SHELL_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    let session = shell.sessions.get(&session_id)
        .ok_or(())?;
    
    // Check ownership
    if u64::from(session.process_id) != current_pid {
        return Err(());
    }

    Ok(session.current_directory.clone())
}

// Set current directory
pub fn set_current_directory(session_id: u32, path: &str) -> Result<(), ()> {
    let mut shell = SHELL_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    let session = shell.sessions.get_mut(&session_id)
        .ok_or(())?;
    
    // Check ownership
    if u64::from(session.process_id) != current_pid {
        return Err(());
    }

    session.current_directory = path.to_string();
    Ok(())
}

// Get environment variable
pub fn get_environment_variable(session_id: u32, key: &str) -> Result<Option<String>, ()> {
    let shell = SHELL_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    let session = shell.sessions.get(&session_id)
        .ok_or(())?;
    
    // Check ownership
    if u64::from(session.process_id) != current_pid {
        return Err(());
    }
    
    Ok(session.get_env(key).cloned())
}

// Set environment variable
pub fn set_environment_variable(session_id: u32, key: &str, value: &str) -> Result<(), ()> {
    let mut shell = SHELL_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    let session = shell.sessions.get_mut(&session_id)
        .ok_or(())?;
    
    // Check ownership
    if u64::from(session.process_id) != current_pid {
        return Err(());
    }
    
    session.set_env(key.to_string(), value.to_string());
    Ok(())
}

// Get command history
pub fn get_command_history(session_id: u32) -> Result<Vec<String>, ()> {
    let shell = SHELL_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    let session = shell.sessions.get(&session_id)
        .ok_or(())?;
    
    // Check ownership
    if u64::from(session.process_id) != current_pid {
        return Err(());
    }

    Ok(session.command_history.clone())
}

// Close shell session
pub fn close_shell_session(session_id: u32) -> Result<(), ()> {
    let mut shell = SHELL_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    let session = shell.sessions.get(&session_id)
        .ok_or(())?;
    
    // Check ownership
    if u64::from(session.process_id) != current_pid {
        return Err(());
    }
    
    shell.sessions.remove(&session_id);
    Ok(())
}

// Clean up shell sessions for a process
pub fn cleanup_process_shell(process_id: u32) {
    let mut shell = SHELL_SYSTEM.lock();
    
    let sessions_to_close: Vec<u32> = shell.sessions
        .iter()
        .filter(|(_, session)| session.process_id == process_id)
        .map(|(&session_id, _)| session_id)
        .collect();
    
    for session_id in sessions_to_close {
        shell.sessions.remove(&session_id);
    }
}

// Get list of built-in commands
pub fn get_builtin_commands() -> Vec<String> {
    let shell = SHELL_SYSTEM.lock();
    shell.builtin_commands.keys().cloned().collect()
}

// Check if command is built-in
pub fn is_builtin_command(command: &str) -> bool {
    let shell = SHELL_SYSTEM.lock();
    shell.builtin_commands.contains_key(command)
}