//! Network subsystem for RaeenOS

use alloc::vec::Vec;

#[derive(Debug)]
pub enum NetworkError {
    InvalidSocket,
    NotConnected,
    Timeout,
    InvalidAddress,
    PortInUse,
    ConnectionRefused,
    WouldBlock,
    PermissionDenied,
    AddressFamilyNotSupported,
    ProtocolNotSupported,
    SocketTypeNotSupported,
}

impl From<NetworkError> for crate::syscall::SyscallError {
    fn from(err: NetworkError) -> Self {
        match err {
            NetworkError::InvalidSocket => crate::syscall::SyscallError::InvalidArgument,
            NetworkError::NotConnected => crate::syscall::SyscallError::InvalidArgument,
            NetworkError::Timeout => crate::syscall::SyscallError::InvalidArgument,
            NetworkError::InvalidAddress => crate::syscall::SyscallError::InvalidArgument,
            NetworkError::PortInUse => crate::syscall::SyscallError::InvalidArgument,
            NetworkError::ConnectionRefused => crate::syscall::SyscallError::InvalidArgument,
            NetworkError::WouldBlock => crate::syscall::SyscallError::InvalidArgument,
            NetworkError::PermissionDenied => crate::syscall::SyscallError::PermissionDenied,
            NetworkError::AddressFamilyNotSupported => crate::syscall::SyscallError::InvalidArgument,
            NetworkError::ProtocolNotSupported => crate::syscall::SyscallError::InvalidArgument,
            NetworkError::SocketTypeNotSupported => crate::syscall::SyscallError::InvalidArgument,
        }
    }
}

pub type NetworkResult<T> = Result<T, NetworkError>;

use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

// Socket domains
const AF_INET: u32 = 2;  // IPv4
const AF_INET6: u32 = 10; // IPv6

// Socket types
const SOCK_STREAM: u32 = 1; // TCP
const SOCK_DGRAM: u32 = 2;  // UDP

// Protocols
const IPPROTO_TCP: u32 = 6;
const IPPROTO_UDP: u32 = 17;

// Socket states
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SocketState {
    Created,
    Bound,
    Listening,
    Connected,
    Closed,
}

// Socket address
#[derive(Debug, Clone)]
struct SocketAddr {
    ip: [u8; 4], // IPv4 for simplicity
    port: u16,
}

impl SocketAddr {
    fn from_bytes(data: &[u8]) -> NetworkResult<Self> {
        if data.len() < 6 {
            return Err(NetworkError::InvalidAddress);
        }
        
        let ip = [data[0], data[1], data[2], data[3]];
        let port = u16::from_be_bytes([data[4], data[5]]);
        
        Ok(SocketAddr { ip, port })
    }
}

// Socket implementation
#[derive(Debug)]
struct Socket {
    domain: u32,
    socket_type: u32,
    protocol: u32,
    state: SocketState,
    local_addr: Option<SocketAddr>,
    remote_addr: Option<SocketAddr>,
    backlog: u32,
    pending_connections: Vec<u32>,
    receive_buffer: Vec<u8>,
    send_buffer: Vec<u8>,
    process_id: u32,
}

impl Socket {
    fn new(domain: u32, socket_type: u32, protocol: u32, process_id: u32) -> Self {
        Self {
            domain,
            socket_type,
            protocol,
            state: SocketState::Created,
            local_addr: None,
            remote_addr: None,
            backlog: 0,
            pending_connections: Vec::new(),
            receive_buffer: Vec::new(),
            send_buffer: Vec::new(),
            process_id,
        }
    }
}

// Network system state
struct NetworkSystem {
    sockets: BTreeMap<u32, Socket>,
    next_socket_fd: u32,
    port_allocations: BTreeMap<u16, u32>, // port -> socket_fd
}

lazy_static! {
    static ref NETWORK_SYSTEM: Mutex<NetworkSystem> = Mutex::new(NetworkSystem {
        sockets: BTreeMap::new(),
        next_socket_fd: 1,
        port_allocations: BTreeMap::new(),
    });
}

pub fn create_socket(domain: u32, socket_type: u32, protocol: u32) -> NetworkResult<u32> {
    let mut network = NETWORK_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    // Check permission
    if !crate::security::request_permission(current_pid, "network.access").unwrap_or(false) {
        return Err(NetworkError::PermissionDenied);
    }
    
    // Validate parameters
    match domain {
        AF_INET => {}, // IPv4 supported
        AF_INET6 => return Err(NetworkError::AddressFamilyNotSupported), // IPv6 not implemented
        _ => return Err(NetworkError::AddressFamilyNotSupported),
    }
    
    match socket_type {
        SOCK_STREAM => {
            if protocol != 0 && protocol != IPPROTO_TCP {
                return Err(NetworkError::ProtocolNotSupported);
            }
        }
        SOCK_DGRAM => {
            if protocol != 0 && protocol != IPPROTO_UDP {
                return Err(NetworkError::ProtocolNotSupported);
            }
        }
        _ => return Err(NetworkError::SocketTypeNotSupported),
    }
    
    let socket_fd = network.next_socket_fd;
    network.next_socket_fd += 1;
    
    let socket = Socket::new(domain, socket_type, protocol, current_pid);
    network.sockets.insert(socket_fd, socket);
    
    Ok(socket_fd)
}

pub fn bind_socket(socket_fd: u32, addr: &[u8]) -> NetworkResult<()> {
    let mut network = NETWORK_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    let socket = network.sockets.get_mut(&socket_fd)
        .ok_or(NetworkError::InvalidSocket)?;
    
    // Check ownership
    if socket.process_id != current_pid {
        return Err(NetworkError::PermissionDenied);
    }
    
    // Check state
    if socket.state != SocketState::Created {
        return Err(NetworkError::AlreadyBound);
    }
    
    let socket_addr = SocketAddr::from_bytes(addr)?;
    
    // Check if port is already in use
    if network.port_allocations.contains_key(&socket_addr.port) {
        return Err(NetworkError::AddressInUse);
    }
    
    // Bind the socket
    socket.local_addr = Some(socket_addr.clone());
    socket.state = SocketState::Bound;
    network.port_allocations.insert(socket_addr.port, socket_fd);
    
    Ok(())
}

pub fn listen_socket(socket_fd: u32, backlog: u32) -> NetworkResult<()> {
    let mut network = NETWORK_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    let socket = network.sockets.get_mut(&socket_fd)
        .ok_or(NetworkError::InvalidSocket)?;
    
    // Check ownership
    if socket.process_id != current_pid {
        return Err(NetworkError::PermissionDenied);
    }
    
    // Check socket type (only TCP can listen)
    if socket.socket_type != SOCK_STREAM {
        return Err(NetworkError::OperationNotSupported);
    }
    
    // Check state
    if socket.state != SocketState::Bound {
        return Err(NetworkError::NotBound);
    }
    
    socket.state = SocketState::Listening;
    socket.backlog = backlog;
    socket.pending_connections.clear();
    
    Ok(())
}

pub fn accept_connection(socket_fd: u32) -> NetworkResult<u32> {
    let mut network = NETWORK_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    let socket = network.sockets.get_mut(&socket_fd)
        .ok_or(NetworkError::InvalidSocket)?;
    
    // Check ownership
    if socket.process_id != current_pid {
        return Err(NetworkError::PermissionDenied);
    }
    
    // Check state
    if socket.state != SocketState::Listening {
        return Err(NetworkError::NotListening);
    }
    
    // Check for pending connections
    if socket.pending_connections.is_empty() {
        return Err(NetworkError::WouldBlock);
    }
    
    // Accept the first pending connection
    let client_fd = socket.pending_connections.remove(0);
    
    // In a real implementation, we would:
    // 1. Create a new socket for the accepted connection
    // 2. Set up the connection state
    // 3. Return the new socket file descriptor
    
    // For now, return a placeholder
    let new_socket_fd = network.next_socket_fd;
    network.next_socket_fd += 1;
    
    let mut client_socket = Socket::new(socket.domain, socket.socket_type, socket.protocol, current_pid);
    client_socket.state = SocketState::Connected;
    client_socket.local_addr = socket.local_addr.clone();
    
    network.sockets.insert(new_socket_fd, client_socket);
    
    Ok(new_socket_fd)
}

pub fn connect_socket(socket_fd: u32, addr: &[u8]) -> NetworkResult<()> {
    let mut network = NETWORK_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    let socket = network.sockets.get_mut(&socket_fd)
        .ok_or(NetworkError::InvalidSocket)?;
    
    // Check ownership
    if socket.process_id != current_pid {
        return Err(NetworkError::PermissionDenied);
    }
    
    // Check state
    if socket.state != SocketState::Created && socket.state != SocketState::Bound {
        return Err(NetworkError::AlreadyConnected);
    }
    
    let remote_addr = SocketAddr::from_bytes(addr)?;
    
    // Check if there's a listening socket on the target address
    let target_socket_fd = network.port_allocations.get(&remote_addr.port)
        .copied()
        .ok_or(NetworkError::ConnectionRefused)?;
    
    let target_socket = network.sockets.get_mut(&target_socket_fd)
        .ok_or(NetworkError::ConnectionRefused)?;
    
    if target_socket.state != SocketState::Listening {
        return Err(NetworkError::ConnectionRefused);
    }
    
    // Add to pending connections if there's space
    if target_socket.pending_connections.len() >= target_socket.backlog as usize {
        return Err(NetworkError::ConnectionRefused);
    }
    
    target_socket.pending_connections.push(socket_fd);
    
    // Update socket state
    socket.remote_addr = Some(remote_addr);
    socket.state = SocketState::Connected;
    
    Ok(())
}

pub fn send_data(socket_fd: u32, data: &[u8], flags: u32) -> NetworkResult<usize> {
    let mut network = NETWORK_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    let socket = network.sockets.get_mut(&socket_fd)
        .ok_or(NetworkError::InvalidSocket)?;
    
    // Check ownership
    if socket.process_id != current_pid {
        return Err(NetworkError::PermissionDenied);
    }
    
    // Check state
    match socket.socket_type {
        SOCK_STREAM => {
            if socket.state != SocketState::Connected {
                return Err(NetworkError::NotConnected);
            }
        }
        SOCK_DGRAM => {
            if socket.state != SocketState::Bound && socket.state != SocketState::Connected {
                return Err(NetworkError::NotBound);
            }
        }
        _ => return Err(NetworkError::OperationNotSupported),
    }
    
    // Add data to send buffer (simplified)
    socket.send_buffer.extend_from_slice(data);
    
    // In a real implementation, we would:
    // 1. Fragment data into packets
    // 2. Add TCP/UDP headers
    // 3. Add IP headers
    // 4. Send via network interface
    
    // For now, simulate successful send
    Ok(data.len())
}

pub fn receive_data(socket_fd: u32, length: usize, flags: u32) -> NetworkResult<Vec<u8>> {
    let mut network = NETWORK_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    let socket = network.sockets.get_mut(&socket_fd)
        .ok_or(NetworkError::InvalidSocket)?;
    
    // Check ownership
    if socket.process_id != current_pid {
        return Err(NetworkError::PermissionDenied);
    }
    
    // Check state
    match socket.socket_type {
        SOCK_STREAM => {
            if socket.state != SocketState::Connected {
                return Err(NetworkError::NotConnected);
            }
        }
        SOCK_DGRAM => {
            if socket.state != SocketState::Bound && socket.state != SocketState::Connected {
                return Err(NetworkError::NotBound);
            }
        }
        _ => return Err(NetworkError::OperationNotSupported),
    }
    
    // Check if data is available
    if socket.receive_buffer.is_empty() {
        return Err(NetworkError::WouldBlock);
    }
    
    // Read data from receive buffer
    let to_read = core::cmp::min(length, socket.receive_buffer.len());
    let data = socket.receive_buffer.drain(0..to_read).collect();
    
    Ok(data)
}

// Close a socket
pub fn close_socket(socket_fd: u32) -> NetworkResult<()> {
    let mut network = NETWORK_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    let socket = network.sockets.get(&socket_fd)
        .ok_or(NetworkError::InvalidSocket)?;
    
    // Check ownership
    if socket.process_id != current_pid {
        return Err(NetworkError::PermissionDenied);
    }
    
    // Free port allocation if bound
    if let Some(local_addr) = &socket.local_addr {
        network.port_allocations.remove(&local_addr.port);
    }
    
    // Remove socket
    network.sockets.remove(&socket_fd);
    
    Ok(())
}

// Get socket information
pub fn get_socket_info(socket_fd: u32) -> NetworkResult<(SocketState, Option<SocketAddr>, Option<SocketAddr>)> {
    let network = NETWORK_SYSTEM.lock();
    let current_pid = crate::process::get_current_process_id();
    
    let socket = network.sockets.get(&socket_fd)
        .ok_or(NetworkError::InvalidSocket)?;
    
    // Check ownership
    if socket.process_id != current_pid {
        return Err(NetworkError::PermissionDenied);
    }
    
    Ok((socket.state, socket.local_addr.clone(), socket.remote_addr.clone()))
}

// Clean up network resources for a process
pub fn cleanup_process_network(process_id: u32) {
    let mut network = NETWORK_SYSTEM.lock();
    
    // Close all sockets owned by the process
    let sockets_to_close: Vec<u32> = network.sockets
        .iter()
        .filter(|(_, socket)| socket.process_id == process_id)
        .map(|(&fd, _)| fd)
        .collect();
    
    for socket_fd in sockets_to_close {
        let _ = close_socket(socket_fd);
    }
}

// Simulate receiving data (would be called by network driver)
pub fn simulate_receive(socket_fd: u32, data: &[u8]) -> NetworkResult<()> {
    let mut network = NETWORK_SYSTEM.lock();
    
    let socket = network.sockets.get_mut(&socket_fd)
        .ok_or(NetworkError::InvalidSocket)?;
    
    socket.receive_buffer.extend_from_slice(data);
    Ok(())
}