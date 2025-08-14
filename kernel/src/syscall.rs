use alloc::string::String;
use alloc::vec::Vec;
use alloc::vec;
use alloc::format;
// use core::arch::asm;
use x86_64::VirtAddr;
// use crate::process::{Priority, ProcessPermissions, SandboxLevel};
use crate::filesystem;

#[derive(Debug, Clone, Copy)]
#[repr(u64)]
pub enum SyscallNumber {
    Exit = 0,
    Fork = 1,
    Exec = 2,
    Wait = 3,
    Kill = 4,
    GetPid = 5,
    GetPpid = 6,
    Sleep = 7,
    Yield = 8,
    
    // File operations
    Open = 10,
    Close = 11,
    Read = 12,
    Write = 13,
    Seek = 14,
    Stat = 15,
    Mkdir = 16,
    Rmdir = 17,
    Unlink = 18,
    
    // Memory management
    Mmap = 20,
    Munmap = 21,
    Mprotect = 22,
    Brk = 23,
    
    // IPC
    Pipe = 30,
    Socket = 31,
    Bind = 32,
    Listen = 33,
    Accept = 34,
    Connect = 35,
    Send = 36,
    Recv = 37,
    
    // RaeenOS specific
    SetGameMode = 100,
    GetSystemInfo = 101,
    SetTheme = 102,
    CreateWindow = 103,
    DestroyWindow = 104,
    DrawPixel = 105,
    DrawRect = 106,
    DrawText = 107,
    GetInput = 108,
    PlaySound = 109,
    
    // Security
    RequestPermission = 200,
    SetSandbox = 201,
    GetPermissions = 202,
    
    // AI Assistant
    AiQuery = 300,
    AiGenerate = 301,
    AiAnalyze = 302,
}

#[derive(Debug)]
pub struct SyscallResult {
    pub success: bool,
    pub value: i64,
    pub error_code: Option<SyscallError>,
}

#[derive(Debug, Clone, Copy)]
pub enum SyscallError {
    InvalidSyscall,
    InvalidArgument,
    PermissionDenied,
    ResourceNotFound,
    ResourceBusy,
    OutOfMemory,
    IoError,
    NetworkError,
    SandboxViolation,
    NotImplemented,
}

impl SyscallResult {
    pub fn success(value: i64) -> Self {
        Self {
            success: true,
            value,
            error_code: None,
        }
    }
    
    pub fn error(error: SyscallError) -> Self {
        Self {
            success: false,
            value: -1,
            error_code: Some(error),
        }
    }
}

// System call handler
pub fn handle_syscall(
    syscall_num: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    arg6: u64,
) -> SyscallResult {
    let syscall = match syscall_num {
        0 => SyscallNumber::Exit,
        1 => SyscallNumber::Fork,
        2 => SyscallNumber::Exec,
        3 => SyscallNumber::Wait,
        4 => SyscallNumber::Kill,
        5 => SyscallNumber::GetPid,
        6 => SyscallNumber::GetPpid,
        7 => SyscallNumber::Sleep,
        8 => SyscallNumber::Yield,
        10 => SyscallNumber::Open,
        11 => SyscallNumber::Close,
        12 => SyscallNumber::Read,
        13 => SyscallNumber::Write,
        14 => SyscallNumber::Seek,
        15 => SyscallNumber::Stat,
        16 => SyscallNumber::Mkdir,
        17 => SyscallNumber::Rmdir,
        18 => SyscallNumber::Unlink,
        20 => SyscallNumber::Mmap,
        21 => SyscallNumber::Munmap,
        22 => SyscallNumber::Mprotect,
        23 => SyscallNumber::Brk,
        30 => SyscallNumber::Pipe,
        31 => SyscallNumber::Socket,
        32 => SyscallNumber::Bind,
        33 => SyscallNumber::Listen,
        34 => SyscallNumber::Accept,
        35 => SyscallNumber::Connect,
        36 => SyscallNumber::Send,
        37 => SyscallNumber::Recv,
        100 => SyscallNumber::SetGameMode,
        101 => SyscallNumber::GetSystemInfo,
        102 => SyscallNumber::SetTheme,
        103 => SyscallNumber::CreateWindow,
        104 => SyscallNumber::DestroyWindow,
        105 => SyscallNumber::DrawPixel,
        106 => SyscallNumber::DrawRect,
        107 => SyscallNumber::DrawText,
        108 => SyscallNumber::GetInput,
        109 => SyscallNumber::PlaySound,
        200 => SyscallNumber::RequestPermission,
        201 => SyscallNumber::SetSandbox,
        202 => SyscallNumber::GetPermissions,
        300 => SyscallNumber::AiQuery,
        301 => SyscallNumber::AiGenerate,
        302 => SyscallNumber::AiAnalyze,
        _ => return SyscallResult::error(SyscallError::InvalidSyscall),
    };
    
    match syscall {
        SyscallNumber::Exit => sys_exit(arg1 as i32),
        SyscallNumber::Fork => sys_fork(),
        SyscallNumber::Exec => sys_exec(arg1, arg2),
        SyscallNumber::Wait => sys_wait(arg1),
        SyscallNumber::Kill => sys_kill(arg1, arg2 as i32),
        SyscallNumber::GetPid => sys_getpid(),
        SyscallNumber::GetPpid => sys_getppid(),
        SyscallNumber::Sleep => sys_sleep(arg1),
        SyscallNumber::Yield => sys_yield(),
        
        SyscallNumber::Open => sys_open(arg1, arg2, arg3),
        SyscallNumber::Close => sys_close(arg1),
        SyscallNumber::Read => sys_read(arg1, arg2, arg3),
        SyscallNumber::Write => sys_write(arg1, arg2, arg3),
        SyscallNumber::Seek => sys_seek(arg1, arg2 as i64, arg3),
        SyscallNumber::Stat => sys_stat(arg1, arg2),
        SyscallNumber::Mkdir => sys_mkdir(arg1, arg2),
        SyscallNumber::Rmdir => sys_rmdir(arg1),
        SyscallNumber::Unlink => sys_unlink(arg1),
        
        SyscallNumber::Mmap => sys_mmap(arg1, arg2, arg3, arg4, arg5, arg6 as i64),
        SyscallNumber::Munmap => sys_munmap(arg1, arg2),
        SyscallNumber::Mprotect => sys_mprotect(arg1, arg2, arg3),
        SyscallNumber::Brk => sys_brk(arg1),
        
        SyscallNumber::Pipe => sys_pipe(arg1),
        SyscallNumber::Socket => sys_socket(arg1, arg2, arg3),
        SyscallNumber::Bind => sys_bind(arg1, arg2, arg3),
        SyscallNumber::Listen => sys_listen(arg1, arg2),
        SyscallNumber::Accept => sys_accept(arg1, arg2, arg3),
        SyscallNumber::Connect => sys_connect(arg1, arg2, arg3),
        SyscallNumber::Send => sys_send(arg1, arg2, arg3, arg4),
        SyscallNumber::Recv => sys_recv(arg1, arg2, arg3, arg4),
        
        SyscallNumber::SetGameMode => sys_set_game_mode(arg1 != 0),
        SyscallNumber::GetSystemInfo => sys_get_system_info(arg1),
        SyscallNumber::SetTheme => sys_set_theme(arg1, arg2),
        SyscallNumber::CreateWindow => sys_create_window(arg1, arg2, arg3, arg4, arg5),
        SyscallNumber::DestroyWindow => sys_destroy_window(arg1),
        SyscallNumber::DrawPixel => sys_draw_pixel(arg1, arg2, arg3, arg4),
        SyscallNumber::DrawRect => sys_draw_rect(arg1, arg2, arg3, arg4, arg5, arg6),
        SyscallNumber::DrawText => sys_draw_text(arg1, arg2, arg3, arg4, arg5),
        SyscallNumber::GetInput => sys_get_input(arg1),
        SyscallNumber::PlaySound => sys_play_sound(arg1, arg2, arg3),
        
        SyscallNumber::RequestPermission => sys_request_permission(arg1),
        SyscallNumber::SetSandbox => sys_set_sandbox(arg1),
        SyscallNumber::GetPermissions => sys_get_permissions(arg1),
        
        SyscallNumber::AiQuery => sys_ai_query(arg1, arg2, arg3),
        SyscallNumber::AiGenerate => sys_ai_generate(arg1, arg2, arg3),
        SyscallNumber::AiAnalyze => sys_ai_analyze(arg1, arg2, arg3),
    }
}

// Process management syscalls
fn sys_exit(exit_code: i32) -> SyscallResult {
    crate::process::terminate_process(crate::process::get_current_process_info().unwrap().0);
    SyscallResult::success(exit_code as i64)
}

fn sys_fork() -> SyscallResult {
    // Implement basic process forking
    match crate::process::fork_process() {
        Ok(child_pid) => {
            // Return child PID to parent, 0 to child
            if child_pid == 0 {
                SyscallResult::success(0) // Child process
            } else {
                SyscallResult::success(child_pid as i64) // Parent process
            }
        }
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument)
    }
}

fn sys_exec(path: u64, args: u64) -> SyscallResult {
    // Implement process execution
    let path_str = unsafe { c_str_from_user(path) };
    
    // Parse arguments (simplified implementation)
    let mut arg_vec = Vec::new();
    if args != 0 {
        // In a real implementation, this would parse an array of string pointers
        arg_vec.push(path_str.clone());
    }
    
    match crate::process::exec_process(&path_str, &arg_vec) {
        Ok(()) => {
            // exec doesn't return on success - the process image is replaced
            SyscallResult::success(0)
        }
        Err(_) => SyscallResult::error(SyscallError::ResourceNotFound)
    }
}

fn sys_wait(pid: u64) -> SyscallResult {
    // Implement process waiting
    match crate::process::wait_for_process(pid) {
        Ok(exit_code) => SyscallResult::success(exit_code as i64),
        Err(_) => SyscallResult::error(SyscallError::ResourceNotFound)
    }
}

fn sys_kill(pid: u64, signal: i32) -> SyscallResult {
    crate::process::terminate_process(pid);
    SyscallResult::success(0)
}

fn sys_getpid() -> SyscallResult {
    if let Some((pid, _, _)) = crate::process::get_current_process_info() {
        SyscallResult::success(pid as i64)
    } else {
        SyscallResult::error(SyscallError::ResourceNotFound)
    }
}

fn sys_getppid() -> SyscallResult {
    // Implement parent PID retrieval
    if let Some((_, parent_pid, _)) = crate::process::get_current_process_info() {
        SyscallResult::success(0)
    } else {
        SyscallResult::error(SyscallError::ResourceNotFound)
    }
}

fn sys_sleep(milliseconds: u64) -> SyscallResult {
    crate::time::sleep_ms(milliseconds);
    SyscallResult::success(0)
}

fn sys_yield() -> SyscallResult {
    crate::process::yield_current();
    SyscallResult::success(0)
}

// File system syscalls
fn sys_open(path: u64, flags: u64, mode: u64) -> SyscallResult {
    let path_str = unsafe { c_str_from_user(path) };
    match filesystem::open(&path_str, flags as u32) {
        Ok(fd) => SyscallResult::success(fd as i64),
        Err(_) => SyscallResult::error(SyscallError::IoError),
    }
}

fn sys_close(fd: u64) -> SyscallResult {
    match filesystem::close(fd) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::IoError),
    }
}

fn sys_read(fd: u64, buffer: u64, count: u64) -> SyscallResult {
    let mut tmp = vec![0u8; count as usize];
    match filesystem::read(fd, &mut tmp) {
        Ok(n) => {
            unsafe { copy_to_user(buffer, &tmp[..n]) };
            SyscallResult::success(n as i64)
        }
        Err(_) => SyscallResult::error(SyscallError::IoError),
    }
}

fn sys_write(fd: u64, buffer: u64, count: u64) -> SyscallResult {
    let tmp = unsafe { slice_from_user(buffer, count as usize) };
    match filesystem::write(fd, &tmp) {
        Ok(n) => SyscallResult::success(n as i64),
        Err(_) => SyscallResult::error(SyscallError::IoError),
    }
}

fn sys_seek(fd: u64, offset: i64, whence: u64) -> SyscallResult {
    let pos = match whence {
        0 => filesystem::SeekFrom::Start(offset as u64),
        1 => filesystem::SeekFrom::Current(offset),
        2 => filesystem::SeekFrom::End(offset),
        _ => return SyscallResult::error(SyscallError::InvalidArgument),
    };
    match filesystem::seek(fd, pos) {
        Ok(new_pos) => SyscallResult::success(new_pos as i64),
        Err(_) => SyscallResult::error(SyscallError::IoError),
    }
}

fn sys_stat(path: u64, statbuf: u64) -> SyscallResult {
    let path_str = unsafe { c_str_from_user(path) };
    match filesystem::metadata(&path_str) {
        Ok(meta) => {
            #[repr(C)]
            struct Stat {
                file_type: u32,
                size: u64,
                permissions: u32,
                created: u64,
                modified: u64,
                accessed: u64,
                uid: u32,
                gid: u32,
            }
            let st = Stat {
                file_type: meta.file_type as u32,
                size: meta.size,
                permissions: meta.permissions,
                created: meta.created,
                modified: meta.modified,
                accessed: meta.accessed,
                uid: meta.uid,
                gid: meta.gid,
            };
            unsafe { copy_to_user(statbuf, unsafe_any_as_bytes(&st)) };
            SyscallResult::success(0)
        }
        Err(_) => SyscallResult::error(SyscallError::ResourceNotFound),
    }
}

fn sys_mkdir(path: u64, mode: u64) -> SyscallResult {
    let path_str = unsafe { c_str_from_user(path) };
    match filesystem::create_directory(&path_str) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::IoError),
    }
}

fn sys_rmdir(path: u64) -> SyscallResult {
    let path_str = unsafe { c_str_from_user(path) };
    match filesystem::remove(&path_str) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::IoError),
    }
}

fn sys_unlink(path: u64) -> SyscallResult {
    let path_str = unsafe { c_str_from_user(path) };
    match filesystem::remove(&path_str) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::IoError),
    }
}

// Memory management syscalls
fn sys_mmap(addr: u64, length: u64, prot: u64, flags: u64, fd: u64, offset: i64) -> SyscallResult {
    // Minimal stub returns start of a newly allocated area in current AS
    let as_id = 1; // kernel AS for now
    let perms = crate::vmm::VmPermissions::Read | crate::vmm::VmPermissions::Write;
    match crate::vmm::allocate_area(as_id, length, crate::vmm::VmAreaType::Heap, perms) {
        Ok(vaddr) => SyscallResult::success(vaddr.as_u64() as i64),
        Err(_) => SyscallResult::error(SyscallError::OutOfMemory),
    }
}

fn sys_munmap(addr: u64, length: u64) -> SyscallResult {
    let as_id = 1;
    match crate::vmm::deallocate_area(as_id, VirtAddr::new(addr)) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument),
    }
}

fn sys_mprotect(addr: u64, length: u64, prot: u64) -> SyscallResult {
    let as_id = 1;
    let mut perms = crate::vmm::VmPermissions::Read;
    if (prot & 0x2) != 0 { perms |= crate::vmm::VmPermissions::Write; }
    if (prot & 0x1) == 0 { perms |= crate::vmm::VmPermissions::NoCache; }
    match crate::vmm::protect_memory(as_id, VirtAddr::new(addr), length, perms) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument),
    }
}

fn sys_brk(addr: u64) -> SyscallResult {
    // Implement heap management
    match crate::memory::set_program_break(addr) {
        Ok(new_break) => SyscallResult::success(new_break as i64),
        Err(_) => SyscallResult::error(SyscallError::OutOfMemory)
    }
}

// IPC syscalls
fn sys_pipe(pipefd: u64) -> SyscallResult {
    // Implement pipe creation
    match crate::ipc::create_pipe() {
        Ok((read_fd, write_fd)) => {
            // Write file descriptors to user buffer
            let fds = [read_fd as u32, write_fd as u32];
            unsafe { 
                copy_to_user(pipefd, unsafe_any_as_bytes(&fds));
            }
            SyscallResult::success(0)
        }
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument)
    }
}

fn sys_socket(domain: u64, socket_type: u64, protocol: u64) -> SyscallResult {
    // Implement socket creation
    match crate::network::create_socket(domain as u32, socket_type as u32, protocol as u32) {
        Ok(socket_fd) => SyscallResult::success(socket_fd as i64),
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument)
    }
}

fn sys_bind(socket_fd: u64, addr: u64, addr_len: u64) -> SyscallResult {
    // Implement socket binding
    let addr_bytes = unsafe {
        core::slice::from_raw_parts(addr as *const u8, addr_len as usize)
    };
    
    match crate::network::bind_socket(socket_fd as u32, addr_bytes) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument)
    }
}

fn sys_listen(socket_fd: u64, backlog: u64) -> SyscallResult {
    // Implement socket listening
    match crate::network::listen_socket(socket_fd as u32, backlog as u32) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument)
    }
}

fn sys_accept(socket_fd: u64, addr: u64, addr_len: u64) -> SyscallResult {
    // Implement socket accepting
    match crate::network::accept_connection(socket_fd as u32) {
        Ok((new_fd, peer_addr)) => {
            if addr != 0 && addr_len > 0 {
                let addr_bytes = peer_addr.as_bytes();
                let copy_len = core::cmp::min(addr_bytes.len(), addr_len as usize);
                unsafe {
                    copy_to_user(addr, &addr_bytes[..copy_len]);
                }
            }
            SyscallResult::success(new_fd as i64)
        }
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument)
    }
}

fn sys_connect(socket_fd: u64, addr: u64, addr_len: u64) -> SyscallResult {
    // Implement socket connection
    let addr_bytes = unsafe {
        core::slice::from_raw_parts(addr as *const u8, addr_len as usize)
    };
    
    match crate::network::connect_socket(socket_fd as u32, addr_bytes) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument)
    }
}

fn sys_send(socket_fd: u64, buffer: u64, length: u64, flags: u64) -> SyscallResult {
    // Implement socket sending
    let data = unsafe {
        core::slice::from_raw_parts(buffer as *const u8, length as usize)
    };
    
    match crate::network::send_data(socket_fd as u32, data, flags as u32) {
        Ok(bytes_sent) => SyscallResult::success(bytes_sent as i64),
        Err(_) => SyscallResult::error(SyscallError::IoError)
    }
}

fn sys_recv(socket_fd: u64, buffer: u64, length: u64, flags: u64) -> SyscallResult {
    // Implement socket receiving
    match crate::network::receive_data(socket_fd as u32, length as usize, flags as u32) {
        Ok(data) => {
            let copy_len = core::cmp::min(data.len(), length as usize);
            unsafe {
                copy_to_user(buffer, &data[..copy_len]);
            }
            SyscallResult::success(copy_len as i64)
        }
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument)
    }
}

// RaeenOS specific syscalls
fn sys_set_game_mode(enabled: bool) -> SyscallResult {
    crate::process::set_gaming_mode(enabled);
    SyscallResult::success(if enabled { 1 } else { 0 })
}

fn sys_get_system_info(info_type: u64) -> SyscallResult {
    // Implement system info retrieval
    match info_type {
        0 => { // CPU info
            let cpu_count = crate::arch::get_cpu_count();
            SyscallResult::success(cpu_count as i64)
        }
        1 => { // Memory info (total memory in MB)
            let total_memory = crate::memory::get_total_memory();
            SyscallResult::success((total_memory / (1024 * 1024)) as i64)
        }
        2 => { // Available memory (free memory in MB)
            let free_memory = crate::memory::get_free_memory();
            SyscallResult::success((free_memory / (1024 * 1024)) as i64)
        }
        3 => { // Process count
            let process_count = crate::process::get_process_count();
            SyscallResult::success(process_count as i64)
        }
        4 => { // Uptime in seconds
            let uptime = crate::time::get_uptime_seconds();
            SyscallResult::success(uptime as i64)
        }
        _ => SyscallResult::error(SyscallError::InvalidArgument)
    }
}

fn sys_set_theme(theme_id: u64, options: u64) -> SyscallResult {
    // Implement theme setting
    match crate::ui::set_system_theme(theme_id as u32, options as u32) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument)
    }
}

fn sys_create_window(x: u64, y: u64, width: u64, height: u64, flags: u64) -> SyscallResult {
    let id = crate::graphics::create_window("App", x as i32, y as i32, width as u32, height as u32, 1);
    SyscallResult::success(id as i64)
}

fn sys_destroy_window(window_id: u64) -> SyscallResult {
    let ok = crate::graphics::destroy_window(window_id as u32);
    SyscallResult::success(if ok { 0 } else { -1 })
}

fn sys_draw_pixel(window_id: u64, x: u64, y: u64, color: u64) -> SyscallResult {
    let c = u32_color_from_u64(color);
    let color = crate::graphics::Color { r: ((c >> 16) & 0xFF) as u8, g: ((c >> 8) & 0xFF) as u8, b: (c & 0xFF) as u8, a: ((c >> 24) & 0xFF) as u8 };
    match crate::graphics::draw_pixel(window_id as u32, x as u32, y as u32, color) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument),
    }
}

fn sys_draw_rect(window_id: u64, x: u64, y: u64, width: u64, height: u64, color: u64) -> SyscallResult {
    let c = u32_color_from_u64(color);
    let color = crate::graphics::Color { r: ((c >> 16) & 0xFF) as u8, g: ((c >> 8) & 0xFF) as u8, b: (c & 0xFF) as u8, a: ((c >> 24) & 0xFF) as u8 };
    let rect = crate::graphics::Rect::new(x as i32, y as i32, width as u32, height as u32);
    match crate::graphics::draw_rect(window_id as u32, rect, color) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument),
    }
}

fn sys_draw_text(window_id: u64, x: u64, y: u64, text: u64, color: u64) -> SyscallResult {
    let text_str = unsafe { c_str_from_user(text) };
    let c = u32_color_from_u64(color);
    let color = crate::graphics::Color { r: ((c >> 16) & 0xFF) as u8, g: ((c >> 8) & 0xFF) as u8, b: (c & 0xFF) as u8, a: ((c >> 24) & 0xFF) as u8 };
    
    match crate::graphics::draw_text(window_id as u32, x as i32, y as i32, &text_str, color) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::IoError)
    }
}

fn sys_get_input(input_type: u64) -> SyscallResult {
    // Implement input retrieval
    match input_type {
        0 => { // Keyboard input
            if let Some(key) = crate::drivers::keyboard::get_key() {
                SyscallResult::success(key as i64)
            } else {
                SyscallResult::success(-1) // No input available
            }
        }
        1 => { // Mouse input
            if let Some((x, y, buttons)) = crate::drivers::mouse::get_mouse_state() {
                let state = ((buttons as u64) << 32) | ((x as u64) << 16) | (y as u64);
                SyscallResult::success(state as i64)
            } else {
                SyscallResult::success(-1) // No input available
            }
        }
        _ => SyscallResult::error(SyscallError::InvalidArgument)
    }
}

fn sys_play_sound(sound_id: u64, volume: u64, flags: u64) -> SyscallResult {
    // Implement sound playing
    match crate::sound::play_sound(sound_id as u32, volume as u8, flags as u32) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::IoError)
    }
}

// Security syscalls
fn sys_request_permission(permission: u64) -> SyscallResult {
    // Implement permission requesting
    let current_pid = match crate::process::get_current_process_info() {
        Some((pid, _, _)) => pid,
        None => return SyscallResult::error(SyscallError::ResourceNotFound)
    };
    
    match crate::security::request_permission(current_pid, permission) {
        Ok(granted) => SyscallResult::success(if granted { 1 } else { 0 }),
        Err(_) => SyscallResult::error(SyscallError::PermissionDenied)
    }
}

fn sys_set_sandbox(level: u64) -> SyscallResult {
    // Implement sandbox setting
    let current_pid = match crate::process::get_current_process_info() {
        Some((pid, _, _)) => pid,
        None => return SyscallResult::error(SyscallError::ResourceNotFound)
    };
    
    match crate::security::set_sandbox_level(current_pid, level as u8) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::PermissionDenied)
    }
}

fn sys_get_permissions(buffer: u64) -> SyscallResult {
    // Implement permission retrieval
    let current_pid = match crate::process::get_current_process_info() {
        Some((pid, _, _)) => pid,
        None => return SyscallResult::error(SyscallError::ResourceNotFound)
    };
    
    match crate::security::get_process_permissions(current_pid) {
        Ok(permissions) => {
            unsafe {
                copy_to_user(buffer, unsafe_any_as_bytes(&permissions));
            }
            SyscallResult::success(core::mem::size_of_val(&permissions) as i64)
        }
        Err(_) => SyscallResult::error(SyscallError::ResourceNotFound)
    }
}

// AI syscalls
fn sys_ai_query(query: u64, response_buffer: u64, buffer_size: u64) -> SyscallResult {
    // Minimal placeholder: echo back input length
    let data = unsafe { slice_from_user(query, core::cmp::min(buffer_size as usize, 256)) };
    let reply = format!("AI: received {} bytes", data.len());
    unsafe { copy_to_user(response_buffer, reply.as_bytes()) };
    SyscallResult::success(reply.len() as i64)
}

fn sys_ai_generate(prompt: u64, output_buffer: u64, buffer_size: u64) -> SyscallResult {
    // Implement AI generation
    let prompt_str = unsafe { c_str_from_user(prompt) };
    
    match crate::rae_assistant::generate_ai_response(&prompt_str) {
        Ok(response) => {
            let response_bytes = response.as_bytes();
            let copy_len = core::cmp::min(response_bytes.len(), buffer_size as usize - 1);
            
            unsafe {
                copy_to_user(output_buffer, &response_bytes[..copy_len]);
                // Null terminate
                core::ptr::write((output_buffer + copy_len as u64) as *mut u8, 0);
            }
            
            SyscallResult::success(copy_len as i64)
        }
        Err(_) => SyscallResult::error(SyscallError::IoError)
    }
}

fn sys_ai_analyze(data: u64, analysis_buffer: u64, buffer_size: u64) -> SyscallResult {
    // Implement AI analysis
    let data_str = unsafe { c_str_from_user(data) };
    
    match crate::rae_assistant::analyze_content(&data_str) {
        Ok(analysis) => {
            let analysis_bytes = analysis.as_bytes();
            let copy_len = core::cmp::min(analysis_bytes.len(), buffer_size as usize - 1);
            
            unsafe {
                copy_to_user(analysis_buffer, &analysis_bytes[..copy_len]);
                // Null terminate
                core::ptr::write((analysis_buffer + copy_len as u64) as *mut u8, 0);
            }
            
            SyscallResult::success(copy_len as i64)
        }
        Err(_) => SyscallResult::error(SyscallError::IoError)
    }
}

// Assembly syscall entry point
#[no_mangle]
pub extern "C" fn syscall_handler(
    rax: u64, // syscall number
    rdi: u64, // arg1
    rsi: u64, // arg2
    rdx: u64, // arg3
    r10: u64, // arg4
    r8: u64,  // arg5
    r9: u64,  // arg6
) -> u64 {
    let result = handle_syscall(rax, rdi, rsi, rdx, r10, r8, r9);
    if result.success {
        result.value as u64
    } else {
        (-(result.error_code.unwrap() as i32)) as u64
    }
}

pub fn init() {
    // For now, syscalls are exposed via the C ABI `syscall_handler` entry.
}

// ------- helper functions for user pointers (temporary, unsafe) -------
unsafe fn c_str_from_user(ptr: u64) -> alloc::string::String {
    let mut v = Vec::new();
    let mut p = ptr as *const u8;
    loop {
        let b = core::ptr::read(p);
        if b == 0 { break; }
        v.push(b);
        p = p.add(1);
        if v.len() > 4096 { break; }
    }
    alloc::string::String::from_utf8_lossy(&v).into_owned()
}

unsafe fn slice_from_user(ptr: u64, len: usize) -> Vec<u8> {
    let mut v = Vec::with_capacity(len);
    v.set_len(len);
    core::ptr::copy_nonoverlapping(ptr as *const u8, v.as_mut_ptr(), len);
    v
}

unsafe fn copy_to_user(dst: u64, src: &[u8]) {
    core::ptr::copy_nonoverlapping(src.as_ptr(), dst as *mut u8, src.len());
}

unsafe fn unsafe_any_as_bytes<T: Sized>(t: &T) -> &[u8] {
    core::slice::from_raw_parts((t as *const T) as *const u8, core::mem::size_of::<T>())
}

fn u32_color_from_u64(color: u64) -> u32 {
    color as u32
}