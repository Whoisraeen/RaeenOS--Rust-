use alloc::vec::Vec;
use alloc::vec;
use alloc::format;
use x86_64::VirtAddr;
use x86_64::structures::gdt::{SegmentSelector};
use x86_64::PrivilegeLevel;

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
    
    // Enhanced graphics
    SetVsync = 120,
    GetFrameStats = 121,
    ClearFramebuffer = 122,
    BlitBuffer = 123,
    SetInputFocus = 124,
    GetWindowList = 125,
    ResizeWindow = 126,
    MoveWindow = 127,
    
    // Signal handling
    Signal = 110,
    SigAction = 111,
    SigReturn = 112,
    
    // Security
    RequestPermission = 200,
    SetSandbox = 201,
    GetPermissions = 202,
    
    // Capabilities
    CapClone = 210,
    CapRevoke = 211,
    CapTransfer = 212,
    CapDelegate = 213,
    
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

// Main syscall dispatcher
pub fn handle_syscall(
    syscall_num: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    arg6: u64,
) -> SyscallResult {
    match syscall_num {
        0 => sys_exit(arg1 as i32),
        1 => sys_fork(),
        2 => sys_exec(arg1, arg2),
        3 => sys_wait(arg1),
        4 => sys_kill(arg1, arg2 as i32),
        5 => sys_getpid(),
        6 => sys_getppid(),
        7 => sys_sleep(arg1),
        8 => sys_yield(),
        
        // File operations
        10 => sys_open(arg1, arg2, arg3),
        11 => sys_close(arg1),
        12 => sys_read(arg1, arg2, arg3),
        13 => sys_write(arg1, arg2, arg3),
        14 => sys_seek(arg1, arg2 as i64, arg3),
        15 => sys_stat(arg1, arg2),
        16 => sys_mkdir(arg1, arg2),
        17 => sys_rmdir(arg1),
        18 => sys_unlink(arg1),
        
        // Memory management
        20 => sys_mmap(arg1, arg2, arg3, arg4, arg5, arg6 as i64),
        21 => sys_munmap(arg1, arg2),
        22 => sys_mprotect(arg1, arg2, arg3),
        23 => sys_brk(arg1),
        
        // IPC
        30 => sys_pipe(arg1),
        31 => sys_socket(arg1, arg2, arg3),
        32 => sys_bind(arg1, arg2, arg3),
        33 => sys_listen(arg1, arg2),
        34 => sys_accept(arg1, arg2, arg3),
        35 => sys_connect(arg1, arg2, arg3),
        36 => sys_send(arg1, arg2, arg3, arg4),
        37 => sys_recv(arg1, arg2, arg3, arg4),
        
        // RaeenOS specific
        100 => sys_set_game_mode(arg1 != 0),
        101 => sys_get_system_info(arg1),
        102 => sys_set_theme(arg1, arg2),
        103 => sys_create_window(arg1, arg2, arg3, arg4, arg5),
        104 => sys_destroy_window(arg1),
        105 => sys_draw_pixel(arg1, arg2, arg3, arg4),
        106 => sys_draw_rect(arg1, arg2, arg3, arg4, arg5, arg6),
        107 => sys_draw_text(arg1, arg2, arg3, arg4, arg5),
        108 => sys_get_input(arg1),
        109 => sys_play_sound(arg1, arg2, arg3),
        
        // Enhanced graphics
        120 => sys_set_vsync(arg1 != 0),
        121 => sys_get_frame_stats(arg1),
        122 => sys_clear_framebuffer(arg1),
        123 => sys_blit_buffer(arg1, arg2, arg3, arg4, arg5, arg6),
        124 => sys_set_input_focus(arg1),
        125 => sys_get_window_list(arg1, arg2),
        126 => sys_resize_window(arg1, arg2, arg3),
        127 => sys_move_window(arg1, arg2, arg3),
        
        // Signal handling
        110 => sys_signal(arg1 as i32, arg2),
        111 => sys_sigaction(arg1 as i32, arg2, arg3),
        112 => sys_sigreturn(),
        
        // Security
        200 => sys_request_permission(arg1),
        201 => sys_set_sandbox(arg1),
        202 => sys_get_permissions(arg1),
        
        // Capabilities
        210 => sys_cap_clone(arg1, arg2),
        211 => sys_cap_revoke(arg1),
        212 => sys_cap_transfer(arg1, arg2, arg3),
        213 => sys_cap_delegate(arg1, arg2, arg3),
        
        // AI Assistant
        300 => sys_ai_query(arg1, arg2, arg3),
        301 => sys_ai_generate(arg1, arg2, arg3),
        302 => sys_ai_analyze(arg1, arg2, arg3),
        
        _ => SyscallResult::error(SyscallError::InvalidSyscall),
    }
}

// Process management syscalls
fn sys_exit(exit_code: i32) -> SyscallResult {
    crate::process::exit_process(exit_code);
    // This line is never reached since exit_process never returns
}

fn sys_fork() -> SyscallResult {
    match crate::process::fork_process() {
        Ok(child_pid) => {
            if child_pid == 0 {
                // Child process
                SyscallResult::success(0)
            } else {
                // Parent process
                SyscallResult::success(child_pid as i64)
            }
        }
        Err(_) => SyscallResult::error(SyscallError::OutOfMemory)
    }
}

fn sys_exec(_path: u64, _args: u64) -> SyscallResult {
    let _path_str = unsafe { c_str_from_user(_path) };
    
    // For now, ignore args and just exec the path
    // exec_current_process function doesn't exist, using a placeholder
    // TODO: Implement proper exec functionality
    match Ok(()) as Result<(), ()> {
        Ok(()) => {
            // This should not return if successful
            SyscallResult::success(0)
        }
        Err(_) => {
            // Failed to exec
            SyscallResult::error(SyscallError::ResourceNotFound)
        }
    }
}

fn sys_wait(pid: u64) -> SyscallResult {
    match crate::process::wait_for_process(pid) {
        Ok(exit_code) => SyscallResult::success(exit_code as i64),
        Err(_) => SyscallResult::error(SyscallError::ResourceNotFound)
    }
}

fn sys_kill(pid: u64, signal: i32) -> SyscallResult {
    // Convert signal number to Signal enum
    let signal_enum = match signal {
        9 => crate::process::Signal::SIGKILL,
        15 => crate::process::Signal::SIGTERM,
        19 => crate::process::Signal::SIGSTOP,
        18 => crate::process::Signal::SIGCONT,
        10 => crate::process::Signal::SIGUSR1,
        12 => crate::process::Signal::SIGUSR2,
        _ => return SyscallResult::error(SyscallError::InvalidArgument),
    };
    match crate::process::send_signal(pid, signal_enum) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => {
            // Check if process exists
            // Check if process exists by trying to get process count
            if crate::process::get_process_count() > 0 {
                SyscallResult::error(SyscallError::PermissionDenied)
            } else {
                SyscallResult::error(SyscallError::ResourceNotFound)
            }
        }
    }
}

fn sys_getpid() -> SyscallResult {
    match crate::process::get_current_process_info() {
        Some((pid, _, _)) => SyscallResult::success(pid as i64),
        None => SyscallResult::error(SyscallError::ResourceNotFound)
    }
}

fn sys_getppid() -> SyscallResult {
    match crate::process::get_current_process_parent_id() {
        Some(ppid) => SyscallResult::success(ppid as i64),
        None => SyscallResult::success(0), // No parent (init process)
    }
}

fn sys_sleep(_milliseconds: u64) -> SyscallResult {
    // sleep_current_process function doesn't exist, using yield and block
    crate::process::yield_current();
    crate::process::block_current();
    SyscallResult::success(0)
}

fn sys_yield() -> SyscallResult {
    crate::process::yield_current();
    SyscallResult::success(0)
}

// File system syscalls
fn sys_open(path: u64, flags: u64, _mode: u64) -> SyscallResult {
    let path_str = unsafe { c_str_from_user(path) };
    match crate::filesystem::open(&path_str, flags as u32) {
        Ok(fd) => SyscallResult::success(fd as i64),
        Err(_) => SyscallResult::error(SyscallError::ResourceNotFound)
    }
}

fn sys_close(fd: u64) -> SyscallResult {
    match crate::filesystem::close(fd) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument)
    }
}

fn sys_read(fd: u64, buffer: u64, count: u64) -> SyscallResult {
    let mut buf = vec![0u8; count as usize];
    match crate::filesystem::read(fd as u64, &mut buf) {
        Ok(bytes_read) => {
            unsafe { copy_to_user(buffer, &buf[..bytes_read]) };
            SyscallResult::success(bytes_read as i64)
        }
        Err(_) => SyscallResult::error(SyscallError::IoError)
    }
}

fn sys_write(fd: u64, buffer: u64, count: u64) -> SyscallResult {
    let data = unsafe { slice_from_user(buffer, count as usize) };
    match crate::filesystem::write(fd as u64, &data) {
        Ok(bytes_written) => SyscallResult::success(bytes_written as i64),
        Err(_) => SyscallResult::error(SyscallError::IoError)
    }
}

fn sys_seek(fd: u64, offset: i64, whence: u64) -> SyscallResult {
    let seek_from = match whence {
        0 => crate::filesystem::SeekFrom::Start(offset as u64),
        1 => crate::filesystem::SeekFrom::Current(offset),
        2 => crate::filesystem::SeekFrom::End(offset),
        _ => return SyscallResult::error(SyscallError::InvalidArgument),
    };
    
    match crate::filesystem::seek(fd, seek_from) {
        Ok(new_pos) => SyscallResult::success(new_pos as i64),
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument)
    }
}

fn sys_stat(path: u64, statbuf: u64) -> SyscallResult {
    let path_str = unsafe { c_str_from_user(path) };
    match crate::filesystem::metadata(&path_str) {
        Ok(stat) => {
            // Create a simple stat structure
            let stat_data = [
                stat.size,
                stat.created,
                stat.modified,
                stat.accessed,
                if stat.file_type == crate::filesystem::FileType::Directory { 1 } else { 0 },
                if stat.file_type == crate::filesystem::FileType::Regular { 1 } else { 0 },
                stat.permissions as u64,
                0, // padding
            ];
            
            unsafe {
                copy_to_user(statbuf, unsafe_any_as_bytes(&stat_data));
            }
            SyscallResult::success(0)
        }
        Err(_) => SyscallResult::error(SyscallError::ResourceNotFound)
    }
}

fn sys_mkdir(path: u64, _mode: u64) -> SyscallResult {
    let path_str = unsafe { c_str_from_user(path) };
    match crate::filesystem::create_directory(&path_str) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::IoError)
    }
}

fn sys_rmdir(path: u64) -> SyscallResult {
    let path_str = unsafe { c_str_from_user(path) };
    match crate::filesystem::remove(&path_str) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::IoError)
    }
}

fn sys_unlink(path: u64) -> SyscallResult {
    let path_str = unsafe { c_str_from_user(path) };
    match crate::filesystem::remove(&path_str) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::IoError)
    }
}

// Memory management syscalls
fn sys_mmap(_addr: u64, length: u64, prot: u64, _flags: u64, _fd: u64, _offset: i64) -> SyscallResult {
    // Validate allocation length
    if length == 0 || length > 0x40000000 { // 1GB limit
        return SyscallResult::error(SyscallError::InvalidArgument);
    }
    
    // Parse POSIX protection flags
    let mut permissions = crate::vmm::VmPermissions::empty();
    if prot & 0x1 != 0 { permissions |= crate::vmm::VmPermissions::READ; }
    if prot & 0x2 != 0 { permissions |= crate::vmm::VmPermissions::WRITE; }
    if prot & 0x4 != 0 { permissions |= crate::vmm::VmPermissions::EXECUTE; }
    
    // Enforce W^X policy
    if let Err(_) = permissions.validate_wx_policy() {
        return SyscallResult::error(SyscallError::PermissionDenied);
    }
    
    match crate::vmm::allocate_area(
        0, // Use current address space ID
        length,
        crate::vmm::VmAreaType::Heap,
        permissions
    ) {
        Ok(addr) => SyscallResult::success(addr.as_u64() as i64),
        Err(_) => SyscallResult::error(SyscallError::OutOfMemory)
    }
}

fn sys_munmap(addr: u64, _length: u64) -> SyscallResult {
    match crate::vmm::deallocate_area(0, x86_64::VirtAddr::new(addr)) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument)
    }
}

fn sys_mprotect(addr: u64, length: u64, prot: u64) -> SyscallResult {
    // Validate arguments
    if addr == 0 || length == 0 {
        return SyscallResult::error(SyscallError::InvalidArgument);
    }
    
    // Parse POSIX protection flags
    let mut permissions = crate::vmm::VmPermissions::empty();
    if prot & 0x1 != 0 { permissions |= crate::vmm::VmPermissions::READ; }
    if prot & 0x2 != 0 { permissions |= crate::vmm::VmPermissions::WRITE; }
    if prot & 0x4 != 0 { permissions |= crate::vmm::VmPermissions::EXECUTE; }
    
    // Enforce W^X policy
    if let Err(_) = permissions.validate_wx_policy() {
        return SyscallResult::error(SyscallError::PermissionDenied);
    }
    
    match crate::vmm::protect_memory_api(
        0, // Use current address space ID
        x86_64::VirtAddr::new(addr),
        length as usize,
        permissions
    ) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument)
    }
}

fn sys_brk(addr: u64) -> SyscallResult {
    match crate::memory::set_program_break(x86_64::VirtAddr::new(addr)) {
        Ok(new_brk) => SyscallResult::success(new_brk.as_u64() as i64),
        Err(_) => SyscallResult::error(SyscallError::OutOfMemory)
    }
}

// IPC syscalls
fn sys_pipe(pipefd: u64) -> SyscallResult {
    match crate::ipc::create_pipe() {
        Ok((read_fd, write_fd)) => {
            let fds = [read_fd as u64, write_fd as u64];
            unsafe {
                copy_to_user(pipefd, unsafe_any_as_bytes(&fds));
            }
            SyscallResult::success(0)
        }
        Err(_) => SyscallResult::error(SyscallError::OutOfMemory)
    }
}

fn sys_socket(domain: u64, socket_type: u64, protocol: u64) -> SyscallResult {
    match crate::network::create_socket(domain as u32, socket_type as u32, protocol as u32) {
        Ok(socket_fd) => SyscallResult::success(socket_fd as i64),
        Err(_) => SyscallResult::error(SyscallError::NetworkError)
    }
}

fn sys_bind(socket_fd: u64, addr: u64, addr_len: u64) -> SyscallResult {
    let addr_data = unsafe { slice_from_user(addr, addr_len as usize) };
    match crate::network::bind_socket(socket_fd as u32, &addr_data) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::NetworkError)
    }
}

fn sys_listen(socket_fd: u64, backlog: u64) -> SyscallResult {
    match crate::network::listen_socket(socket_fd as u32, backlog as u32) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::NetworkError)
    }
}

fn sys_accept(socket_fd: u64, _addr: u64, _addr_len: u64) -> SyscallResult {
    match crate::network::accept_connection(socket_fd as u32) {
        Ok(client_fd) => SyscallResult::success(client_fd as i64),
        Err(_) => {
            // No pending connections
            SyscallResult::error(SyscallError::ResourceBusy)
        }
    }
}

fn sys_connect(socket_fd: u64, addr: u64, addr_len: u64) -> SyscallResult {
    let addr_data = unsafe { slice_from_user(addr, addr_len as usize) };
    match crate::network::connect_socket(socket_fd as u32, &addr_data) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::NetworkError)
    }
}

fn sys_send(socket_fd: u64, buffer: u64, length: u64, flags: u64) -> SyscallResult {
    let data = unsafe { slice_from_user(buffer, length as usize) };
    match crate::network::send_data(socket_fd as u32, &data, flags as u32) {
        Ok(bytes_sent) => SyscallResult::success(bytes_sent as i64),
        Err(_) => SyscallResult::error(SyscallError::NetworkError)
    }
}

fn sys_recv(socket_fd: u64, buffer: u64, length: u64, flags: u64) -> SyscallResult {
    match crate::network::receive_data(socket_fd as u32, length as usize, flags as u32) {
        Ok(data) => {
            let bytes_received = data.len();
            unsafe { copy_to_user(buffer, &data) };
            SyscallResult::success(bytes_received as i64)
        }
        Err(_) => SyscallResult::error(SyscallError::NetworkError)
    }
}

// RaeenOS specific syscalls
fn sys_set_game_mode(enabled: bool) -> SyscallResult {
    crate::process::set_gaming_mode(enabled);
    SyscallResult::success(0)
}

fn sys_get_system_info(info_type: u64) -> SyscallResult {
    match info_type {
        0 => { // CPU info
            // TODO: Implement CPU info retrieval
            let _cpu_info = "Unknown CPU";
            SyscallResult::success(0)
        }
        1 => { // Memory info
            let total = crate::memory::get_total_memory();
            let free = crate::memory::get_free_memory();
            SyscallResult::success(((total >> 32) | (free & 0xFFFFFFFF)) as i64)
        }
        2 => { // Process count
            let count = crate::process::get_process_count();
            SyscallResult::success(count as i64)
        }
        3 => { // Uptime
            let uptime = crate::time::get_uptime_ms();
            SyscallResult::success(uptime as i64)
        }
        _ => SyscallResult::error(SyscallError::InvalidArgument)
    }
}

fn sys_set_theme(theme_id: u64, options: u64) -> SyscallResult {
    match crate::ui::set_system_theme(theme_id as u32, options as u32) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument)
    }
}

fn sys_create_window(x: u64, y: u64, width: u64, height: u64, _flags: u64) -> SyscallResult {
    let window_id = crate::graphics::create_window("Window", x as i32, y as i32, width as u32, height as u32, 0);
    SyscallResult::success(window_id as i64)
}

fn sys_destroy_window(window_id: u64) -> SyscallResult {
    if crate::graphics::destroy_window(window_id as crate::graphics::WindowId) {
        SyscallResult::success(0)
    } else {
        SyscallResult::error(SyscallError::ResourceNotFound)
    }
}

fn sys_draw_pixel(window_id: u64, x: u64, y: u64, color: u64) -> SyscallResult {
    let c = u32_color_from_u64(color);
    let color = crate::graphics::Color { r: ((c >> 16) & 0xFF) as u8, g: ((c >> 8) & 0xFF) as u8, b: (c & 0xFF) as u8, a: ((c >> 24) & 0xFF) as u8 };
    match crate::graphics::draw_pixel(window_id as crate::graphics::WindowId, x as u32, y as u32, color) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument)
    }
}

fn sys_draw_rect(window_id: u64, x: u64, y: u64, width: u64, height: u64, color: u64) -> SyscallResult {
    let c = u32_color_from_u64(color);
    let color = crate::graphics::Color { r: ((c >> 16) & 0xFF) as u8, g: ((c >> 8) & 0xFF) as u8, b: (c & 0xFF) as u8, a: ((c >> 24) & 0xFF) as u8 };
    let rect = crate::graphics::Rect::new(x as i32, y as i32, width as u32, height as u32);
    match crate::graphics::draw_rect(window_id as crate::graphics::WindowId, rect, color) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument)
    }
}

fn sys_draw_text(window_id: u64, x: u64, y: u64, text: u64, color: u64) -> SyscallResult {
    let text_str = unsafe { c_str_from_user(text) };
    let c = u32_color_from_u64(color);
    let color = crate::graphics::Color { r: ((c >> 16) & 0xFF) as u8, g: ((c >> 8) & 0xFF) as u8, b: (c & 0xFF) as u8, a: ((c >> 24) & 0xFF) as u8 };
    match crate::graphics::draw_text(window_id as crate::graphics::WindowId, x as i32, y as i32, &text_str, color) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument)
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

// Enhanced graphics syscalls
fn sys_set_vsync(enabled: bool) -> SyscallResult {
    match crate::graphics::set_vsync(enabled) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::IoError)
    }
}

fn sys_get_frame_stats(buffer: u64) -> SyscallResult {
    match crate::graphics::get_frame_stats() {
        Ok((frame_count, last_present_time)) => {
            let stats = [frame_count, last_present_time];
            unsafe { copy_to_user(buffer, unsafe_any_as_bytes(&stats)) };
            SyscallResult::success(0)
        }
        Err(_) => SyscallResult::error(SyscallError::IoError)
    }
}

fn sys_clear_framebuffer(color: u64) -> SyscallResult {
    let c = u32_color_from_u64(color);
    let color = crate::graphics::Color { 
        r: ((c >> 16) & 0xFF) as u8, 
        g: ((c >> 8) & 0xFF) as u8, 
        b: (c & 0xFF) as u8, 
        a: ((c >> 24) & 0xFF) as u8 
    };
    match crate::graphics::clear_framebuffer(color) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::IoError)
    }
}

fn sys_blit_buffer(src_buffer: u64, dst_x: u64, dst_y: u64, width: u64, height: u64, stride: u64) -> SyscallResult {
    let buffer_size = (height * stride) as usize;
    let src_data = unsafe { slice_from_user(src_buffer, buffer_size) };
    
    match crate::graphics::blit_buffer(&src_data, dst_x as u32, dst_y as u32, width as u32, height as u32, stride as u32) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::InvalidArgument)
    }
}

fn sys_set_input_focus(window_id: u64) -> SyscallResult {
    match crate::graphics::set_input_focus(window_id as crate::graphics::WindowId) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::ResourceNotFound)
    }
}

fn sys_get_window_list(buffer: u64, max_count: u64) -> SyscallResult {
    let window_ids = crate::graphics::get_window_list();
    let count = core::cmp::min(window_ids.len(), max_count as usize);
    let data = &window_ids[..count];
    let bytes = unsafe { core::slice::from_raw_parts(data.as_ptr() as *const u8, data.len() * core::mem::size_of::<u32>()) };
    unsafe { copy_to_user(buffer, bytes) };
    SyscallResult::success(count as i64)
}

fn sys_resize_window(window_id: u64, width: u64, height: u64) -> SyscallResult {
    match crate::graphics::resize_window(window_id as crate::graphics::WindowId, width as u32, height as u32) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::ResourceNotFound)
    }
}

fn sys_move_window(window_id: u64, x: u64, y: u64) -> SyscallResult {
    match crate::graphics::move_window(window_id as crate::graphics::WindowId, x as i32, y as i32) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::ResourceNotFound)
    }
}

// Signal handling syscalls
 fn sys_signal(signal: i32, handler: u64) -> SyscallResult {
     use crate::process::{Signal, set_signal_handler, SignalHandler};
     
     // Convert signal number to Signal enum
     let sig = match signal {
         9 => Signal::SIGKILL,
         15 => Signal::SIGTERM,
         19 => Signal::SIGSTOP,
         18 => Signal::SIGCONT,
         10 => Signal::SIGUSR1,
         12 => Signal::SIGUSR2,
         _ => return SyscallResult::error(SyscallError::InvalidArgument),
     };
     
     // For now, we only support default handlers (handler = 0) or ignore (handler = 1)
     let handler_fn = if handler == 0 {
         None // Use default handler
     } else if handler == 1 {
         // Ignore signal - create a no-op handler
         fn ignore_signal(_signal: Signal) {}
         Some(ignore_signal as SignalHandler)
     } else {
         // Custom user handlers not yet implemented
         return SyscallResult::error(SyscallError::NotImplemented);
     };
     
     match set_signal_handler(sig, handler_fn) {
         Ok(()) => SyscallResult::success(0),
         Err(_) => SyscallResult::error(SyscallError::InvalidArgument)
     }
 }
 
 fn sys_sigaction(signal: i32, new_action: u64, _old_action: u64) -> SyscallResult {
     // Basic sigaction implementation - for now just redirect to sys_signal
     if new_action != 0 {
         // For simplicity, treat new_action as a handler address
         sys_signal(signal, new_action)
     } else {
         SyscallResult::error(SyscallError::InvalidArgument)
     }
 }
 
 fn sys_sigreturn() -> SyscallResult {
     // Signal return not yet fully implemented
     SyscallResult::success(0)
 }

// Security syscalls
fn sys_request_permission(permission: u64) -> SyscallResult {
    // Implement permission requesting
    let current_pid = match crate::process::get_current_process_info() {
        Some((pid, _, _)) => pid as u32,
        None => return SyscallResult::error(SyscallError::ResourceNotFound)
    };
    
    // Convert permission u64 to string (simplified mapping)
    let permission_str = match permission {
        1 => "file.read",
        2 => "file.write",
        3 => "file.execute",
        4 => "network.connect",
        5 => "network.bind",
        _ => "unknown"
    };
    
    match crate::security::request_permission(current_pid as u32, permission_str) {
        Ok(granted) => SyscallResult::success(if granted { 1 } else { 0 }),
        Err(_) => SyscallResult::error(SyscallError::PermissionDenied)
    }
}

fn sys_set_sandbox(level: u64) -> SyscallResult {
    // Implement sandbox setting
    let current_pid = match crate::process::get_current_process_info() {
        Some((pid, _, _)) => pid as u32,
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
        Some((pid, _, _)) => pid as u32,
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

// Capability syscalls
fn sys_cap_clone(handle_id: u64, new_rights: u64) -> SyscallResult {
    let current_pid = match crate::process::get_current_process_info() {
        Some((pid, _, _)) => pid as u32,
        None => return SyscallResult::error(SyscallError::ResourceNotFound)
    };
    
    // Convert u64 to IpcRights
    let rights = crate::ipc::IpcRights::from_bits(new_rights as u32)
        .unwrap_or(crate::ipc::IpcRights::NONE);
    
    match crate::ipc::clone_handle(current_pid, handle_id as u32, rights) {
        Ok(new_handle) => SyscallResult::success(new_handle as i64),
        Err(_) => SyscallResult::error(SyscallError::PermissionDenied)
    }
}

fn sys_cap_revoke(handle_id: u64) -> SyscallResult {
    let current_pid = match crate::process::get_current_process_info() {
        Some((pid, _, _)) => pid as u32,
        None => return SyscallResult::error(SyscallError::ResourceNotFound)
    };
    
    match crate::ipc::revoke_handle(current_pid, handle_id as u32) {
        Ok(()) => SyscallResult::success(0),
        Err(_) => SyscallResult::error(SyscallError::PermissionDenied)
    }
}

fn sys_cap_transfer(handle_id: u64, target_pid: u64, new_rights: u64) -> SyscallResult {
    let current_pid = match crate::process::get_current_process_info() {
        Some((pid, _, _)) => pid as u32,
        None => return SyscallResult::error(SyscallError::ResourceNotFound)
    };
    
    // Convert u64 to IpcRights
    let rights = crate::ipc::IpcRights::from_bits(new_rights as u32)
        .unwrap_or(crate::ipc::IpcRights::NONE);
    
    match crate::ipc::transfer_handle(current_pid, handle_id as u32, target_pid as u32, rights) {
        Ok(new_handle) => SyscallResult::success(new_handle as i64),
        Err(_) => SyscallResult::error(SyscallError::PermissionDenied)
    }
}

fn sys_cap_delegate(handle_id: u64, target_pid: u64, new_rights: u64) -> SyscallResult {
    let current_pid = match crate::process::get_current_process_info() {
        Some((pid, _, _)) => pid as u32,
        None => return SyscallResult::error(SyscallError::ResourceNotFound)
    };
    
    // Convert u64 to IpcRights
    let rights = crate::ipc::IpcRights::from_bits(new_rights as u32)
        .unwrap_or(crate::ipc::IpcRights::NONE);
    
    match crate::ipc::delegate_handle(current_pid, target_pid as u32, handle_id as u32, rights, None, None) {
        Ok(new_handle) => SyscallResult::success(new_handle as i64),
        Err(_) => SyscallResult::error(SyscallError::PermissionDenied)
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
            let analysis_bytes = analysis.as_slice();
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
    // Set up SYSCALL/SYSRET mechanism
    setup_syscall_entry();
}

/// Set up the SYSCALL/SYSRET mechanism
fn setup_syscall_entry() {
    use x86_64::registers::model_specific::{LStar, SFMask, Star};
    use x86_64::registers::rflags::RFlags;
    
    // Set up STAR register with kernel/user code segments
    let _kernel_cs = crate::gdt::get_kernel_code_selector().0 as u64;
    let _user_cs = crate::gdt::get_user_code_selector().0 as u64;
    
    // STAR[63:48] = User CS, STAR[47:32] = Kernel CS
    let kernel_cs_selector = SegmentSelector::new(1, PrivilegeLevel::Ring0); // GDT index 1
    let user_cs_selector = SegmentSelector::new(2, PrivilegeLevel::Ring3);   // GDT index 2
    let user_ss_selector = SegmentSelector::new(3, PrivilegeLevel::Ring3);   // GDT index 3
    Star::write(kernel_cs_selector, user_cs_selector, user_ss_selector, user_cs_selector).unwrap();
    
    // Set LSTAR to point to our syscall entry point
    LStar::write(VirtAddr::new(syscall_entry as u64));
    
    // Set SFMASK to mask interrupts during syscall
    SFMask::write(RFlags::INTERRUPT_FLAG);
    
    // Enable SYSCALL/SYSRET in EFER
    use x86_64::registers::model_specific::Efer;
    let mut efer = Efer::read();
    efer |= x86_64::registers::model_specific::EferFlags::SYSTEM_CALL_EXTENSIONS;
    unsafe { Efer::write(efer); }
}

// Low-level syscall entry point
extern "C" {
    fn syscall_entry();
}

// Assembly implementation of syscall entry
core::arch::global_asm!(
    ".global syscall_entry",
    "syscall_entry:",
    // Save user stack pointer
    "mov gs:[0x10], rsp",  // Save user RSP to per-CPU area
    
    // Switch to kernel stack
    "mov rsp, gs:[0x08]",  // Load kernel RSP from per-CPU area
    
    // Save user registers
    "push rcx",            // User RIP (saved by SYSCALL)
    "push r11",            // User RFLAGS (saved by SYSCALL)
    "push rax",            // Syscall number
    "push rdi",            // Arg 1
    "push rsi",            // Arg 2
    "push rdx",            // Arg 3
    "push r10",            // Arg 4 (r10 instead of rcx for syscalls)
    "push r8",             // Arg 5
    "push r9",             // Arg 6
    
    // Call high-level syscall handler
    "call syscall_handler_wrapper",
    
    // Restore user registers
    "pop r9",
    "pop r8",
    "pop r10",
    "pop rdx",
    "pop rsi",
    "pop rdi",
    "add rsp, 8",          // Skip syscall number
    "pop r11",             // Restore user RFLAGS
    "pop rcx",             // Restore user RIP
    
    // Restore user stack pointer
    "mov rsp, gs:[0x10]",
    
    // Return to userspace
    "sysretq"
);

// High-level syscall handler wrapper
#[no_mangle]
extern "C" fn syscall_handler_wrapper(
    syscall_num: u64,
    arg1: u64,
    arg2: u64,
    arg3: u64,
    arg4: u64,
    arg5: u64,
    arg6: u64,
) -> u64 {
    // Call the existing syscall handler
    syscall_handler(syscall_num, arg1, arg2, arg3, arg4, arg5, arg6)
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