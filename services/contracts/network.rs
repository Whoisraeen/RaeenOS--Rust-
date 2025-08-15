//! Network service contract for rae-netd
//! Defines IPC interface for user-space network stack

use alloc::vec::Vec;
use alloc::string::String;
use serde::{Serialize, Deserialize};
use super::{ServiceResponse, error_codes};

/// Network service requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkRequest {
    // Socket operations
    CreateSocket { domain: SocketDomain, socket_type: SocketType, protocol: u32 },
    BindSocket { socket_id: u32, address: SocketAddress },
    ListenSocket { socket_id: u32, backlog: u32 },
    AcceptSocket { socket_id: u32 },
    ConnectSocket { socket_id: u32, address: SocketAddress },
    SendData { socket_id: u32, data: Vec<u8>, flags: u32 },
    ReceiveData { socket_id: u32, max_length: usize, flags: u32 },
    CloseSocket { socket_id: u32 },
    
    // Network interface management
    ListInterfaces,
    GetInterfaceInfo { interface_name: String },
    SetInterfaceState { interface_name: String, enabled: bool },
    
    // Routing and ARP
    AddRoute { destination: IpAddress, gateway: IpAddress, interface: String },
    RemoveRoute { destination: IpAddress },
    GetRoutingTable,
    GetArpTable,
    
    // DNS resolution
    ResolveHostname { hostname: String },
    ReverseLookup { ip_address: IpAddress },
    
    // DHCP client
    StartDhcpClient { interface_name: String },
    StopDhcpClient { interface_name: String },
    GetDhcpLease { interface_name: String },
}

/// Network service responses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkResponse {
    SocketCreated { socket_id: u32 },
    SocketBound,
    SocketListening,
    ConnectionAccepted { new_socket_id: u32, peer_address: SocketAddress },
    SocketConnected,
    DataSent { bytes_sent: usize },
    DataReceived { data: Vec<u8>, sender_address: Option<SocketAddress> },
    SocketClosed,
    
    InterfaceList { interfaces: Vec<NetworkInterface> },
    InterfaceInfo { interface: NetworkInterface },
    InterfaceStateChanged,
    
    RouteAdded,
    RouteRemoved,
    RoutingTable { routes: Vec<RouteEntry> },
    ArpTable { entries: Vec<ArpEntry> },
    
    HostnameResolved { ip_addresses: Vec<IpAddress> },
    ReverseLookupResult { hostname: String },
    
    DhcpClientStarted,
    DhcpClientStopped,
    DhcpLease { lease: DhcpLeaseInfo },
}

/// Socket domains
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SocketDomain {
    Inet,   // IPv4
    Inet6,  // IPv6
    Unix,   // Unix domain sockets
}

/// Socket types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SocketType {
    Stream,    // TCP
    Datagram,  // UDP
    Raw,       // Raw sockets
}

/// Socket addresses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SocketAddress {
    Inet { ip: IpAddress, port: u16 },
    Inet6 { ip: IpAddress, port: u16, flow_info: u32, scope_id: u32 },
    Unix { path: String },
}

/// IP addresses
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpAddress {
    V4([u8; 4]),
    V6([u8; 16]),
}

/// Network interface information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub mac_address: [u8; 6],
    pub ip_addresses: Vec<IpAddress>,
    pub mtu: u32,
    pub enabled: bool,
    pub link_up: bool,
    pub statistics: InterfaceStatistics,
}

/// Interface statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceStatistics {
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_errors: u64,
    pub tx_errors: u64,
    pub rx_dropped: u64,
    pub tx_dropped: u64,
}

/// Routing table entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouteEntry {
    pub destination: IpAddress,
    pub netmask: IpAddress,
    pub gateway: Option<IpAddress>,
    pub interface: String,
    pub metric: u32,
}

/// ARP table entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArpEntry {
    pub ip_address: IpAddress,
    pub mac_address: [u8; 6],
    pub interface: String,
    pub state: ArpEntryState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ArpEntryState {
    Incomplete,
    Reachable,
    Stale,
    Delay,
    Probe,
    Failed,
}

/// DHCP lease information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DhcpLeaseInfo {
    pub ip_address: IpAddress,
    pub subnet_mask: IpAddress,
    pub gateway: Option<IpAddress>,
    pub dns_servers: Vec<IpAddress>,
    pub lease_time: u32,
    pub renewal_time: u32,
    pub rebinding_time: u32,
}

/// Network service performance metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMetrics {
    pub packets_processed: u64,
    pub bytes_transferred: u64,
    pub active_connections: u32,
    pub average_latency_us: u32,
    pub packet_loss_rate: f32,
    pub queue_depth: u32,
}

/// Network service configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub enable_ipv6: bool,
    pub tcp_window_size: u32,
    pub udp_buffer_size: u32,
    pub nic_queue_count: u32,
    pub interrupt_coalescing: bool,
    pub busy_poll_enabled: bool,
    pub congestion_control: CongestionControl,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CongestionControl {
    Cubic,
    Bbr,
    Reno,
}

/// Convenience type alias for network service responses
pub type NetworkServiceResponse<T> = ServiceResponse<T>;

/// Network service error codes (extending common error codes)
pub mod network_errors {
    use super::error_codes;
    
    pub const SOCKET_NOT_FOUND: u32 = error_codes::INTERNAL_ERROR + 1;
    pub const ADDRESS_IN_USE: u32 = error_codes::INTERNAL_ERROR + 2;
    pub const CONNECTION_REFUSED: u32 = error_codes::INTERNAL_ERROR + 3;
    pub const NETWORK_UNREACHABLE: u32 = error_codes::INTERNAL_ERROR + 4;
    pub const HOST_UNREACHABLE: u32 = error_codes::INTERNAL_ERROR + 5;
    pub const CONNECTION_TIMEOUT: u32 = error_codes::INTERNAL_ERROR + 6;
    pub const INTERFACE_NOT_FOUND: u32 = error_codes::INTERNAL_ERROR + 7;
    pub const ROUTE_NOT_FOUND: u32 = error_codes::INTERNAL_ERROR + 8;
    pub const DNS_RESOLUTION_FAILED: u32 = error_codes::INTERNAL_ERROR + 9;
    pub const DHCP_FAILED: u32 = error_codes::INTERNAL_ERROR + 10;
}