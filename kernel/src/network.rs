use alloc::collections::{BTreeMap, VecDeque};
use alloc::string::String;
use alloc::vec::Vec;
use alloc::boxed::Box;
use core::fmt;
use spin::{Mutex, RwLock};
use x86_64::instructions::port::Port;

static NETWORK_STACK: RwLock<NetworkStack> = RwLock::new(NetworkStack::new());

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MacAddress([u8; 6]);

impl MacAddress {
    pub const fn new(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }
    
    pub fn bytes(&self) -> &[u8; 6] {
        &self.0
    }
    
    pub fn is_broadcast(&self) -> bool {
        self.0 == [0xFF; 6]
    }
    
    pub fn is_multicast(&self) -> bool {
        (self.0[0] & 0x01) != 0
    }
}

impl fmt::Display for MacAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
               self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Ipv4Address([u8; 4]);

impl Ipv4Address {
    pub const fn new(a: u8, b: u8, c: u8, d: u8) -> Self {
        Self([a, b, c, d])
    }
    
    pub fn from_bytes(bytes: [u8; 4]) -> Self {
        Self(bytes)
    }
    
    pub fn bytes(&self) -> &[u8; 4] {
        &self.0
    }
    
    pub fn as_u32(&self) -> u32 {
        u32::from_be_bytes(self.0)
    }
    
    pub fn is_loopback(&self) -> bool {
        self.0[0] == 127
    }
    
    pub fn is_private(&self) -> bool {
        match self.0[0] {
            10 => true,
            172 => self.0[1] >= 16 && self.0[1] <= 31,
            192 => self.0[1] == 168,
            _ => false,
        }
    }
    
    pub fn is_broadcast(&self) -> bool {
        self.0 == [255, 255, 255, 255]
    }
}

impl fmt::Display for Ipv4Address {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}.{}.{}.{}", self.0[0], self.0[1], self.0[2], self.0[3])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SocketAddress {
    pub ip: Ipv4Address,
    pub port: u16,
}

impl SocketAddress {
    pub fn new(ip: Ipv4Address, port: u16) -> Self {
        Self { ip, port }
    }
}

impl fmt::Display for SocketAddress {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:{}", self.ip, self.port)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Icmp = 1,
    Tcp = 6,
    Udp = 17,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketType {
    Raw,
    Tcp,
    Udp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SocketState {
    Closed,
    Listen,
    SynSent,
    SynReceived,
    Established,
    FinWait1,
    FinWait2,
    CloseWait,
    Closing,
    LastAck,
    TimeWait,
}

#[derive(Debug)]
pub struct NetworkInterface {
    pub name: String,
    pub mac_address: MacAddress,
    pub ip_address: Option<Ipv4Address>,
    pub netmask: Option<Ipv4Address>,
    pub gateway: Option<Ipv4Address>,
    pub mtu: u16,
    pub is_up: bool,
    pub rx_packets: u64,
    pub tx_packets: u64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

impl NetworkInterface {
    pub fn new(name: String, mac_address: MacAddress) -> Self {
        Self {
            name,
            mac_address,
            ip_address: None,
            netmask: None,
            gateway: None,
            mtu: 1500,
            is_up: false,
            rx_packets: 0,
            tx_packets: 0,
            rx_bytes: 0,
            tx_bytes: 0,
        }
    }
    
    pub fn set_ip_config(&mut self, ip: Ipv4Address, netmask: Ipv4Address, gateway: Option<Ipv4Address>) {
        self.ip_address = Some(ip);
        self.netmask = Some(netmask);
        self.gateway = gateway;
    }
    
    pub fn bring_up(&mut self) {
        self.is_up = true;
    }
    
    pub fn bring_down(&mut self) {
        self.is_up = false;
    }
}

#[derive(Debug)]
pub struct EthernetFrame {
    pub dest_mac: MacAddress,
    pub src_mac: MacAddress,
    pub ethertype: u16,
    pub payload: Vec<u8>,
}

impl EthernetFrame {
    pub fn new(dest_mac: MacAddress, src_mac: MacAddress, ethertype: u16, payload: Vec<u8>) -> Self {
        Self {
            dest_mac,
            src_mac,
            ethertype,
            payload,
        }
    }
    
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut frame = Vec::with_capacity(14 + self.payload.len());
        
        // Destination MAC
        frame.extend_from_slice(self.dest_mac.bytes());
        // Source MAC
        frame.extend_from_slice(self.src_mac.bytes());
        // EtherType
        frame.extend_from_slice(&self.ethertype.to_be_bytes());
        // Payload
        frame.extend_from_slice(&self.payload);
        
        frame
    }
    
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 14 {
            return None;
        }
        
        let dest_mac = MacAddress::new([
            data[0], data[1], data[2], data[3], data[4], data[5]
        ]);
        let src_mac = MacAddress::new([
            data[6], data[7], data[8], data[9], data[10], data[11]
        ]);
        let ethertype = u16::from_be_bytes([data[12], data[13]]);
        let payload = data[14..].to_vec();
        
        Some(Self::new(dest_mac, src_mac, ethertype, payload))
    }
}

#[derive(Debug)]
pub struct Ipv4Packet {
    pub version: u8,
    pub header_length: u8,
    pub dscp: u8,
    pub ecn: u8,
    pub total_length: u16,
    pub identification: u16,
    pub flags: u8,
    pub fragment_offset: u16,
    pub ttl: u8,
    pub protocol: Protocol,
    pub checksum: u16,
    pub src_ip: Ipv4Address,
    pub dest_ip: Ipv4Address,
    pub options: Vec<u8>,
    pub payload: Vec<u8>,
}

impl Ipv4Packet {
    pub fn new(src_ip: Ipv4Address, dest_ip: Ipv4Address, protocol: Protocol, payload: Vec<u8>) -> Self {
        Self {
            version: 4,
            header_length: 5, // 20 bytes
            dscp: 0,
            ecn: 0,
            total_length: 20 + payload.len() as u16,
            identification: 0,
            flags: 0x02, // Don't fragment
            fragment_offset: 0,
            ttl: 64,
            protocol,
            checksum: 0, // Will be calculated
            src_ip,
            dest_ip,
            options: Vec::new(),
            payload,
        }
    }
    
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut packet = Vec::with_capacity(self.total_length as usize);
        
        // Version and header length
        packet.push((self.version << 4) | self.header_length);
        // DSCP and ECN
        packet.push((self.dscp << 2) | self.ecn);
        // Total length
        packet.extend_from_slice(&self.total_length.to_be_bytes());
        // Identification
        packet.extend_from_slice(&self.identification.to_be_bytes());
        // Flags and fragment offset
        let flags_and_frag = ((self.flags as u16) << 13) | self.fragment_offset;
        packet.extend_from_slice(&flags_and_frag.to_be_bytes());
        // TTL
        packet.push(self.ttl);
        // Protocol
        packet.push(self.protocol as u8);
        // Checksum (placeholder)
        packet.extend_from_slice(&[0, 0]);
        // Source IP
        packet.extend_from_slice(self.src_ip.bytes());
        // Destination IP
        packet.extend_from_slice(self.dest_ip.bytes());
        // Options
        packet.extend_from_slice(&self.options);
        
        // Calculate and insert checksum
        let checksum = Self::calculate_checksum(&packet[0..20]);
        packet[10] = (checksum >> 8) as u8;
        packet[11] = checksum as u8;
        
        // Payload
        packet.extend_from_slice(&self.payload);
        
        packet
    }
    
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < 20 {
            return None;
        }
        
        let version = (data[0] >> 4) & 0x0F;
        let header_length = data[0] & 0x0F;
        let dscp = (data[1] >> 2) & 0x3F;
        let ecn = data[1] & 0x03;
        let total_length = u16::from_be_bytes([data[2], data[3]]);
        let identification = u16::from_be_bytes([data[4], data[5]]);
        let flags_and_frag = u16::from_be_bytes([data[6], data[7]]);
        let flags = ((flags_and_frag >> 13) & 0x07) as u8;
        let fragment_offset = flags_and_frag & 0x1FFF;
        let ttl = data[8];
        let protocol = match data[9] {
            1 => Protocol::Icmp,
            6 => Protocol::Tcp,
            17 => Protocol::Udp,
            _ => return None,
        };
        let checksum = u16::from_be_bytes([data[10], data[11]]);
        let src_ip = Ipv4Address::from_bytes([data[12], data[13], data[14], data[15]]);
        let dest_ip = Ipv4Address::from_bytes([data[16], data[17], data[18], data[19]]);
        
        let header_len = (header_length as usize) * 4;
        let options = if header_len > 20 {
            data[20..header_len].to_vec()
        } else {
            Vec::new()
        };
        
        let payload = if data.len() > header_len {
            data[header_len..].to_vec()
        } else {
            Vec::new()
        };
        
        Some(Self {
            version,
            header_length,
            dscp,
            ecn,
            total_length,
            identification,
            flags,
            fragment_offset,
            ttl,
            protocol,
            checksum,
            src_ip,
            dest_ip,
            options,
            payload,
        })
    }
    
    fn calculate_checksum(header: &[u8]) -> u16 {
        let mut sum = 0u32;
        
        for chunk in header.chunks(2) {
            if chunk.len() == 2 {
                sum += u16::from_be_bytes([chunk[0], chunk[1]]) as u32;
            } else {
                sum += (chunk[0] as u32) << 8;
            }
        }
        
        while (sum >> 16) != 0 {
            sum = (sum & 0xFFFF) + (sum >> 16);
        }
        
        !sum as u16
    }
}

#[derive(Debug)]
pub struct TcpSegment {
    pub src_port: u16,
    pub dest_port: u16,
    pub sequence_number: u32,
    pub acknowledgment_number: u32,
    pub header_length: u8,
    pub flags: u8,
    pub window_size: u16,
    pub checksum: u16,
    pub urgent_pointer: u16,
    pub options: Vec<u8>,
    pub payload: Vec<u8>,
}

impl TcpSegment {
    pub const FLAG_FIN: u8 = 0x01;
    pub const FLAG_SYN: u8 = 0x02;
    pub const FLAG_RST: u8 = 0x04;
    pub const FLAG_PSH: u8 = 0x08;
    pub const FLAG_ACK: u8 = 0x10;
    pub const FLAG_URG: u8 = 0x20;
    
    pub fn new(src_port: u16, dest_port: u16, seq: u32, ack: u32, flags: u8, window: u16, payload: Vec<u8>) -> Self {
        Self {
            src_port,
            dest_port,
            sequence_number: seq,
            acknowledgment_number: ack,
            header_length: 5, // 20 bytes
            flags,
            window_size: window,
            checksum: 0,
            urgent_pointer: 0,
            options: Vec::new(),
            payload,
        }
    }
    
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut segment = Vec::with_capacity(20 + self.payload.len());
        
        // Source port
        segment.extend_from_slice(&self.src_port.to_be_bytes());
        // Destination port
        segment.extend_from_slice(&self.dest_port.to_be_bytes());
        // Sequence number
        segment.extend_from_slice(&self.sequence_number.to_be_bytes());
        // Acknowledgment number
        segment.extend_from_slice(&self.acknowledgment_number.to_be_bytes());
        // Header length and flags
        segment.push((self.header_length << 4) | 0); // Reserved bits
        segment.push(self.flags);
        // Window size
        segment.extend_from_slice(&self.window_size.to_be_bytes());
        // Checksum (placeholder)
        segment.extend_from_slice(&[0, 0]);
        // Urgent pointer
        segment.extend_from_slice(&self.urgent_pointer.to_be_bytes());
        // Options
        segment.extend_from_slice(&self.options);
        // Payload
        segment.extend_from_slice(&self.payload);
        
        segment
    }
}

#[derive(Debug)]
pub struct UdpDatagram {
    pub src_port: u16,
    pub dest_port: u16,
    pub length: u16,
    pub checksum: u16,
    pub payload: Vec<u8>,
}

impl UdpDatagram {
    pub fn new(src_port: u16, dest_port: u16, payload: Vec<u8>) -> Self {
        Self {
            src_port,
            dest_port,
            length: 8 + payload.len() as u16,
            checksum: 0,
            payload,
        }
    }
    
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut datagram = Vec::with_capacity(8 + self.payload.len());
        
        // Source port
        datagram.extend_from_slice(&self.src_port.to_be_bytes());
        // Destination port
        datagram.extend_from_slice(&self.dest_port.to_be_bytes());
        // Length
        datagram.extend_from_slice(&self.length.to_be_bytes());
        // Checksum
        datagram.extend_from_slice(&self.checksum.to_be_bytes());
        // Payload
        datagram.extend_from_slice(&self.payload);
        
        datagram
    }
}

#[derive(Debug)]
pub struct Socket {
    pub id: u32,
    pub socket_type: SocketType,
    pub state: SocketState,
    pub local_address: Option<SocketAddress>,
    pub remote_address: Option<SocketAddress>,
    pub rx_buffer: VecDeque<Vec<u8>>,
    pub tx_buffer: VecDeque<Vec<u8>>,
    pub sequence_number: u32,
    pub acknowledgment_number: u32,
    pub window_size: u16,
}

impl Socket {
    pub fn new(id: u32, socket_type: SocketType) -> Self {
        Self {
            id,
            socket_type,
            state: SocketState::Closed,
            local_address: None,
            remote_address: None,
            rx_buffer: VecDeque::new(),
            tx_buffer: VecDeque::new(),
            sequence_number: 0,
            acknowledgment_number: 0,
            window_size: 8192,
        }
    }
    
    pub fn bind(&mut self, address: SocketAddress) -> Result<(), NetworkError> {
        if self.local_address.is_some() {
            return Err(NetworkError::AlreadyBound);
        }
        self.local_address = Some(address);
        Ok(())
    }
    
    pub fn connect(&mut self, address: SocketAddress) -> Result<(), NetworkError> {
        if self.socket_type != SocketType::Tcp {
            return Err(NetworkError::InvalidOperation);
        }
        
        self.remote_address = Some(address);
        self.state = SocketState::SynSent;
        
        // TODO: Send SYN packet
        
        Ok(())
    }
    
    pub fn listen(&mut self) -> Result<(), NetworkError> {
        if self.socket_type != SocketType::Tcp {
            return Err(NetworkError::InvalidOperation);
        }
        
        if self.local_address.is_none() {
            return Err(NetworkError::NotBound);
        }
        
        self.state = SocketState::Listen;
        Ok(())
    }
    
    pub fn send(&mut self, data: Vec<u8>) -> Result<usize, NetworkError> {
        if self.socket_type == SocketType::Tcp && self.state != SocketState::Established {
            return Err(NetworkError::NotConnected);
        }
        
        let len = data.len();
        self.tx_buffer.push_back(data);
        Ok(len)
    }
    
    pub fn receive(&mut self) -> Option<Vec<u8>> {
        self.rx_buffer.pop_front()
    }
}

#[derive(Debug)]
pub enum NetworkError {
    InterfaceNotFound,
    InvalidAddress,
    AlreadyBound,
    NotBound,
    NotConnected,
    InvalidOperation,
    BufferFull,
    Timeout,
    HardwareError,
    ChecksumError,
    FragmentationError,
}

impl fmt::Display for NetworkError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            NetworkError::InterfaceNotFound => write!(f, "Network interface not found"),
            NetworkError::InvalidAddress => write!(f, "Invalid address"),
            NetworkError::AlreadyBound => write!(f, "Socket already bound"),
            NetworkError::NotBound => write!(f, "Socket not bound"),
            NetworkError::NotConnected => write!(f, "Socket not connected"),
            NetworkError::InvalidOperation => write!(f, "Invalid operation"),
            NetworkError::BufferFull => write!(f, "Buffer full"),
            NetworkError::Timeout => write!(f, "Operation timeout"),
            NetworkError::HardwareError => write!(f, "Hardware error"),
            NetworkError::ChecksumError => write!(f, "Checksum error"),
            NetworkError::FragmentationError => write!(f, "Fragmentation error"),
        }
    }
}

pub type NetworkResult<T> = Result<T, NetworkError>;

#[derive(Debug)]
pub struct ArpTable {
    entries: BTreeMap<Ipv4Address, MacAddress>,
}

impl ArpTable {
    pub fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
        }
    }
    
    pub fn insert(&mut self, ip: Ipv4Address, mac: MacAddress) {
        self.entries.insert(ip, mac);
    }
    
    pub fn lookup(&self, ip: Ipv4Address) -> Option<MacAddress> {
        self.entries.get(&ip).copied()
    }
    
    pub fn remove(&mut self, ip: Ipv4Address) -> Option<MacAddress> {
        self.entries.remove(&ip)
    }
}

#[derive(Debug)]
pub struct RoutingTable {
    routes: Vec<Route>,
}

#[derive(Debug, Clone)]
pub struct Route {
    pub destination: Ipv4Address,
    pub netmask: Ipv4Address,
    pub gateway: Option<Ipv4Address>,
    pub interface: String,
    pub metric: u32,
}

impl RoutingTable {
    pub fn new() -> Self {
        Self {
            routes: Vec::new(),
        }
    }
    
    pub fn add_route(&mut self, route: Route) {
        self.routes.push(route);
        // Sort by metric (lower is better)
        self.routes.sort_by_key(|r| r.metric);
    }
    
    pub fn lookup(&self, destination: Ipv4Address) -> Option<&Route> {
        for route in &self.routes {
            if self.matches_route(&route, destination) {
                return Some(route);
            }
        }
        None
    }
    
    fn matches_route(&self, route: &Route, destination: Ipv4Address) -> bool {
        let dest_u32 = destination.as_u32();
        let route_u32 = route.destination.as_u32();
        let mask_u32 = route.netmask.as_u32();
        
        (dest_u32 & mask_u32) == (route_u32 & mask_u32)
    }
}

#[derive(Debug)]
pub struct NetworkStack {
    interfaces: BTreeMap<String, NetworkInterface>,
    sockets: BTreeMap<u32, Socket>,
    next_socket_id: u32,
    arp_table: ArpTable,
    routing_table: RoutingTable,
    packet_queue: VecDeque<(String, Vec<u8>)>, // (interface_name, packet_data)
}

impl NetworkStack {
    pub const fn new() -> Self {
        Self {
            interfaces: BTreeMap::new(),
            sockets: BTreeMap::new(),
            next_socket_id: 1,
            arp_table: ArpTable::new(),
            routing_table: RoutingTable::new(),
            packet_queue: VecDeque::new(),
        }
    }
    
    pub fn add_interface(&mut self, interface: NetworkInterface) {
        self.interfaces.insert(interface.name.clone(), interface);
    }
    
    pub fn get_interface(&self, name: &str) -> Option<&NetworkInterface> {
        self.interfaces.get(name)
    }
    
    pub fn get_interface_mut(&mut self, name: &str) -> Option<&mut NetworkInterface> {
        self.interfaces.get_mut(name)
    }
    
    pub fn create_socket(&mut self, socket_type: SocketType) -> u32 {
        let id = self.next_socket_id;
        self.next_socket_id += 1;
        
        let socket = Socket::new(id, socket_type);
        self.sockets.insert(id, socket);
        
        id
    }
    
    pub fn get_socket(&self, id: u32) -> Option<&Socket> {
        self.sockets.get(&id)
    }
    
    pub fn get_socket_mut(&mut self, id: u32) -> Option<&mut Socket> {
        self.sockets.get_mut(&id)
    }
    
    pub fn close_socket(&mut self, id: u32) -> NetworkResult<()> {
        if let Some(mut socket) = self.sockets.remove(&id) {
            if socket.socket_type == SocketType::Tcp && socket.state == SocketState::Established {
                // Send FIN packet
                socket.state = SocketState::FinWait1;
                // TODO: Actually send FIN
            }
            Ok(())
        } else {
            Err(NetworkError::InvalidOperation)
        }
    }
    
    pub fn send_packet(&mut self, interface_name: &str, dest_ip: Ipv4Address, protocol: Protocol, data: Vec<u8>) -> NetworkResult<()> {
        let interface = self.interfaces.get(interface_name)
            .ok_or(NetworkError::InterfaceNotFound)?;
        
        if !interface.is_up {
            return Err(NetworkError::InvalidOperation);
        }
        
        let src_ip = interface.ip_address.ok_or(NetworkError::InvalidAddress)?;
        
        // Create IP packet
        let ip_packet = Ipv4Packet::new(src_ip, dest_ip, protocol, data);
        let ip_data = ip_packet.to_bytes();
        
        // Look up MAC address
        let dest_mac = if let Some(mac) = self.arp_table.lookup(dest_ip) {
            mac
        } else {
            // Send ARP request
            self.send_arp_request(interface_name, dest_ip)?;
            return Err(NetworkError::Timeout); // Would normally wait for ARP response
        };
        
        // Create Ethernet frame
        let eth_frame = EthernetFrame::new(
            dest_mac,
            interface.mac_address,
            0x0800, // IPv4
            ip_data
        );
        
        // Queue packet for transmission
        self.packet_queue.push_back((interface_name.to_string(), eth_frame.to_bytes()));
        
        Ok(())
    }
    
    pub fn receive_packet(&mut self, interface_name: &str, data: Vec<u8>) -> NetworkResult<()> {
        let interface = self.interfaces.get_mut(interface_name)
            .ok_or(NetworkError::InterfaceNotFound)?;
        
        interface.rx_packets += 1;
        interface.rx_bytes += data.len() as u64;
        
        // Parse Ethernet frame
        let eth_frame = EthernetFrame::from_bytes(&data)
            .ok_or(NetworkError::InvalidOperation)?;
        
        match eth_frame.ethertype {
            0x0800 => self.handle_ipv4_packet(&eth_frame.payload)?,
            0x0806 => self.handle_arp_packet(&eth_frame.payload)?,
            _ => {}, // Unknown protocol
        }
        
        Ok(())
    }
    
    fn handle_ipv4_packet(&mut self, data: &[u8]) -> NetworkResult<()> {
        let ip_packet = Ipv4Packet::from_bytes(data)
            .ok_or(NetworkError::InvalidOperation)?;
        
        match ip_packet.protocol {
            Protocol::Icmp => self.handle_icmp_packet(&ip_packet)?,
            Protocol::Tcp => self.handle_tcp_packet(&ip_packet)?,
            Protocol::Udp => self.handle_udp_packet(&ip_packet)?,
        }
        
        Ok(())
    }
    
    fn handle_icmp_packet(&mut self, ip_packet: &Ipv4Packet) -> NetworkResult<()> {
        // Basic ICMP echo reply
        if !ip_packet.payload.is_empty() && ip_packet.payload[0] == 8 { // Echo request
            let mut reply_payload = ip_packet.payload.clone();
            reply_payload[0] = 0; // Echo reply
            
            // TODO: Send ICMP reply
        }
        Ok(())
    }
    
    fn handle_tcp_packet(&mut self, ip_packet: &Ipv4Packet) -> NetworkResult<()> {
        // TODO: Parse TCP segment and handle connection state
        Ok(())
    }
    
    fn handle_udp_packet(&mut self, ip_packet: &Ipv4Packet) -> NetworkResult<()> {
        // TODO: Parse UDP datagram and deliver to socket
        Ok(())
    }
    
    fn handle_arp_packet(&mut self, data: &[u8]) -> NetworkResult<()> {
        if data.len() < 28 {
            return Err(NetworkError::InvalidOperation);
        }
        
        let operation = u16::from_be_bytes([data[6], data[7]]);
        
        if operation == 1 { // ARP request
            let sender_ip = Ipv4Address::from_bytes([data[14], data[15], data[16], data[17]]);
            let sender_mac = MacAddress::new([data[8], data[9], data[10], data[11], data[12], data[13]]);
            
            // Add to ARP table
            self.arp_table.insert(sender_ip, sender_mac);
            
            // TODO: Send ARP reply if target IP matches our interface
        } else if operation == 2 { // ARP reply
            let sender_ip = Ipv4Address::from_bytes([data[14], data[15], data[16], data[17]]);
            let sender_mac = MacAddress::new([data[8], data[9], data[10], data[11], data[12], data[13]]);
            
            // Add to ARP table
            self.arp_table.insert(sender_ip, sender_mac);
        }
        
        Ok(())
    }
    
    fn send_arp_request(&mut self, interface_name: &str, target_ip: Ipv4Address) -> NetworkResult<()> {
        let interface = self.interfaces.get(interface_name)
            .ok_or(NetworkError::InterfaceNotFound)?;
        
        let src_ip = interface.ip_address.ok_or(NetworkError::InvalidAddress)?;
        
        // Create ARP request packet
        let mut arp_packet = Vec::with_capacity(28);
        
        // Hardware type (Ethernet)
        arp_packet.extend_from_slice(&[0x00, 0x01]);
        // Protocol type (IPv4)
        arp_packet.extend_from_slice(&[0x08, 0x00]);
        // Hardware address length
        arp_packet.push(6);
        // Protocol address length
        arp_packet.push(4);
        // Operation (request)
        arp_packet.extend_from_slice(&[0x00, 0x01]);
        // Sender hardware address
        arp_packet.extend_from_slice(interface.mac_address.bytes());
        // Sender protocol address
        arp_packet.extend_from_slice(src_ip.bytes());
        // Target hardware address (unknown)
        arp_packet.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        // Target protocol address
        arp_packet.extend_from_slice(target_ip.bytes());
        
        // Create Ethernet frame
        let eth_frame = EthernetFrame::new(
            MacAddress::new([0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]), // Broadcast
            interface.mac_address,
            0x0806, // ARP
            arp_packet
        );
        
        // Queue packet for transmission
        self.packet_queue.push_back((interface_name.to_string(), eth_frame.to_bytes()));
        
        Ok(())
    }
    
    pub fn process_packets(&mut self) {
        // Process queued packets
        while let Some((interface_name, packet_data)) = self.packet_queue.pop_front() {
            if let Some(interface) = self.interfaces.get_mut(&interface_name) {
                interface.tx_packets += 1;
                interface.tx_bytes += packet_data.len() as u64;
                
                // TODO: Actually transmit packet via hardware driver
            }
        }
    }
}

// Public API functions
pub fn init() {
    let mut stack = NETWORK_STACK.write();
    
    // Create loopback interface
    let mut loopback = NetworkInterface::new(
        "lo".to_string(),
        MacAddress::new([0x00, 0x00, 0x00, 0x00, 0x00, 0x00])
    );
    loopback.set_ip_config(
        Ipv4Address::new(127, 0, 0, 1),
        Ipv4Address::new(255, 0, 0, 0),
        None
    );
    loopback.bring_up();
    stack.add_interface(loopback);
    
    // Add default route
    let default_route = Route {
        destination: Ipv4Address::new(0, 0, 0, 0),
        netmask: Ipv4Address::new(0, 0, 0, 0),
        gateway: Some(Ipv4Address::new(192, 168, 1, 1)),
        interface: "eth0".to_string(),
        metric: 100,
    };
    stack.routing_table.add_route(default_route);
}

pub fn add_interface(name: String, mac_address: MacAddress) -> NetworkResult<()> {
    let mut stack = NETWORK_STACK.write();
    let interface = NetworkInterface::new(name, mac_address);
    stack.add_interface(interface);
    Ok(())
}

pub fn configure_interface(name: &str, ip: Ipv4Address, netmask: Ipv4Address, gateway: Option<Ipv4Address>) -> NetworkResult<()> {
    let mut stack = NETWORK_STACK.write();
    let interface = stack.get_interface_mut(name)
        .ok_or(NetworkError::InterfaceNotFound)?;
    
    interface.set_ip_config(ip, netmask, gateway);
    interface.bring_up();
    Ok(())
}

pub fn create_socket(socket_type: SocketType) -> u32 {
    NETWORK_STACK.write().create_socket(socket_type)
}

pub fn bind_socket(socket_id: u32, address: SocketAddress) -> NetworkResult<()> {
    let mut stack = NETWORK_STACK.write();
    let socket = stack.get_socket_mut(socket_id)
        .ok_or(NetworkError::InvalidOperation)?;
    socket.bind(address)
}

pub fn connect_socket(socket_id: u32, address: SocketAddress) -> NetworkResult<()> {
    let mut stack = NETWORK_STACK.write();
    let socket = stack.get_socket_mut(socket_id)
        .ok_or(NetworkError::InvalidOperation)?;
    socket.connect(address)
}

pub fn listen_socket(socket_id: u32) -> NetworkResult<()> {
    let mut stack = NETWORK_STACK.write();
    let socket = stack.get_socket_mut(socket_id)
        .ok_or(NetworkError::InvalidOperation)?;
    socket.listen()
}

pub fn send_data(socket_id: u32, data: Vec<u8>) -> NetworkResult<usize> {
    let mut stack = NETWORK_STACK.write();
    let socket = stack.get_socket_mut(socket_id)
        .ok_or(NetworkError::InvalidOperation)?;
    socket.send(data)
}

pub fn receive_data(socket_id: u32) -> Option<Vec<u8>> {
    let mut stack = NETWORK_STACK.write();
    let socket = stack.get_socket_mut(socket_id)?;
    socket.receive()
}

pub fn close_socket(socket_id: u32) -> NetworkResult<()> {
    NETWORK_STACK.write().close_socket(socket_id)
}

pub fn get_interface_list() -> Vec<String> {
    NETWORK_STACK.read().interfaces.keys().cloned().collect()
}

pub fn get_interface_info(name: &str) -> Option<(Ipv4Address, MacAddress, bool)> {
    let stack = NETWORK_STACK.read();
    let interface = stack.get_interface(name)?;
    Some((interface.ip_address?, interface.mac_address, interface.is_up))
}

pub fn ping(target: Ipv4Address) -> NetworkResult<u32> {
    // TODO: Implement ICMP ping
    Ok(0) // Return RTT in milliseconds
}

pub fn process_network_packets() {
    NETWORK_STACK.write().process_packets();
}